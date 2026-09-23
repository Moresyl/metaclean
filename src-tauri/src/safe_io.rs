use std::{
    ffi::{OsStr, OsString},
    fs,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::error::{display_path, CleanError, Result};

fn validate_output_path(path: &Path) -> Result<()> {
    if path.to_string_lossy().len() > crate::MAX_PATH_BYTES {
        return Err(CleanError::InvalidFormat(format!(
            "输出路径最多 {} 字节",
            crate::MAX_PATH_BYTES
        )));
    }
    Ok(())
}

fn is_link_or_reparse_point(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

#[cfg(target_os = "macos")]
fn is_trusted_macos_system_alias(path: &Path) -> bool {
    let expected_target = match path {
        path if path == Path::new("/etc") => "/private/etc",
        path if path == Path::new("/tmp") => "/private/tmp",
        path if path == Path::new("/var") => "/private/var",
        _ => return false,
    };
    fs::canonicalize(path)
        .map(|target| target == Path::new(expected_target))
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_trusted_macos_system_alias(_path: &Path) -> bool {
    false
}

/// Reject linked path components, not only a linked final file. A linked
/// parent directory can redirect a normal-looking path into another tree.
pub(crate) fn path_contains_link(path: &Path) -> Result<bool> {
    for component in path.ancestors() {
        if component.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(component) {
            Ok(metadata)
                if is_link_or_reparse_point(&metadata)
                    && !is_trusted_macos_system_alias(component) =>
            {
                return Ok(true);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}

#[derive(Debug, Clone)]
pub struct FileMetadataSnapshot {
    permissions: fs::Permissions,
    accessed: filetime::FileTime,
    modified: filetime::FileTime,
    extended_attributes: Vec<ExtendedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn matches(&self, other: &Self) -> bool {
        // Access time is intentionally excluded: merely opening a file can
        // update it on some filesystems, so treating it as a source mutation
        // would reject an otherwise unchanged candidate. Modification time,
        // permissions and xattrs are the metadata that can affect the output.
        self.modified == other.modified
            && permissions_match(&self.permissions, &other.permissions)
            && same_extended_attributes(&self.extended_attributes, &other.extended_attributes)
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

fn permissions_match(left: &fs::Permissions, right: &fs::Permissions) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        left.mode() == right.mode()
    }
    #[cfg(not(unix))]
    {
        left.readonly() == right.readonly()
    }
}

fn same_extended_attributes(left: &[ExtendedAttribute], right: &[ExtendedAttribute]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort_by(|left, right| left.name.cmp(&right.name));
    right.sort_by(|left, right| left.name.cmp(&right.name));
    left == right
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

fn open_without_following_links(path: &Path) -> Result<fs::File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    #[cfg(windows)]
    {
        if is_link_or_reparse_point(&file.metadata()?) {
            return Err(CleanError::Symlink(display_path(path)));
        }
    }
    Ok(file)
}

pub fn validate_input(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_or_reparse_point(&metadata) || path_contains_link(path)? {
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

pub fn read_validated_input(path: &Path) -> Result<(fs::Metadata, Vec<u8>)> {
    validate_input(path)?;
    let file = open_without_following_links(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(CleanError::InvalidFormat("输入不是普通文件".into()));
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(CleanError::TooLarge(display_path(path)));
    }

    let mut bytes = Vec::with_capacity(metadata.len().min(MAX_INPUT_BYTES) as usize);
    file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(CleanError::TooLarge(display_path(path)));
    }
    if bytes.len() as u64 != metadata.len() {
        return Err(CleanError::SourceChanged(display_path(path)));
    }
    validate_input(path)?;
    Ok((metadata, bytes))
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
    validate_output_path(path)?;
    if path_contains_link(path)? {
        return Err(CleanError::Symlink(display_path(path)));
    }
    let temp = prepare_temp_with_metadata(
        path,
        bytes,
        source_metadata,
        preserve_timestamps,
        remove_private_xattrs,
    )?;
    commit_temp(path, temp, source_metadata, false)?;
    Ok(())
}

pub fn atomic_replace_if_unchanged(
    path: &Path,
    expected: &[u8],
    bytes: &[u8],
    source_metadata: &FileMetadataSnapshot,
    preserve_timestamps: bool,
    remove_private_xattrs: bool,
) -> Result<()> {
    validate_output_path(path)?;
    if path_contains_link(path)? {
        return Err(CleanError::Symlink(display_path(path)));
    }
    ensure_source_unchanged(path, expected, source_metadata)?;
    let temp = prepare_temp_with_metadata(
        path,
        bytes,
        Some(source_metadata),
        preserve_timestamps,
        remove_private_xattrs,
    )?;
    // Writing a large temporary file can take long enough for another process
    // to edit the source. Keep the final source check immediately before the
    // rename, after all expensive work has completed.
    ensure_source_unchanged(path, expected, source_metadata)?;
    commit_temp(path, temp, Some(source_metadata), true)
}

fn prepare_temp_with_metadata(
    path: &Path,
    bytes: &[u8],
    source_metadata: Option<&FileMetadataSnapshot>,
    preserve_timestamps: bool,
    remove_private_xattrs: bool,
) -> Result<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .ok_or_else(|| CleanError::InvalidFormat("输出路径没有父目录".into()))?;
    fs::create_dir_all(parent)?;
    if path_contains_link(path)? {
        return Err(CleanError::Symlink(display_path(path)));
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file_mut().sync_all()?;
    if let Some(metadata) = source_metadata {
        if preserve_timestamps {
            filetime::set_file_times(temp.path(), metadata.accessed, metadata.modified)?;
        }
        metadata.apply_extended_attributes(temp.path(), remove_private_xattrs)?;
        fs::set_permissions(temp.path(), metadata.permissions.clone())?;
    }
    Ok(temp)
}

fn commit_temp(
    path: &Path,
    temp: tempfile::NamedTempFile,
    _source_metadata: Option<&FileMetadataSnapshot>,
    require_existing_file: bool,
) -> Result<()> {
    if path_contains_link(path)? {
        return Err(CleanError::Symlink(display_path(path)));
    }
    if require_existing_file {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Err(CleanError::SourceChanged(display_path(path))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(CleanError::SourceChanged(display_path(path)))
            }
            Err(error) => return Err(CleanError::Io(error)),
        }
    }
    temp.persist(path)
        .map_err(|error| CleanError::Io(error.error))?;
    // Windows does not reliably carry the read-only bit across a rename.
    // Synchronise that bit after commit, but deliberately keep this best
    // effort: the replacement is already committed and returning an error
    // here would report failure while leaving the new output on disk.
    #[cfg(windows)]
    if let Some(metadata) = _source_metadata {
        let _ = fs::set_permissions(path, metadata.permissions.clone());
    }
    Ok(())
}

/// Re-check both the exact source bytes and the metadata snapshot taken before
/// cleaning. A byte-only check is insufficient: a changed macOS quarantine
/// attribute or permission bit could otherwise be silently copied from stale
/// state into a new output. This guard is intentionally reusable before output
/// allocation and immediately before replacement.
pub fn ensure_source_unchanged(
    path: &Path,
    expected: &[u8],
    expected_metadata: &FileMetadataSnapshot,
) -> Result<()> {
    let (metadata, current) = read_validated_input(path)?;
    if current != expected {
        return Err(CleanError::SourceChanged(display_path(path)));
    }
    let current_snapshot = FileMetadataSnapshot::capture(path, &metadata)?;
    if !expected_metadata.matches(&current_snapshot) {
        return Err(CleanError::SourceChanged(display_path(path)));
    }
    Ok(())
}

pub fn atomic_create_unique_with_metadata(
    preferred: &Path,
    bytes: &[u8],
    source_metadata: Option<&FileMetadataSnapshot>,
    preserve_timestamps: bool,
    remove_private_xattrs: bool,
) -> Result<PathBuf> {
    validate_output_path(preferred)?;
    if path_contains_link(preferred)? {
        return Err(CleanError::Symlink(display_path(preferred)));
    }
    let parent = preferred
        .parent()
        .ok_or_else(|| CleanError::InvalidFormat("输出路径没有父目录".into()))?;
    fs::create_dir_all(parent)?;
    if path_contains_link(preferred)? {
        return Err(CleanError::Symlink(display_path(preferred)));
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file_mut().sync_all()?;
    if let Some(metadata) = source_metadata {
        if preserve_timestamps {
            filetime::set_file_times(temp.path(), metadata.accessed, metadata.modified)?;
        }
        metadata.apply_extended_attributes(temp.path(), remove_private_xattrs)?;
        fs::set_permissions(temp.path(), metadata.permissions.clone())?;
    }

    for index in 1..=10_000 {
        let candidate = numbered_path(preferred, index);
        validate_output_path(&candidate)?;
        if path_contains_link(&candidate)? {
            return Err(CleanError::Symlink(display_path(&candidate)));
        }
        match temp.persist_noclobber(&candidate) {
            Ok(_) => {
                // Windows may need a best-effort post-rename permission sync;
                // never turn that already-committed output into a reported
                // failure.
                #[cfg(windows)]
                if let Some(metadata) = source_metadata {
                    let _ = fs::set_permissions(&candidate, metadata.permissions.clone());
                }
                return Ok(candidate);
            }
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temp = error.file;
            }
            Err(error) => return Err(CleanError::Io(error.error)),
        }
    }
    Err(CleanError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "无法分配不覆盖现有文件的输出路径",
    )))
}

pub(crate) fn remove_created_output(path: &Path) {
    #[cfg(windows)]
    if let Ok(metadata) = fs::metadata(path) {
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
    }
    let _ = fs::remove_file(path);
}

fn numbered_path(preferred: &Path, index: usize) -> PathBuf {
    if index == 1 {
        return preferred.to_owned();
    }
    let parent = preferred.parent().unwrap_or_else(|| Path::new("."));
    let stem = preferred
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("cleaned");
    let name = match preferred.extension().and_then(|value| value.to_str()) {
        Some(extension) => format!("{stem}-{index}.{extension}"),
        None => format!("{stem}-{index}"),
    };
    parent.join(name)
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
    fn generates_backup_and_numbered_collision_paths() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("report.pdf");
        assert_eq!(backup_path(&source), dir.path().join("report.pdf.bak"));

        let preferred = dir.path().join("report.cleaned.pdf");
        assert_eq!(
            numbered_path(&preferred, 2),
            dir.path().join("report.cleaned-2.pdf")
        );

        let no_extension = dir.path().join("output");
        assert_eq!(numbered_path(&no_extension, 2), dir.path().join("output-2"));
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
    fn write_paths_are_bounded_before_filesystem_work() {
        let oversized = PathBuf::from(format!("{}{}.json", "x".repeat(crate::MAX_PATH_BYTES), "x"));
        assert!(matches!(
            atomic_write_with_metadata(&oversized, b"{}", None, false, false),
            Err(CleanError::InvalidFormat(_))
        ));
        assert!(matches!(
            atomic_create_unique_with_metadata(&oversized, b"{}", None, false, false),
            Err(CleanError::InvalidFormat(_))
        ));
    }

    #[test]
    fn bounded_read_and_guarded_replace_refuse_changed_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"original").unwrap();
        let (metadata, original) = read_validated_input(&path).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&path, &metadata).unwrap();

        fs::write(&path, b"edited elsewhere").unwrap();
        let result =
            atomic_replace_if_unchanged(&path, &original, b"cleaned", &snapshot, true, false);
        assert!(matches!(result, Err(CleanError::SourceChanged(_))));
        assert_eq!(fs::read(&path).unwrap(), b"edited elsewhere");
    }

    #[test]
    fn guarded_replace_keeps_the_last_source_check_after_temp_preparation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"original").unwrap();
        let (metadata, original) = read_validated_input(&path).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&path, &metadata).unwrap();

        let temp =
            prepare_temp_with_metadata(&path, b"cleaned", Some(&snapshot), true, false).unwrap();
        fs::write(&path, b"edited while preparing output").unwrap();
        assert!(matches!(
            ensure_source_unchanged(&path, &original, &snapshot),
            Err(CleanError::SourceChanged(_))
        ));
        drop(temp);
        assert_eq!(fs::read(&path).unwrap(), b"edited while preparing output");
    }

    #[test]
    fn replacement_commit_never_recreates_a_deleted_source() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        fs::write(&source, b"source").unwrap();
        let temp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        fs::remove_file(&source).unwrap();

        let error = commit_temp(&source, temp, None, true).unwrap_err();
        assert!(matches!(error, CleanError::SourceChanged(_)));
        assert!(!source.exists());
    }

    #[test]
    fn guarded_replace_refuses_metadata_changes_even_when_bytes_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"stable").unwrap();
        let (metadata, original) = read_validated_input(&path).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&path, &metadata).unwrap();
        let changed = filetime::FileTime::from_unix_time(1_500_000_000, 0);
        filetime::set_file_mtime(&path, changed).unwrap();

        let result = ensure_source_unchanged(&path, &original, &snapshot);
        assert!(matches!(result, Err(CleanError::SourceChanged(_))));
        assert_eq!(fs::read(&path).unwrap(), b"stable");
    }

    #[test]
    fn guarded_replace_refuses_permission_changes_even_when_bytes_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"stable").unwrap();
        let (metadata, original) = read_validated_input(&path).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&path, &metadata).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(!permissions.readonly());
        fs::set_permissions(&path, permissions).unwrap();

        let result = ensure_source_unchanged(&path, &original, &snapshot);
        assert!(matches!(result, Err(CleanError::SourceChanged(_))));
        assert_eq!(fs::read(&path).unwrap(), b"stable");
    }

    #[test]
    fn removes_readonly_created_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("created.txt");
        fs::write(&path, b"created").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();

        remove_created_output(&path);

        assert!(!path.exists());
    }

    #[cfg(windows)]
    #[test]
    fn bounded_read_does_not_follow_windows_file_symlinks() {
        use std::os::windows::fs::symlink_file;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("private.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, b"private").unwrap();
        if symlink_file(&target, &link).is_err() {
            return;
        }
        assert!(matches!(
            read_validated_input(&link),
            Err(CleanError::Symlink(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_files_reached_through_symlinked_parent_directories() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        let file = target.join("private.txt");
        fs::write(&file, b"private").unwrap();
        let link = dir.path().join("linked-parent");
        symlink(&target, &link).unwrap();
        let through_link = link.join("private.txt");

        assert!(matches!(
            validate_input(&through_link),
            Err(CleanError::Symlink(_))
        ));
        assert!(matches!(
            read_validated_input(&through_link),
            Err(CleanError::Symlink(_))
        ));
        let output_through_link = link.join("cleaned.txt");
        assert!(matches!(
            atomic_create_unique_with_metadata(&output_through_link, b"clean", None, false, false),
            Err(CleanError::Symlink(_))
        ));
        assert!(matches!(
            atomic_write_with_metadata(&output_through_link, b"clean", None, false, false),
            Err(CleanError::Symlink(_))
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn allows_files_under_apple_system_aliases() {
        for alias in ["/etc", "/tmp", "/var"] {
            assert!(is_trusted_macos_system_alias(Path::new(alias)));
        }
    }

    #[cfg(windows)]
    #[test]
    fn rejects_files_reached_through_reparse_parent_directories() {
        use std::os::windows::fs::symlink_dir;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        let file = target.join("private.txt");
        fs::write(&file, b"private").unwrap();
        let link = dir.path().join("linked-parent");
        if symlink_dir(&target, &link).is_err() {
            // Creating reparse points requires a developer-mode or elevated
            // Windows test environment; keep the rest of the suite portable.
            return;
        }
        let through_link = link.join("private.txt");

        assert!(matches!(
            validate_input(&through_link),
            Err(CleanError::Symlink(_))
        ));
        assert!(matches!(
            read_validated_input(&through_link),
            Err(CleanError::Symlink(_))
        ));
        let output_through_link = link.join("cleaned.txt");
        assert!(matches!(
            atomic_create_unique_with_metadata(&output_through_link, b"clean", None, false, false),
            Err(CleanError::Symlink(_))
        ));
        assert!(matches!(
            atomic_write_with_metadata(&output_through_link, b"clean", None, false, false),
            Err(CleanError::Symlink(_))
        ));
    }

    #[test]
    fn unique_atomic_create_never_clobbers_an_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let preferred = dir.path().join("report.cleaned.txt");
        fs::write(&preferred, b"existing").unwrap();
        let output =
            atomic_create_unique_with_metadata(&preferred, b"new", None, false, false).unwrap();

        assert_eq!(output, dir.path().join("report.cleaned-2.txt"));
        assert_eq!(fs::read(preferred).unwrap(), b"existing");
        assert_eq!(fs::read(output).unwrap(), b"new");
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
    fn unique_metadata_aware_copy_preserves_readonly_sources() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let preferred = dir.path().join("source.cleaned.txt");
        fs::write(&source, b"source").unwrap();
        let mut permissions = fs::metadata(&source).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&source, permissions).unwrap();
        let metadata = fs::metadata(&source).unwrap();
        let snapshot = FileMetadataSnapshot::capture(&source, &metadata).unwrap();

        let output =
            atomic_create_unique_with_metadata(&preferred, b"clean", Some(&snapshot), true, false)
                .unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"clean");
        assert_eq!(
            fs::metadata(output).unwrap().permissions().readonly(),
            metadata.permissions().readonly()
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
