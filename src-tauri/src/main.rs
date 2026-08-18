// Keep release builds attached to the Windows GUI subsystem so starting the
// desktop application never opens an extra console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) = metaclean_lib::run_cli_action() {
        std::process::exit(code);
    }
    metaclean_lib::run();
}
