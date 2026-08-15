use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cleaners::{image, office, pdf, web_text},
    error::{display_path, CleanError, Result},
    models::{CleanResult, Finding, OutputMode, ScanReport},
    safe_io::{atomic_write, backup_path, cleaned_path, unique_path, validate_input},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Jpeg,
    Png,
    Webp,
    Office,
    Pdf,
    Text,
    Unsupported,
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn detect(path: &Path, data: &[u8]) -> Format {
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Format::Jpeg;
    }
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Format::Png;
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Format::Webp;
    }
    if data.starts_with(b"%PDF-") {
        return Format::Pdf;
    }
    let ext = extension(path);
    if data.starts_with(b"PK") && matches!(ext.as_str(), "docx" | "xlsx" | "pptx" | "odt") {
        return Format::Office;
    }
    if matches!(
        ext.as_str(),
        "txt" | "md" | "markdown" | "html" | "htm" | "svg" | "xml" | "json" | "csv"
    ) && std::str::from_utf8(data).is_ok()
    {
        return Format::Text;
    }
    Format::Unsupported
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Jpeg => "JPEG",
        Format::Png => "PNG",
        Format::Webp => "WebP",
        Format::Office => "Office",
        Format::Pdf => "PDF",
        Format::Text => "Text",
        Format::Unsupported => "Unsupported",
    }
}

fn inspect_data(path: &Path, format: Format, data: &[u8]) -> Result<Vec<Finding>> {
    match format {
        Format::Jpeg => image::inspect_jpeg(data),
        Format::Png => image::inspect_png(data),
        Format::Webp => image::inspect_webp(data),
        Format::Office => office::inspect(data),
        Format::Pdf => pdf::inspect(data),
        Format::Text => Ok(web_text::inspect(
            std::str::from_utf8(data)
                .map_err(|_| CleanError::InvalidFormat("文本不是 UTF-8 编码".into()))?,
            &extension(path),
        )),
        Format::Unsupported => Err(CleanError::Unsupported("未知格式".into())),
    }
}

fn clean_data(path: &Path, format: Format, data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    match format {
        Format::Jpeg => image::clean_jpeg(data),
        Format::Png => image::clean_png(data),
        Format::Webp => image::clean_webp(data),
        Format::Office => office::clean(data),
        Format::Pdf => pdf::clean(data),
        Format::Text => {
            let (cleaned, findings) = web_text::clean(
                std::str::from_utf8(data)
                    .map_err(|_| CleanError::InvalidFormat("文本不是 UTF-8 编码".into()))?,
                &extension(path),
            );
            Ok((cleaned.into_bytes(), findings))
        }
        Format::Unsupported => Err(CleanError::Unsupported("未知格式".into())),
    }
}

pub fn scan_file(path: &Path) -> ScanReport {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("未知文件")
        .to_owned();
    let base = |format: String,
                size: u64,
                supported: bool,
                findings: Vec<Finding>,
                error: Option<String>| ScanReport {
        path: display_path(path),
        name: name.clone(),
        format,
        size,
        supported,
        findings,
        error,
    };
    let metadata = match validate_input(path) {
        Ok(value) => value,
        Err(error) => {
            return base(
                "Unknown".into(),
                0,
                false,
                Vec::new(),
                Some(error.to_string()),
            )
        }
    };
    let data = match fs::read(path) {
        Ok(value) => value,
        Err(error) => {
            return base(
                "Unknown".into(),
                metadata.len(),
                false,
                Vec::new(),
                Some(error.to_string()),
            )
        }
    };
    let format = detect(path, &data);
    match inspect_data(path, format, &data) {
        Ok(findings) => base(
            format_name(format).into(),
            metadata.len(),
            format != Format::Unsupported,
            findings,
            None,
        ),
        Err(error) => base(
            format_name(format).into(),
            metadata.len(),
            false,
            Vec::new(),
            Some(error.to_string()),
        ),
    }
}

pub fn clean_file(source: &Path, mode: &OutputMode) -> CleanResult {
    let fail = |error: String| CleanResult {
        source_path: display_path(source),
        output_path: None,
        backup_path: None,
        removed: Vec::new(),
        success: false,
        error: Some(error),
    };
    if let Err(error) = validate_input(source) {
        return fail(error.to_string());
    }
    let data = match fs::read(source) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let format = detect(source, &data);
    let (cleaned, removed) = match clean_data(source, format, &data) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let (output, backup): (PathBuf, Option<PathBuf>) = match mode {
        OutputMode::Copy => (unique_path(cleaned_path(source)), None),
        OutputMode::Replace => {
            let backup = unique_path(backup_path(source));
            if let Err(error) = atomic_write(&backup, &data) {
                return fail(format!("创建备份失败：{error}"));
            }
            (source.to_owned(), Some(backup))
        }
    };
    if let Err(error) = atomic_write(&output, &cleaned) {
        return CleanResult {
            source_path: display_path(source),
            output_path: None,
            backup_path: backup.as_deref().map(display_path),
            removed,
            success: false,
            error: Some(error.to_string()),
        };
    }
    CleanResult {
        source_path: display_path(source),
        output_path: Some(display_path(&output)),
        backup_path: backup.as_deref().map(display_path),
        removed,
        success: true,
        error: None,
    }
}

pub fn scan_paths(paths: &[String]) -> Vec<ScanReport> {
    paths
        .iter()
        .map(|path| scan_file(Path::new(path)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_all_supported_signatures_and_names() {
        let cases = [
            (
                "photo.bin",
                b"\xff\xd8\xffrest".as_slice(),
                Format::Jpeg,
                "JPEG",
            ),
            ("photo.bin", b"\x89PNG\r\n\x1a\nrest", Format::Png, "PNG"),
            ("photo.bin", b"RIFF\x04\0\0\0WEBP", Format::Webp, "WebP"),
            ("file.bin", b"%PDF-1.7", Format::Pdf, "PDF"),
            ("file.docx", b"PKarchive", Format::Office, "Office"),
            ("file.md", b"plain text", Format::Text, "Text"),
            ("file.bin", b"unknown", Format::Unsupported, "Unsupported"),
        ];
        for (name, data, expected, label) in cases {
            let detected = detect(Path::new(name), data);
            assert_eq!(detected, expected);
            assert_eq!(format_name(detected), label);
        }
        assert_eq!(extension(Path::new("PHOTO.JPEG")), "jpeg");
    }

    #[test]
    fn scan_reports_missing_invalid_and_multiple_paths() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.txt");
        let report = scan_file(&missing);
        assert!(!report.supported);
        assert!(report.error.is_some());

        let valid = dir.path().join("valid.txt");
        fs::write(&valid, "clean").unwrap();
        let paths = vec![
            valid.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];
        let reports = scan_paths(&paths);
        assert_eq!(reports.len(), 2);
        assert!(reports[0].supported);
        assert!(!reports[1].supported);
    }

    #[test]
    fn scan_and_clean_text_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let report = scan_file(&source);
        assert_eq!(report.findings[0].count, 1);
        let result = clean_file(&source, &OutputMode::Copy);
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(result.output_path.unwrap()).unwrap(),
            "ab"
        );
        assert_eq!(fs::read_to_string(source).unwrap(), "a\u{200b}b");
    }
    #[test]
    fn replace_creates_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let result = clean_file(&source, &OutputMode::Replace);
        assert!(result.success);
        assert_eq!(fs::read_to_string(&source).unwrap(), "ab");
        assert_eq!(
            fs::read_to_string(result.backup_path.unwrap()).unwrap(),
            "a\u{200b}b"
        );
    }
    #[test]
    fn rejects_unknown_binary() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("data.bin");
        fs::write(&source, [0, 1, 2]).unwrap();
        assert!(!clean_file(&source, &OutputMode::Copy).success);
    }

    #[test]
    #[ignore = "requires METACLEAN_OFFICE_SAMPLE_DIR with DOCX/XLSX/PPTX/ODT fixtures"]
    fn cleans_external_office_validation_samples() {
        let root = PathBuf::from(
            std::env::var("METACLEAN_OFFICE_SAMPLE_DIR")
                .expect("METACLEAN_OFFICE_SAMPLE_DIR must point to validation fixtures"),
        );
        for name in ["sample.docx", "sample.xlsx", "sample.pptx", "sample.odt"] {
            let source = root.join(name);
            let report = scan_file(&source);
            assert!(report.supported, "{}: {:?}", name, report.error);
            assert!(
                !report.findings.is_empty(),
                "{name}: expected metadata findings"
            );
            let result = clean_file(&source, &OutputMode::Copy);
            assert!(result.success, "{}: {:?}", name, result.error);
            let output = PathBuf::from(result.output_path.expect("cleaned output path"));
            assert!(
                output.exists(),
                "{}: output was not written",
                output.display()
            );
        }
    }
}
