mod cleaners;
mod engine;
mod error;
mod intake;
mod models;
mod safe_io;
mod shell_integration;

use models::{CleanRequest, CleanResult, ScanReport};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{menu::MenuItem, tray::TrayIconBuilder, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::StateFlags;

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);
const PORTABLE_MARKER: &str = "metaclean-portable.marker";
const MAX_BATCH_FILES: usize = 10_000;
const UPDATE_NETWORK_HELP: &str = "无法连接已签名更新源。请检查 GitHub 网络或 HTTPS_PROXY 后重试，也可从正式发布页手动下载安装包。 / Could not reach the signed update feed. Check GitHub access or HTTPS_PROXY, then retry, or download the installer from the Releases page.";
const UPDATE_CHANGED: &str = "可用版本在确认后发生了变化，请先重新检查并查看新版本说明。 / The available release changed after confirmation. Check again and review the new release before installing.";

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
    format!("{action}。{UPDATE_NETWORK_HELP}\n技术详情 / Technical detail: {error}")
}

fn reviewed_update_matches(available: &str, expected: &str) -> bool {
    available.strip_prefix('v').unwrap_or(available)
        == expected.strip_prefix('v').unwrap_or(expected)
}

fn validate_batch_size(count: usize) -> Result<(), String> {
    if count > MAX_BATCH_FILES {
        return Err(format!(
            "单次任务最多处理 {MAX_BATCH_FILES} 个输入，当前收到 {count} 个"
        ));
    }
    Ok(())
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

#[tauri::command]
async fn scan_files(paths: Vec<String>) -> Result<Vec<ScanReport>, String> {
    validate_batch_size(paths.len())?;
    tauri::async_runtime::spawn_blocking(move || engine::scan_paths(&paths))
        .await
        .map_err(|error| format!("扫描任务异常结束：{error}"))
}

#[tauri::command]
async fn expand_paths(paths: Vec<String>) -> Result<intake::IntakeResult, String> {
    validate_batch_size(paths.len())?;
    tauri::async_runtime::spawn_blocking(move || intake::expand_paths(&paths))
        .await
        .map_err(|error| format!("目录导入任务异常结束：{error}"))
}

#[tauri::command]
async fn clean_files(request: CleanRequest) -> Result<Vec<CleanResult>, String> {
    validate_batch_size(request.paths.len())?;
    tauri::async_runtime::spawn_blocking(move || {
        request
            .paths
            .iter()
            .map(|path| {
                engine::clean_file_isolated_with_options(
                    std::path::Path::new(path),
                    &request.mode,
                    request.preserve_timestamps,
                    request.preserve_orientation,
                    request.preserve_color_profile,
                    request.remove_extended_attributes,
                )
            })
            .collect()
    })
    .await
    .map_err(|error| format!("清理任务异常结束：{error}"))
}

#[tauri::command]
fn export_audit_report(path: String, contents: String) -> Result<(), String> {
    export_audit_report_to(std::path::Path::new(&path), &contents)
}

fn export_audit_report_to(destination: &std::path::Path, contents: &str) -> Result<(), String> {
    const MAX_REPORT_BYTES: usize = 10 * 1024 * 1024;
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
        .map_err(|error| format!("导出审计报告失败：{error}"))
}

#[tauri::command]
fn get_launch_paths() -> Vec<String> {
    shell_integration::launch_paths()
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
    .map_err(|error| format!("更新 Windows 右键菜单失败：{error}"))
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
    let navigation_menu = SubmenuBuilder::new(app, "导航 / Navigate")
        .items(&[&clean_page, &history_page, &privacy_page, &settings_page])
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

#[tauri::command]
async fn install_update_and_restart(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<bool, String> {
    if !detect_update_runtime().self_update_supported {
        return Err("当前安装方式不支持应用内更新，请从官方发布页下载新版本。".into());
    }

    let updater = app
        .updater_builder()
        .build()
        .map_err(|error| format!("初始化更新器失败：{error}"))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|error| updater_network_error("检查更新失败 / Update check failed", error))?
    else {
        return Ok(false);
    };

    if !reviewed_update_matches(&update.version, &expected_version) {
        return Err(format!(
            "{UPDATE_CHANGED}\nExpected {expected_version}; found {}",
            update.version
        ));
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
            return Err(format!("安装更新失败：{error}"));
        }
        Ok(true)
    }

    #[cfg(not(target_os = "windows"))]
    {
        update
            .install(bytes)
            .map_err(|error| format!("安装更新失败：{error}"))?;
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
            "clean" | "history" | "privacy" => emit_navigation(app, event.id().as_ref()),
            "quit" => {
                ALLOW_EXIT.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
            export_audit_report,
            get_launch_paths,
            get_context_menu_status,
            set_context_menu_enabled,
            set_close_to_tray,
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
        close_action, export_audit_report_to, portable_marker_exists, reviewed_update_matches,
        self_update_supported_for, updater_network_error, validate_batch_size, CloseAction,
        MAX_BATCH_FILES, PORTABLE_MARKER,
    };

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
        assert!(!reviewed_update_matches("0.6.2", "0.6.1"));
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
        export_audit_report_to(&report, "{\"schemaVersion\":1}").expect("export report");
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
    }
}
