use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuStatus {
    pub available: bool,
    pub enabled: bool,
    pub detail: String,
}

pub fn launch_paths() -> Vec<String> {
    std::env::args_os()
        .skip(1)
        .take(100)
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[cfg(windows)]
mod platform {
    use std::{env, io};
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    use super::ContextMenuStatus;

    const EXTENSIONS: &[&str] = &[
        ".jpg",
        ".jpeg",
        ".png",
        ".webp",
        ".gif",
        ".mp3",
        ".wav",
        ".flac",
        ".docx",
        ".xlsx",
        ".pptx",
        ".odt",
        ".pdf",
        ".txt",
        ".md",
        ".markdown",
        ".html",
        ".htm",
        ".svg",
        ".xml",
        ".json",
        ".csv",
    ];
    const VERB: &str = "MetaClean";

    fn verb_path(extension: &str) -> String {
        format!(r"Software\Classes\SystemFileAssociations\{extension}\shell\{VERB}")
    }

    pub fn status() -> ContextMenuStatus {
        let classes = RegKey::predef(HKEY_CURRENT_USER);
        let enabled = EXTENSIONS
            .iter()
            .all(|extension| classes.open_subkey(verb_path(extension)).is_ok());
        ContextMenuStatus {
            available: true,
            enabled,
            detail: if enabled {
                "已为支持的文件类型启用资源管理器菜单".into()
            } else {
                "可在 Windows 资源管理器“显示更多选项”中启用".into()
            },
        }
    }

    pub fn install() -> io::Result<ContextMenuStatus> {
        let executable = env::current_exe()?;
        let executable = executable.to_string_lossy();
        let classes = RegKey::predef(HKEY_CURRENT_USER);
        for extension in EXTENSIONS {
            let (verb, _) = classes.create_subkey(verb_path(extension))?;
            verb.set_value("", &"使用 MetaClean 扫描 / Scan with MetaClean")?;
            verb.set_value("Icon", &format!(r#""{executable}""#))?;
            let (command, _) = verb.create_subkey("command")?;
            command.set_value("", &format!(r#""{executable}" "%1""#))?;
        }
        Ok(status())
    }

    pub fn remove() -> io::Result<ContextMenuStatus> {
        let classes = RegKey::predef(HKEY_CURRENT_USER);
        for extension in EXTENSIONS {
            match classes.delete_subkey_all(verb_path(extension)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(status())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::ContextMenuStatus;
    pub fn status() -> ContextMenuStatus {
        ContextMenuStatus {
            available: false,
            enabled: false,
            detail: "右键菜单集成仅适用于 Windows".into(),
        }
    }
    pub fn install() -> std::io::Result<ContextMenuStatus> {
        Ok(status())
    }
    pub fn remove() -> std::io::Result<ContextMenuStatus> {
        Ok(status())
    }
}

pub use platform::{install, remove, status};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_paths_never_include_executable_argument() {
        assert!(launch_paths()
            .iter()
            .all(|path| std::path::Path::new(path).is_file()));
    }
}
