mod about;
mod cleaners;
mod engine;
mod error;
mod intake;
mod models;
mod safe_io;
mod shell_integration;

use error::bounded_message;
use models::{
    BoundedAuditPath, BoundedBatchId, BoundedPaths, BoundedReportContents, BoundedUpdateVersion,
    CleanRequest, CleanResult, ScanReport,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{menu::MenuItem, tray::TrayIconBuilder, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::StateFlags;

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);
static ACTIVE_READ_TASKS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_CLEAN_TASKS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_SCAN_BATCHES: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
static ACTIVE_CLEAN_BATCHES: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
const PORTABLE_MARKER: &str = "metaclean-portable.marker";
const MAX_BATCH_FILES: usize = 10_000;
const MAX_BATCH_ID_BYTES: usize = 128;
pub(crate) const MAX_PATH_BYTES: usize = 32 * 1024;
pub(crate) const MAX_BATCH_PATH_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_REPORT_BYTES: usize = 10 * 1024 * 1024;
const MAX_UPDATE_VERSION_BYTES: usize = 128;
const UPDATE_NETWORK_HELP: &str = "无法连接已签名更新源。请检查 GitHub 网络或 HTTPS_PROXY 后重试，也可从正式发布页手动下载安装包。 / Could not reach the signed update feed. Check GitHub access or HTTPS_PROXY, then retry, or download the installer from the Releases page.";
const UPDATE_CHANGED: &str = "可用版本在确认后发生了变化，请先重新检查并查看新版本说明。 / The available release changed after confirmation. Check again and review the new release before installing.";
const UPDATE_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const CLEANUP_CLOSE_BLOCKED: &str = "任务正在进行，请等待完成；清理任务可以先取消。 / Work is still in progress; wait for it to finish, or cancel the cleanup first.";
const PROGRESS_EVENT_BATCH: usize = 16;
const PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq, Eq)]
enum CloseAction {
    Exit,
    HideToTray,
}

fn close_action(close_to_tray: bool) -> CloseAction {
    if close_to_tray {
        CloseAction::HideToTray
    } else {
        CloseAction::Exit
    }
}

fn updater_network_error(action: &str, error: impl std::fmt::Display) -> String {
    bounded_message(format!(
        "{action}。{UPDATE_NETWORK_HELP}\n技术详情 / Technical detail: {error}"
    ))
}

fn reviewed_update_matches(available: &str, expected: &str) -> bool {
    fn stable_version(value: &str) -> Option<&str> {
        if value.len() > MAX_UPDATE_VERSION_BYTES {
            return None;
        }
        let trimmed = value.trim();
        let value = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);
        let mut parts = value.split('.');
        let valid = parts.clone().count() == 3
            && parts.all(|part| {
                !part.is_empty()
                    && (part == "0" || !part.starts_with('0'))
                    && part
                        .parse::<u64>()
                        .is_ok_and(|number| number <= 9_007_199_254_740_991)
            });
        valid.then_some(value)
    }

    match (stable_version(available), stable_version(expected)) {
        (Some(available), Some(expected)) => available == expected,
        _ => false,
    }
}

fn validate_batch_size(count: usize) -> Result<(), String> {
    if count > MAX_BATCH_FILES {
        return Err(format!(
            "单次任务最多处理 {MAX_BATCH_FILES} 个输入，当前收到 {count} 个"
        ));
    }
    Ok(())
}

fn validate_batch_id(batch_id: &str) -> Result<(), String> {
    if batch_id.len() > MAX_BATCH_ID_BYTES {
        return Err(format!("批次标识最多 {MAX_BATCH_ID_BYTES} 字节"));
    }
    Ok(())
}

fn deduplicate_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(paths.len());
    paths
        .into_iter()
        .filter(|path| seen.insert(intake::path_identity(path)))
        .collect()
}

fn validate_path_input(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("输入路径不能为空".into());
    }
    if path.len() > MAX_PATH_BYTES {
        return Err(format!("单个输入路径最多 {MAX_PATH_BYTES} 字节"));
    }
    Ok(())
}

fn validate_path_inputs(paths: &[String]) -> Result<(), String> {
    let mut total = 0usize;
    for path in paths {
        validate_path_input(path)?;
        total = total
            .checked_add(path.len())
            .ok_or_else(|| format!("输入路径总量超过 {MAX_BATCH_PATH_BYTES} 字节"))?;
    }
    if total > MAX_BATCH_PATH_BYTES {
        return Err(format!("单次输入路径总量最多 {MAX_BATCH_PATH_BYTES} 字节"));
    }
    Ok(())
}

fn prepare_batch_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    validate_path_inputs(&paths)?;
    let paths = deduplicate_paths(paths);
    validate_batch_size(paths.len())?;
    Ok(paths)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRuntime {
    self_update_supported: bool,
    portable: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDownloadProgress {
    stage: &'static str,
    downloaded: u64,
    total: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchProgress {
    operation: &'static str,
    batch_id: String,
    completed: usize,
    total: usize,
    failed: usize,
    cancelled: bool,
}

struct BatchProgressState {
    completed: usize,
    failed: usize,
    last_emitted: usize,
    last_emitted_at: Instant,
}

impl BatchProgressState {
    fn new() -> Self {
        Self {
            completed: 0,
            failed: 0,
            last_emitted: 0,
            last_emitted_at: Instant::now(),
        }
    }

    fn record(
        &mut self,
        operation: &'static str,
        batch_id: &str,
        total: usize,
        failed: bool,
    ) -> Option<BatchProgress> {
        self.completed += 1;
        self.failed += usize::from(failed);
        let now = Instant::now();
        let due = self.completed == total
            || self.completed.saturating_sub(self.last_emitted) >= PROGRESS_EVENT_BATCH
            || now.duration_since(self.last_emitted_at) >= PROGRESS_EVENT_INTERVAL;
        if !due {
            return None;
        }
        self.last_emitted = self.completed;
        self.last_emitted_at = now;
        Some(BatchProgress {
            operation,
            batch_id: batch_id.to_owned(),
            completed: self.completed,
            total,
            failed: self.failed,
            cancelled: false,
        })
    }

    fn final_event(
        &mut self,
        operation: &'static str,
        batch_id: &str,
        total: usize,
        cancelled: bool,
    ) -> Option<BatchProgress> {
        if !cancelled && (self.completed == 0 || self.completed == self.last_emitted) {
            return None;
        }
        self.last_emitted = self.completed;
        self.last_emitted_at = Instant::now();
        Some(BatchProgress {
            operation,
            batch_id: batch_id.to_owned(),
            completed: self.completed,
            total,
            failed: self.failed,
            cancelled,
        })
    }
}

fn active_clean_batches() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    ACTIVE_CLEAN_BATCHES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_scan_batches() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    ACTIVE_SCAN_BATCHES.get_or_init(|| Mutex::new(HashMap::new()))
}

struct ActiveScanBatchGuard {
    batch_id: Option<String>,
}

impl ActiveScanBatchGuard {
    fn register(batch_id: Option<&str>, cancellation: Arc<AtomicBool>) -> Result<Self, String> {
        let Some(batch_id) = batch_id.filter(|value| !value.is_empty()) else {
            return Ok(Self { batch_id: None });
        };
        let mut active = active_scan_batches()
            .lock()
            .map_err(|_| "扫描任务状态不可用，请重试".to_owned())?;
        if active.contains_key(batch_id) {
            return Err("扫描批次标识已在使用，请重试".into());
        }
        active.insert(batch_id.to_owned(), cancellation);
        Ok(Self {
            batch_id: Some(batch_id.to_owned()),
        })
    }
}

impl Drop for ActiveScanBatchGuard {
    fn drop(&mut self) {
        if let Some(batch_id) = self.batch_id.as_deref() {
            if let Ok(mut active) = active_scan_batches().lock() {
                active.remove(batch_id);
            }
        }
    }
}

fn clean_batch_active() -> bool {
    if ACTIVE_CLEAN_TASKS.load(Ordering::SeqCst) > 0 {
        return true;
    }
    // A poisoned registry is treated as active: closing while cancellation
    // state is unavailable is less safe than keeping the application open.
    active_clean_batches()
        .lock()
        .map(|active| !active.is_empty())
        .unwrap_or(true)
}

fn read_task_active() -> bool {
    ACTIVE_READ_TASKS.load(Ordering::SeqCst) > 0
}

fn work_active() -> bool {
    read_task_active() || clean_batch_active()
}

struct ActiveReadGuard;

impl ActiveReadGuard {
    fn start() -> Self {
        ACTIVE_READ_TASKS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for ActiveReadGuard {
    fn drop(&mut self) {
        ACTIVE_READ_TASKS.fetch_sub(1, Ordering::SeqCst);
    }
}

struct ActiveCleanBatchGuard {
    batch_id: Option<String>,
}

impl ActiveCleanBatchGuard {
    fn register(batch_id: &str, cancellation: Arc<AtomicBool>) -> Result<Self, String> {
        if batch_id.is_empty() {
            ACTIVE_CLEAN_TASKS.fetch_add(1, Ordering::SeqCst);
            return Ok(Self { batch_id: None });
        }
        let mut active = active_clean_batches()
            .lock()
            .map_err(|_| "清理任务状态不可用，请重试".to_owned())?;
        if active.contains_key(batch_id) {
            return Err("清理批次标识已在使用，请重试".into());
        }
        active.insert(batch_id.to_owned(), cancellation);
        ACTIVE_CLEAN_TASKS.fetch_add(1, Ordering::SeqCst);
        Ok(Self {
            batch_id: Some(batch_id.to_owned()),
        })
    }
}

impl Drop for ActiveCleanBatchGuard {
    fn drop(&mut self) {
        if let Some(batch_id) = self.batch_id.as_deref() {
            if let Ok(mut active) = active_clean_batches().lock() {
                active.remove(batch_id);
            }
        }
        ACTIVE_CLEAN_TASKS.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tauri::command]
async fn scan_files(
    app: tauri::AppHandle,
    paths: BoundedPaths,
    batch_id: Option<BoundedBatchId>,
) -> Result<Vec<ScanReport>, String> {
    if let Some(batch_id) = batch_id.as_ref() {
        validate_batch_id(batch_id.as_str())?;
    }
    let paths = prepare_batch_paths(paths.into_inner())?;
    let cancellation = Arc::new(AtomicBool::new(false));
    let active_batch = ActiveScanBatchGuard::register(
        batch_id.as_ref().map(BoundedBatchId::as_str),
        cancellation.clone(),
    )?;
    let active_read = ActiveReadGuard::start();
    let progress_batch_id = batch_id.map(BoundedBatchId::into_inner).unwrap_or_default();
    let progress_total = paths.len();
    let progress_state = Mutex::new(BatchProgressState::new());
    tauri::async_runtime::spawn_blocking(move || {
        let _active_batch = active_batch;
        let _active_read = active_read;
        let on_progress = |report: &ScanReport| {
            if progress_batch_id.is_empty() {
                return;
            }
            let Ok(mut progress) = progress_state.lock() else {
                return;
            };
            let event = progress.record(
                "scan",
                &progress_batch_id,
                progress_total,
                report.error.is_some(),
            );
            drop(progress);
            if let Some(event) = event {
                let _ = app.emit("batch-progress", event);
            }
        };
        let reports =
            engine::scan_paths_cancellable_with_progress(&paths, &cancellation, &on_progress);
        if !progress_batch_id.is_empty() {
            if let Ok(mut progress) = progress_state.lock() {
                let event = progress.final_event(
                    "scan",
                    &progress_batch_id,
                    progress_total,
                    cancellation.load(Ordering::SeqCst),
                );
                drop(progress);
                if let Some(event) = event {
                    let _ = app.emit("batch-progress", event);
                }
            }
        }
        reports
    })
    .await
    .map_err(|error| bounded_message(format!("扫描任务异常结束：{error}")))
}

#[tauri::command]
async fn expand_paths(paths: BoundedPaths) -> Result<intake::IntakeResult, String> {
    let paths = prepare_batch_paths(paths.into_inner())?;
    let active_read = ActiveReadGuard::start();
    tauri::async_runtime::spawn_blocking(move || {
        let _active_read = active_read;
        intake::expand_paths(&paths)
    })
    .await
    .map_err(|error| bounded_message(format!("目录导入任务异常结束：{error}")))
}

#[tauri::command]
async fn clean_files(
    app: tauri::AppHandle,
    request: CleanRequest,
) -> Result<Vec<CleanResult>, String> {
    validate_batch_id(request.batch_id.as_str())?;
    let paths = prepare_batch_paths(request.paths.into_inner())?;
    let total = paths.len();
    let batch_id = request.batch_id.into_inner();
    let cancellation = Arc::new(AtomicBool::new(false));
    let active_batch = ActiveCleanBatchGuard::register(&batch_id, cancellation.clone())?;
    let progress_batch_id = batch_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _active_batch = active_batch;
        let mut progress = BatchProgressState::new();
        let mut results = Vec::with_capacity(total);
        for path in &paths {
            if cancellation.load(Ordering::SeqCst) {
                break;
            }
            let result = engine::clean_file_isolated_with_options(
                std::path::Path::new(path),
                &request.mode,
                request.preserve_timestamps,
                request.preserve_orientation,
                request.preserve_color_profile,
                request.remove_extended_attributes,
            );
            let event = progress.record("clean", &progress_batch_id, total, !result.success);
            results.push(result);
            if !progress_batch_id.is_empty() {
                if let Some(event) = event {
                    let _ = app.emit("batch-progress", event);
                }
            }
        }
        if !progress_batch_id.is_empty() {
            if let Some(event) = progress.final_event(
                "clean",
                &progress_batch_id,
                total,
                cancellation.load(Ordering::SeqCst),
            ) {
                let _ = app.emit("batch-progress", event);
            }
        }
        results
    })
    .await
    .map_err(|error| bounded_message(format!("清理任务异常结束：{error}")));
    result
}

#[tauri::command]
fn cancel_clean_batch(batch_id: BoundedBatchId) -> Result<bool, String> {
    validate_batch_id(batch_id.as_str())?;
    if batch_id.is_empty() {
        return Ok(false);
    }
    let active = active_clean_batches()
        .lock()
        .map_err(|_| "清理任务状态不可用，请重试".to_owned())?;
    if let Some(flag) = active.get(batch_id.as_str()) {
        flag.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
fn cancel_scan_batch(batch_id: BoundedBatchId) -> Result<bool, String> {
    validate_batch_id(batch_id.as_str())?;
    if batch_id.is_empty() {
        return Ok(false);
    }
    let active = active_scan_batches()
        .lock()
        .map_err(|_| "扫描任务状态不可用，请重试".to_owned())?;
    if let Some(flag) = active.get(batch_id.as_str()) {
        flag.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
fn export_audit_report(
    path: BoundedAuditPath,
    contents: BoundedReportContents,
) -> Result<(), String> {
    validate_path_input(path.as_str())?;
    export_audit_report_to(std::path::Path::new(path.as_str()), contents.as_str())
}

fn export_audit_report_to(destination: &std::path::Path, contents: &str) -> Result<(), String> {
    if contents.len() > MAX_REPORT_BYTES {
        return Err("审计报告超过 10 MiB 上限".into());
    }
    if destination
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("json"))
    {
        return Err("审计报告必须使用 .json 扩展名".into());
    }
    safe_io::atomic_write_with_metadata(destination, contents.as_bytes(), None, false, false)
        .map_err(|error| bounded_message(format!("导出审计报告失败：{error}")))
}

fn bounded_launch_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    prepare_batch_paths(paths)
}

#[tauri::command]
fn get_launch_paths() -> Result<Vec<String>, String> {
    bounded_launch_paths(shell_integration::launch_paths())
}

#[tauri::command]
fn get_context_menu_status() -> shell_integration::ContextMenuStatus {
    shell_integration::status()
}

#[tauri::command]
fn set_context_menu_enabled(enabled: bool) -> Result<shell_integration::ContextMenuStatus, String> {
    if enabled {
        shell_integration::install()
    } else {
        shell_integration::remove()
    }
    .map_err(|error| bounded_message(format!("更新 Windows 右键菜单失败：{error}")))
}

#[tauri::command]
fn set_close_to_tray(enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::SeqCst);
}

fn portable_marker_exists(executable: &std::path::Path) -> bool {
    executable
        .parent()
        .is_some_and(|directory| directory.join(PORTABLE_MARKER).is_file())
}

fn self_update_supported_for(portable: bool, linux: bool, app_image: bool) -> bool {
    !portable && (!linux || app_image)
}

fn detect_update_runtime() -> UpdateRuntime {
    let portable = std::env::current_exe()
        .ok()
        .is_some_and(|executable| portable_marker_exists(&executable));
    let linux = cfg!(target_os = "linux");
    let app_image = std::env::var_os("APPIMAGE").is_some();
    UpdateRuntime {
        self_update_supported: self_update_supported_for(portable, linux, app_image),
        portable,
    }
}

#[cfg(target_os = "macos")]
fn install_application_menu(app: &tauri::App) -> tauri::Result<()> {
    let show_window = MenuItemBuilder::with_id("show", "显示主窗口 / Show MetaClean")
        .accelerator("CmdOrCtrl+Shift+O")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置 / Settings")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let quit_app = MenuItemBuilder::with_id("quit", "退出 / Exit")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let app_menu = SubmenuBuilder::new(app, "MetaClean")
        .item(&show_window)
        .item(&settings)
        .separator()
        .item(&quit_app)
        .build()?;
    let clean_page = MenuItemBuilder::with_id("clean", "文件净化 / Clean files")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let history_page = MenuItemBuilder::with_id("history", "处理记录 / History")
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
    let privacy_page = MenuItemBuilder::with_id("privacy", "隐私说明 / Privacy")
        .accelerator("CmdOrCtrl+3")
        .build(app)?;
    let settings_page = MenuItemBuilder::with_id("settings-page", "设置 / Settings")
        .accelerator("CmdOrCtrl+4")
        .build(app)?;
    let about_page = MenuItemBuilder::with_id("about", "关于 / About")
        .accelerator("CmdOrCtrl+5")
        .build(app)?;
    let navigation_menu = SubmenuBuilder::new(app, "导航 / Navigate")
        .items(&[
            &clean_page,
            &history_page,
            &privacy_page,
            &settings_page,
            &about_page,
        ])
        .build()?;
    let window_menu = SubmenuBuilder::new(app, "窗口 / Window")
        .minimize()
        .build()?;
    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &navigation_menu, &window_menu])
        .build()?;
    app.set_menu(menu)?;
    Ok(())
}

#[tauri::command]
fn get_update_runtime() -> UpdateRuntime {
    detect_update_runtime()
}

#[tauri::command(rename_all = "camelCase")]
async fn install_update_and_restart(
    app: tauri::AppHandle,
    expected_version: BoundedUpdateVersion,
) -> Result<bool, String> {
    if !detect_update_runtime().self_update_supported {
        return Err("当前安装方式不支持应用内更新，请从官方发布页下载新版本。".into());
    }

    let updater = app
        .updater_builder()
        .timeout(UPDATE_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| bounded_message(format!("初始化更新器失败：{error}")))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|error| updater_network_error("检查更新失败 / Update check failed", error))?
    else {
        return Ok(false);
    };

    if !reviewed_update_matches(&update.version, expected_version.as_str()) {
        return Err(UPDATE_CHANGED.into());
    }

    let progress_app = app.clone();
    let mut downloaded = 0_u64;
    let bytes = update
        .download(
            move |chunk_length, total| {
                downloaded = downloaded.saturating_add(chunk_length as u64);
                let _ = progress_app.emit(
                    "update-progress",
                    UpdateDownloadProgress {
                        stage: "downloading",
                        downloaded,
                        total,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|error| updater_network_error("下载更新失败 / Update download failed", error))?;

    let _ = app.emit(
        "update-progress",
        UpdateDownloadProgress {
            stage: "installing",
            downloaded: 0,
            total: None,
        },
    );

    #[cfg(target_os = "windows")]
    {
        ALLOW_EXIT.store(true, Ordering::SeqCst);
        app.remove_tray_by_id("main");
        if let Err(error) = update.install(bytes) {
            ALLOW_EXIT.store(false, Ordering::SeqCst);
            return Err(bounded_message(format!("安装更新失败：{error}")));
        }
        Ok(true)
    }

    #[cfg(not(target_os = "windows"))]
    {
        update
            .install(bytes)
            .map_err(|error| bounded_message(format!("安装更新失败：{error}")))?;
        ALLOW_EXIT.store(true, Ordering::SeqCst);
        app.remove_tray_by_id("main");
        app.restart();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION)
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            install_application_menu(app)?;

            let open = MenuItem::with_id(app, "open", "打开 MetaClean / Open", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 / Exit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("MetaClean · 本地文件隐私清理")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => {
                        if work_active() {
                            let _ = app.emit("close-blocked", CLEANUP_CLOSE_BLOCKED);
                            return;
                        }
                        ALLOW_EXIT.store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "settings" | "settings-page" => emit_navigation(app, "settings"),
            "clean" | "history" | "privacy" | "about" => emit_navigation(app, event.id().as_ref()),
            "quit" => {
                if work_active() {
                    let _ = app.emit("close-blocked", CLEANUP_CLOSE_BLOCKED);
                    return;
                }
                ALLOW_EXIT.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if work_active() {
                    api.prevent_close();
                    let _ = window.emit("close-blocked", CLEANUP_CLOSE_BLOCKED);
                    return;
                }
                api.prevent_close();
                match close_action(CLOSE_TO_TRAY.load(Ordering::SeqCst)) {
                    CloseAction::HideToTray => {
                        let _ = window.hide();
                    }
                    CloseAction::Exit => {
                        ALLOW_EXIT.store(true, Ordering::SeqCst);
                        let app = window.app_handle();
                        app.remove_tray_by_id("main");
                        app.exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            scan_files,
            expand_paths,
            clean_files,
            cancel_clean_batch,
            cancel_scan_batch,
            export_audit_report,
            get_launch_paths,
            get_context_menu_status,
            set_context_menu_enabled,
            set_close_to_tray,
            about::get_about_info,
            get_update_runtime,
            install_update_and_restart
        ])
        .build(tauri::generate_context!())
        .expect("failed to build MetaClean")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !ALLOW_EXIT.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}

fn emit_navigation(app: &tauri::AppHandle, page: &str) {
    show_main_window(app);
    let _ = app.emit("menu:navigate", page);
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run_cli_action() -> Option<i32> {
    let action = std::env::args().nth(1)?;
    let result = match action.as_str() {
        "--install-context-menu" => shell_integration::install(),
        "--remove-context-menu" => shell_integration::remove(),
        _ => return None,
    };
    match result {
        Ok(status) => {
            println!("{}", status.detail);
            Some(0)
        }
        Err(error) => {
            eprintln!("{error}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod update_tests {
    use super::{
        bounded_launch_paths, cancel_clean_batch, cancel_scan_batch, clean_batch_active,
        close_action, deduplicate_paths, export_audit_report, export_audit_report_to,
        portable_marker_exists, prepare_batch_paths, read_task_active, reviewed_update_matches,
        self_update_supported_for, updater_network_error, validate_batch_id, validate_batch_size,
        validate_path_inputs, ActiveCleanBatchGuard, ActiveReadGuard, ActiveScanBatchGuard,
        BatchProgressState, CloseAction, MAX_BATCH_FILES, MAX_BATCH_ID_BYTES, MAX_BATCH_PATH_BYTES,
        MAX_PATH_BYTES, MAX_REPORT_BYTES, PORTABLE_MARKER, PROGRESS_EVENT_BATCH,
        UPDATE_REQUEST_TIMEOUT,
    };
    use crate::models::{
        BoundedAuditPath, BoundedBatchId, BoundedPaths, BoundedReportContents,
        BoundedUpdateVersion, CleanRequest, MAX_RAW_BATCH_ITEMS,
    };
    use serde::Deserialize;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    static CLEAN_BATCH_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn close_behavior_defaults_to_exit_and_can_hide_to_tray() {
        assert_eq!(close_action(false), CloseAction::Exit);
        assert_eq!(close_action(true), CloseAction::HideToTray);
    }

    #[test]
    fn rejects_oversized_ipc_batches_before_starting_worker_tasks() {
        assert!(validate_batch_size(MAX_BATCH_FILES).is_ok());
        let error = validate_batch_size(MAX_BATCH_FILES + 1).unwrap_err();
        assert!(error.contains("10000"));
        assert!(error.contains("10001"));
    }

    #[test]
    fn deserializes_path_batches_within_their_allocation_budget() {
        let too_many = serde_json::json!((0..=MAX_BATCH_FILES)
            .map(|index| format!("C:\\file-{index}.txt"))
            .collect::<Vec<_>>());
        assert!(serde_json::from_value::<BoundedPaths>(too_many).is_err());
        let repeated = serde_json::json!(vec!["C:\\same.txt"; MAX_BATCH_FILES + 1]);
        let repeated = serde_json::from_value::<BoundedPaths>(repeated)
            .expect("duplicate paths are de-duplicated before the unique limit");
        assert_eq!(repeated.into_inner(), vec!["C:\\same.txt"]);
        let too_many_repeated = serde_json::json!(vec!["C:\\same.txt"; MAX_RAW_BATCH_ITEMS + 1]);
        assert!(serde_json::from_value::<BoundedPaths>(too_many_repeated).is_err());
        let too_wide = serde_json::json!(["x".repeat(MAX_BATCH_PATH_BYTES + 1)]);
        assert!(serde_json::from_value::<BoundedPaths>(too_wide).is_err());
        let empty = serde_json::json!([""]);
        assert!(serde_json::from_value::<BoundedPaths>(empty).is_err());
    }

    #[test]
    fn updater_network_operations_have_a_bounded_deadline() {
        assert_eq!(UPDATE_REQUEST_TIMEOUT, Duration::from_secs(300));
    }

    #[test]
    fn batches_progress_events_without_losing_final_counts() {
        let mut progress = BatchProgressState::new();
        for completed in 1..PROGRESS_EVENT_BATCH {
            progress.last_emitted_at = Instant::now();
            assert!(
                progress
                    .record("scan", "batch", PROGRESS_EVENT_BATCH + 2, false,)
                    .is_none(),
                "unexpected event at item {completed}"
            );
        }
        progress.last_emitted_at = Instant::now();
        let event = progress
            .record("scan", "batch", PROGRESS_EVENT_BATCH + 2, true)
            .expect("batch threshold should emit");
        assert_eq!(event.completed, PROGRESS_EVENT_BATCH);
        assert_eq!(event.failed, 1);
        assert!(!event.cancelled);
        progress.last_emitted_at = Instant::now();
        assert!(progress
            .record("scan", "batch", PROGRESS_EVENT_BATCH + 2, false,)
            .is_none());
        let final_event = progress
            .final_event("scan", "batch", PROGRESS_EVENT_BATCH + 2, false)
            .expect("final count should flush");
        assert_eq!(final_event.completed, PROGRESS_EVENT_BATCH + 1);
        assert_eq!(final_event.failed, 1);
    }

    #[test]
    fn cancellation_always_emits_a_terminal_progress_event() {
        let mut progress = BatchProgressState::new();
        assert!(progress.final_event("clean", "batch", 0, false).is_none());
        let event = progress
            .final_event("clean", "batch", 10, true)
            .expect("cancellation must be visible even before the first item");
        assert_eq!(event.completed, 0);
        assert_eq!(event.total, 10);
        assert!(event.cancelled);
    }

    #[test]
    fn batch_identifiers_are_bounded_without_breaking_legacy_empty_tokens() {
        assert!(validate_batch_id("").is_ok());
        assert!(validate_batch_id(&"x".repeat(MAX_BATCH_ID_BYTES)).is_ok());
        let error = validate_batch_id(&"x".repeat(MAX_BATCH_ID_BYTES + 1)).unwrap_err();
        assert!(error.contains("128"));
        assert!(BoundedBatchId::deserialize(serde_json::Value::String(
            "x".repeat(MAX_BATCH_ID_BYTES),
        ))
        .is_ok());
        assert!(BoundedBatchId::deserialize(serde_json::Value::String(
            "x".repeat(MAX_BATCH_ID_BYTES + 1),
        ))
        .is_err());
        assert!(BoundedUpdateVersion::deserialize(serde_json::Value::String(
            "x".repeat(MAX_BATCH_ID_BYTES),
        ))
        .is_ok());
        assert!(BoundedUpdateVersion::deserialize(serde_json::Value::String(
            "x".repeat(MAX_BATCH_ID_BYTES + 1),
        ))
        .is_err());
    }

    #[test]
    fn keeps_clean_ipc_backward_compatible_without_a_batch_token() {
        let _serial = CLEAN_BATCH_TEST_LOCK
            .lock()
            .expect("lock clean batch tests");
        let request: CleanRequest = serde_json::from_value(serde_json::json!({
            "paths": ["note.txt"],
            "mode": "copy"
        }))
        .expect("legacy cleanup request should still deserialize");
        assert!(request.batch_id.is_empty());

        let guard = ActiveCleanBatchGuard::register(
            request.batch_id.as_str(),
            Arc::new(AtomicBool::new(false)),
        )
        .expect("register legacy clean batch");
        assert!(clean_batch_active());
        drop(guard);
        assert!(!clean_batch_active());
    }

    #[test]
    fn cancellation_marks_only_the_requested_active_batch() {
        let _serial = CLEAN_BATCH_TEST_LOCK
            .lock()
            .expect("lock clean batch tests");
        let batch_id = "cancel-test".to_owned();
        let flag = Arc::new(AtomicBool::new(false));
        let guard = ActiveCleanBatchGuard::register(&batch_id, flag.clone())
            .expect("register active batch");
        assert!(ActiveCleanBatchGuard::register(&batch_id, flag.clone()).is_err());
        assert!(cancel_clean_batch(
            BoundedBatchId::deserialize(serde_json::Value::String(batch_id)).unwrap()
        )
        .expect("cancel active batch"));
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
        assert!(clean_batch_active());
        assert!(!cancel_clean_batch(
            BoundedBatchId::deserialize(serde_json::Value::String("missing".into())).unwrap()
        )
        .expect("missing batch is harmless"));
        assert!(!cancel_clean_batch(BoundedBatchId::default()).expect("empty batch is harmless"));
        drop(guard);
        assert!(!clean_batch_active());
        assert!(ActiveCleanBatchGuard::register("", flag).is_ok());
    }

    #[test]
    fn cancellation_marks_only_the_requested_scan_batch() {
        let batch_id = "scan-cancel-test";
        let flag = Arc::new(AtomicBool::new(false));
        let guard = ActiveScanBatchGuard::register(Some(batch_id), flag.clone())
            .expect("register active scan");
        assert!(ActiveScanBatchGuard::register(Some(batch_id), flag.clone()).is_err());
        assert!(cancel_scan_batch(
            BoundedBatchId::deserialize(serde_json::Value::String(batch_id.into())).unwrap()
        )
        .expect("cancel active scan"));
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!cancel_scan_batch(
            BoundedBatchId::deserialize(serde_json::Value::String("missing".into())).unwrap()
        )
        .expect("missing scan is harmless"));
        assert!(!cancel_scan_batch(BoundedBatchId::default()).expect("empty scan is harmless"));
        drop(guard);
        assert!(ActiveScanBatchGuard::register(None, flag).is_ok());
    }

    #[test]
    fn active_read_tasks_also_block_window_exit() {
        assert!(!read_task_active());
        let guard = ActiveReadGuard::start();
        assert!(read_task_active());
        drop(guard);
        assert!(!read_task_active());
    }

    #[test]
    fn deduplicates_repeated_ipc_paths_while_preserving_first_order() {
        assert_eq!(
            deduplicate_paths(vec![
                "C:\\one.txt".into(),
                "C:\\two.txt".into(),
                "C:\\one.txt".into(),
            ]),
            vec!["C:\\one.txt", "C:\\two.txt"],
        );
    }

    #[test]
    fn deduplicates_before_enforcing_the_batch_limit() {
        let repeated = vec!["C:\\same.txt".to_owned(); MAX_BATCH_FILES + 1];
        assert_eq!(prepare_batch_paths(repeated).unwrap(), vec!["C:\\same.txt"]);

        let distinct = (0..=MAX_BATCH_FILES)
            .map(|index| format!("C:\\file-{index}.txt"))
            .collect();
        let error = prepare_batch_paths(distinct).unwrap_err();
        assert!(error.contains("10000"));
        assert!(error.contains("10001"));
    }

    #[test]
    fn bounds_path_length_and_total_ipc_path_bytes() {
        assert!(validate_path_inputs(&[]).is_ok());
        assert!(validate_path_inputs(&["C:\\note.txt".into()]).is_ok());

        let empty = validate_path_inputs(&[String::new()]).unwrap_err();
        assert!(empty.contains("不能为空"));

        let oversized = validate_path_inputs(&["x".repeat(MAX_PATH_BYTES + 1)]).unwrap_err();
        assert!(oversized.contains("32768"));

        let total_paths = (0..=MAX_BATCH_PATH_BYTES / (MAX_PATH_BYTES - 1))
            .map(|index| format!("{index:05}{}", "x".repeat(MAX_PATH_BYTES - 6)))
            .collect::<Vec<_>>();
        let total = validate_path_inputs(&total_paths).unwrap_err();
        assert!(total.contains("67108864"));
    }

    #[test]
    fn bounds_paths_returned_from_startup_arguments_before_ipc() {
        let oversized = bounded_launch_paths(vec!["x".repeat(MAX_PATH_BYTES + 1)]).unwrap_err();
        assert!(oversized.contains("32768"));
        let total_paths = (0..=MAX_BATCH_PATH_BYTES / (MAX_PATH_BYTES - 1))
            .map(|index| format!("{index:05}{}", "x".repeat(MAX_PATH_BYTES - 6)))
            .collect();
        let total = bounded_launch_paths(total_paths).unwrap_err();
        assert!(total.contains("67108864"));
    }

    #[cfg(windows)]
    #[test]
    fn deduplicates_windows_paths_without_case_or_separator_drift() {
        assert_eq!(
            deduplicate_paths(vec!["C:\\One-Ä.txt".into(), "c:/one-ä.txt".into()]),
            vec!["C:\\One-Ä.txt"],
        );
    }

    #[cfg(windows)]
    #[test]
    fn deduplicates_windows_device_and_unc_path_aliases() {
        assert_eq!(
            deduplicate_paths(vec![
                "C:\\Data\\note.txt".into(),
                "\\\\?\\C:\\Data\\note.txt".into(),
                "\\\\?\\UNC\\server\\share\\note.txt".into(),
                "\\\\server\\share\\note.txt".into(),
            ]),
            vec!["C:\\Data\\note.txt", "\\\\?\\UNC\\server\\share\\note.txt"],
        );
    }

    #[test]
    fn updater_network_errors_lead_with_actionable_help() {
        let message = updater_network_error(
            "检查更新失败 / Update check failed",
            "error sending request for url",
        );
        assert!(message.starts_with("检查更新失败 / Update check failed。无法连接已签名更新源。"));
        assert!(message.contains("HTTPS_PROXY"));
        assert!(message.ends_with("error sending request for url"));
    }

    #[test]
    fn install_requires_the_exact_reviewed_update_version() {
        assert!(reviewed_update_matches("0.6.1", "0.6.1"));
        assert!(reviewed_update_matches("v0.6.1", "0.6.1"));
        assert!(reviewed_update_matches("V0.6.1", "0.6.1"));
        assert!(reviewed_update_matches("  v0.6.1  ", "0.6.1"));
        assert!(!reviewed_update_matches("0.6.2", "0.6.1"));
        assert!(!reviewed_update_matches("0.6", "0.6.0"));
        assert!(!reviewed_update_matches("0.6.1-beta.1", "0.6.1-beta.1"));
        assert!(!reviewed_update_matches("0.6.1", "latest"));
        assert!(!reviewed_update_matches("01.2.3", "01.2.3"));
        assert!(!reviewed_update_matches(
            "999999999999999999.0.1",
            "999999999999999999.0.1"
        ));
        assert!(!reviewed_update_matches(
            &"0".repeat(super::MAX_UPDATE_VERSION_BYTES + 1),
            "0.0.0"
        ));
        assert!(!reviewed_update_matches(
            "0.0.0",
            &"0".repeat(super::MAX_UPDATE_VERSION_BYTES + 1)
        ));
    }

    #[test]
    fn portable_mode_requires_the_package_marker_next_to_the_executable() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let executable = directory.path().join("MetaClean.exe");
        std::fs::write(&executable, b"binary").expect("write executable fixture");
        assert!(!portable_marker_exists(&executable));
        std::fs::write(directory.path().join(PORTABLE_MARKER), b"portable\n")
            .expect("write portable marker");
        assert!(portable_marker_exists(&executable));
    }

    #[test]
    fn self_update_refuses_portable_and_non_appimage_linux_runtimes() {
        assert!(self_update_supported_for(false, false, false));
        assert!(self_update_supported_for(false, true, true));
        assert!(!self_update_supported_for(true, false, false));
        assert!(!self_update_supported_for(false, true, false));
    }

    #[test]
    fn exports_only_bounded_json_audit_reports() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let report = directory.path().join("audit.json");
        let report_path = BoundedAuditPath::deserialize(serde_json::Value::String(
            report.to_string_lossy().into_owned(),
        ))
        .expect("bounded report path");
        let report_contents = BoundedReportContents::deserialize(serde_json::Value::String(
            "{\"schemaVersion\":1}".into(),
        ))
        .expect("bounded report contents");
        export_audit_report(report_path, report_contents).expect("export report");
        assert_eq!(
            std::fs::read_to_string(report).expect("read report"),
            "{\"schemaVersion\":1}"
        );
        assert!(export_audit_report_to(&directory.path().join("audit.txt"), "{}").is_err());
        assert!(export_audit_report_to(
            &directory.path().join("large.json"),
            &"x".repeat(10 * 1024 * 1024 + 1),
        )
        .is_err());
        let oversized_path = format!("{}audit.json", "x".repeat(MAX_PATH_BYTES));
        let oversized_path =
            BoundedAuditPath::deserialize(serde_json::Value::String(oversized_path));
        assert!(oversized_path.is_err());
        let oversized_contents = BoundedReportContents::deserialize(serde_json::Value::String(
            "x".repeat(MAX_REPORT_BYTES + 1),
        ));
        assert!(oversized_contents.is_err());
    }
}
