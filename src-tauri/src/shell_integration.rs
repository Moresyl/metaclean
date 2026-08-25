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
    use crate::engine::SUPPORTED_EXTENSIONS;

    const VERB: &str = "MetaClean";

    fn verb_path(extension: &str) -> String {
        format!(r"Software\Classes\SystemFileAssociations\.{extension}\shell\{VERB}")
    }

    fn command_value(executable: &str) -> String {
        format!(r#""{executable}" "%1""#)
    }

    pub fn status() -> ContextMenuStatus {
        let classes = RegKey::predef(HKEY_CURRENT_USER);
        let executable = env::current_exe()
            .ok()
            .map(|path| path.to_string_lossy().into_owned());
        let enabled = executable.as_deref().is_some_and(|executable| {
            let expected = command_value(executable);
            SUPPORTED_EXTENSIONS.iter().all(|extension| {
                classes
                    .open_subkey(format!(r"{}\command", verb_path(extension)))
                    .and_then(|command| command.get_value::<String, _>(""))
                    .is_ok_and(|command| command == expected)
            })
        });
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
        for extension in SUPPORTED_EXTENSIONS {
            let (verb, _) = classes.create_subkey(verb_path(extension))?;
            verb.set_value("", &"使用 MetaClean 扫描 / Scan with MetaClean")?;
            verb.set_value("Icon", &format!(r#""{executable}""#))?;
            let (command, _) = verb.create_subkey("command")?;
            command.set_value("", &command_value(&executable))?;
        }
        Ok(status())
    }

    pub fn remove() -> io::Result<ContextMenuStatus> {
        let classes = RegKey::predef(HKEY_CURRENT_USER);
        for extension in SUPPORTED_EXTENSIONS {
            match classes.delete_subkey_all(verb_path(extension)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(status())
    }

    #[cfg(test)]
    mod tests {
        use super::command_value;

        #[test]
        fn context_menu_command_quotes_both_executable_and_file() {
            assert_eq!(
                command_value(r"C:\Program Files\MetaClean\MetaClean.exe"),
                r#""C:\Program Files\MetaClean\MetaClean.exe" "%1""#
            );
        }
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
