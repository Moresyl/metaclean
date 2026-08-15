use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::error::{display_path, CleanError, Result};

pub const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

pub fn validate_input(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(CleanError::Symlink(display_path(path)));
    }
    if !metadata.is_file() {
        return Err(CleanError::InvalidFormat("输入不是普通文件".into()));
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(CleanError::TooLarge(display_path(path)));
    }
    Ok(metadata)
}

pub fn cleaned_path(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("cleaned");
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let name = if extension.is_empty() {
        format!("{stem}.cleaned")
    } else {
        format!("{stem}.cleaned.{extension}")
    };
    source.with_file_name(name)
}

pub fn backup_path(source: &Path) -> PathBuf {
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup");
    source.with_file_name(format!("{name}.bak"))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(CleanError::Symlink(display_path(path)));
    }
    let parent = path
        .parent()
        .ok_or_else(|| CleanError::InvalidFormat("输出路径没有父目录".into()))?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file_mut().sync_all()?;
    temp.persist(path)
        .map_err(|error| CleanError::Io(error.error))?;
    Ok(())
}

pub fn unique_path(preferred: PathBuf) -> PathBuf {
    if !preferred.exists() {
        return preferred;
    }
    let parent = preferred.parent().unwrap_or_else(|| Path::new("."));
    let stem = preferred
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("cleaned");
    let extension = preferred.extension().and_then(|value| value.to_str());
    for index in 2..10_000 {
        let name = match extension {
            Some(ext) => format!("{stem}-{index}.{ext}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    preferred
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_regular_files_and_rejects_directories_and_large_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("regular.txt");
        fs::write(&file, b"ok").unwrap();
        assert_eq!(validate_input(&file).unwrap().len(), 2);
        assert!(matches!(
            validate_input(dir.path()),
            Err(CleanError::InvalidFormat(_))
        ));

        let large = dir.path().join("large.bin");
        let handle = fs::File::create(&large).unwrap();
        handle.set_len(MAX_INPUT_BYTES + 1).unwrap();
        assert!(matches!(
            validate_input(&large),
            Err(CleanError::TooLarge(_))
        ));
    }

    #[test]
    fn generates_backup_and_unique_collision_paths() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("report.pdf");
        assert_eq!(backup_path(&source), dir.path().join("report.pdf.bak"));

        let preferred = dir.path().join("report.cleaned.pdf");
        fs::write(&preferred, b"existing").unwrap();
        assert_eq!(
            unique_path(preferred),
            dir.path().join("report.cleaned-2.pdf")
        );

        let no_extension = dir.path().join("output");
        fs::write(&no_extension, b"existing").unwrap();
        assert_eq!(unique_path(no_extension), dir.path().join("output-2"));
    }

    #[test]
    fn generates_cleaned_name_before_extension() {
        assert_eq!(
            cleaned_path(Path::new("report.pdf")),
            PathBuf::from("report.cleaned.pdf")
        );
    }

    #[test]
    fn atomic_write_replaces_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read(path).unwrap(), b"new");
    }
}
