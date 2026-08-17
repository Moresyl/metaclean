use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cleaners::{image, media, office, pdf, video, web_text},
    error::{display_path, CleanError, Result},
    models::{CleanResult, Finding, OutputMode, ScanReport},
    safe_io::{
        atomic_write_with_metadata, backup_path, cleaned_path, unique_path, validate_input,
        FileMetadataSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Jpeg,
    Png,
    Webp,
    Gif,
    Mp3,
    Wav,
    Flac,
    IsoMedia,
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

pub fn has_supported_extension(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "gif"
            | "mp3"
            | "wav"
            | "flac"
            | "mp4"
            | "mov"
            | "m4v"
            | "m4a"
            | "docx"
            | "xlsx"
            | "pptx"
            | "odt"
            | "pdf"
            | "txt"
            | "md"
            | "markdown"
            | "html"
            | "htm"
            | "svg"
            | "xml"
            | "json"
            | "csv"
    )
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
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Format::Gif;
    }
    if data.starts_with(b"ID3") || (data.len() >= 2 && data[0] == 0xff && data[1] & 0xe0 == 0xe0) {
        return Format::Mp3;
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        return Format::Wav;
    }
    if data.starts_with(b"fLaC") {
        return Format::Flac;
    }
    let ext = extension(path);
    if matches!(ext.as_str(), "mp4" | "mov" | "m4v" | "m4a") && video::is_iso_media(data) {
        return Format::IsoMedia;
    }
    if data.starts_with(b"%PDF-") {
        return Format::Pdf;
    }
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
        Format::Gif => "GIF",
        Format::Mp3 => "MP3",
        Format::Wav => "WAV",
        Format::Flac => "FLAC",
        Format::IsoMedia => "MP4 / QuickTime",
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
        Format::Gif => media::inspect_gif(data),
        Format::Mp3 => media::inspect_mp3(data),
        Format::Wav => media::inspect_wav(data),
        Format::Flac => media::inspect_flac(data),
        Format::IsoMedia => video::inspect(data),
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

fn clean_data(
    path: &Path,
    format: Format,
    data: &[u8],
    preserve_orientation: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    match format {
        Format::Jpeg => image::clean_jpeg_with_options(data, preserve_orientation),
        Format::Png => image::clean_png(data),
        Format::Webp => image::clean_webp(data),
        Format::Gif => media::clean_gif(data),
        Format::Mp3 => media::clean_mp3(data),
        Format::Wav => media::clean_wav(data),
        Format::Flac => media::clean_flac(data),
        Format::IsoMedia => video::clean(data),
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

pub fn clean_file_with_options(
    source: &Path,
    mode: &OutputMode,
    preserve_timestamps: bool,
    preserve_orientation: bool,
) -> CleanResult {
    let fail = |error: String| CleanResult {
        source_path: display_path(source),
        output_path: None,
        backup_path: None,
        removed: Vec::new(),
        success: false,
        error: Some(error),
    };
    let source_metadata = match validate_input(source) {
        Ok(metadata) => metadata,
        Err(error) => return fail(error.to_string()),
    };
    let metadata_snapshot = FileMetadataSnapshot::from_metadata(&source_metadata);
    let data = match fs::read(source) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let format = detect(source, &data);
    let (cleaned, removed) = match clean_data(source, format, &data, preserve_orientation) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let (output, backup): (PathBuf, Option<PathBuf>) = match mode {
        OutputMode::Copy => (unique_path(cleaned_path(source)), None),
        OutputMode::Replace => {
            let backup = unique_path(backup_path(source));
            if let Err(error) =
                atomic_write_with_metadata(&backup, &data, Some(&metadata_snapshot), true)
            {
                return fail(format!("创建备份失败：{error}"));
            }
            (source.to_owned(), Some(backup))
        }
    };
    if let Err(error) = atomic_write_with_metadata(
        &output,
        &cleaned,
        Some(&metadata_snapshot),
        preserve_timestamps,
    ) {
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

    fn chunk(kind: &[u8; 4], payload: &[u8], big_endian: bool) -> Vec<u8> {
        let mut output = Vec::new();
        let length = payload.len() as u32;
        if big_endian {
            output.extend_from_slice(&length.to_be_bytes());
            output.extend_from_slice(kind);
        } else {
            output.extend_from_slice(kind);
            output.extend_from_slice(&length.to_le_bytes());
        }
        output.extend_from_slice(payload);
        if !big_endian && payload.len() % 2 == 1 {
            output.push(0);
        }
        if big_endian {
            output.extend_from_slice(&[0, 0, 0, 0]);
        }
        output
    }

    fn supported_media_samples() -> Vec<(&'static str, Vec<u8>)> {
        fn atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
            bytes.extend_from_slice(kind);
            bytes.extend_from_slice(payload);
            bytes
        }

        let jpeg = vec![0xff, 0xd8, 0xff, 0xfe, 0, 5, b't', b'a', b'g', 0xff, 0xd9];

        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(chunk(b"tEXt", b"Author\0Alice", true));
        png.extend(chunk(b"IEND", b"", true));

        let mut webp = b"RIFF\0\0\0\0WEBP".to_vec();
        webp.extend_from_slice(b"EXIF\x04\0\0\0data");
        let webp_size = (webp.len() - 8) as u32;
        webp[4..8].copy_from_slice(&webp_size.to_le_bytes());

        let mut gif = b"GIF89a\x01\0\x01\0\0\0\0".to_vec();
        gif.extend_from_slice(b"\x21\xfe\x03tag\0\x3b");

        let mp3 = b"ID3\x04\0\0\0\0\0\x03tag\xff\xfb\x90\x64audio".to_vec();

        let mut wav = b"RIFF\0\0\0\0WAVE".to_vec();
        wav.extend(chunk(b"fmt ", &[1, 0, 1, 0], false));
        wav.extend(chunk(b"LIST", b"INFOIARTAlice", false));
        wav.extend(chunk(b"data", &[1, 2, 3, 4], false));
        let wav_size = (wav.len() - 8) as u32;
        wav[4..8].copy_from_slice(&wav_size.to_le_bytes());

        let mut flac = b"fLaC".to_vec();
        flac.extend_from_slice(&[0, 0, 0, 34]);
        flac.extend_from_slice(&[0; 34]);
        flac.extend_from_slice(&[0x84, 0, 0, 5]);
        flac.extend_from_slice(b"Alice");
        flac.extend_from_slice(b"\xff\xf8audio");

        let mut video = atom(b"ftyp", b"isom\0\0\0\0isommp42");
        let user_data = atom(b"udta", b"author=Alice;location=Shanghai");
        video.extend(atom(b"moov", &user_data));
        video.extend(atom(b"mdat", b"\0\0\0\x01VIDEO-FRAMES"));

        vec![
            ("photo.jpg", jpeg),
            ("graphic.png", png),
            ("graphic.webp", webp),
            ("animation.gif", gif),
            ("recording.mp3", mp3),
            ("recording.wav", wav),
            ("recording.flac", flac),
            ("movie.mp4", video),
        ]
    }

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
            ("photo.bin", b"GIF89a", Format::Gif, "GIF"),
            ("audio.bin", b"\xff\xfb", Format::Mp3, "MP3"),
            ("audio.bin", b"RIFF\x04\0\0\0WAVE", Format::Wav, "WAV"),
            ("audio.bin", b"fLaC", Format::Flac, "FLAC"),
            (
                "movie.mp4",
                b"\0\0\0\x18ftypisom\0\0\0\0isommp42",
                Format::IsoMedia,
                "MP4 / QuickTime",
            ),
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
    fn recognizes_every_supported_intake_extension() {
        for extension in [
            "jpg", "jpeg", "png", "webp", "gif", "mp3", "wav", "flac", "docx", "xlsx", "pptx",
            "odt", "pdf", "txt", "md", "markdown", "html", "htm", "svg", "xml", "json", "csv",
            "mp4", "mov", "m4v", "m4a",
        ] {
            assert!(has_supported_extension(Path::new(&format!(
                "file.{extension}"
            ))));
        }
        assert!(!has_supported_extension(Path::new("video.mkv")));
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
        let result = clean_file_with_options(&source, &OutputMode::Copy, true, true);
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(result.output_path.unwrap()).unwrap(),
            "ab"
        );
        assert_eq!(fs::read_to_string(source).unwrap(), "a\u{200b}b");
    }

    #[test]
    fn scans_and_cleans_every_native_media_format_through_the_engine() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in supported_media_samples() {
            let source = dir.path().join(name);
            fs::write(&source, bytes).unwrap();
            let report = scan_file(&source);
            assert!(report.supported, "{name}: {:?}", report.error);
            assert!(!report.findings.is_empty(), "{name}: expected metadata");
            let result = clean_file_with_options(&source, &OutputMode::Copy, true, true);
            assert!(result.success, "{name}: {:?}", result.error);
            let output = PathBuf::from(result.output_path.unwrap());
            assert!(output.exists());
            let cleaned_report = scan_file(&output);
            assert!(
                cleaned_report.supported,
                "{name}: {:?}",
                cleaned_report.error
            );
            assert!(
                cleaned_report.findings.is_empty(),
                "{name}: metadata remained"
            );
        }
    }

    #[test]
    fn malformed_supported_media_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("broken.png", b"\x89PNG\r\n\x1a\n".as_slice()),
            ("broken.gif", b"GIF89a".as_slice()),
            ("broken.mp3", b"ID3\x04".as_slice()),
            ("broken.wav", b"RIFF\0\0\0\0WAVE".as_slice()),
            ("broken.flac", b"fLaC".as_slice()),
            ("broken.mp4", b"\0\0\0\x18ftypisom".as_slice()),
        ] {
            let source = dir.path().join(name);
            fs::write(&source, bytes).unwrap();
            let report = scan_file(&source);
            assert!(!report.supported);
            assert!(report.error.is_some());
            assert!(!clean_file_with_options(&source, &OutputMode::Copy, true, true).success);
            assert!(!cleaned_path(&source).exists());
        }
    }
    #[test]
    fn replace_creates_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let result = clean_file_with_options(&source, &OutputMode::Replace, true, true);
        assert!(result.success);
        assert_eq!(fs::read_to_string(&source).unwrap(), "ab");
        assert_eq!(
            fs::read_to_string(result.backup_path.unwrap()).unwrap(),
            "a\u{200b}b"
        );
    }

    #[test]
    fn preserves_source_mtime_by_default_and_can_refresh_it() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let old = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(&source, old).unwrap();
        let preserved = clean_file_with_options(&source, &OutputMode::Copy, true, true);
        let preserved_metadata = fs::metadata(preserved.output_path.unwrap()).unwrap();
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&preserved_metadata),
            old
        );

        let refreshed = clean_file_with_options(&source, &OutputMode::Copy, false, true);
        let refreshed_metadata = fs::metadata(refreshed.output_path.unwrap()).unwrap();
        assert_ne!(
            filetime::FileTime::from_last_modification_time(&refreshed_metadata),
            old
        );
    }
    #[test]
    fn rejects_unknown_binary() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("data.bin");
        fs::write(&source, [0, 1, 2]).unwrap();
        assert!(!clean_file_with_options(&source, &OutputMode::Copy, true, true).success);
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
            let result = clean_file_with_options(&source, &OutputMode::Copy, true, true);
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
