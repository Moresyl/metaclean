use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

#[cfg(test)]
use std::fs;

use crate::{
    cleaners::{asf, avi, bmp, heif, image, jxl, media, mkv, office, pdf, tiff, video, web_text},
    error::{display_path, CleanError, Result},
    models::{CleanResult, Finding, FindingSeverity, OutputMode, ScanReport},
    safe_io::{
        atomic_create_unique_with_metadata, atomic_replace_if_unchanged, backup_path, cleaned_path,
        ensure_source_unchanged, privacy_extended_attribute_count, read_validated_input,
        remove_created_output, FileMetadataSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Jpeg,
    Png,
    Webp,
    Jxl,
    Gif,
    Bmp,
    Tiff,
    Raw,
    Raf,
    Heif,
    CanonRaw,
    Mp3,
    Wav,
    Flac,
    Aiff,
    IsoMedia,
    Avi,
    Asf,
    Matroska,
    Office,
    Pdf,
    Text,
    Unsupported,
}

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg",
    "jpeg",
    "jpe",
    "png",
    "webp",
    "jxl",
    "gif",
    "bmp",
    "dib",
    "tif",
    "tiff",
    "heic",
    "heif",
    "heics",
    "heifs",
    "hif",
    "avif",
    "avifs",
    "cr2",
    "cr3",
    "crw",
    "nef",
    "nrw",
    "arw",
    "srf",
    "sr2",
    "orf",
    "rw2",
    "rwl",
    "dng",
    "pef",
    "srw",
    "raf",
    "3fr",
    "erf",
    "mef",
    "mos",
    "iiq",
    "kdc",
    "dcr",
    "k25",
    "mp3",
    "wav",
    "flac",
    "aif",
    "aiff",
    "aifc",
    "mp4",
    "mov",
    "m4v",
    "m4a",
    "3g2",
    "3gp",
    "3gp2",
    "3gpp",
    "f4a",
    "f4b",
    "f4p",
    "f4v",
    "lrv",
    "m4b",
    "m4p",
    "mqv",
    "qt",
    "avi",
    "asf",
    "wmv",
    "wma",
    "mkv",
    "mka",
    "mks",
    "mk3d",
    "webm",
    "docx",
    "xlsx",
    "pptx",
    "odt",
    "ods",
    "odp",
    "odg",
    "odf",
    "odb",
    "odm",
    "ott",
    "ots",
    "otp",
    "otg",
    "epub",
    "pdf",
    "txt",
    "md",
    "markdown",
    "html",
    "htm",
    "xhtml",
    "svg",
    "xml",
    "json",
    "csv",
    "tsv",
    "yaml",
    "yml",
    "log",
    "srt",
    "vtt",
    "css",
    "scss",
    "less",
    "ini",
    "conf",
    "cfg",
    "toml",
    "properties",
];

const ISO_MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "m4a", "3g2", "3gp", "3gp2", "3gpp", "f4a", "f4b", "f4p", "f4v", "lrv",
    "m4b", "m4p", "mqv", "qt",
];

/// Raw negatives that are TIFF containers underneath a private magic word, plus
/// Canon's CR3, which is an ISO base media file instead. They are held apart
/// from ordinary TIFF because a raw decoder needs the camera model to dispatch,
/// so the model stays where an ordinary image would lose it.
const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "rwl", "dng", "pef", "srw",
    "3fr", "erf", "mef", "mos", "iiq", "kdc", "dcr", "k25",
];

const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "md",
    "markdown",
    "html",
    "htm",
    "xhtml",
    "svg",
    "xml",
    "json",
    "csv",
    "tsv",
    "yaml",
    "yml",
    "log",
    "srt",
    "vtt",
    "css",
    "scss",
    "less",
    "ini",
    "conf",
    "cfg",
    "toml",
    "properties",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextEncoding {
    Utf8 { bom: bool },
    Utf16Le,
    Utf16Be,
}

fn text_encoding(data: &[u8]) -> Result<TextEncoding> {
    if data.starts_with(&[0xff, 0xfe]) {
        if !(data.len() - 2).is_multiple_of(2) {
            return Err(CleanError::InvalidFormat("UTF-16 LE 文本长度无效".into()));
        }
        let valid = char::decode_utf16(
            data[2..]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
        )
        .all(|character| character.is_ok());
        return valid
            .then_some(TextEncoding::Utf16Le)
            .ok_or_else(|| CleanError::InvalidFormat("UTF-16 LE 文本包含无效字符".into()));
    }
    if data.starts_with(&[0xfe, 0xff]) {
        if !(data.len() - 2).is_multiple_of(2) {
            return Err(CleanError::InvalidFormat("UTF-16 BE 文本长度无效".into()));
        }
        let valid = char::decode_utf16(
            data[2..]
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]])),
        )
        .all(|character| character.is_ok());
        return valid
            .then_some(TextEncoding::Utf16Be)
            .ok_or_else(|| CleanError::InvalidFormat("UTF-16 BE 文本包含无效字符".into()));
    }
    let (body, bom) = data
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .map_or((data, false), |value| (value, true));
    std::str::from_utf8(body)
        .map(|_| TextEncoding::Utf8 { bom })
        .map_err(|_| {
            CleanError::InvalidFormat("文本不是有效的 UTF-8 或带 BOM 的 UTF-16 编码".into())
        })
}

fn decode_text(data: &[u8]) -> Result<(String, TextEncoding)> {
    let encoding = text_encoding(data)?;
    let value = match encoding {
        TextEncoding::Utf8 { bom: true } => std::str::from_utf8(&data[3..])
            .map(str::to_owned)
            .map_err(|_| {
                CleanError::InvalidFormat("文本不是有效的 UTF-8 或带 BOM 的 UTF-16 编码".into())
            })?,
        TextEncoding::Utf8 { bom: false } => {
            std::str::from_utf8(data).map(str::to_owned).map_err(|_| {
                CleanError::InvalidFormat("文本不是有效的 UTF-8 或带 BOM 的 UTF-16 编码".into())
            })?
        }
        TextEncoding::Utf16Le => decode_utf16(data, true)?,
        TextEncoding::Utf16Be => decode_utf16(data, false)?,
    };
    Ok((value, encoding))
}

fn decode_utf16(data: &[u8], little_endian: bool) -> Result<String> {
    let decoded = char::decode_utf16(data[2..].chunks_exact(2).map(|pair| {
        if little_endian {
            u16::from_le_bytes([pair[0], pair[1]])
        } else {
            u16::from_be_bytes([pair[0], pair[1]])
        }
    }))
    .collect::<std::result::Result<String, _>>();
    decoded.map_err(|_| {
        CleanError::InvalidFormat(
            if little_endian {
                "UTF-16 LE 文本包含无效字符"
            } else {
                "UTF-16 BE 文本包含无效字符"
            }
            .into(),
        )
    })
}

fn encode_text(value: &str, encoding: TextEncoding) -> Vec<u8> {
    match encoding {
        TextEncoding::Utf8 { bom: true } => {
            let mut output = vec![0xef, 0xbb, 0xbf];
            output.extend_from_slice(value.as_bytes());
            output
        }
        TextEncoding::Utf8 { bom: false } => value.as_bytes().to_vec(),
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            let mut output = if encoding == TextEncoding::Utf16Le {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in value.encode_utf16() {
                let bytes = if encoding == TextEncoding::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                output.extend_from_slice(&bytes);
            }
            output
        }
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn has_supported_extension(path: &Path) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension(path).as_str())
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
    if extension(path) == "jxl" && jxl::is_jxl(data) {
        return Format::Jxl;
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Format::Gif;
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        return Format::Wav;
    }
    if avi::is_avi(data) {
        return Format::Avi;
    }
    if media::is_flac(data) {
        return Format::Flac;
    }
    if media::is_aiff(data) {
        return Format::Aiff;
    }
    if asf::is_asf(data) {
        return Format::Asf;
    }
    if mkv::is_matroska(data) {
        return Format::Matroska;
    }
    let ext = extension(path);
    if tiff::is_raf(data) {
        return Format::Raf;
    }
    if tiff::is_tiff(data) {
        return if RAW_EXTENSIONS.contains(&ext.as_str()) {
            Format::Raw
        } else {
            Format::Tiff
        };
    }
    // A CR3 is an ISO base media file like a HEIC and cleans the same way, but
    // it is a negative and deserves to be named as one in the report.
    if heif::is_canon_raw(data) {
        return Format::CanonRaw;
    }
    if heif::is_heif(data) {
        return Format::Heif;
    }
    if ISO_MEDIA_EXTENSIONS.contains(&ext.as_str()) && video::is_iso_media(data) {
        return Format::IsoMedia;
    }
    if bmp::is_bmp(data) {
        return Format::Bmp;
    }
    // MP3 is last of the binary formats: even a validated frame header is a
    // short signature which plenty of other containers can contain by chance.
    if media::is_mp3(data) {
        return Format::Mp3;
    }
    if data.starts_with(b"%PDF-") {
        return Format::Pdf;
    }
    if office::is_supported_container(data, &ext) {
        return Format::Office;
    }
    if TEXT_EXTENSIONS.contains(&ext.as_str()) && text_encoding(data).is_ok() {
        return Format::Text;
    }
    Format::Unsupported
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Jpeg => "JPEG",
        Format::Png => "PNG",
        Format::Webp => "WebP",
        Format::Jxl => "JPEG XL",
        Format::Gif => "GIF",
        Format::Bmp => "BMP",
        Format::Tiff => "TIFF",
        Format::Raw => "RAW",
        Format::Raf => "RAF",
        Format::Heif => "HEIF / AVIF",
        Format::CanonRaw => "Canon CR3",
        Format::Mp3 => "MP3",
        Format::Wav => "WAV",
        Format::Flac => "FLAC",
        Format::Aiff => "AIFF",
        Format::IsoMedia => "MP4 / QuickTime",
        Format::Avi => "AVI",
        Format::Asf => "WMV / ASF",
        Format::Matroska => "Matroska / WebM",
        Format::Office => "Office",
        Format::Pdf => "PDF",
        Format::Text => "Text",
        Format::Unsupported => "Unsupported",
    }
}

fn extended_attribute_finding(count: usize) -> Option<Finding> {
    (count > 0).then(|| Finding {
        category: "macos_xattr".into(),
        label: "macOS provenance attributes".into(),
        count,
        severity: FindingSeverity::Informational,
    })
}

fn inspect_data(path: &Path, format: Format, data: &[u8]) -> Result<Vec<Finding>> {
    match format {
        Format::Jpeg => image::inspect_jpeg(data),
        Format::Png => image::inspect_png(data),
        Format::Webp => image::inspect_webp(data),
        Format::Jxl => jxl::inspect(data),
        Format::Gif => media::inspect_gif(data),
        Format::Bmp => bmp::inspect(data),
        Format::Tiff => tiff::inspect_tiff(data, false),
        Format::Raw => tiff::inspect_tiff(data, true),
        Format::Raf => tiff::inspect_raf(data),
        Format::Heif | Format::CanonRaw => heif::inspect(data),
        Format::Mp3 => media::inspect_mp3(data),
        Format::Wav => media::inspect_wav(data),
        Format::Flac => media::inspect_flac(data),
        Format::Aiff => media::inspect_aiff(data),
        Format::IsoMedia => video::inspect(data),
        Format::Avi => avi::inspect(data),
        Format::Asf => asf::inspect(data),
        Format::Matroska => mkv::inspect(data),
        Format::Office => office::inspect(data, &extension(path)),
        Format::Pdf => pdf::inspect(data),
        Format::Text => {
            let (value, _) = decode_text(data)?;
            Ok(web_text::inspect(&value, &extension(path)))
        }
        Format::Unsupported => Err(CleanError::Unsupported("未知格式".into())),
    }
}

fn clean_data(
    path: &Path,
    format: Format,
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    match format {
        Format::Jpeg => {
            image::clean_jpeg_with_options(data, preserve_orientation, preserve_color_profile)
        }
        Format::Png => image::clean_png_with_options(data, preserve_color_profile),
        Format::Webp => image::clean_webp_with_options(data, preserve_color_profile),
        Format::Jxl => jxl::clean(data),
        Format::Gif => media::clean_gif(data),
        Format::Bmp => bmp::clean(data, preserve_color_profile),
        Format::Tiff => {
            tiff::clean_tiff_with_options(data, false, preserve_orientation, preserve_color_profile)
        }
        Format::Raw => {
            tiff::clean_tiff_with_options(data, true, preserve_orientation, preserve_color_profile)
        }
        Format::Raf => {
            tiff::clean_raf_with_options(data, preserve_orientation, preserve_color_profile)
        }
        Format::Heif | Format::CanonRaw => heif::clean(data),
        Format::Mp3 => media::clean_mp3(data),
        Format::Wav => media::clean_wav(data),
        Format::Flac => media::clean_flac(data),
        Format::Aiff => media::clean_aiff(data),
        Format::IsoMedia => video::clean(data),
        Format::Avi => avi::clean(data),
        Format::Asf => asf::clean(data),
        Format::Matroska => mkv::clean(data),
        Format::Office => office::clean(data, &extension(path)),
        Format::Pdf => pdf::clean(data),
        Format::Text => {
            let (value, encoding) = decode_text(data)?;
            let (cleaned, findings) = web_text::clean(&value, &extension(path));
            Ok((encode_text(&cleaned, encoding), findings))
        }
        Format::Unsupported => Err(CleanError::Unsupported("未知格式".into())),
    }
}

fn verify_cleaned_data(
    path: &Path,
    expected_format: Format,
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<()> {
    let detected_format = detect(path, data);
    if detected_format != expected_format {
        return Err(CleanError::Verification(format!(
            "输出格式从 {} 变为 {}",
            format_name(expected_format),
            format_name(detected_format)
        )));
    }
    match expected_format {
        Format::Jpeg => {
            image::verify_jpeg_cleaned(data, preserve_orientation, preserve_color_profile)
        }
        Format::Png => image::verify_png_cleaned(data, preserve_color_profile),
        Format::Webp => image::verify_webp_cleaned(data, preserve_color_profile),
        Format::Jxl => jxl::verify_cleaned(data),
        Format::Bmp => bmp::verify_cleaned(data, preserve_color_profile),
        Format::Tiff => {
            tiff::verify_tiff_cleaned(data, false, preserve_orientation, preserve_color_profile)
        }
        Format::Raw => {
            tiff::verify_tiff_cleaned(data, true, preserve_orientation, preserve_color_profile)
        }
        Format::Raf => tiff::verify_raf_cleaned(data, preserve_orientation, preserve_color_profile),
        Format::Heif | Format::CanonRaw => heif::verify_cleaned(data),
        Format::Avi => avi::verify_cleaned(data),
        Format::Asf => asf::verify_cleaned(data),
        Format::Matroska => mkv::verify_cleaned(data),
        Format::Unsupported => Err(CleanError::Unsupported("未知格式".into())),
        _ => {
            let residual = inspect_data(path, expected_format, data)?;
            if residual.is_empty() {
                Ok(())
            } else {
                let count = residual.iter().map(|finding| finding.count).sum::<usize>();
                Err(CleanError::Verification(format!(
                    "{} 中仍发现 {count} 项应移除的痕迹",
                    format_name(expected_format)
                )))
            }
        }
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
    let (metadata, data) = match read_validated_input(path) {
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
    let format = detect(path, &data);
    match inspect_data(path, format, &data) {
        Ok(mut findings) => match privacy_extended_attribute_count(path) {
            Ok(count) => {
                if let Some(finding) = extended_attribute_finding(count) {
                    findings.push(finding);
                }
                base(
                    format_name(format).into(),
                    metadata.len(),
                    format != Format::Unsupported,
                    findings,
                    None,
                )
            }
            Err(error) => base(
                format_name(format).into(),
                metadata.len(),
                false,
                findings,
                Some(error.to_string()),
            ),
        },
        Err(error) => base(
            format_name(format).into(),
            metadata.len(),
            false,
            Vec::new(),
            Some(error.to_string()),
        ),
    }
}

fn scanner_failure_report(path: &str) -> ScanReport {
    ScanReport {
        path: display_path(Path::new(path)),
        name: Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("未知文件")
            .to_owned(),
        format: "Unknown".into(),
        size: 0,
        supported: false,
        findings: Vec::new(),
        error: Some("内部扫描器发生异常，已安全隔离该文件".into()),
    }
}

fn scan_path_isolated_with(path: &str, scan: impl FnOnce(&Path) -> ScanReport) -> ScanReport {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan(Path::new(path))))
        .unwrap_or_else(|_| scanner_failure_report(path))
}

fn scan_path_isolated(path: &str) -> ScanReport {
    scan_path_isolated_with(path, scan_file)
}

fn cleaner_failure_result(path: &Path) -> CleanResult {
    CleanResult {
        source_path: display_path(path),
        output_path: None,
        backup_path: None,
        source_size: None,
        output_size: None,
        removed: Vec::new(),
        success: false,
        error: Some("内部清理器发生异常，已安全隔离该文件；请检查输出与备份状态".into()),
    }
}

fn clean_path_isolated_with(path: &Path, clean: impl FnOnce(&Path) -> CleanResult) -> CleanResult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| clean(path)))
        .unwrap_or_else(|_| cleaner_failure_result(path))
}

pub fn clean_file_isolated_with_options(
    source: &Path,
    mode: &OutputMode,
    preserve_timestamps: bool,
    preserve_orientation: bool,
    preserve_color_profile: bool,
    remove_extended_attributes: bool,
) -> CleanResult {
    clean_path_isolated_with(source, |path| {
        clean_file_with_options(
            path,
            mode,
            preserve_timestamps,
            preserve_orientation,
            preserve_color_profile,
            remove_extended_attributes,
        )
    })
}

pub fn clean_file_with_options(
    source: &Path,
    mode: &OutputMode,
    preserve_timestamps: bool,
    preserve_orientation: bool,
    preserve_color_profile: bool,
    remove_extended_attributes: bool,
) -> CleanResult {
    let fail = |error: String| CleanResult {
        source_path: display_path(source),
        output_path: None,
        backup_path: None,
        source_size: None,
        output_size: None,
        removed: Vec::new(),
        success: false,
        error: Some(error),
    };
    let (source_metadata, data) = match read_validated_input(source) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let metadata_snapshot = match FileMetadataSnapshot::capture(source, &source_metadata) {
        Ok(snapshot) => snapshot,
        Err(error) => return fail(error.to_string()),
    };
    if matches!(mode, OutputMode::Replace) && source_metadata.permissions().readonly() {
        return fail(CleanError::ReadOnly(display_path(source)).to_string());
    }
    let format = detect(source, &data);
    let (cleaned, mut removed) = match clean_data(
        source,
        format,
        &data,
        preserve_orientation,
        preserve_color_profile,
    ) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
    let extended_attribute_count = metadata_snapshot.privacy_extended_attribute_count();
    if remove_extended_attributes {
        if let Some(finding) = extended_attribute_finding(extended_attribute_count) {
            removed.push(finding);
        }
    }
    if let Err(error) = verify_cleaned_data(
        source,
        format,
        &cleaned,
        preserve_orientation,
        preserve_color_profile,
    ) {
        return fail(error.to_string());
    }
    let (output, backup): (PathBuf, Option<PathBuf>) = match mode {
        OutputMode::Copy => {
            if let Err(error) = ensure_source_unchanged(source, &data, &metadata_snapshot) {
                return fail(error.to_string());
            }
            let output = match atomic_create_unique_with_metadata(
                &cleaned_path(source),
                &cleaned,
                Some(&metadata_snapshot),
                preserve_timestamps,
                remove_extended_attributes,
            ) {
                Ok(path) => path,
                Err(error) => return fail(error.to_string()),
            };
            if let Err(error) = ensure_source_unchanged(source, &data, &metadata_snapshot) {
                remove_created_output(&output);
                return fail(error.to_string());
            }
            (output, None)
        }
        OutputMode::Replace => {
            if let Err(error) = ensure_source_unchanged(source, &data, &metadata_snapshot) {
                return fail(error.to_string());
            }
            let backup = match atomic_create_unique_with_metadata(
                &backup_path(source),
                &data,
                Some(&metadata_snapshot),
                true,
                false,
            ) {
                Ok(path) => path,
                Err(error) => return fail(format!("创建备份失败：{error}")),
            };
            (source.to_owned(), Some(backup))
        }
    };
    let write_result = match mode {
        OutputMode::Copy => Ok(()),
        OutputMode::Replace => atomic_replace_if_unchanged(
            &output,
            &data,
            &cleaned,
            &metadata_snapshot,
            preserve_timestamps,
            remove_extended_attributes,
        ),
    };
    if let Err(error) = write_result {
        return CleanResult {
            source_path: display_path(source),
            output_path: None,
            backup_path: backup.as_deref().map(display_path),
            source_size: Some(data.len() as u64),
            output_size: None,
            removed,
            success: false,
            error: Some(error.to_string()),
        };
    }
    CleanResult {
        source_path: display_path(source),
        output_path: Some(display_path(&output)),
        backup_path: backup.as_deref().map(display_path),
        source_size: Some(data.len() as u64),
        output_size: Some(cleaned.len() as u64),
        removed,
        success: true,
        error: None,
    }
}

#[cfg(test)]
pub fn scan_paths(paths: &[String]) -> Vec<ScanReport> {
    scan_paths_with_cancellation(paths, None, None)
}

#[cfg(test)]
pub fn scan_paths_cancellable(paths: &[String], cancellation: &AtomicBool) -> Vec<ScanReport> {
    scan_paths_with_cancellation(paths, Some(cancellation), None)
}

pub fn scan_paths_cancellable_with_progress(
    paths: &[String],
    cancellation: &AtomicBool,
    progress: &(dyn Fn(&ScanReport) + Send + Sync),
) -> Vec<ScanReport> {
    scan_paths_with_cancellation(paths, Some(cancellation), Some(progress))
}

fn scan_paths_with_cancellation(
    paths: &[String],
    cancellation: Option<&AtomicBool>,
    progress: Option<&(dyn Fn(&ScanReport) + Send + Sync)>,
) -> Vec<ScanReport> {
    const MAX_SCAN_WORKERS: usize = 2;
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_SCAN_WORKERS)
        .min(paths.len());
    scan_paths_with_workers(paths, cancellation, progress, workers, &scan_path_isolated)
}

fn scan_paths_with_workers(
    paths: &[String],
    cancellation: Option<&AtomicBool>,
    progress: Option<&(dyn Fn(&ScanReport) + Send + Sync)>,
    workers: usize,
    scan: &(dyn Fn(&str) -> ScanReport + Sync),
) -> Vec<ScanReport> {
    if workers <= 1 {
        return paths
            .iter()
            .take_while(|_| !cancellation.is_some_and(|token| token.load(Ordering::SeqCst)))
            .map(|path| {
                let report = scan(path);
                if let Some(progress) = progress {
                    progress(&report);
                }
                report
            })
            .collect();
    }
    // Keep a small dynamic work queue so one slow input cannot leave another
    // scanner idle. Reports are stored by input index to preserve the queue's
    // stable order for the frontend and history layer.
    let next_index = AtomicUsize::new(0);
    let reports = Mutex::new((0..paths.len()).map(|_| None).collect::<Vec<_>>());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                if cancellation.is_some_and(|token| token.load(Ordering::SeqCst)) {
                    break;
                }
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(index) else {
                    break;
                };
                let report = scan(path);
                if let Some(progress) = progress {
                    progress(&report);
                }
                let Ok(mut reports) = reports.lock() else {
                    break;
                };
                reports[index] = Some(report);
            });
        }
    });
    reports
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .into_iter()
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::Instant};
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn builds_only_nonempty_extended_attribute_findings() {
        assert!(extended_attribute_finding(0).is_none());
        let finding = extended_attribute_finding(2).unwrap();
        assert_eq!(finding.category, "macos_xattr");
        assert_eq!(finding.count, 2);
        assert_eq!(finding.severity, FindingSeverity::Informational);
    }

    #[test]
    fn isolates_scanner_panics_without_exposing_the_panic_payload() {
        let path = "private-document.pdf";
        let report = scan_path_isolated_with(path, |_| panic!("secret parser state"));

        assert_eq!(report.name, path);
        assert!(!report.supported);
        let error = report.error.unwrap();
        assert!(error.contains("安全隔离"));
        assert!(!error.contains("secret parser state"));
    }

    #[test]
    fn isolates_cleaner_panics_without_writing_or_exposing_the_payload() {
        let path = Path::new("private-document.pdf");
        let result = clean_path_isolated_with(path, |_| panic!("secret parser state"));

        assert_eq!(result.source_path, "private-document.pdf");
        assert!(!result.success);
        assert!(result.output_path.is_none());
        assert!(result.backup_path.is_none());
        let error = result.error.unwrap();
        assert!(error.contains("安全隔离"));
        assert!(!error.contains("secret parser state"));
    }

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
            output.extend_from_slice(&image::png_crc32(&output[4..]).to_be_bytes());
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

        fn aiff_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut bytes = kind.to_vec();
            bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            bytes.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                bytes.push(0);
            }
            bytes
        }

        let jpeg = vec![0xff, 0xd8, 0xff, 0xfe, 0, 5, b't', b'a', b'g', 0xff, 0xd9];

        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(chunk(
            b"IHDR",
            &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0],
            true,
        ));
        png.extend(chunk(b"tEXt", b"Author\0Alice", true));
        png.extend(chunk(b"IDAT", b"pixels", true));
        png.extend(chunk(b"IEND", b"", true));

        let mut webp = b"RIFF\0\0\0\0WEBP".to_vec();
        webp.extend_from_slice(b"EXIF\x04\0\0\0data");
        webp.extend(chunk(b"VP8 ", b"image", false));
        let webp_size = (webp.len() - 8) as u32;
        webp[4..8].copy_from_slice(&webp_size.to_le_bytes());

        let mut jxl = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
        jxl.extend(atom(b"ftyp", b"jxl \0\0\0\0jxl "));
        jxl.extend(atom(b"Exif", b"\0\0\0\0II*\0Alice"));
        jxl.extend(atom(b"jxlc", b"\xff\x0aIMAGE-CODESTREAM"));

        let mut gif = b"GIF89a\x01\0\x01\0\0\0\0".to_vec();
        gif.extend_from_slice(b"\x21\xfe\x03tag\0\x3b");

        let mut mp3 = b"ID3\x04\0\0\0\0\0\x03tag".to_vec();
        let mut mp3_frame = vec![0xff, 0xfb, 0x90, 0x64];
        mp3_frame.resize(417, 0);
        mp3.extend(mp3_frame);

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

        let mut aiff = b"FORM\0\0\0\0AIFF".to_vec();
        let mut aiff_common = [0; 18];
        aiff_common[1] = 1;
        aiff.extend(aiff_chunk(b"COMM", &aiff_common));
        aiff.extend(aiff_chunk(b"AUTH", b"Alice"));
        aiff.extend(aiff_chunk(b"SSND", b"\0\0\0\0\0\0\0\0AUDIO-SAMPLES"));
        let aiff_size = (aiff.len() - 8) as u32;
        aiff[4..8].copy_from_slice(&aiff_size.to_be_bytes());

        let mut video = atom(b"ftyp", b"isom\0\0\0\0isommp42");
        let user_data = atom(b"udta", b"author=Alice;location=Shanghai");
        video.extend(atom(b"moov", &user_data));
        video.extend(atom(b"mdat", b"\0\0\0\x01VIDEO-FRAMES"));

        // Two pixels behind a core info header, with a note stapled past the end
        // where no viewer would ever show it.
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&30u32.to_le_bytes());
        bmp.extend_from_slice(&[0, 0, 0, 0]);
        bmp.extend_from_slice(&26u32.to_le_bytes());
        bmp.extend_from_slice(&12u32.to_le_bytes());
        bmp.extend_from_slice(&1i16.to_le_bytes());
        bmp.extend_from_slice(&1i16.to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&[0, 0, 0, 0]);
        bmp.extend_from_slice(b"Exif\0\0Alice");

        let tiff = tiff_sample();

        let mut avi = b"RIFF\0\0\0\0AVI ".to_vec();
        avi.extend(chunk(b"IART", b"Alice Zhang\0", false));
        avi.extend(chunk(b"LIST", b"movi00dcFRAME", false));
        let avi_size = (avi.len() - 8) as u32;
        avi[4..8].copy_from_slice(&avi_size.to_le_bytes());

        let matroska = matroska_sample();

        vec![
            ("photo.jpg", jpeg),
            ("graphic.png", png),
            ("graphic.webp", webp),
            ("graphic.jxl", jxl),
            ("animation.gif", gif),
            ("photo.bmp", bmp),
            ("photo.tif", tiff.clone()),
            ("photo.nef", tiff),
            ("recording.mp3", mp3),
            ("recording.wav", wav),
            ("recording.flac", flac),
            ("recording.aiff", aiff),
            ("movie.mp4", video),
            ("movie.avi", avi),
            ("movie.mkv", matroska),
        ]
    }

    fn office_sample(extension: &str) -> Vec<u8> {
        let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        match extension {
            "docx" => {
                writer.start_file("[Content_Types].xml", options).unwrap();
                writer.write_all(b"<Types/>").unwrap();
                writer.start_file("word/document.xml", options).unwrap();
                writer.write_all(b"<w:document/>").unwrap();
            }
            "epub" => {
                writer.start_file("mimetype", stored).unwrap();
                writer.write_all(b"application/epub+zip").unwrap();
                writer
                    .start_file("META-INF/container.xml", options)
                    .unwrap();
                writer.write_all(b"<container/>").unwrap();
            }
            "ods" => {
                writer.start_file("mimetype", stored).unwrap();
                writer
                    .write_all(b"application/vnd.oasis.opendocument.spreadsheet")
                    .unwrap();
                writer.start_file("content.xml", options).unwrap();
                writer.write_all(b"<office:document/>").unwrap();
            }
            _ => unreachable!(),
        }
        writer.finish().unwrap().into_inner()
    }

    /// A little endian TIFF whose single directory holds nothing but the two
    /// strings a camera signs its work with.
    fn tiff_sample() -> Vec<u8> {
        let entry = |tag: u16, value: &[u8; 4]| {
            let mut bytes = tag.to_le_bytes().to_vec();
            bytes.extend_from_slice(&2u16.to_le_bytes());
            bytes.extend_from_slice(&4u32.to_le_bytes());
            bytes.extend_from_slice(value);
            bytes
        };
        let mut file = b"II\x2a\x00".to_vec();
        file.extend_from_slice(&8u32.to_le_bytes());
        file.extend_from_slice(&2u16.to_le_bytes());
        file.extend(entry(0x0131, b"Bob\0"));
        file.extend(entry(0x010e, b"Cam\0"));
        file.extend_from_slice(&0u32.to_le_bytes());
        file
    }

    /// EBML in miniature: a segment carrying the writing application and a tag
    /// block, over a cluster that must survive untouched.
    fn matroska_sample() -> Vec<u8> {
        fn element(id: &[u8], payload: &[u8]) -> Vec<u8> {
            let mut bytes = id.to_vec();
            let mut length = (payload.len() as u32).to_be_bytes().to_vec();
            length[0] |= 0x10;
            bytes.extend(length);
            bytes.extend_from_slice(payload);
            bytes
        }
        let info = element(&[0x57, 0x41], b"mkvmerge on alice-laptop");
        let tags = element(
            &[0x12, 0x54, 0xc3, 0x67],
            b"<SimpleTag>ARTIST=Alice</SimpleTag>",
        );
        let mut segment = element(&[0x15, 0x49, 0xa9, 0x66], &info);
        segment.extend(element(&[0x1f, 0x43, 0xb6, 0x75], b"VIDEO-FRAMES"));
        segment.extend(tags);
        let mut file = element(&[0x1a, 0x45, 0xdf, 0xa3], b"\x42\x82\x88matroska");
        file.extend(element(&[0x18, 0x53, 0x80, 0x67], &segment));
        file
    }

    #[test]
    fn detects_all_supported_signatures_and_names() {
        let aiff_chunk = |kind: &[u8; 4], payload: &[u8]| {
            let mut bytes = kind.to_vec();
            bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            bytes.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                bytes.push(0);
            }
            bytes
        };
        let mut aiff = b"FORM\0\0\0\0AIFF".to_vec();
        let mut common = [0; 18];
        common[1] = 1;
        aiff.extend(aiff_chunk(b"COMM", &common));
        aiff.extend(aiff_chunk(b"SSND", b"\0\0\0\0\0\0\0\0"));
        let aiff_size = (aiff.len() - 8) as u32;
        aiff[4..8].copy_from_slice(&aiff_size.to_be_bytes());
        let mut mp3 = vec![0xff, 0xfb, 0x90, 0x64];
        mp3.resize(417, 0);
        let cases = [
            (
                "photo.bin",
                b"\xff\xd8\xffrest".as_slice(),
                Format::Jpeg,
                "JPEG",
            ),
            ("photo.bin", b"\x89PNG\r\n\x1a\nrest", Format::Png, "PNG"),
            ("photo.bin", b"RIFF\x04\0\0\0WEBP", Format::Webp, "WebP"),
            ("photo.jxl", b"\xff\x0arest", Format::Jxl, "JPEG XL"),
            ("photo.bin", b"GIF89a", Format::Gif, "GIF"),
            ("audio.bin", mp3.as_slice(), Format::Mp3, "MP3"),
            ("audio.bin", b"RIFF\x04\0\0\0WAVE", Format::Wav, "WAV"),
            ("audio.bin", b"fLaC", Format::Flac, "FLAC"),
            ("audio.aiff", aiff.as_slice(), Format::Aiff, "AIFF"),
            (
                "movie.mp4",
                b"\0\0\0\x18ftypisom\0\0\0\0isommp42",
                Format::IsoMedia,
                "MP4 / QuickTime",
            ),
            ("movie.avi", b"RIFF\x04\0\0\0AVI ", Format::Avi, "AVI"),
            (
                "movie.mkv",
                b"\x1a\x45\xdf\xa3\x84\x42\x82\x88x",
                Format::Matroska,
                "Matroska / WebM",
            ),
            (
                "photo.heic",
                b"\0\0\0\x18ftypheic\0\0\0\0heicmif1",
                Format::Heif,
                "HEIF / AVIF",
            ),
            (
                "photo.cr3",
                b"\0\0\0\x18ftypcrx \0\0\0\0crx isom",
                Format::CanonRaw,
                "Canon CR3",
            ),
            ("file.bin", b"%PDF-1.7", Format::Pdf, "PDF"),
            ("file.md", b"plain text", Format::Text, "Text"),
            ("file.bin", b"unknown", Format::Unsupported, "Unsupported"),
        ];
        for (name, data, expected, label) in cases {
            let detected = detect(Path::new(name), data);
            assert_eq!(detected, expected, "{name}");
            assert_eq!(format_name(detected), label);
        }

        // The same TIFF byte stream is a photograph or a negative depending on
        // what the camera called it, and only the negative keeps its model.
        let tiff = tiff_sample();
        assert_eq!(detect(Path::new("photo.tif"), &tiff), Format::Tiff);
        assert_eq!(detect(Path::new("photo.nef"), &tiff), Format::Raw);
        assert_eq!(format_name(Format::Raw), "RAW");
        for extension in ["docx", "epub", "ods"] {
            let sample = office_sample(extension);
            let detected = detect(Path::new(&format!("file.{extension}")), &sample);
            assert_eq!(detected, Format::Office, "{extension}");
            assert_eq!(format_name(detected), "Office");
        }
        assert_eq!(
            detect(Path::new("renamed.docx"), b"PKarchive"),
            Format::Unsupported
        );

        let mut raf = b"FUJIFILMCCD-RAW ".to_vec();
        raf.resize(93, 0);
        assert_eq!(detect(Path::new("photo.raf"), &raf), Format::Raf);

        let mut asf = vec![
            0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62,
            0xce, 0x6c,
        ];
        asf.resize(64, 0);
        assert_eq!(detect(Path::new("movie.wmv"), &asf), Format::Asf);
        assert_eq!(format_name(Format::Asf), "WMV / ASF");

        assert_eq!(extension(Path::new("PHOTO.JPEG")), "jpeg");
    }

    #[test]
    fn recognizes_every_supported_intake_extension() {
        assert_eq!(SUPPORTED_EXTENSIONS.len(), 113);
        for extension in SUPPORTED_EXTENSIONS {
            assert!(has_supported_extension(Path::new(&format!(
                "file.{extension}"
            ))));
        }
        for unsupported in ["archive.rar", "page.psd", "sheet.numbers", "clip.ogv"] {
            assert!(!has_supported_extension(Path::new(unsupported)));
        }
    }

    #[test]
    fn detects_every_iso_media_and_utf8_text_alias() {
        let iso_media = b"\0\0\0\x18ftypisom\0\0\0\0isommp42";
        for extension in ISO_MEDIA_EXTENSIONS {
            assert_eq!(
                detect(Path::new(&format!("movie.{extension}")), iso_media),
                Format::IsoMedia,
                "{extension}"
            );
        }
        for extension in TEXT_EXTENSIONS {
            assert_eq!(
                detect(Path::new(&format!("note.{extension}")), b"plain UTF-8 text"),
                Format::Text,
                "{extension}"
            );
        }
    }

    #[test]
    fn verification_rejects_residual_traces_and_format_changes() {
        let error = verify_cleaned_data(
            Path::new("note.txt"),
            Format::Text,
            "a\u{200b}b".as_bytes(),
            true,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("仍发现 1 项应移除的痕迹"));
        verify_cleaned_data(
            Path::new("note.txt"),
            Format::Text,
            b"clean text",
            true,
            true,
        )
        .unwrap();
        assert!(verify_cleaned_data(
            Path::new("photo.jpg"),
            Format::Jpeg,
            b"not a jpeg",
            true,
            true,
        )
        .is_err());
    }

    #[test]
    fn preserves_or_removes_jpeg_icc_through_the_public_engine_path() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("profile.jpg");
        let payload = b"ICC_PROFILE\0\x01\x01display-profile";
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe2];
        jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(payload);
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        fs::write(&source, jpeg).unwrap();

        let report = scan_file(&source);
        assert_eq!(report.findings[0].category, "color_profile");

        let preserved =
            clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
        assert!(preserved.success, "{:?}", preserved.error);
        assert!(preserved.removed.is_empty());
        let preserved_report = scan_file(Path::new(preserved.output_path.as_deref().unwrap()));
        assert_eq!(preserved_report.findings[0].category, "color_profile");

        let stripped =
            clean_file_with_options(&source, &OutputMode::Copy, true, true, false, false);
        assert!(stripped.success, "{:?}", stripped.error);
        assert_eq!(stripped.removed[0].category, "color_profile");
        assert!(
            scan_file(Path::new(stripped.output_path.as_deref().unwrap()))
                .findings
                .is_empty()
        );
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
    fn cancellable_scan_stops_before_reading_when_requested() {
        let cancellation = AtomicBool::new(true);
        let reports = scan_paths_cancellable(&["missing.txt".into()], &cancellation);
        assert!(reports.is_empty());
    }

    #[test]
    fn cancellable_scan_reports_count_only_progress() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("notes.txt");
        fs::write(&existing, "plain text").unwrap();
        let paths = vec![
            existing.to_string_lossy().into_owned(),
            "missing.txt".into(),
        ];
        let cancellation = AtomicBool::new(false);
        let progress = std::sync::Mutex::new(Vec::new());
        let reports = scan_paths_cancellable_with_progress(&paths, &cancellation, &|report| {
            progress.lock().unwrap().push(report.error.is_some());
        });
        assert_eq!(reports.len(), paths.len());
        let progress = progress.into_inner().unwrap();
        assert_eq!(progress.len(), paths.len());
        assert_eq!(progress.iter().filter(|failed| **failed).count(), 1);
    }

    #[test]
    fn dynamic_scan_workers_balance_slow_inputs_and_keep_result_order() {
        let paths = vec![
            "slow-input".to_owned(),
            "fast-input".to_owned(),
            "last-input".to_owned(),
        ];
        let barrier = std::sync::Barrier::new(2);
        let completion_order = std::sync::Mutex::new(Vec::new());
        let scan = |path: &str| {
            if path == "slow-input" {
                barrier.wait();
                std::thread::sleep(std::time::Duration::from_millis(20));
            } else if path == "fast-input" {
                barrier.wait();
            }
            ScanReport {
                path: path.to_owned(),
                name: path.to_owned(),
                format: "test".into(),
                size: 0,
                supported: true,
                findings: Vec::new(),
                error: None,
            }
        };
        let progress = |report: &ScanReport| {
            completion_order.lock().unwrap().push(report.path.clone());
        };

        let reports = scan_paths_with_workers(&paths, None, Some(&progress), 2, &scan);

        assert_eq!(
            completion_order.into_inner().unwrap(),
            ["fast-input", "last-input", "slow-input"]
        );
        assert_eq!(
            reports
                .iter()
                .map(|report| report.path.as_str())
                .collect::<Vec<_>>(),
            paths.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scan_and_clean_text_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let report = scan_file(&source);
        assert_eq!(report.findings[0].count, 1);
        let result = clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
        assert!(result.success);
        assert_eq!(result.source_size, Some(5));
        assert_eq!(result.output_size, Some(2));
        assert_eq!(
            fs::read_to_string(result.output_path.unwrap()).unwrap(),
            "ab"
        );
        assert_eq!(fs::read_to_string(source).unwrap(), "a\u{200b}b");
    }

    #[test]
    fn preserves_utf_bom_endianness_and_line_endings_while_cleaning_text() {
        let dir = tempfile::tempdir().unwrap();
        for (index, encoding) in [
            TextEncoding::Utf8 { bom: true },
            TextEncoding::Utf16Le,
            TextEncoding::Utf16Be,
        ]
        .into_iter()
        .enumerate()
        {
            let source = dir.path().join(format!("encoded-{index}.txt"));
            fs::write(&source, encode_text("a\u{200b}b\r\n", encoding)).unwrap();
            let report = scan_file(&source);
            assert!(report.supported, "{:?}", report.error);
            assert_eq!(report.findings[0].category, "unicode");

            let result =
                clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
            assert!(result.success, "{:?}", result.error);
            let output = fs::read(result.output_path.unwrap()).unwrap();
            let (cleaned, output_encoding) = decode_text(&output).unwrap();
            assert_eq!(output_encoding, encoding);
            assert_eq!(cleaned, "ab\r\n");
        }
    }

    #[test]
    fn cleans_structured_metadata_in_utf16_and_rejects_invalid_surrogates() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("page.html");
        fs::write(
            &source,
            encode_text(
                "<meta name=\"aut\u{200b}hor\" content=\"private\">\r\n<p>safe</p>",
                TextEncoding::Utf16Le,
            ),
        )
        .unwrap();
        let report = scan_file(&source);
        assert!(report.supported, "{:?}", report.error);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.category == "document_metadata"));
        let result = clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
        assert!(result.success, "{:?}", result.error);
        let output = fs::read(result.output_path.unwrap()).unwrap();
        assert_eq!(decode_text(&output).unwrap().0, "\r\n<p>safe</p>");

        let invalid = dir.path().join("invalid.txt");
        fs::write(&invalid, [0xff, 0xfe, 0x00, 0xd8]).unwrap();
        let report = scan_file(&invalid);
        assert!(!report.supported);
        assert!(report.error.is_some());
        let failed = clean_file_with_options(&invalid, &OutputMode::Copy, true, true, true, false);
        assert!(!failed.success);
        assert!(failed.output_path.is_none());

        for (name, bytes) in [
            ("odd-utf16.txt", vec![0xff, 0xfe, 0x61]),
            ("invalid-utf8.txt", vec![0xc3, 0x28]),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, bytes).unwrap();
            let report = scan_file(&path);
            assert!(!report.supported, "{name} must fail closed");
            let result = clean_file_with_options(&path, &OutputMode::Copy, true, true, true, false);
            assert!(!result.success, "{name} must not produce output");
            assert!(result.output_path.is_none());
        }
    }

    #[test]
    #[ignore = "explicit local performance benchmark"]
    fn benchmark_real_batch_engine_paths() {
        const FILES: usize = 128;
        let directory = tempfile::tempdir().expect("create benchmark directory");
        let mut paths = Vec::with_capacity(FILES);
        let mut payload_bytes = 0usize;
        for index in 0..FILES {
            let nested = directory
                .path()
                .join(format!("level-{}", index % 4))
                .join(format!("bucket-{}", index % 8));
            fs::create_dir_all(&nested).expect("create benchmark nesting");
            let size = match index % 3 {
                0 => 4 * 1024,
                1 => 64 * 1024,
                _ => 512 * 1024,
            };
            let mut content = vec![b'a'; size];
            content[size / 2..size / 2 + "\u{200b}".len()].copy_from_slice("\u{200b}".as_bytes());
            let path = nested.join(format!("fixture-{index}.txt"));
            fs::write(&path, &content).expect("write benchmark fixture");
            payload_bytes += size;
            paths.push(path.to_string_lossy().into_owned());
        }

        let scan_started = Instant::now();
        let reports = scan_paths(&paths);
        let scan_elapsed = scan_started.elapsed();
        assert_eq!(reports.len(), FILES);
        assert!(reports.iter().all(|report| report.supported));

        let mut clean_latencies = Vec::with_capacity(FILES);
        let clean_started = Instant::now();
        for path in &paths {
            let started = Instant::now();
            let result = clean_file_with_options(
                Path::new(path),
                &OutputMode::Copy,
                true,
                true,
                true,
                false,
            );
            assert!(
                result.success,
                "benchmark cleanup failed: {:?}",
                result.error
            );
            clean_latencies.push(started.elapsed());
        }
        let clean_elapsed = clean_started.elapsed();
        let first_ms = clean_latencies[0].as_secs_f64() * 1000.0;
        clean_latencies.sort_unstable();
        let p95_index = (FILES * 95).div_ceil(100).saturating_sub(1);
        let p95_ms = clean_latencies[p95_index].as_secs_f64() * 1000.0;
        eprintln!(
            "METACLEAN_BENCH {{\"files\":{FILES},\"payload_bytes\":{payload_bytes},\"scan_ms\":{:.2},\"scan_files_per_sec\":{:.2},\"clean_ms\":{:.2},\"clean_files_per_sec\":{:.2},\"clean_first_file_ms\":{first_ms:.2},\"clean_p95_file_ms\":{p95_ms:.2}}}",
            scan_elapsed.as_secs_f64() * 1000.0,
            FILES as f64 / scan_elapsed.as_secs_f64(),
            clean_elapsed.as_secs_f64() * 1000.0,
            FILES as f64 / clean_elapsed.as_secs_f64(),
        );
    }

    #[test]
    #[ignore = "explicit local performance benchmark"]
    fn benchmark_mixed_failure_batch_engine_paths() {
        const VALID_FILES: usize = 96;
        const INVALID_FILES: usize = 16;
        const MISSING_FILES: usize = 16;
        const TOTAL_FILES: usize = VALID_FILES + INVALID_FILES + MISSING_FILES;
        let directory = tempfile::tempdir().expect("create mixed benchmark directory");
        let mut paths = Vec::with_capacity(TOTAL_FILES);
        let mut payload_bytes = 0usize;
        for index in 0..VALID_FILES {
            let nested = directory
                .path()
                .join(format!("valid-level-{}", index % 4))
                .join(format!("valid-bucket-{}", index % 8));
            fs::create_dir_all(&nested).expect("write mixed benchmark nesting");
            let size = match index % 3 {
                0 => 4 * 1024,
                1 => 64 * 1024,
                _ => 512 * 1024,
            };
            let mut content = vec![b'a'; size];
            content[size / 2..size / 2 + "\u{200b}".len()].copy_from_slice("\u{200b}".as_bytes());
            let path = nested.join(format!("valid-{index}.txt"));
            fs::write(&path, &content).expect("write valid mixed fixture");
            payload_bytes += size;
            paths.push(path.to_string_lossy().into_owned());
        }
        for index in 0..INVALID_FILES {
            let path = directory.path().join(format!("unsupported-{index}.bin"));
            fs::write(&path, [0, 159, 146, 150]).expect("write unsupported mixed fixture");
            paths.push(path.to_string_lossy().into_owned());
        }
        for index in 0..MISSING_FILES {
            paths.push(
                directory
                    .path()
                    .join(format!("missing-{index}.txt"))
                    .to_string_lossy()
                    .into_owned(),
            );
        }

        let scan_started = Instant::now();
        let reports = scan_paths(&paths);
        let scan_elapsed = scan_started.elapsed();
        assert_eq!(reports.len(), TOTAL_FILES);
        assert_eq!(
            reports.iter().filter(|report| report.supported).count(),
            VALID_FILES
        );
        assert_eq!(
            reports
                .iter()
                .filter(|report| report.error.is_some())
                .count(),
            INVALID_FILES + MISSING_FILES
        );

        let mut clean_latencies = Vec::with_capacity(TOTAL_FILES);
        let mut successes = 0usize;
        let mut failures = 0usize;
        let clean_started = Instant::now();
        for path in &paths {
            let started = Instant::now();
            let result = clean_file_isolated_with_options(
                Path::new(path),
                &OutputMode::Copy,
                true,
                true,
                true,
                false,
            );
            clean_latencies.push(started.elapsed());
            if result.success {
                successes += 1;
                assert!(result.output_path.is_some());
            } else {
                failures += 1;
                assert!(result.output_path.is_none());
            }
        }
        let clean_elapsed = clean_started.elapsed();
        assert_eq!(successes, VALID_FILES);
        assert_eq!(failures, INVALID_FILES + MISSING_FILES);
        let first_ms = clean_latencies[0].as_secs_f64() * 1000.0;
        clean_latencies.sort_unstable();
        let p95_index = (TOTAL_FILES * 95).div_ceil(100).saturating_sub(1);
        let p95_ms = clean_latencies[p95_index].as_secs_f64() * 1000.0;
        eprintln!(
            "METACLEAN_BENCH_MIXED {{\"files\":{TOTAL_FILES},\"valid_files\":{VALID_FILES},\"failed_files\":{},\"payload_bytes\":{payload_bytes},\"scan_ms\":{:.2},\"scan_files_per_sec\":{:.2},\"clean_ms\":{:.2},\"clean_files_per_sec\":{:.2},\"clean_first_result_ms\":{first_ms:.2},\"clean_p95_result_ms\":{p95_ms:.2}}}",
            INVALID_FILES + MISSING_FILES,
            scan_elapsed.as_secs_f64() * 1000.0,
            TOTAL_FILES as f64 / scan_elapsed.as_secs_f64(),
            clean_elapsed.as_secs_f64() * 1000.0,
            TOTAL_FILES as f64 / clean_elapsed.as_secs_f64(),
        );
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
            let result =
                clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
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
    fn scans_and_cleans_every_iso_and_text_alias_through_the_engine() {
        let dir = tempfile::tempdir().unwrap();
        let video = supported_media_samples()
            .into_iter()
            .find(|(name, _)| *name == "movie.mp4")
            .unwrap()
            .1;
        for extension in ISO_MEDIA_EXTENSIONS {
            let source = dir.path().join(format!("movie.{extension}"));
            fs::write(&source, &video).unwrap();
            let report = scan_file(&source);
            assert!(report.supported, "{extension}: {:?}", report.error);
            let result =
                clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
            assert!(result.success, "{extension}: {:?}", result.error);
            let cleaned = scan_file(Path::new(result.output_path.as_deref().unwrap()));
            assert!(cleaned.supported, "{extension}: {:?}", cleaned.error);
            assert!(cleaned.findings.is_empty(), "{extension}");
        }

        for extension in TEXT_EXTENSIONS {
            let source = dir.path().join(format!("note.{extension}"));
            fs::write(&source, "a\u{200b}b").unwrap();
            let report = scan_file(&source);
            assert!(report.supported, "{extension}: {:?}", report.error);
            let result =
                clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
            assert!(result.success, "{extension}: {:?}", result.error);
            assert_eq!(
                fs::read_to_string(result.output_path.unwrap()).unwrap(),
                "ab",
                "{extension}"
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
            ("broken.aiff", b"FORM\0\0\0\x04AIFF".as_slice()),
            ("broken.mp4", b"\0\0\0\x18ftypisom".as_slice()),
        ] {
            let source = dir.path().join(name);
            fs::write(&source, bytes).unwrap();
            let report = scan_file(&source);
            assert!(!report.supported);
            assert!(report.error.is_some());
            assert!(
                !clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false)
                    .success
            );
            assert!(!cleaned_path(&source).exists());
        }
    }

    #[test]
    fn native_cleaners_never_panic_at_any_truncated_prefix() {
        for (name, bytes) in supported_media_samples() {
            let path = Path::new(name);
            let format = detect(path, &bytes);
            assert_ne!(format, Format::Unsupported, "invalid full fixture: {name}");
            for end in 0..bytes.len() {
                let prefix = &bytes[..end];
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = inspect_data(path, format, prefix);
                    let _ = clean_data(path, format, prefix, true, true);
                }));
                assert!(result.is_ok(), "{name} panicked at byte boundary {end}");
            }
        }
    }

    #[test]
    fn native_cleaners_never_panic_on_single_byte_corruption() {
        for (name, bytes) in supported_media_samples() {
            let path = Path::new(name);
            let format = detect(path, &bytes);
            assert_ne!(format, Format::Unsupported, "invalid full fixture: {name}");
            for index in 0..bytes.len() {
                for replacement in [0, 0xff] {
                    if bytes[index] == replacement {
                        continue;
                    }
                    let mut corrupted = bytes.clone();
                    corrupted[index] = replacement;
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let _ = inspect_data(path, format, &corrupted);
                        let _ = clean_data(path, format, &corrupted, true, true);
                    }));
                    assert!(
                        result.is_ok(),
                        "{name} panicked after byte {index} became {replacement:#04x}"
                    );
                }
            }
        }
    }

    #[test]
    fn replace_creates_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let result =
            clean_file_with_options(&source, &OutputMode::Replace, true, true, true, false);
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
        let preserved =
            clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
        let preserved_metadata = fs::metadata(preserved.output_path.unwrap()).unwrap();
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&preserved_metadata),
            old
        );

        let refreshed =
            clean_file_with_options(&source, &OutputMode::Copy, false, true, true, false);
        let refreshed_metadata = fs::metadata(refreshed.output_path.unwrap()).unwrap();
        assert_ne!(
            filetime::FileTime::from_last_modification_time(&refreshed_metadata),
            old
        );
    }

    #[cfg(windows)]
    #[test]
    fn read_only_sources_copy_safely_and_replace_without_creating_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("read-only.txt");
        fs::write(&source, "a\u{200b}b").unwrap();
        let mut permissions = fs::metadata(&source).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&source, permissions).unwrap();

        let copied = clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
        assert!(copied.success, "{:?}", copied.error);
        let output = PathBuf::from(copied.output_path.unwrap());
        assert_eq!(fs::read_to_string(&output).unwrap(), "ab");
        assert!(fs::metadata(&output).unwrap().permissions().readonly());
        assert!(fs::metadata(&source).unwrap().permissions().readonly());

        let replaced =
            clean_file_with_options(&source, &OutputMode::Replace, true, true, true, false);
        assert!(!replaced.success, "read-only replacement must fail closed");
        assert!(replaced.backup_path.is_none());
        assert!(replaced.error.unwrap().contains("只读"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "a\u{200b}b");
        assert!(fs::metadata(&source).unwrap().permissions().readonly());
        assert!(!dir.path().join("read-only.txt.bak").exists());
    }

    #[test]
    fn rejects_unknown_binary() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("data.bin");
        fs::write(&source, [0, 1, 2]).unwrap();
        assert!(
            !clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false).success
        );
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
            let result =
                clean_file_with_options(&source, &OutputMode::Copy, true, true, true, false);
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
