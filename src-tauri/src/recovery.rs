//! A durable, path-free indication of interrupted cleanup. Never resumes work.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(crate) struct CleanupJournal {
    file: File,
    path: PathBuf,
}

fn directory(app_data: &Path) -> io::Result<PathBuf> {
    let path = app_data.join("cleanup-recovery");
    if crate::safe_io::path_contains_link(&path).map_err(io::Error::other)? {
        return Err(io::Error::other(
            "Recovery directory must not contain links",
        ));
    }
    Ok(path)
}

impl CleanupJournal {
    pub(crate) fn begin(app_data: &Path) -> io::Result<Self> {
        let directory = directory(app_data)?;
        fs::create_dir_all(&directory)?;
        // Publish only after locking and flushing: another running instance must
        // never mistake a newly created record for an abandoned cleanup.
        let mut temporary = tempfile::Builder::new()
            .prefix("batch-")
            .rand_bytes(16)
            .suffix(".tmp")
            .tempfile_in(directory)?;
        temporary.as_file().lock()?;
        temporary.write_all(b"MC1\n")?;
        temporary.as_file().sync_all()?;
        let path = temporary.path().with_extension("pending");
        let file = temporary
            .persist_noclobber(&path)
            .map_err(|error| error.error)?;
        Ok(Self { file, path })
    }

    pub(crate) fn finish(self) -> io::Result<()> {
        // Keep the lock while removing the record. On panic or process death,
        // closing the handle releases the lock but leaves the record behind.
        let result = fs::remove_file(&self.path);
        drop(self.file);
        result
    }
}

pub(crate) fn take_interrupted(app_data: &Path) -> io::Result<bool> {
    let directory = directory(app_data)?;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let mut interrupted = false;
    for (index, entry) in entries.enumerate() {
        if index >= 10_000 {
            return Err(io::Error::other("Too many recovery records"));
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name
            .to_str()
            .and_then(|name| name.strip_prefix("batch-"))
            .and_then(|name| name.strip_suffix(".pending"))
        else {
            continue;
        };
        if id.len() != 16 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            continue;
        }
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || crate::safe_io::path_contains_link(&path).map_err(io::Error::other)?
        {
            continue;
        }
        let file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => continue,
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
        // Record contents are not needed. Even a damaged record is reason to
        // warn conservatively; no file names or operation data are ever read.
        match fs::remove_file(path) {
            Ok(()) => interrupted = true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(interrupted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_completion_removes_path_free_record() {
        use std::io::{Read, Seek, SeekFrom};
        let root = tempfile::tempdir().unwrap();
        let mut journal = CleanupJournal::begin(root.path()).unwrap();
        journal.file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        journal.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"MC1\n");
        journal.finish().unwrap();
        assert!(!take_interrupted(root.path()).unwrap());
    }

    #[test]
    fn active_batches_are_skipped_and_abandoned_batches_consumed_once() {
        let root = tempfile::tempdir().unwrap();
        let active = CleanupJournal::begin(root.path()).unwrap();
        let abandoned = CleanupJournal::begin(root.path()).unwrap();
        assert!(!take_interrupted(root.path()).unwrap());
        drop(abandoned);
        assert!(take_interrupted(root.path()).unwrap());
        assert!(active.path.exists());
        assert!(!take_interrupted(root.path()).unwrap());
        active.finish().unwrap();
    }

    #[test]
    fn damaged_record_still_warns_without_reading_it() {
        let root = tempfile::tempdir().unwrap();
        let journal = CleanupJournal::begin(root.path()).unwrap();
        let path = journal.path.clone();
        drop(journal);
        fs::write(path, b"").unwrap();
        assert!(take_interrupted(root.path()).unwrap());
        assert!(!take_interrupted(root.path()).unwrap());
    }

    #[test]
    fn missing_directory_and_unrelated_files_are_untouched() {
        let root = tempfile::tempdir().unwrap();
        assert!(!take_interrupted(root.path()).unwrap());
        let journal = CleanupJournal::begin(root.path()).unwrap();
        journal.finish().unwrap();
        let path = directory(root.path())
            .unwrap()
            .join("batch-unrelated.pending");
        fs::write(&path, b"unrelated").unwrap();
        assert!(!take_interrupted(root.path()).unwrap());
        assert_eq!(fs::read(path).unwrap(), b"unrelated");
    }

    #[test]
    fn inaccessible_recovery_location_prevents_beginning() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("cleanup-recovery"), b"not a directory").unwrap();
        assert!(CleanupJournal::begin(root.path()).is_err());
        assert!(take_interrupted(root.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn linked_directories_and_records_are_not_followed() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let directory = root.path().join("cleanup-recovery");
        symlink(outside.path(), &directory).unwrap();
        assert!(CleanupJournal::begin(root.path()).is_err());
        assert!(take_interrupted(root.path()).is_err());
        fs::remove_file(&directory).unwrap();
        fs::create_dir(&directory).unwrap();
        let target = outside.path().join("unrelated");
        fs::write(&target, b"unchanged").unwrap();
        let link = directory.join("batch-0123456789abcdef.pending");
        symlink(&target, &link).unwrap();
        assert!(!take_interrupted(root.path()).unwrap());
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(target).unwrap(), b"unchanged");
    }
}
