mod cleaners;
mod engine;
mod error;
mod models;
mod safe_io;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan_files, clean_files])
        .run(tauri::generate_context!())
        .expect("failed to run MetaClean");
}
