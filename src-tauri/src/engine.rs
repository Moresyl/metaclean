use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cleaners::{asf, avi, bmp, heif, image, media, mkv, office, pdf, tiff, video, web_text},
    error::{display_path, CleanError, Result},
    models::{CleanResult, Finding, FindingSeverity, OutputMode, ScanReport},
    safe_io::{
        atomic_write_with_metadata, backup_path, cleaned_path, privacy_extended_attribute_count,
        unique_path, validate_input, FileMetadataSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Jpeg,
    Png,
    Webp,
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
    "jpg", "jpeg", "jpe", "png", "webp", "gif", "bmp", "dib", "tif", "tiff", "heic", "heif",
    "heics", "heifs", "hif", "avif", "avifs", "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf",
    "sr2", "orf", "rw2", "rwl", "dng", "pef", "srw", "raf", "3fr", "erf", "mef", "mos", "iiq",
    "kdc", "dcr", "k25", "mp3", "wav", "flac", "mp4", "mov", "m4v", "m4a", "3g2", "3gp", "3gp2",
    "3gpp", "f4a", "f4b", "f4p", "f4v", "lrv", "m4b", "m4p", "mqv", "qt", "avi", "asf", "wmv",
    "wma", "mkv", "mka", "mks", "mk3d", "webm", "docx", "xlsx", "pptx", "odt", "epub", "pdf",
    "txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv", "yaml",
    "yml", "log", "srt", "vtt",
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
    "txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv", "yaml",
    "yml", "log", "srt", "vtt",
];

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
    // MP3 is last of the binary formats: a bare frame header is only two bytes
    // of sync, which plenty of other containers match by accident.
    if data.starts_with(b"ID3") || (data.len() >= 2 && data[0] == 0xff && data[1] & 0xe0 == 0xe0) {
        return Format::Mp3;
    }
    if data.starts_with(b"%PDF-") {
        return Format::Pdf;
    }
    if data.starts_with(b"PK") && matches!(ext.as_str(), "docx" | "xlsx" | "pptx" | "odt" | "epub")
    {
        return Format::Office;
    }
    if TEXT_EXTENSIONS.contains(&ext.as_str()) && std::str::from_utf8(data).is_ok() {
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
        Format::Bmp => "BMP",
        Format::Tiff => "TIFF",
        Format::Raw => "RAW",
        Format::Raf => "RAF",
        Format::Heif => "HEIF / AVIF",
        Format::CanonRaw => "Canon CR3",
        Format::Mp3 => "MP3",
        Format::Wav => "WAV",
        Format::Flac => "FLAC",
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
        Format::Gif => media::inspect_gif(data),
        Format::Bmp => bmp::inspect(data),
        Format::Tiff => tiff::inspect_tiff(data, false),
        Format::Raw => tiff::inspect_tiff(data, true),
        Format::Raf => tiff::inspect_raf(data),
        Format::Heif | Format::CanonRaw => heif::inspect(data),
        Format::Mp3 => media::inspect_mp3(data),
        Format::Wav => media::inspect_wav(data),
        Format::Flac => media::inspect_flac(data),
        Format::IsoMedia => video::inspect(data),
        Format::Avi => avi::inspect(data),
        Format::Asf => asf::inspect(data),
        Format::Matroska => mkv::inspect(data),
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
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    match format {
        Format::Jpeg => {
            image::clean_jpeg_with_options(data, preserve_orientation, preserve_color_profile)
        }
        Format::Png => image::clean_png_with_options(data, preserve_color_profile),
        Format::Webp => image::clean_webp_with_options(data, preserve_color_profile),
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
        Format::IsoMedia => video::clean(data),
        Format::Avi => avi::clean(data),
        Format::Asf => asf::clean(data),
        Format::Matroska => mkv::clean(data),
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
    let source_metadata = match validate_input(source) {
        Ok(metadata) => metadata,
        Err(error) => return fail(error.to_string()),
    };
    let metadata_snapshot = match FileMetadataSnapshot::capture(source, &source_metadata) {
        Ok(snapshot) => snapshot,
        Err(error) => return fail(error.to_string()),
    };
    let data = match fs::read(source) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };
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
        OutputMode::Copy => (unique_path(cleaned_path(source)), None),
        OutputMode::Replace => {
            let backup = unique_path(backup_path(source));
            if let Err(error) =
                atomic_write_with_metadata(&backup, &data, Some(&metadata_snapshot), true, false)
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
        remove_extended_attributes,
    ) {
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
    fn builds_only_nonempty_extended_attribute_findings() {
        assert!(extended_attribute_finding(0).is_none());
        let finding = extended_attribute_finding(2).unwrap();
        assert_eq!(finding.category, "macos_xattr");
        assert_eq!(finding.count, 2);
        assert_eq!(finding.severity, FindingSeverity::Informational);
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
            ("animation.gif", gif),
            ("photo.bmp", bmp),
            ("photo.tif", tiff.clone()),
            ("photo.nef", tiff),
            ("recording.mp3", mp3),
            ("recording.wav", wav),
            ("recording.flac", flac),
            ("movie.mp4", video),
            ("movie.avi", avi),
            ("movie.mkv", matroska),
        ]
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
            ("file.docx", b"PKarchive", Format::Office, "Office"),
            ("book.epub", b"PKarchive", Format::Office, "Office"),
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
        assert_eq!(SUPPORTED_EXTENSIONS.len(), 91);
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
