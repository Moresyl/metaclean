mod cleaners;
mod engine;
mod error;
mod models;
mod safe_io;
mod shell_integration;

use models::{CleanRequest, CleanResult, ScanReport};

#[tauri::command]
fn scan_files(paths: Vec<String>) -> Vec<ScanReport> {
    engine::scan_paths(&paths)
}

#[tauri::command]
fn clean_files(request: CleanRequest) -> Vec<CleanResult> {
    request
        .paths
        .iter()
        .map(|path| engine::clean_file(std::path::Path::new(path), &request.mode))
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
        .invoke_handler(tauri::generate_handler![
            scan_files,
            clean_files,
            get_launch_paths,
            get_context_menu_status,
            set_context_menu_enabled
        ])
        .run(tauri::generate_context!())
        .expect("failed to run MetaClean");
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
