mod cleaners;
mod engine;
mod error;
mod intake;
mod models;
mod safe_io;
mod shell_integration;

use models::{CleanRequest, CleanResult, ScanReport};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tauri_plugin_window_state::StateFlags;

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn scan_files(paths: Vec<String>) -> Vec<ScanReport> {
    engine::scan_paths(&paths)
}

#[tauri::command]
fn expand_paths(paths: Vec<String>) -> intake::IntakeResult {
    intake::expand_paths(&paths)
}

#[tauri::command]
fn clean_files(request: CleanRequest) -> Vec<CleanResult> {
    request
        .paths
        .iter()
        .map(|path| {
            engine::clean_file_with_options(
                std::path::Path::new(path),
                &request.mode,
                request.preserve_timestamps,
                request.preserve_orientation,
                request.preserve_color_profile,
                request.remove_extended_attributes,
            )
        })
        .collect()
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
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .setup(|app| {
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
                .fullscreen()
                .build()?;
            let window_menu = MenuBuilder::new(app)
                .items(&[&app_menu, &navigation_menu, &window_menu])
                .build()?;
            app.set_menu(window_menu)?;

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
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            scan_files,
            expand_paths,
            clean_files,
            get_launch_paths,
            get_context_menu_status,
            set_context_menu_enabled
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
