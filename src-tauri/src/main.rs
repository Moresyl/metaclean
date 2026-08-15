fn main() {
    if let Some(code) = metaclean_lib::run_cli_action() {
        std::process::exit(code);
    }
    metaclean_lib::run();
}
