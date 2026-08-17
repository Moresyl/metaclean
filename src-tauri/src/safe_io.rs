use std::{
    ffi::{OsStr, OsString},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::error::{display_path, CleanError, Result};

#[derive(Debug, Clone)]
pub struct FileMetadataSnapshot {
    permissions: fs::Permissions,
    accessed: filetime::FileTime,
    modified: filetime::FileTime,
    extended_attributes: Vec<ExtendedAttribute>,
}

#[derive(Debug, Clone)]
struct ExtendedAttribute {
    name: OsString,
    value: Vec<u8>,
}

impl FileMetadataSnapshot {
    pub fn capture(path: &Path, metadata: &fs::Metadata) -> Result<Self> {
        Ok(Self {
            permissions: metadata.permissions(),
            accessed: filetime::FileTime::from_last_access_time(metadata),
            modified: filetime::FileTime::from_last_modification_time(metadata),
            extended_attributes: read_extended_attributes(path)?,
        })
    }

    pub fn privacy_extended_attribute_count(&self) -> usize {
        self.extended_attributes
            .iter()
            .filter(|attribute| is_private_macos_attribute(&attribute.name))
            .count()
    }

    fn apply_extended_attributes(&self, path: &Path, remove_private: bool) -> Result<()> {
        #[cfg(target_os = "macos")]
        for attribute in &self.extended_attributes {
            if !remove_private || !is_private_macos_attribute(&attribute.name) {
                xattr::set(path, &attribute.name, &attribute.value)?;
            }
        }
        #[cfg(not(target_os = "macos"))]
        for attribute in &self.extended_attributes {
            let _ = (&attribute.name, &attribute.value, path, remove_private);
        }
        Ok(())
    }
}

fn is_private_macos_attribute(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().as_ref(),
        "com.apple.quarantine"
            | "com.apple.provenance"
            | "com.apple.lastuseddate#PS"
            | "com.apple.metadata:kMDItemDownloadedDate"
            | "com.apple.metadata:kMDItemFinderComment"
            | "com.apple.metadata:kMDItemWhereFroms"
    )
}

#[cfg(target_os = "macos")]
fn read_extended_attributes(path: &Path) -> Result<Vec<ExtendedAttribute>> {
    let mut attributes = Vec::new();
    for name in xattr::list(path)? {
        if let Some(value) = xattr::get(path, &name)? {
            attributes.push(ExtendedAttribute { name, value });
        }
    }
    Ok(attributes)
}

#[cfg(not(target_os = "macos"))]
fn read_extended_attributes(_path: &Path) -> Result<Vec<ExtendedAttribute>> {
    Ok(Vec::new())
}

pub fn privacy_extended_attribute_count(path: &Path) -> Result<usize> {
    Ok(read_extended_attributes(path)?
        .iter()
        .filter(|attribute| is_private_macos_attribute(&attribute.name))
        .count())
}

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

pub fn atomic_write_with_metadata(
    path: &Path,
    bytes: &[u8],
    source_metadata: Option<&FileMetadataSnapshot>,
    preserve_timestamps: bool,
    remove_private_xattrs: bool,
) -> Result<()> {
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
    if let Some(metadata) = source_metadata {
        if preserve_timestamps {
            filetime::set_file_times(temp.path(), metadata.accessed, metadata.modified)?;
        }
        metadata.apply_extended_attributes(temp.path(), remove_private_xattrs)?;
    }
    temp.persist(path)
        .map_err(|error| CleanError::Io(error.error))?;
    if let Some(metadata) = source_metadata {
        fs::set_permissions(path, metadata.permissions.clone())?;
    }
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
        atomic_write_with_metadata(&path, b"new", None, false, false).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"new");
    }

    #[test]
    fn metadata_aware_write_preserves_permissions_and_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let output = dir.path().join("output.txt");
        fs::write(&source, b"source").unwrap();
        let expected = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(&source, expected).unwrap();
        let mut permissions = fs::metadata(&source).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&source, permissions).unwrap();
        let source_metadata = fs::metadata(&source).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&source, &source_metadata).unwrap();
        assert_eq!(snapshot.privacy_extended_attribute_count(), 0);
        atomic_write_with_metadata(&output, b"clean", Some(&snapshot), true, false).unwrap();
        let output_metadata = fs::metadata(&output).unwrap();
        assert_eq!(
            output_metadata.permissions().readonly(),
            source_metadata.permissions().readonly()
        );
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&output_metadata),
            expected
        );
    }

    #[test]
    fn selects_only_known_privacy_macos_attributes() {
        for name in [
            "com.apple.quarantine",
            "com.apple.provenance",
            "com.apple.lastuseddate#PS",
            "com.apple.metadata:kMDItemDownloadedDate",
            "com.apple.metadata:kMDItemFinderComment",
            "com.apple.metadata:kMDItemWhereFroms",
        ] {
            assert!(is_private_macos_attribute(OsStr::new(name)), "{name}");
        }
        for name in [
            "com.apple.FinderInfo",
            "com.apple.ResourceFork",
            "com.apple.metadata:_kMDItemUserTags",
            "user.custom",
        ] {
            assert!(!is_private_macos_attribute(OsStr::new(name)), "{name}");
        }

        #[cfg(not(target_os = "macos"))]
        {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("attributes.txt");
            fs::write(&path, b"test").unwrap();
            let metadata = fs::metadata(&path).unwrap();
            let snapshot = FileMetadataSnapshot {
                permissions: metadata.permissions(),
                accessed: filetime::FileTime::from_last_access_time(&metadata),
                modified: filetime::FileTime::from_last_modification_time(&metadata),
                extended_attributes: vec![
                    ExtendedAttribute {
                        name: OsString::from("com.apple.quarantine"),
                        value: b"private".to_vec(),
                    },
                    ExtendedAttribute {
                        name: OsString::from("user.custom"),
                        value: b"preserve".to_vec(),
                    },
                ],
            };
            assert_eq!(snapshot.privacy_extended_attribute_count(), 1);
            snapshot.apply_extended_attributes(&path, true).unwrap();
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_xattrs_are_preserved_by_default_and_private_ones_are_opt_in() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let preserved = dir.path().join("preserved.txt");
        let stripped = dir.path().join("stripped.txt");
        fs::write(&source, b"source").unwrap();
        xattr::set(
            &source,
            "com.apple.metadata:kMDItemWhereFroms",
            b"private origin",
        )
        .unwrap();
        xattr::set(&source, "com.metaclean.keep", b"keep me").unwrap();
        let metadata = fs::metadata(&source).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&source, &metadata).unwrap();
        assert_eq!(snapshot.privacy_extended_attribute_count(), 1);

        atomic_write_with_metadata(&preserved, b"clean", Some(&snapshot), true, false).unwrap();
        assert_eq!(
            xattr::get(&preserved, "com.apple.metadata:kMDItemWhereFroms").unwrap(),
            Some(b"private origin".to_vec())
        );
        assert_eq!(
            xattr::get(&preserved, "com.metaclean.keep").unwrap(),
            Some(b"keep me".to_vec())
        );

        atomic_write_with_metadata(&stripped, b"clean", Some(&snapshot), true, true).unwrap();
        assert_eq!(
            xattr::get(&stripped, "com.apple.metadata:kMDItemWhereFroms").unwrap(),
            None
        );
        assert_eq!(
            xattr::get(&stripped, "com.metaclean.keep").unwrap(),
            Some(b"keep me".to_vec())
        );
    }
}
