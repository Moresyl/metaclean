use std::{fs, path::Path};

use serde::Serialize;

use crate::engine;

const MAX_DISCOVERED_FILES: usize = 10_000;
const MAX_RECURSION_DEPTH: usize = 64;
const MAX_REPORTED_ISSUES: usize = 100;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntakeIssue {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntakeResult {
    pub files: Vec<String>,
    pub skipped_count: usize,
    pub issues: Vec<IntakeIssue>,
    pub limit_reached: bool,
}

impl IntakeResult {
    fn skip(&mut self, path: &Path, reason: impl Into<String>) {
        self.skipped_count += 1;
        if self.issues.len() < MAX_REPORTED_ISSUES {
            self.issues.push(IntakeIssue {
                path: path.to_string_lossy().into_owned(),
                reason: reason.into(),
            });
        }
    }
}

pub fn expand_paths(paths: &[String]) -> IntakeResult {
    let mut result = IntakeResult::default();
    for path in paths {
        if result.limit_reached {
            break;
        }
        visit(Path::new(path), false, 0, &mut result);
    }
    result.files.sort();
    result.files.dedup();
    result
}

fn visit(path: &Path, from_directory: bool, depth: usize, result: &mut IntakeResult) {
    if result.files.len() >= MAX_DISCOVERED_FILES {
        result.limit_reached = true;
        result.skip(path, "已达到单次导入 10000 个文件的安全上限");
        return;
    }
    if depth > MAX_RECURSION_DEPTH {
        result.skip(path, "目录层级超过 64 层安全上限");
        return;
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            result.skip(path, format!("无法读取：{error}"));
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        result.skip(path, "跳过符号链接");
        return;
    }
    if metadata.is_file() {
        if !from_directory || engine::has_supported_extension(path) {
            result.files.push(path.to_string_lossy().into_owned());
        } else {
            result.skip(path, "暂不支持此扩展名");
        }
        return;
    }
    if !metadata.is_dir() {
        result.skip(path, "不是常规文件或目录");
        return;
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            result.skip(path, format!("无法读取目录：{error}"));
            return;
        }
    };
    let mut children = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(error) => result.skip(path, format!("无法读取目录项：{error}")),
        }
    }
    children.sort();
    for child in children {
        if result.limit_reached {
            break;
        }
        visit(&child, true, depth + 1, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_supported_files_recursively_and_reports_skips() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("photos").join("trip");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.path().join("document.pdf"), b"%PDF-1.4").unwrap();
        fs::write(nested.join("photo.JPG"), b"jpeg").unwrap();
        fs::write(nested.join("movie.mp4"), b"video").unwrap();
        let result = expand_paths(&[root.path().to_string_lossy().into_owned()]);
        assert_eq!(result.files.len(), 2);
        assert!(result.files.iter().any(|path| path.ends_with("photo.JPG")));
        assert_eq!(result.skipped_count, 1);
        assert!(result.issues[0].path.ends_with("movie.mp4"));
    }

    #[test]
    fn keeps_explicit_unknown_files_for_visible_engine_rejection() {
        let root = tempfile::tempdir().unwrap();
        let unknown = root.path().join("unknown.bin");
        fs::write(&unknown, b"unknown").unwrap();
        let result = expand_paths(&[unknown.to_string_lossy().into_owned()]);
        assert_eq!(result.files, vec![unknown.to_string_lossy().into_owned()]);
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn reports_missing_inputs_without_aborting_other_paths() {
        let root = tempfile::tempdir().unwrap();
        let valid = root.path().join("note.txt");
        fs::write(&valid, b"text").unwrap();
        let missing = root.path().join("missing");
        let result = expand_paths(&[
            missing.to_string_lossy().into_owned(),
            valid.to_string_lossy().into_owned(),
        ]);
        assert_eq!(result.files, vec![valid.to_string_lossy().into_owned()]);
        assert_eq!(result.skipped_count, 1);
        assert!(result.issues[0].reason.starts_with("无法读取"));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_directory_symlinks() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("photo.jpg"), b"jpeg").unwrap();
        let link = root.path().join("link");
        symlink(&target, &link).unwrap();
        let result = expand_paths(&[link.to_string_lossy().into_owned()]);
        assert!(result.files.is_empty());
        assert_eq!(result.issues[0].reason, "跳过符号链接");
    }
}
