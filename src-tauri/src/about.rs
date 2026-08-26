//! Runtime facts shown on the About page.
//!
//! These values come from the running binary instead of package.json so a
//! support report describes the application the user actually launched.

use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    version: String,
    platform: String,
    arch: String,
    app_data_dir: Option<PathBuf>,
    executable_dir: Option<PathBuf>,
}

fn about_info(
    version: String,
    app_data_dir: Option<PathBuf>,
    executable_dir: Option<PathBuf>,
) -> AboutInfo {
    AboutInfo {
        version,
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        app_data_dir,
        executable_dir,
    }
}

#[tauri::command]
pub fn get_about_info(app: tauri::AppHandle) -> AboutInfo {
    about_info(
        app.package_info().version.to_string(),
        app.path().app_data_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(PathBuf::from)),
    )
}

#[cfg(test)]
mod tests {
    use super::{about_info, PathBuf};

    #[test]
    fn keeps_runtime_version_and_support_paths_together() {
        let info = about_info(
            "0.7.1".into(),
            Some(PathBuf::from("app-data")),
            Some(PathBuf::from("install")),
        );

        assert_eq!(info.version, "0.7.1");
        assert_eq!(info.platform, std::env::consts::OS);
        assert_eq!(info.arch, std::env::consts::ARCH);
        assert_eq!(info.app_data_dir, Some(PathBuf::from("app-data")));
        assert_eq!(info.executable_dir, Some(PathBuf::from("install")));
    }
}
