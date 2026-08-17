mod cleaners;
mod engine;
mod error;
mod intake;
mod models;
mod safe_io;
mod shell_integration;

use models::{CleanRequest, CleanResult, ScanReport};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{menu::MenuItem, tray::TrayIconBuilder, Manager};

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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
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
