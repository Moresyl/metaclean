use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};
use std::ops::Range;

#[path = "jpeg_gainmap.rs"]
mod gainmap;
#[path = "jpeg_mpf.rs"]
mod mpf;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_TRAILER: [u8; 4] = [0; 4];
const JPEG_ADOBE_HEADER_LEN: usize = 12;
const JPEG_GAINMAP_NAMESPACE: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
type JpegSegment<'a> = (u8, &'a [u8], Range<usize>);
type WebpChunk = ([u8; 4], Range<usize>);

pub(crate) fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn valid_png_header(payload: &[u8]) -> bool {
    if payload.len() != 13 {
        return false;
    }
    let width = u32::from_be_bytes(payload[..4].try_into().unwrap());
    let height = u32::from_be_bytes(payload[4..8].try_into().unwrap());
    let bit_depth = payload[8];
    let colour_type = payload[9];
    let valid_depth = match colour_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 | 4 | 6 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        _ => false,
    };
    width != 0
        && height != 0
        && width <= i32::MAX as u32
        && height <= i32::MAX as u32
        && valid_depth
        && payload[10] == 0
        && payload[11] == 0
        && matches!(payload[12], 0 | 1)
}

fn finding(label: &str, count: usize) -> Finding {
    Finding {
        category: "image_metadata".into(),
        label: label.into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

fn color_profile_finding(count: usize) -> Finding {
    Finding {
        category: "color_profile".into(),
        label: "ICC 色彩配置文件".into(),
        count,
        severity: FindingSeverity::Informational,
    }
}

fn is_jpeg_icc_profile(marker: u8, payload: &[u8]) -> bool {
    marker == 0xE2 && payload.starts_with(b"ICC_PROFILE\0")
}

fn is_private_jpeg_marker(marker: u8) -> bool {
    matches!(marker, 0x00 | 0xE1 | 0xE3..=0xED | 0xEF | 0xFE)
}

fn jpeg_gainmap_metadata_length(payload: &[u8]) -> Result<Option<usize>> {
    let Some(metadata) = payload.strip_prefix(JPEG_GAINMAP_NAMESPACE) else {
        return Ok(None);
    };
    let invalid = || CleanError::InvalidFormat("JPEG HDR 增益图显示参数无效或版本不受支持".into());
    if metadata.get(..4) != Some(&[0, 0, 0, 0]) {
        return Err(invalid());
    }
    // A primary-image declaration contains only the two version fields.
    if metadata.len() == 4 {
        return Ok(Some(payload.len()));
    }
    let flags = metadata[4];
    if flags & !0xcc != 0 {
        return Err(invalid());
    }
    let channels = if flags & 0x80 != 0 { 3 } else { 1 };
    let common_denominator = flags & 0x08 != 0;
    let length = if common_denominator {
        17 + channels * 20
    } else {
        21 + channels * 40
    };
    let display = metadata.get(..length).ok_or_else(invalid)?;
    if common_denominator {
        if display[5..9] == [0, 0, 0, 0] {
            return Err(invalid());
        }
    } else if display[5..]
        .chunks_exact(8)
        .any(|fraction| fraction[4..] == [0, 0, 0, 0])
    {
        return Err(invalid());
    }
    Ok(Some(JPEG_GAINMAP_NAMESPACE.len() + length))
}

fn is_private_jpeg_segment(marker: u8, payload: &[u8]) -> Result<bool> {
    if marker == 0xe1 {
        if let Some(metadata) = gainmap::parse(payload)? {
            return Ok(metadata.serialize()? != payload);
        }
    }
    let private_app2 = if marker == 0xE2
        && !is_jpeg_icc_profile(marker, payload)
        && !payload.starts_with(b"MPF\0")
    {
        match jpeg_gainmap_metadata_length(payload)? {
            Some(length) => payload.len() > length,
            None => true,
        }
    } else {
        false
    };
    Ok(is_private_jpeg_marker(marker)
        || (marker == 0xE0 && (!payload.starts_with(b"JFIF\0") || payload.len() > 14))
        || (marker == 0xEE
            && (!payload.starts_with(b"Adobe") || payload.len() > JPEG_ADOBE_HEADER_LEN))
        || private_app2)
}

pub fn inspect_jpeg(data: &[u8]) -> Result<Vec<Finding>> {
    if let Some(container) = mpf::parse(data)? {
        return container.inspect(data);
    }
    gainmap::validate_single(data)?;
    inspect_single_jpeg(data)
}

fn inspect_single_jpeg(data: &[u8]) -> Result<Vec<Finding>> {
    let segments = jpeg_segments(data)?;
    let mut exif = 0;
    let mut xmp = 0;
    let mut provenance = 0;
    let mut comments = 0;
    let mut color_profiles = 0;
    let mut orientations = 0;
    for (marker, payload, _) in segments {
        if marker == 0xE1 {
            if let Some(display) = minimal_display_metadata(payload) {
                orientations += usize::from(display.orientation.is_some());
                continue;
            }
            if let Some(metadata) = gainmap::parse(payload)? {
                xmp += usize::from(metadata.serialize()? != payload);
                continue;
            }
        }
        match marker {
            0xE1 if payload.starts_with(b"Exif\0\0") => exif += 1,
            0xE1 => xmp += 1,
            0xE2 if is_jpeg_icc_profile(marker, payload) => color_profiles += 1,
            0xE2 if payload.starts_with(b"MPF\0") => comments += mpf::privacy_count(payload)?,
            0xEB => provenance += 1,
            marker if is_private_jpeg_segment(marker, payload)? => comments += 1,
            _ => {}
        }
    }
    let mut findings = Vec::new();
    if exif > 0 {
        findings.push(finding("EXIF / GPS 元数据", exif));
    }
    if xmp > 0 {
        findings.push(finding("XMP 元数据", xmp));
    }
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "JUMBF / C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    if comments > 0 {
        findings.push(finding("图片注释 / IPTC / 应用数据", comments));
    }
    if color_profiles > 0 {
        findings.push(color_profile_finding(color_profiles));
    }
    if orientations > 0 {
        findings.push(Finding {
            category: "image_orientation".into(),
            label: "图片方向".into(),
            count: orientations,
            severity: FindingSeverity::Informational,
        });
    }
    Ok(findings)
}

fn jpeg_segments(data: &[u8]) -> Result<Vec<JpegSegment<'_>>> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return Err(CleanError::InvalidFormat("不是有效 JPEG".into()));
    }
    let mut result = Vec::new();
    let mut offset = 2;
    let mut terminated = false;
    while offset < data.len() {
        if offset + 1 >= data.len() {
            return Err(CleanError::InvalidFormat("JPEG 标记被截断".into()));
        }
        if data[offset] != 0xFF {
            return Err(CleanError::InvalidFormat(
                "JPEG 扫描外存在未标记数据".into(),
            ));
        }
        let start = offset;
        while offset < data.len() && data[offset] == 0xFF {
            offset += 1;
        }
        if offset >= data.len() {
            break;
        }
        let marker = data[offset];
        offset += 1;
        if marker == 0xD9 {
            if offset < data.len() {
                result.push((0x00, &data[offset..], offset..data.len()));
            }
            terminated = true;
            break;
        }
        if marker == 0xD8 || marker == 0x00 {
            return Err(CleanError::InvalidFormat("JPEG 包含无效标记".into()));
        }
        if matches!(marker, 0x01 | 0xD0..=0xD7) {
            continue;
        }
        if offset + 2 > data.len() {
            return Err(CleanError::InvalidFormat("JPEG 段长度缺失".into()));
        }
        let length = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        if length < 2 || offset + length > data.len() {
            return Err(CleanError::InvalidFormat("JPEG 段越界".into()));
        }
        let payload = &data[offset + 2..offset + length];
        if marker == 0xE2 {
            jpeg_gainmap_metadata_length(payload)?;
        }
        if marker == 0xEE && payload.starts_with(b"Adobe") && payload.len() < JPEG_ADOBE_HEADER_LEN
        {
            return Err(CleanError::InvalidFormat(
                "JPEG Adobe 色彩信息被截断".into(),
            ));
        }
        if marker == 0xE0 && payload.starts_with(b"JFIF\0") {
            if payload.len() < 14 {
                return Err(CleanError::InvalidFormat("JPEG JFIF 显示信息被截断".into()));
            }
            let thumbnail_bytes = usize::from(payload[12]) * usize::from(payload[13]) * 3;
            if payload.len() < 14 + thumbnail_bytes || (payload[12] == 0) != (payload[13] == 0) {
                return Err(CleanError::InvalidFormat(
                    "JPEG JFIF 缩略图尺寸或数据无效".into(),
                ));
            }
        }
        if marker == 0xDA {
            let components = payload.first().copied().unwrap_or(0) as usize;
            if components == 0 || payload.len() != 1 + components * 2 + 3 {
                return Err(CleanError::InvalidFormat("JPEG SOS 扫描头无效".into()));
            }
        }
        result.push((marker, payload, start..offset + length));
        offset += length;
        if marker == 0xDA {
            loop {
                if offset + 1 >= data.len() {
                    return Err(CleanError::InvalidFormat(
                        "JPEG 扫描数据缺少结束标记".into(),
                    ));
                }
                if data[offset] != 0xFF {
                    offset += 1;
                    continue;
                }
                match data[offset + 1] {
                    0x00 | 0xD0..=0xD7 => offset += 2,
                    0xFF => offset += 1,
                    _ => break,
                }
            }
        }
    }
    if !terminated {
        return Err(CleanError::InvalidFormat("JPEG 缺少结束标记".into()));
    }
    Ok(result)
}

pub fn clean_jpeg_with_options(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    if let Some(container) = mpf::parse(data)? {
        return container.clean(data, preserve_orientation, preserve_color_profile);
    }
    gainmap::validate_single(data)?;
    clean_single_jpeg(data, preserve_orientation, preserve_color_profile)
}

fn clean_single_jpeg(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_single_jpeg(data)?
        .into_iter()
        .filter(|finding| match finding.category.as_str() {
            "color_profile" => !preserve_color_profile,
            "image_orientation" => !preserve_orientation,
            _ => true,
        })
        .collect();
    let segments = jpeg_segments(data)?;
    let mut display = jpeg_display_metadata(&segments)?;
    if !preserve_orientation {
        display.orientation = None;
    }
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for (marker, payload, range) in segments {
        let mpf_payload = if marker == 0xE2 && payload.starts_with(b"MPF\0") {
            Some(mpf::clean_metadata(payload)?)
        } else {
            None
        };
        let remove = is_private_jpeg_segment(marker, payload)?
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload))
            || mpf_payload.is_some();
        if remove {
            output.extend_from_slice(&data[cursor..range.start]);
            if marker == 0xe1 {
                if let Some(metadata) = gainmap::parse(payload)? {
                    let payload = metadata.serialize()?;
                    output.extend_from_slice(&[0xff, 0xe1]);
                    output.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
                    output.extend(payload);
                }
            } else if marker == 0xE0 && payload.starts_with(b"JFIF\0") {
                // Keep version, density and pixel aspect ratio; retire only the
                // preview and any trailing application data, never the main image.
                output.extend_from_slice(&[0xff, 0xe0, 0, 16]);
                output.extend_from_slice(&payload[..12]);
                output.extend_from_slice(&[0, 0]);
            } else if marker == 0xEE && payload.starts_with(b"Adobe") {
                // The fixed header controls RGB/CMYK/YCCK interpretation. Keep
                // its version, flags and transform exactly, without private tails.
                output.extend_from_slice(&[0xff, 0xee, 0, 14]);
                output.extend_from_slice(&payload[..JPEG_ADOBE_HEADER_LEN]);
            } else if marker == 0xE2 && payload.starts_with(JPEG_GAINMAP_NAMESPACE) {
                let length = jpeg_gainmap_metadata_length(payload)?.unwrap();
                output.extend_from_slice(&[0xff, 0xe2]);
                output.extend_from_slice(&((length + 2) as u16).to_be_bytes());
                output.extend_from_slice(&payload[..length]);
            } else if let Some(payload) = mpf_payload {
                output.extend_from_slice(&[0xff, 0xe2]);
                output.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
                output.extend_from_slice(&payload);
            }
            cursor = range.end;
        }
    }
    output.extend_from_slice(&data[cursor..]);
    if let Some(segment) = display_segment(display) {
        output.splice(2..2, segment);
    }
    Ok((output, findings))
}

pub fn verify_jpeg_cleaned(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<()> {
    if let Some(container) = mpf::parse(data)? {
        return container.verify(data, preserve_orientation, preserve_color_profile);
    }
    gainmap::validate_single(data)?;
    verify_single_jpeg(data, preserve_orientation, preserve_color_profile)
}

fn verify_single_jpeg(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<()> {
    let segments = jpeg_segments(data)?;
    let mut display_segments = 0;
    for (marker, payload, _) in segments {
        if marker == 0xE1 {
            if let Some(display) = minimal_display_metadata(payload) {
                if preserve_orientation || display.orientation.is_none() {
                    display_segments += 1;
                    continue;
                }
            }
        }
        if is_private_jpeg_segment(marker, payload)?
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload))
            || marker == 0xE2 && payload.starts_with(b"MPF\0") && mpf::privacy_count(payload)? != 0
        {
            return Err(CleanError::Verification(
                "JPEG 中仍存在应移除的元数据段".into(),
            ));
        }
    }
    if display_segments > 1 {
        return Err(CleanError::Verification(
            "JPEG 中存在多个保留的显示信息段".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DisplayMetadata {
    orientation: Option<u16>,
    density: Option<PrintDensity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PrintDensity {
    // Keep the original rational pairs, including asymmetric axes. Converting
    // to integer DPI or to JFIF would lose precision or change reader priority.
    x: (u32, u32),
    y: (u32, u32),
    unit: u16,
}

fn invalid_density() -> CleanError {
    CleanError::InvalidFormat("JPEG EXIF 打印密度不完整、冲突或无效".into())
}

fn minimal_display_metadata(payload: &[u8]) -> Option<DisplayMetadata> {
    let display = tiff_display_metadata(payload.strip_prefix(b"Exif\0\0")?).ok()?;
    (display_segment(display)?.get(4..) == Some(payload)).then_some(display)
}

fn jpeg_display_metadata(segments: &[JpegSegment<'_>]) -> Result<DisplayMetadata> {
    let mut display = DisplayMetadata::default();
    for (marker, payload, _) in segments {
        if *marker == 0xE1 {
            if let Some(tiff) = payload.strip_prefix(b"Exif\0\0") {
                let next = tiff_display_metadata(tiff)?;
                display.orientation = display.orientation.or(next.orientation);
                if let Some(density) = next.density {
                    if display.density.is_some_and(|existing| existing != density) {
                        return Err(invalid_density());
                    }
                    display.density = Some(density);
                }
            }
        }
    }
    Ok(display)
}

fn tiff_display_metadata(tiff: &[u8]) -> Result<DisplayMetadata> {
    if tiff.len() < 8 {
        return Ok(DisplayMetadata::default());
    }
    let little_endian = match &tiff[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Ok(DisplayMetadata::default()),
    };
    let read_u16 = |bytes: &[u8]| -> Option<u16> {
        let value: [u8; 2] = bytes.get(..2)?.try_into().ok()?;
        Some(if little_endian {
            u16::from_le_bytes(value)
        } else {
            u16::from_be_bytes(value)
        })
    };
    let read_u32 = |bytes: &[u8]| -> Option<u32> {
        let value: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
        Some(if little_endian {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        })
    };
    if read_u16(&tiff[2..]) != Some(42) {
        return Ok(DisplayMetadata::default());
    }
    let ifd_offset = usize::try_from(read_u32(&tiff[4..]).ok_or_else(invalid_density)?)
        .map_err(|_| invalid_density())?;
    let count = usize::from(
        read_u16(tiff.get(ifd_offset..).ok_or_else(invalid_density)?)
            .ok_or_else(invalid_density)?,
    );
    let directory_end = ifd_offset
        .checked_add(6 + count * 12)
        .ok_or_else(invalid_density)?;
    if ifd_offset < 8 || directory_end > tiff.len() {
        return Err(invalid_density());
    }
    let mut display = DisplayMetadata::default();
    let (mut x, mut y, mut unit) = (None, None, None);
    for index in 0..count {
        let start = ifd_offset
            .checked_add(2 + index * 12)
            .ok_or_else(invalid_density)?;
        let end = start.checked_add(12).ok_or_else(invalid_density)?;
        let entry = tiff.get(start..end).ok_or_else(invalid_density)?;
        let tag = read_u16(entry).unwrap();
        let kind = read_u16(&entry[2..]).unwrap();
        let count = read_u32(&entry[4..]).unwrap();
        if tag == 0x0112 && kind == 3 && count == 1 && display.orientation.is_none() {
            display.orientation = read_u16(&entry[8..]).filter(|value| (1..=8).contains(value));
        } else if matches!(tag, 0x011a | 0x011b) {
            if kind != 5 || count != 1 {
                return Err(invalid_density());
            }
            let offset =
                usize::try_from(read_u32(&entry[8..]).unwrap()).map_err(|_| invalid_density())?;
            let end = offset.checked_add(8).ok_or_else(invalid_density)?;
            if offset < 8 || offset < directory_end && end > ifd_offset {
                return Err(invalid_density());
            }
            let bytes = tiff.get(offset..).ok_or_else(invalid_density)?;
            let numerator = read_u32(bytes).ok_or_else(invalid_density)?;
            let denominator = read_u32(bytes.get(4..).ok_or_else(invalid_density)?)
                .ok_or_else(invalid_density)?;
            let destination = if tag == 0x011a { &mut x } else { &mut y };
            if numerator == 0
                || denominator == 0
                || destination.replace((numerator, denominator)).is_some()
            {
                return Err(invalid_density());
            }
        } else if tag == 0x0128 {
            let value = read_u16(&entry[8..]).unwrap();
            if kind != 3 || count != 1 || !(1..=3).contains(&value) || unit.replace(value).is_some()
            {
                return Err(invalid_density());
            }
        }
    }
    display.density = match (x, y, unit) {
        (None, None, None) => None,
        (Some(x), Some(y), Some(unit)) => Some(PrintDensity { x, y, unit }),
        _ => return Err(invalid_density()),
    };
    Ok(display)
}

fn display_segment(display: DisplayMetadata) -> Option<Vec<u8>> {
    let Some(density) = display.density else {
        return display.orientation.map(orientation_segment);
    };
    let count: u16 = 3 + u16::from(display.orientation.is_some());
    let rational_offset = 8 + 2 + u32::from(count) * 12 + 4;
    let mut payload = b"Exif\0\0MM\0*\0\0\0\x08".to_vec();
    payload.extend_from_slice(&count.to_be_bytes());
    for (tag, kind, value) in [
        (
            0x0112u16,
            3u16,
            u32::from(display.orientation.unwrap_or(1)) << 16,
        ),
        (0x011a, 5, rational_offset),
        (0x011b, 5, rational_offset + 8),
        (0x0128, 3, u32::from(density.unit) << 16),
    ] {
        if tag == 0x0112 && display.orientation.is_none() {
            continue;
        }
        payload.extend_from_slice(&tag.to_be_bytes());
        payload.extend_from_slice(&kind.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload.extend_from_slice(&0u32.to_be_bytes());
    for value in [density.x.0, density.x.1, density.y.0, density.y.1] {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(&payload);
    Some(segment)
}

fn orientation_segment(orientation: u16) -> Vec<u8> {
    let mut payload = b"Exif\0\0MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01".to_vec();
    payload.extend_from_slice(&orientation.to_be_bytes());
    payload.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(&payload);
    segment
}

fn png_chunks(data: &[u8]) -> Result<Vec<([u8; 4], std::ops::Range<usize>)>> {
    if !data.starts_with(PNG_SIGNATURE) {
        return Err(CleanError::InvalidFormat("不是有效 PNG".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 8;
    let mut has_iend = false;
    let mut has_ihdr = false;
    let mut has_idat = false;
    let mut finished_idat = false;
    while offset + 12 <= data.len() {
        let length = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12 + length)
            .ok_or_else(|| CleanError::InvalidFormat("PNG 块长度溢出".into()))?;
        if end > data.len() {
            return Err(CleanError::InvalidFormat("PNG 块越界".into()));
        }
        let kind: [u8; 4] = data[offset + 4..offset + 8].try_into().unwrap();
        if !kind.iter().all(u8::is_ascii_alphabetic) || !kind[2].is_ascii_uppercase() {
            return Err(CleanError::InvalidFormat("PNG 块类型码无效".into()));
        }
        if kind[0].is_ascii_uppercase() && !matches!(&kind, b"IHDR" | b"PLTE" | b"IDAT" | b"IEND") {
            return Err(CleanError::InvalidFormat("PNG 包含不支持的关键块".into()));
        }
        let expected_crc = u32::from_be_bytes(data[end - 4..end].try_into().unwrap());
        if png_crc32(&data[offset + 4..end - 4]) != expected_crc {
            return Err(CleanError::InvalidFormat("PNG 块 CRC 校验失败".into()));
        }
        if chunks.is_empty() && (&kind != b"IHDR" || length != 13) {
            return Err(CleanError::InvalidFormat(
                "PNG 首块必须是 13 字节 IHDR".into(),
            ));
        }
        if &kind == b"IHDR" {
            if has_ihdr || !valid_png_header(&data[offset + 8..end - 4]) {
                return Err(CleanError::InvalidFormat("PNG IHDR 块无效或重复".into()));
            }
            has_ihdr = true;
        }
        if &kind == b"IDAT" {
            if finished_idat {
                return Err(CleanError::InvalidFormat("PNG IDAT 块不连续".into()));
            }
            has_idat = true;
        } else if has_idat {
            finished_idat = true;
        }
        if &kind == b"IEND" && length != 0 {
            return Err(CleanError::InvalidFormat("PNG IEND 块长度无效".into()));
        }
        chunks.push((kind, offset..end));
        offset = end;
        if &kind == b"IEND" {
            if offset < data.len() {
                chunks.push((PNG_TRAILER, offset..data.len()));
            }
            has_iend = true;
            break;
        }
    }
    if !has_iend || !has_ihdr || !has_idat {
        return Err(CleanError::InvalidFormat(
            "PNG 缺少 IHDR、IDAT 或 IEND 块".into(),
        ));
    }
    Ok(chunks)
}

fn is_private_png_chunk(kind: &[u8; 4]) -> bool {
    // Retain defined image, colour, animation and layout information. An
    // unrecognized ancillary payload is not evidence that the file is clean.
    !matches!(
        kind,
        b"IHDR"
            | b"PLTE"
            | b"IDAT"
            | b"IEND"
            | b"tRNS"
            | b"gAMA"
            | b"cHRM"
            | b"iCCP"
            | b"sBIT"
            | b"sRGB"
            | b"cICP"
            | b"mDCV"
            | b"cLLI"
            | b"bKGD"
            | b"hIST"
            | b"pHYs"
            | b"sPLT"
            | b"acTL"
            | b"fcTL"
            | b"fdAT"
            | b"oFFs"
            | b"pCAL"
            | b"sCAL"
            | b"sTER"
            | b"gIFg"
            | b"gIFt"
    )
}

pub fn inspect_png(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = png_chunks(data)?;
    let metadata = chunks
        .iter()
        .filter(|(kind, _)| is_private_png_chunk(kind) && kind != b"caBX")
        .count();
    let provenance = chunks.iter().filter(|(kind, _)| kind == b"caBX").count();
    let color_profiles = chunks.iter().filter(|(kind, _)| kind == b"iCCP").count();
    let mut findings = Vec::new();
    if metadata > 0 {
        findings.push(finding("PNG 文本 / EXIF / 时间及附加元数据", metadata));
    }
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    if color_profiles > 0 {
        findings.push(color_profile_finding(color_profiles));
    }
    Ok(findings)
}

pub fn clean_png_with_options(
    data: &[u8],
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_png(data)?
        .into_iter()
        .filter(|finding| finding.category != "color_profile" || !preserve_color_profile)
        .collect();
    let chunks = png_chunks(data)?;
    let mut output = Vec::with_capacity(data.len());
    output.extend_from_slice(PNG_SIGNATURE);
    for (kind, range) in chunks {
        if !is_private_png_chunk(&kind) && (preserve_color_profile || &kind != b"iCCP") {
            output.extend_from_slice(&data[range]);
        }
    }
    Ok((output, findings))
}

pub fn verify_png_cleaned(data: &[u8], preserve_color_profile: bool) -> Result<()> {
    if png_chunks(data)?
        .iter()
        .any(|(kind, _)| is_private_png_chunk(kind) || (!preserve_color_profile && kind == b"iCCP"))
    {
        return Err(CleanError::Verification(
            "PNG 中仍存在应移除的元数据块".into(),
        ));
    }
    Ok(())
}

pub fn inspect_webp(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = webp_chunks(data)?;
    let mut count = 0;
    let mut provenance = 0;
    for (kind, range) in &chunks {
        if kind == b"C2PA" {
            provenance += 1;
        } else if is_private_webp_chunk(kind) {
            count += 1;
        }
        if kind == b"ANMF" {
            for (child, _) in webp_frame_chunks(data, range)? {
                if &child == b"C2PA" {
                    provenance += 1;
                } else if is_private_webp_chunk(&child) {
                    count += 1;
                }
            }
        }
    }
    let color_profiles = chunks.iter().filter(|(kind, _)| kind == b"ICCP").count();
    let mut findings = Vec::new();
    if count > 0 {
        findings.push(finding("WebP EXIF / XMP 及附加元数据", count));
    }
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "WebP C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    if color_profiles > 0 {
        findings.push(color_profile_finding(color_profiles));
    }
    Ok(findings)
}

fn is_private_webp_chunk(kind: &[u8; 4]) -> bool {
    !matches!(
        kind,
        b"VP8X" | b"VP8 " | b"VP8L" | b"ALPH" | b"ICCP" | b"ANIM" | b"ANMF"
    )
}

fn webp_chunk_ranges(data: &[u8], span: Range<usize>) -> Result<Vec<WebpChunk>> {
    let mut chunks = Vec::new();
    let mut offset = span.start;
    while span.end.saturating_sub(offset) >= 8 {
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let payload_end = offset
            .checked_add(8)
            .and_then(|start| start.checked_add(length))
            .ok_or_else(|| CleanError::InvalidFormat("WebP 块长度溢出".into()))?;
        let end = payload_end
            .checked_add(length & 1)
            .filter(|end| *end <= span.end)
            .ok_or_else(|| CleanError::InvalidFormat("WebP 块越界".into()))?;
        if length & 1 != 0 && data[payload_end] != 0 {
            return Err(CleanError::InvalidFormat("WebP 块填充字节无效".into()));
        }
        chunks.push((data[offset..offset + 4].try_into().unwrap(), offset..end));
        offset = end;
    }
    if offset != span.end {
        return Err(CleanError::InvalidFormat("WebP 块尾存在截断数据".into()));
    }
    Ok(chunks)
}

fn webp_frame_chunks(data: &[u8], range: &Range<usize>) -> Result<Vec<WebpChunk>> {
    let size =
        u32::from_le_bytes(data[range.start + 4..range.start + 8].try_into().unwrap()) as usize;
    if size < 16 || data[range.start + 23] & 0xfc != 0 {
        return Err(CleanError::InvalidFormat("WebP 动画帧头无效".into()));
    }
    let chunks = webp_chunk_ranges(data, range.start + 24..range.start + 8 + size)?;
    let layout: Vec<_> = chunks
        .iter()
        .filter(|(kind, _)| !is_private_webp_chunk(kind))
        .map(|(kind, _)| *kind)
        .collect();
    if !matches!(layout.as_slice(), [kind] if matches!(kind, b"VP8 " | b"VP8L"))
        && !matches!(layout.as_slice(), [alpha, image] if alpha == b"ALPH" && image == b"VP8 ")
    {
        return Err(CleanError::InvalidFormat(
            "WebP 动画帧图像块组合无效".into(),
        ));
    }
    let image = chunks
        .iter()
        .find(|(kind, _)| matches!(kind, b"VP8 " | b"VP8L"))
        .ok_or_else(|| CleanError::InvalidFormat("WebP 动画帧缺少图像数据".into()))?;
    let dimensions = webp_image_dimensions(data, image)?;
    let payload = range.start + 8;
    if dimensions
        != (
            webp_u24(&data[payload + 6..payload + 9]) + 1,
            webp_u24(&data[payload + 9..payload + 12]) + 1,
        )
    {
        return Err(CleanError::InvalidFormat(
            "WebP 动画帧与编码图像尺寸不匹配".into(),
        ));
    }
    Ok(chunks)
}

fn webp_u24(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}

fn webp_image_dimensions(data: &[u8], (kind, range): &WebpChunk) -> Result<(u32, u32)> {
    let size =
        u32::from_le_bytes(data[range.start + 4..range.start + 8].try_into().unwrap()) as usize;
    let payload = &data[range.start + 8..range.start + 8 + size];
    let invalid = || CleanError::InvalidFormat("WebP 编码图像头无效或版本不受支持".into());
    match kind {
        b"VP8 " => {
            if payload.len() < 10
                || payload[0] & 0x09 != 0
                || payload[0] & 0x10 == 0
                || payload[3..6] != [0x9d, 0x01, 0x2a]
            {
                return Err(invalid());
            }
            let width = u16::from_le_bytes(payload[6..8].try_into().unwrap()) & 0x3fff;
            let height = u16::from_le_bytes(payload[8..10].try_into().unwrap()) & 0x3fff;
            if width == 0 || height == 0 {
                return Err(invalid());
            }
            Ok((u32::from(width), u32::from(height)))
        }
        b"VP8L" => {
            if payload.len() < 5 || payload[0] != 0x2f {
                return Err(invalid());
            }
            let header = u32::from_le_bytes(payload[1..5].try_into().unwrap());
            if header >> 29 != 0 {
                return Err(invalid());
            }
            Ok(((header & 0x3fff) + 1, ((header >> 14) & 0x3fff) + 1))
        }
        _ => Err(invalid()),
    }
}

fn webp_chunks(data: &[u8]) -> Result<Vec<WebpChunk>> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err(CleanError::InvalidFormat("不是有效 WebP".into()));
    }
    let declared = (u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize)
        .checked_add(8)
        .ok_or_else(|| CleanError::InvalidFormat("WebP RIFF 长度溢出".into()))?;
    if declared != data.len() {
        return Err(CleanError::InvalidFormat("WebP RIFF 长度不匹配".into()));
    }
    let chunks = webp_chunk_ranges(data, 12..declared)?;
    if !chunks
        .iter()
        .any(|(kind, _)| matches!(kind, b"VP8 " | b"VP8L" | b"ANMF"))
    {
        return Err(CleanError::InvalidFormat("WebP 缺少图像数据块".into()));
    }
    let extended: Vec<_> = chunks.iter().filter(|(kind, _)| kind == b"VP8X").collect();
    if extended.len() > 1
        || extended
            .iter()
            .any(|(_, range)| data[range.start + 4..range.start + 8] != 10u32.to_le_bytes())
    {
        return Err(CleanError::InvalidFormat("WebP VP8X 块无效或重复".into()));
    }
    if let Some((_, range)) = extended.first() {
        let payload = range.start + 8;
        if chunks.first().is_none_or(|(kind, _)| kind != b"VP8X")
            || data[payload] & 0xc1 != 0
            || data[payload + 1..payload + 4].iter().any(|byte| *byte != 0)
        {
            return Err(CleanError::InvalidFormat(
                "WebP VP8X 块位置或保留位无效".into(),
            ));
        }
    }
    let still_images = chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"VP8 " | b"VP8L"))
        .count();
    let animation_frames = chunks.iter().filter(|(kind, _)| kind == b"ANMF").count();
    if still_images > 1 || (still_images > 0 && animation_frames > 0) {
        return Err(CleanError::InvalidFormat(
            "WebP 图像与动画载荷组合无效".into(),
        ));
    }
    for image in chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"VP8 " | b"VP8L"))
    {
        let dimensions = webp_image_dimensions(data, image)?;
        if let Some((_, range)) = extended.first() {
            if dimensions
                != (
                    webp_u24(&data[range.start + 12..range.start + 15]) + 1,
                    webp_u24(&data[range.start + 15..range.start + 18]) + 1,
                )
            {
                return Err(CleanError::InvalidFormat(
                    "WebP 画布与编码图像尺寸不匹配".into(),
                ));
            }
        }
    }
    let animation_headers: Vec<_> = chunks.iter().filter(|(kind, _)| kind == b"ANIM").collect();
    if animation_headers.len() > 1
        || animation_headers
            .iter()
            .any(|(_, range)| data[range.start + 4..range.start + 8] != 6u32.to_le_bytes())
    {
        return Err(CleanError::InvalidFormat(
            "WebP 动画控制块无效或重复".into(),
        ));
    }
    let animation_flag = extended
        .first()
        .is_some_and(|(_, range)| data[range.start + 8] & 0x02 != 0);
    if animation_flag != (animation_frames > 0)
        || (animation_headers.is_empty() != (animation_frames == 0))
        || animation_frames > 0
            && (animation_headers.len() != 1 || chunks.iter().any(|(kind, _)| kind == b"ALPH"))
    {
        return Err(CleanError::InvalidFormat(
            "WebP 动画标记与载荷不匹配".into(),
        ));
    }
    if let Some((_, extended_range)) = extended.first() {
        let width = webp_u24(&data[extended_range.start + 12..extended_range.start + 15]) + 1;
        let height = webp_u24(&data[extended_range.start + 15..extended_range.start + 18]) + 1;
        if u64::from(width) * u64::from(height) > u64::from(u32::MAX) {
            return Err(CleanError::InvalidFormat("WebP 画布尺寸超出限制".into()));
        }
        for (_, range) in chunks.iter().filter(|(kind, _)| kind == b"ANMF") {
            webp_frame_chunks(data, range)?;
            let payload = range.start + 8;
            let x = webp_u24(&data[payload..payload + 3]) * 2;
            let y = webp_u24(&data[payload + 3..payload + 6]) * 2;
            let frame_width = webp_u24(&data[payload + 6..payload + 9]) + 1;
            let frame_height = webp_u24(&data[payload + 9..payload + 12]) + 1;
            if x + frame_width > width || y + frame_height > height {
                return Err(CleanError::InvalidFormat("WebP 动画帧超出画布".into()));
            }
        }
    }
    Ok(chunks)
}

pub fn clean_webp_with_options(
    data: &[u8],
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_webp(data)?
        .into_iter()
        .filter(|finding| finding.category != "color_profile" || !preserve_color_profile)
        .collect();
    let chunks = webp_chunks(data)?;
    let mut output = Vec::with_capacity(data.len());
    output.extend_from_slice(&data[..12]);
    for (kind, range) in chunks {
        if is_private_webp_chunk(&kind) || (!preserve_color_profile && &kind == b"ICCP") {
            continue;
        }
        let start = output.len();
        if &kind == b"ANMF" {
            output.extend_from_slice(&data[range.start..range.start + 24]);
            for (child, child_range) in webp_frame_chunks(data, &range)? {
                if !is_private_webp_chunk(&child) {
                    output.extend_from_slice(&data[child_range]);
                }
            }
            let size = u32::try_from(output.len() - start - 8)
                .map_err(|_| CleanError::InvalidFormat("WebP 动画帧长度溢出".into()))?;
            output[start + 4..start + 8].copy_from_slice(&size.to_le_bytes());
        } else {
            output.extend_from_slice(&data[range]);
        }
        if &kind == b"VP8X" && output.len() >= start + 9 {
            output[start + 8] &= !(0x08 | 0x04);
            if !preserve_color_profile {
                output[start + 8] &= !0x20;
            }
        }
    }
    let riff_size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Ok((output, findings))
}

pub fn verify_webp_cleaned(data: &[u8], preserve_color_profile: bool) -> Result<()> {
    for (kind, range) in webp_chunks(data)? {
        let private_frame_data = &kind == b"ANMF"
            && webp_frame_chunks(data, &range)?
                .iter()
                .any(|(child, _)| is_private_webp_chunk(child));
        if is_private_webp_chunk(&kind)
            || (!preserve_color_profile && &kind == b"ICCP")
            || private_frame_data
        {
            return Err(CleanError::Verification(
                "WebP 中仍存在应移除的元数据块".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut segment = vec![0xff, marker];
        segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        segment.extend_from_slice(payload);
        segment
    }

    fn jpeg_with_exif() -> Vec<u8> {
        vec![
            0xff, 0xd8, 0xff, 0xe1, 0, 10, b'E', b'x', b'i', b'f', 0, 0, 1, 2, 0xff, 0xd9,
        ]
    }
    #[test]
    fn strips_jpeg_exif_segment() {
        let source = jpeg_with_exif();
        let (cleaned, findings) = clean_jpeg_with_options(&source, true, true).unwrap();
        assert_eq!(cleaned, vec![0xff, 0xd8, 0xff, 0xd9]);
        assert_eq!(findings[0].count, 1);
    }

    #[test]
    fn preserves_hdr_xmp_display_values_without_identity_fields() {
        let xml = concat!(
            "http://ns.adobe.com/xap/1.0/\0",
            "<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='private toolkit'>",
            "<r:RDF xmlns:r='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>",
            "<r:Description xmlns:h='http://ns.adobe.com/hdr-gain-map/1.0/' ",
            "xmlns:d='http://purl.org/dc/elements/1.1/' d:creator='private author' ",
            "h:Version='1.0' h:GainMapMin='-0.12500000000000001' ",
            "h:GainMapMax='3.25000000000000001' h:HDRCapacityMax='4.5' ",
            "h:Gamma='1' h:BaseRenditionIsHDR='False'/></r:RDF></x:xmpmeta>"
        );
        let source = [
            vec![0xff, 0xd8],
            jpeg_segment(0xe1, xml.as_bytes()),
            vec![0xff, 0xd9],
        ]
        .concat();
        for orientation in [false, true] {
            for profile in [false, true] {
                let (output, findings) =
                    clean_jpeg_with_options(&source, orientation, profile).unwrap();
                assert!(output.windows(10).any(|bytes| bytes == b"GainMapMax"));
                for value in [b"-0.12500000000000001".as_slice(), b"3.25000000000000001"] {
                    assert!(output.windows(value.len()).any(|bytes| bytes == value));
                }
                assert!(!output.windows(7).any(|bytes| bytes == b"private"));
                assert_eq!(findings.len(), 1);
                assert!(inspect_jpeg(&output).unwrap().is_empty());
                verify_jpeg_cleaned(&output, orientation, profile).unwrap();
                assert_eq!(
                    clean_jpeg_with_options(&output, orientation, profile)
                        .unwrap()
                        .0,
                    output
                );
            }
        }
    }

    #[test]
    fn removes_jpeg_app0_previews_and_private_data_without_changing_density() {
        let header = b"JFIF\0\x01\x02\x02\x00\x76\x00\x3b";
        let mut preview = header.to_vec();
        preview.extend_from_slice(&[1, 1, 10, 20, 30]);
        preview.extend_from_slice(b"private thumbnail tail");
        let mut source = vec![0xff, 0xd8];
        source.extend(jpeg_segment(0xe0, &preview));
        source.extend(jpeg_segment(0xe0, b"JFXX\0\x10private old preview"));
        source.extend(jpeg_segment(0xe0, b"editor\0private identity"));
        source.extend_from_slice(&[0xff, 0xd9]);
        assert_eq!(
            inspect_jpeg(&source)
                .unwrap()
                .iter()
                .map(|item| item.count)
                .sum::<usize>(),
            3
        );
        assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        let mut expected = vec![0xff, 0xd8];
        let mut retained = header.to_vec();
        retained.extend_from_slice(&[0, 0]);
        expected.extend(jpeg_segment(0xe0, &retained));
        expected.extend_from_slice(&[0xff, 0xd9]);
        for orientation in [false, true] {
            for profile in [false, true] {
                let (cleaned, findings) =
                    clean_jpeg_with_options(&source, orientation, profile).unwrap();
                assert_eq!(cleaned, expected);
                assert_eq!(findings[0].count, 3);
                assert!(inspect_jpeg(&cleaned).unwrap().is_empty());
                verify_jpeg_cleaned(&cleaned, orientation, profile).unwrap();
            }
        }
    }

    #[test]
    fn rejects_truncated_jfif_headers_and_thumbnail_pixels() {
        for payload in [
            b"JFIF\0".as_slice(),
            b"JFIF\0\x01\x02\x01\x00\x48\x00\x48\x02\x02\x00",
        ] {
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xe0, payload));
            source.extend_from_slice(&[0xff, 0xd9]);
            assert!(inspect_jpeg(&source).is_err());
            assert!(clean_jpeg_with_options(&source, true, true).is_err());
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        }
    }

    #[test]
    fn preserves_jfif_display_header_without_a_preview_exactly() {
        for units in [0, 1, 2] {
            let mut payload = b"JFIF\0\x01\x02\x00\x00\x48\x00\x90\x00\x00".to_vec();
            payload[7] = units;
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xe0, &payload));
            source.extend_from_slice(&[0xff, 0xd9]);
            assert!(inspect_jpeg(&source).unwrap().is_empty());
            assert_eq!(
                clean_jpeg_with_options(&source, true, true).unwrap().0,
                source
            );
            verify_jpeg_cleaned(&source, true, true).unwrap();
        }
    }

    #[test]
    fn removes_unknown_app2_payloads_and_gainmap_tails_without_changing_display_data() {
        for flags in [0, 0x80, 0x08, 0x88, 0xcc] {
            let mut hdr = JPEG_GAINMAP_NAMESPACE.to_vec();
            hdr.extend_from_slice(&[0, 0, 0, 0, flags]);
            let channels = if flags & 0x80 != 0 { 3 } else { 1 };
            if flags & 8 != 0 {
                hdr.extend_from_slice(&1u32.to_be_bytes());
                hdr.extend_from_slice(&0u32.to_be_bytes());
                hdr.extend_from_slice(&2u32.to_be_bytes());
                for _ in 0..channels {
                    for value in [-1i32, 3, 1, 0, 0] {
                        hdr.extend_from_slice(&value.to_be_bytes());
                    }
                }
            } else {
                for numerator in [0i32, 2] {
                    hdr.extend_from_slice(&numerator.to_be_bytes());
                    hdr.extend_from_slice(&1u32.to_be_bytes());
                }
                for _ in 0..channels {
                    for numerator in [-1i32, 3, 1, 0, 0] {
                        hdr.extend_from_slice(&numerator.to_be_bytes());
                        hdr.extend_from_slice(&1u32.to_be_bytes());
                    }
                }
            }
            let mut extended = hdr.clone();
            extended.extend_from_slice(b"private gain map tail");
            let profile = b"ICC_PROFILE\0\x01\x01profile";
            let declaration = [JPEG_GAINMAP_NAMESPACE, &[0, 0, 0, 0]].concat();
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xe2, b"private editor identity"));
            source.extend(jpeg_segment(0xe2, b"FPXR\0private preview"));
            source.extend(jpeg_segment(0xe2, &declaration));
            source.extend(jpeg_segment(0xe2, &extended));
            source.extend(jpeg_segment(0xe2, profile));
            source.extend_from_slice(&[0xff, 0xd9]);
            assert_eq!(
                inspect_jpeg(&source)
                    .unwrap()
                    .iter()
                    .filter(|item| item.severity == FindingSeverity::Privacy)
                    .map(|item| item.count)
                    .sum::<usize>(),
                3
            );
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
            for orientation in [false, true] {
                for keep_profile in [false, true] {
                    let mut expected = vec![0xff, 0xd8];
                    expected.extend(jpeg_segment(0xe2, &declaration));
                    expected.extend(jpeg_segment(0xe2, &hdr));
                    if keep_profile {
                        expected.extend(jpeg_segment(0xe2, profile));
                    }
                    expected.extend_from_slice(&[0xff, 0xd9]);
                    let (cleaned, _) =
                        clean_jpeg_with_options(&source, orientation, keep_profile).unwrap();
                    assert_eq!(cleaned, expected);
                    verify_jpeg_cleaned(&cleaned, orientation, keep_profile).unwrap();
                }
            }
        }
    }

    #[test]
    fn rejects_incomplete_unsupported_and_zero_denominator_gainmap_headers() {
        let mut valid = JPEG_GAINMAP_NAMESPACE.to_vec();
        valid.extend_from_slice(&[0, 0, 0, 0, 0]);
        for _ in 0..7 {
            valid.extend_from_slice(&0u32.to_be_bytes());
            valid.extend_from_slice(&1u32.to_be_bytes());
        }
        let header = JPEG_GAINMAP_NAMESPACE.len();
        let mut invalid_payloads = vec![valid[..header + 3].to_vec(), valid[..header + 5].to_vec()];
        for position in [header, header + 2, header + 4, header + 12, header + 28] {
            let mut corrupt = valid.clone();
            corrupt[position] = if position < header + 5 { 1 } else { 0 };
            invalid_payloads.push(corrupt);
        }
        for payload in invalid_payloads {
            let source = [
                vec![0xff, 0xd8],
                jpeg_segment(0xe2, &payload),
                vec![0xff, 0xd9],
            ]
            .concat();
            assert!(inspect_jpeg(&source).is_err());
            assert!(clean_jpeg_with_options(&source, true, true).is_err());
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        }
    }

    #[test]
    fn removes_private_app14_data_and_preserves_adobe_color_interpretation() {
        for transform in [0, 1, 2] {
            let mut header = b"Adobe\x00\x64\x12\x34\x56\x78\x00".to_vec();
            header[11] = transform;
            let mut extended = header.clone();
            extended.extend_from_slice(b"private editor identity");
            let scan = [
                0xff, 0xda, 0, 8, 1, 1, 0, 0, 0x3f, 0, 0x11, 0xff, 0x00, 0x22,
            ];
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xee, &extended));
            source.extend_from_slice(&scan);
            source.extend(jpeg_segment(0xee, b"editor\0private identity"));
            source.extend(jpeg_segment(0xee, b"Adob"));
            source.extend_from_slice(&[0xff, 0xd9]);
            let mut expected = vec![0xff, 0xd8];
            expected.extend(jpeg_segment(0xee, &header));
            expected.extend_from_slice(&scan);
            expected.extend_from_slice(&[0xff, 0xd9]);
            assert_eq!(
                inspect_jpeg(&source)
                    .unwrap()
                    .iter()
                    .map(|item| item.count)
                    .sum::<usize>(),
                3
            );
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
            for orientation in [false, true] {
                for profile in [false, true] {
                    let (cleaned, findings) =
                        clean_jpeg_with_options(&source, orientation, profile).unwrap();
                    assert_eq!(cleaned, expected);
                    assert_eq!(findings[0].count, 3);
                    assert!(inspect_jpeg(&cleaned).unwrap().is_empty());
                    verify_jpeg_cleaned(&cleaned, orientation, profile).unwrap();
                }
            }
        }
    }

    #[test]
    fn preserves_complete_adobe_app14_headers_exactly() {
        for transform in [0, 1, 2, 255] {
            let mut payload = b"Adobe\x00\x65\xab\xcd\xef\x01\x00".to_vec();
            payload[11] = transform;
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xee, &payload));
            source.extend_from_slice(&[0xff, 0xd9]);
            assert!(inspect_jpeg(&source).unwrap().is_empty());
            for orientation in [false, true] {
                for profile in [false, true] {
                    assert_eq!(
                        clean_jpeg_with_options(&source, orientation, profile)
                            .unwrap()
                            .0,
                        source
                    );
                    verify_jpeg_cleaned(&source, orientation, profile).unwrap();
                }
            }
        }
    }

    #[test]
    fn rejects_truncated_adobe_app14_color_headers() {
        let header = b"Adobe\x00\x64\x00\x00\x00\x00\x02";
        for length in 5..header.len() {
            let mut source = vec![0xff, 0xd8];
            source.extend(jpeg_segment(0xee, &header[..length]));
            source.extend_from_slice(&[0xff, 0xd9]);
            assert!(inspect_jpeg(&source).is_err());
            assert!(clean_jpeg_with_options(&source, true, true).is_err());
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        }
    }

    #[test]
    fn reports_and_removes_every_private_jpeg_segment_class() {
        let mut source = vec![0xff, 0xd8];
        source.extend(jpeg_segment(0xe1, b"Exif\0\0data"));
        source.extend(jpeg_segment(0xe1, b"http://ns.adobe.com/xap/1.0/"));
        source.extend(jpeg_segment(0xeb, b"c2pa"));
        source.extend(jpeg_segment(0xec, b"Ducky private editor data"));
        source.extend(jpeg_segment(0xed, b"iptc"));
        source.extend(jpeg_segment(0xef, b"private application data"));
        source.extend_from_slice(&[0xff, 0xd9]);
        let findings = inspect_jpeg(&source).unwrap();
        assert_eq!(findings.len(), 4);
        assert_eq!(findings.iter().map(|item| item.count).sum::<usize>(), 6);
        assert!(findings.iter().any(|item| item.category == "provenance"));
        let (cleaned, _) = clean_jpeg_with_options(&source, false, true).unwrap();
        assert_eq!(cleaned, vec![0xff, 0xd8, 0xff, 0xd9]);
    }

    #[test]
    fn rejects_malformed_jpeg_segment_layouts() {
        assert!(inspect_jpeg(b"not jpeg").is_err());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff]).is_err());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff, 0xe1]).is_err());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff, 0xe1, 0, 1, 0xff, 0xd9]).is_err());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff, 0xe1, 0, 20, 0xff, 0xd9]).is_err());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff, 0xd0, 0xff, 0xd9]).is_ok());
        assert!(inspect_jpeg(&[0xff, 0xd8, 0xff, 0xda, 0, 2, 0xff, 0xd9]).is_err());
    }

    #[test]
    fn removes_private_segments_between_progressive_scans_and_after_eoi() {
        let scan_header = [0xff, 0xda, 0, 8, 1, 1, 0, 0, 0x3f, 0];
        let mut source = vec![0xff, 0xd8];
        source.extend_from_slice(&scan_header);
        source.extend_from_slice(&[0x11, 0xff, 0x00, 0x22, 0xff, 0xd0]);
        source.extend(jpeg_segment(0xe1, b"Exif\0\0between-scans"));
        source.extend_from_slice(&scan_header);
        source.extend_from_slice(&[0x33, 0x44, 0xff, 0xd9]);
        source.extend_from_slice(b"private trailer");

        let findings = inspect_jpeg(&source).unwrap();
        assert_eq!(
            findings.iter().map(|finding| finding.count).sum::<usize>(),
            2
        );
        let (cleaned, _) = clean_jpeg_with_options(&source, false, true).unwrap();
        assert!(!cleaned.windows(4).any(|window| window == b"Exif"));
        assert!(!cleaned.windows(7).any(|window| window == b"private"));
        assert!(cleaned.ends_with(&[0xff, 0xd9]));
        verify_jpeg_cleaned(&cleaned, false, true).unwrap();
    }

    fn jpeg_with_orientation(orientation: u16) -> Vec<u8> {
        let mut source = vec![0xff, 0xd8];
        source.extend(orientation_segment(orientation));
        source.extend_from_slice(&[0xff, 0xfe, 0, 5, b'g', b'p', b's', 0xff, 0xd9]);
        source
    }

    #[test]
    fn preserves_only_a_minimal_orientation_tag() {
        let source = jpeg_with_orientation(6);
        let (cleaned, findings) = clean_jpeg_with_options(&source, true, true).unwrap();
        let segments = jpeg_segments(&cleaned).unwrap();
        assert_eq!(
            jpeg_display_metadata(&segments).unwrap().orientation,
            Some(6)
        );
        assert!(!cleaned.windows(3).any(|window| window == b"gps"));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn removes_orientation_when_disabled() {
        let source = jpeg_with_orientation(6);
        let (cleaned, _) = clean_jpeg_with_options(&source, false, true).unwrap();
        assert_eq!(cleaned, vec![0xff, 0xd8, 0xff, 0xd9]);
    }

    fn density_jpeg(little: bool, orientation: bool) -> Vec<u8> {
        let u16_bytes = |value: u16| {
            if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            }
        };
        let u32_bytes = |value: u32| {
            if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            }
        };
        let mut tiff = if little {
            b"II".to_vec()
        } else {
            b"MM".to_vec()
        };
        tiff.extend(u16_bytes(42));
        tiff.extend(u32_bytes(8));
        let count = 3 + u16::from(orientation);
        let offset = 14 + u32::from(count) * 12;
        tiff.extend(u16_bytes(count));
        for (tag, kind, value) in [
            (0x0112, 3, 6),
            (0x011a, 5, offset),
            (0x011b, 5, offset + 8),
            (0x0128, 3, 3),
        ] {
            if tag == 0x0112 && !orientation {
                continue;
            }
            tiff.extend(u16_bytes(tag));
            tiff.extend(u16_bytes(kind));
            tiff.extend(u32_bytes(1));
            if kind == 3 {
                tiff.extend(u16_bytes(value as u16));
                tiff.extend([0, 0]);
            } else {
                tiff.extend(u32_bytes(value));
            }
        }
        tiff.extend(u32_bytes(0));
        for value in [30001, 100, 15001, 100] {
            tiff.extend(u32_bytes(value));
        }
        tiff.extend_from_slice(b"private author trailer");
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend(tiff);
        let mut source = vec![0xff, 0xd8];
        source.extend(jpeg_segment(0xe1, &payload));
        source.extend_from_slice(&[0xff, 0xd9]);
        source
    }

    #[test]
    fn preserves_exact_density_in_both_byte_orders_and_orientation_modes() {
        for little in [false, true] {
            for has_orientation in [false, true] {
                for preserve_orientation in [false, true] {
                    let source = density_jpeg(little, has_orientation);
                    let (cleaned, removed) =
                        clean_jpeg_with_options(&source, preserve_orientation, true).unwrap();
                    let display = jpeg_display_metadata(&jpeg_segments(&cleaned).unwrap()).unwrap();
                    assert_eq!(
                        display.density,
                        Some(PrintDensity {
                            x: (30001, 100),
                            y: (15001, 100),
                            unit: 3
                        })
                    );
                    assert_eq!(
                        display.orientation,
                        (has_orientation && preserve_orientation).then_some(6)
                    );
                    assert!(!cleaned.windows(7).any(|bytes| bytes == b"private"));
                    assert_eq!(removed[0].category, "image_metadata");
                    verify_jpeg_cleaned(&cleaned, preserve_orientation, true).unwrap();
                    let rescanned = inspect_jpeg(&cleaned).unwrap();
                    assert!(rescanned
                        .iter()
                        .all(|finding| finding.category == "image_orientation"
                            && finding.severity == FindingSeverity::Informational));
                    assert_eq!(
                        clean_jpeg_with_options(&cleaned, preserve_orientation, true)
                            .unwrap()
                            .0,
                        cleaned
                    );
                }
            }
        }
    }

    #[test]
    fn retained_orientation_is_informational_but_can_still_be_removed() {
        let source = clean_jpeg_with_options(&jpeg_with_orientation(6), true, true)
            .unwrap()
            .0;
        let findings = inspect_jpeg(&source).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "image_orientation");
        assert_eq!(findings[0].severity, FindingSeverity::Informational);
        assert!(clean_jpeg_with_options(&source, true, true)
            .unwrap()
            .1
            .is_empty());
        let (cleaned, removed) = clean_jpeg_with_options(&source, false, true).unwrap();
        assert_eq!(removed[0].category, "image_orientation");
        assert!(inspect_jpeg(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_partial_duplicate_and_conflicting_densities() {
        let source = density_jpeg(false, false);
        // TIFF starts at byte12; IFD entries start at22, rational values at62.
        for (offset, bytes) in [
            (62, 0u32.to_be_bytes().to_vec()),
            (66, 0u32.to_be_bytes().to_vec()),
            (30, u32::MAX.to_be_bytes().to_vec()),
            (24, 3u16.to_be_bytes().to_vec()),
            (26, 2u32.to_be_bytes().to_vec()),
            (34, 0x011au16.to_be_bytes().to_vec()),
            (46, 0x013bu16.to_be_bytes().to_vec()),
            (54, 4u16.to_be_bytes().to_vec()),
        ] {
            let mut broken = source.clone();
            broken[offset..offset + bytes.len()].copy_from_slice(&bytes);
            assert!(
                clean_jpeg_with_options(&broken, true, true).is_err(),
                "offset {offset}"
            );
        }
        let mut conflicting = source.clone();
        conflicting[62..66].copy_from_slice(&301u32.to_be_bytes());
        let segment = jpeg_segments(&conflicting).unwrap()[0].2.clone();
        let mut combined = source;
        combined.splice(2..2, conflicting[segment].iter().copied());
        assert!(clean_jpeg_with_options(&combined, true, true).is_err());
    }
    #[test]
    fn rejects_truncated_png() {
        assert!(inspect_png(b"\x89PNG\r\n\x1a\n\0").is_err());
    }

    fn png_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output.extend_from_slice(&png_crc32(&output[4..]).to_be_bytes());
        output
    }

    fn png_start() -> Vec<u8> {
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend(png_chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]));
        source
    }

    fn finish_png(source: &mut Vec<u8>) {
        source.extend(png_chunk(b"IDAT", b"pixels"));
        source.extend(png_chunk(b"IEND", b""));
    }

    fn webp_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut output = kind.to_vec();
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            output.push(0);
        }
        output
    }

    fn webp_file(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        for chunk in chunks {
            source.extend(chunk);
        }
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        source
    }

    fn webp_lossy_data(width: u16, height: u16, pixels: &[u8]) -> Vec<u8> {
        let mut payload = vec![0x30, 0, 0, 0x9d, 0x01, 0x2a];
        payload.extend(width.to_le_bytes());
        payload.extend(height.to_le_bytes());
        payload.extend(pixels);
        payload
    }

    #[test]
    fn rejects_webp_invalid_coded_headers_at_all_privacy_boundaries() {
        let valid = webp_lossy_data(2, 3, b"opaque compressed data");
        let mut wrong_signature = valid.clone();
        wrong_signature[3] = 0;
        let mut interframe = valid.clone();
        interframe[0] |= 1;
        let mut hidden_frame = valid.clone();
        hidden_frame[0] &= !0x10;
        let mut reserved_version = valid.clone();
        reserved_version[0] |= 8;
        for (kind, payload) in [
            (b"VP8 ", webp_lossy_data(0, 3, b"pixels")),
            (b"VP8 ", webp_lossy_data(2, 0, b"pixels")),
            (b"VP8 ", valid[..9].to_vec()),
            (b"VP8 ", wrong_signature),
            (b"VP8 ", interframe),
            (b"VP8 ", hidden_frame),
            (b"VP8 ", reserved_version),
            (b"VP8L", vec![0x2f, 0, 0, 0]),
            (b"VP8L", vec![0, 0, 0, 0, 0, 1]),
            (b"VP8L", vec![0x2f, 0, 0, 0, 0x20, 1]),
        ] {
            let source = webp_file(&[webp_chunk(kind, &payload)]);
            for preserve_profile in [false, true] {
                assert!(inspect_webp(&source).is_err());
                assert!(clean_webp_with_options(&source, preserve_profile).is_err());
                assert!(verify_webp_cleaned(&source, preserve_profile).is_err());
            }
        }
    }

    #[test]
    fn rejects_webp_coded_dimensions_that_disagree_with_canvas_or_frame() {
        let image = webp_chunk(b"VP8 ", &webp_lossy_data(2, 3, b"pixels"));
        let mut frame = vec![0; 16];
        frame.extend(&image);
        for chunks in [
            vec![webp_chunk(b"VP8X", &[0; 10]), image],
            vec![
                webp_chunk(b"VP8X", &[2, 0, 0, 0, 3, 0, 0, 3, 0, 0]),
                webp_chunk(b"ANIM", &[0; 6]),
                webp_chunk(b"ANMF", &frame),
            ],
        ] {
            let source = webp_file(&chunks);
            assert!(inspect_webp(&source).is_err());
            for preserve_profile in [false, true] {
                assert!(clean_webp_with_options(&source, preserve_profile).is_err());
                assert!(verify_webp_cleaned(&source, preserve_profile).is_err());
            }
        }
    }

    #[test]
    fn preserves_webp_valid_coded_headers_and_extended_canvas() {
        let mut images = Vec::new();
        for version in 0..=3 {
            let mut payload = webp_lossy_data(0x3fff, 3, b"coded pixels");
            payload[0] |= version << 1;
            images.push((b"VP8 ", payload, 0x3fff, 3));
        }
        for alpha_hint in [0, 1 << 28] {
            let header = 0x3fff_u32 | (2 << 14) | alpha_hint;
            let mut payload = vec![0x2f];
            payload.extend(header.to_le_bytes());
            payload.extend(b"coded pixels");
            images.push((b"VP8L", payload, 0x4000, 3));
        }
        for (kind, payload, width, height) in images {
            let image = webp_chunk(kind, &payload);
            let mut extended = vec![0; 10];
            extended[4..7].copy_from_slice(&(width - 1_u32).to_le_bytes()[..3]);
            extended[7..10].copy_from_slice(&(height - 1_u32).to_le_bytes()[..3]);
            for chunks in [
                vec![image.clone()],
                vec![webp_chunk(b"VP8X", &extended), image.clone()],
            ] {
                let source = webp_file(&chunks);
                assert!(inspect_webp(&source).unwrap().is_empty());
                assert_eq!(clean_webp_with_options(&source, true).unwrap().0, source);
                verify_webp_cleaned(&source, true).unwrap();
            }
        }
    }

    #[test]
    fn removes_webp_private_application_chunks_inside_animation_frames() {
        let pixels = b"compressed pixels with EXIF and private application words";
        let image = webp_chunk(b"VP8 ", &webp_lossy_data(1, 1, pixels));
        let mut frame = vec![0; 16];
        frame.extend(&image);
        frame.extend(webp_chunk(b"prIv", b"private frame identity"));
        frame.extend(webp_chunk(b"JUMB", b"private frame provenance"));
        for animated in [false, true] {
            for preserve_profile in [false, true] {
                let mut extended = vec![0; 10];
                extended[0] = if animated { 0x22 } else { 0x20 };
                let mut chunks = vec![
                    webp_chunk(b"VP8X", &extended),
                    webp_chunk(b"ICCP", b"profile"),
                ];
                if animated {
                    chunks.push(webp_chunk(b"ANIM", &[0; 6]));
                    chunks.push(webp_chunk(b"ANMF", &frame));
                } else {
                    chunks.push(image.clone());
                }
                chunks.extend([
                    webp_chunk(b"prIv", b"private application identity"),
                    webp_chunk(b"JUMB", b"private application provenance"),
                    webp_chunk(b"ZERO", b""),
                ]);
                let source = webp_file(&chunks);
                let expected = if animated { 5 } else { 3 };
                let inspected = inspect_webp(&source).unwrap();
                assert_eq!(
                    inspected
                        .iter()
                        .find(|item| item.category == "image_metadata")
                        .map(|item| item.count),
                    Some(expected)
                );
                assert!(verify_webp_cleaned(&source, preserve_profile).is_err());
                let (cleaned, removed) =
                    clean_webp_with_options(&source, preserve_profile).unwrap();
                assert_eq!(
                    removed
                        .iter()
                        .find(|item| item.category == "image_metadata")
                        .unwrap()
                        .count,
                    expected
                );
                assert!(cleaned.windows(pixels.len()).any(|window| window == pixels));
                for kind in [b"prIv", b"JUMB", b"ZERO"] {
                    assert!(!cleaned.windows(4).any(|window| window == kind));
                }
                assert_eq!(
                    cleaned.windows(4).any(|window| window == b"ICCP"),
                    preserve_profile
                );
                verify_webp_cleaned(&cleaned, preserve_profile).unwrap();
                assert!(inspect_webp(&cleaned)
                    .unwrap()
                    .iter()
                    .all(|item| item.category == "color_profile"));
                assert_eq!(
                    clean_webp_with_options(&cleaned, preserve_profile)
                        .unwrap()
                        .0,
                    cleaned
                );
            }
        }
    }

    #[test]
    fn refuses_unsafe_webp_animation_frames_and_padding() {
        let image = webp_chunk(b"VP8 ", &webp_lossy_data(1, 1, b"image"));
        let mut frame = vec![0; 16];
        frame.extend(&image);
        let mut malformed = vec![vec![0; 15], vec![0; 16]];
        let mut reserved = frame.clone();
        reserved[15] = 4;
        malformed.push(reserved);
        let mut outside_canvas = frame.clone();
        outside_canvas[6] = 1;
        malformed.push(outside_canvas);
        let mut bad_padding = frame.clone();
        *bad_padding.last_mut().unwrap() = 42;
        malformed.push(bad_padding);
        let mut out_of_bounds = frame.clone();
        out_of_bounds[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        malformed.push(out_of_bounds);
        let mut trailing = frame.clone();
        trailing.push(42);
        malformed.push(trailing);
        let mut duplicate = frame.clone();
        duplicate.extend(&image);
        malformed.push(duplicate);
        for kind in [b"VP8X", b"ANMF", b"ICCP", b"ALPH"] {
            let mut invalid_layout = frame.clone();
            invalid_layout.extend(webp_chunk(kind, b"extra"));
            malformed.push(invalid_layout);
        }
        let mut lossless_alpha = vec![0; 16];
        lossless_alpha.extend(webp_chunk(b"ALPH", b"alpha"));
        lossless_alpha.extend(webp_chunk(b"VP8L", b"image"));
        malformed.push(lossless_alpha);
        for payload in malformed {
            let source = webp_file(&[
                webp_chunk(b"VP8X", &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                webp_chunk(b"ANIM", &[0; 6]),
                webp_chunk(b"ANMF", &payload),
            ]);
            for preserve_profile in [false, true] {
                assert!(inspect_webp(&source).is_err());
                assert!(clean_webp_with_options(&source, preserve_profile).is_err());
                assert!(verify_webp_cleaned(&source, preserve_profile).is_err());
            }
        }
        let mut nonzero_padding = image;
        *nonzero_padding.last_mut().unwrap() = 42;
        let source = webp_file(&[nonzero_padding]);
        assert!(inspect_webp(&source).is_err());
        assert!(clean_webp_with_options(&source, true).is_err());
        assert!(verify_webp_cleaned(&source, true).is_err());
    }

    #[test]
    fn counts_webp_frame_provenance_and_preserves_animation_display_fields() {
        let mut extended = [0; 10];
        extended[0] = 0x12;
        extended[4] = 7;
        extended[7] = 7;
        let mut frame = [0; 16].to_vec();
        frame[0] = 1;
        frame[3] = 2;
        frame[6] = 3;
        frame[9] = 2;
        frame[12..15].copy_from_slice(&[75, 0, 0]);
        frame[15] = 3;
        let mut clean_frame = frame.clone();
        clean_frame.extend(webp_chunk(b"ALPH", b"alpha"));
        clean_frame.extend(webp_chunk(
            b"VP8 ",
            &webp_lossy_data(4, 3, b"pixels mention C2PA and JUMB"),
        ));
        frame = clean_frame.clone();
        frame.extend(webp_chunk(b"C2PA", b"private provenance"));
        let control = [17, 31, 47, 128, 3, 0];
        let source = webp_file(&[
            webp_chunk(b"VP8X", &extended),
            webp_chunk(b"ANIM", &control),
            webp_chunk(b"ANMF", &frame),
        ]);
        let inspected = inspect_webp(&source).unwrap();
        assert_eq!(inspected.len(), 1);
        assert_eq!(inspected[0].category, "provenance");
        assert_eq!(inspected[0].count, 1);
        let (cleaned, removed) = clean_webp_with_options(&source, true).unwrap();
        assert_eq!(removed, inspected);
        assert_eq!(
            cleaned,
            webp_file(&[
                webp_chunk(b"VP8X", &extended),
                webp_chunk(b"ANIM", &control),
                webp_chunk(b"ANMF", &clean_frame),
            ])
        );
        verify_webp_cleaned(&cleaned, true).unwrap();
        for chunks in [
            vec![webp_chunk(b"VP8X", &extended), webp_chunk(b"ANMF", &frame)],
            vec![webp_chunk(b"ANIM", &control), webp_chunk(b"ANMF", &frame)],
            vec![
                webp_chunk(b"VP8X", &extended),
                webp_chunk(b"ANIM", &control),
                webp_chunk(b"ANIM", &control),
                webp_chunk(b"ANMF", &frame),
            ],
            vec![
                webp_chunk(b"VP8X", &extended),
                webp_chunk(b"ANIM", b"short"),
                webp_chunk(b"ANMF", &frame),
            ],
            vec![
                webp_chunk(b"VP8X", &extended[..9]),
                webp_chunk(b"ANIM", &control),
                webp_chunk(b"ANMF", &frame),
            ],
            vec![
                webp_chunk(b"VP8X", &[0; 10]),
                webp_chunk(b"ANIM", &control),
                webp_chunk(b"VP8 ", b"image"),
            ],
        ] {
            let invalid = webp_file(&chunks);
            assert!(inspect_webp(&invalid).is_err());
            assert!(clean_webp_with_options(&invalid, true).is_err());
            assert!(verify_webp_cleaned(&invalid, true).is_err());
        }
    }

    #[test]
    fn strips_png_text_chunks() {
        let mut source = png_start();
        source.extend(png_chunk(b"tEXt", b"Author\0Alice"));
        finish_png(&mut source);
        let (cleaned, findings) = clean_png_with_options(&source, true).unwrap();
        assert_eq!(findings[0].count, 1);
        assert!(!cleaned.windows(4).any(|window| window == b"tEXt"));
    }

    #[test]
    fn reports_and_removes_png_provenance_chunks() {
        let mut source = png_start();
        source.extend(png_chunk(b"caBX", b"claim"));
        finish_png(&mut source);
        let findings = inspect_png(&source).unwrap();
        assert_eq!(findings[0].category, "provenance");
        let (cleaned, _) = clean_png_with_options(&source, true).unwrap();
        assert!(!cleaned.windows(4).any(|window| window == b"caBX"));
    }

    #[test]
    fn reports_and_removes_png_modification_times() {
        let time = png_chunk(b"tIME", &[0x07, 0xea, 10, 1, 12, 34, 56]);
        for after_pixels in [false, true] {
            let mut expected = png_start();
            expected.extend(png_chunk(b"pHYs", &[0, 0, 0x2e, 0x23, 0, 0, 0x17, 0x12, 1]));
            let mut source = expected.clone();
            if !after_pixels {
                source.extend_from_slice(&time);
            }
            source.extend(png_chunk(b"IDAT", b"pixels"));
            if after_pixels {
                source.extend_from_slice(&time);
            }
            source.extend(png_chunk(b"IEND", b""));
            finish_png(&mut expected);

            let findings = inspect_png(&source).unwrap();
            assert_eq!(findings.len(), 1);
            assert_eq!(findings[0].category, "image_metadata");
            assert_eq!(findings[0].count, 1);
            for preserve_profile in [false, true] {
                assert!(verify_png_cleaned(&source, preserve_profile).is_err());
                let (cleaned, removed) = clean_png_with_options(&source, preserve_profile).unwrap();
                assert_eq!(removed.len(), 1);
                assert_eq!(cleaned, expected);
                assert!(inspect_png(&cleaned).unwrap().is_empty());
                verify_png_cleaned(&cleaned, preserve_profile).unwrap();
            }
        }
    }

    #[test]
    fn reports_and_removes_png_data_after_iend() {
        let mut source = png_start();
        finish_png(&mut source);
        let clean_length = source.len();
        source.extend_from_slice(b"private trailer");

        let findings = inspect_png(&source).unwrap();
        assert_eq!(findings[0].count, 1);
        let (cleaned, _) = clean_png_with_options(&source, true).unwrap();
        assert!(!cleaned.windows(7).any(|window| window == b"private"));
        assert_eq!(cleaned.len(), clean_length);
        verify_png_cleaned(&cleaned, true).unwrap();
    }

    #[test]
    fn rejects_unrecognized_critical_png_chunks() {
        for kind in [b"VpAg", b"VPAG"] {
            for after_pixels in [false, true] {
                let mut source = png_start();
                if after_pixels {
                    source.extend(png_chunk(b"IDAT", b"pixels"));
                }
                source.extend(png_chunk(kind, b"unsupported image interpretation"));
                if after_pixels {
                    source.extend(png_chunk(b"IEND", b""));
                } else {
                    finish_png(&mut source);
                }
                assert!(matches!(
                    inspect_png(&source),
                    Err(CleanError::InvalidFormat(_))
                ));
                for preserve_profile in [false, true] {
                    assert!(matches!(
                        clean_png_with_options(&source, preserve_profile),
                        Err(CleanError::InvalidFormat(_))
                    ));
                    assert!(matches!(
                        verify_png_cleaned(&source, preserve_profile),
                        Err(CleanError::InvalidFormat(_))
                    ));
                }
            }
        }
    }

    #[test]
    fn reports_and_removes_png_unrecognized_ancillary_payloads() {
        for kind in [
            b"uNKn", b"unKn", b"uNKP", b"unKP", b"gIFx", b"dSIG", b"fRAc",
        ] {
            for after_pixels in [false, true] {
                let mut expected = png_start();
                expected.extend(png_chunk(b"pHYs", &[0, 0, 0x2e, 0x23, 0, 0, 0x17, 0x12, 1]));
                let mut source = expected.clone();
                if !after_pixels {
                    source.extend(png_chunk(kind, b"private application identity"));
                }
                source.extend(png_chunk(b"IDAT", b"pixels"));
                if after_pixels {
                    source.extend(png_chunk(kind, b"private application identity"));
                }
                source.extend(png_chunk(b"IEND", b""));
                finish_png(&mut expected);
                let findings = inspect_png(&source).unwrap();
                assert_eq!(findings.len(), 1, "chunk {kind:?}");
                assert_eq!(findings[0].category, "image_metadata");
                assert_eq!(findings[0].count, 1);
                for preserve_profile in [false, true] {
                    assert!(verify_png_cleaned(&source, preserve_profile).is_err());
                    let (cleaned, removed) =
                        clean_png_with_options(&source, preserve_profile).unwrap();
                    assert_eq!(cleaned, expected);
                    assert_eq!(removed, findings);
                    assert!(inspect_png(&cleaned).unwrap().is_empty());
                    verify_png_cleaned(&cleaned, preserve_profile).unwrap();
                }
            }
        }
    }

    #[test]
    fn preserves_png_hdr_and_registered_layout_chunks() {
        let mut source = png_start();
        source.extend(png_chunk(b"cICP", &[9, 16, 0, 1]));
        source.extend(png_chunk(b"mDCV", &[0; 24]));
        source.extend(png_chunk(b"cLLI", &[0; 8]));
        source.extend(png_chunk(b"oFFs", &[0; 9]));
        source.extend(png_chunk(b"sCAL", b"\x011.25\x002.5"));
        source.extend(png_chunk(
            b"pCAL",
            b"calibration\0\0\0\0\0\0\0\0\x01\0\x02unit\0\x30\0\x31",
        ));
        source.extend(png_chunk(b"sTER", &[0]));
        source.extend(png_chunk(b"gIFg", &[0; 4]));
        finish_png(&mut source);
        assert!(inspect_png(&source).unwrap().is_empty());
        for preserve_profile in [false, true] {
            let (cleaned, removed) = clean_png_with_options(&source, preserve_profile).unwrap();
            assert_eq!(cleaned, source);
            assert!(removed.is_empty());
            verify_png_cleaned(&cleaned, preserve_profile).unwrap();
        }
    }

    #[test]
    fn rejects_png_chunks_that_cross_the_container_boundary() {
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend_from_slice(&100u32.to_be_bytes());
        source.extend_from_slice(b"tEXt");
        source.extend_from_slice(b"short");
        assert!(inspect_png(&source).is_err());

        let mut missing_image = PNG_SIGNATURE.to_vec();
        missing_image.extend(png_chunk(b"IHDR", &[0; 13]));
        missing_image.extend(png_chunk(b"IEND", b""));
        assert!(inspect_png(&missing_image).is_err());

        let mut bad_crc = png_start();
        finish_png(&mut bad_crc);
        bad_crc[20] ^= 1;
        assert!(inspect_png(&bad_crc).is_err());

        let mut invalid_dimensions = PNG_SIGNATURE.to_vec();
        invalid_dimensions.extend(png_chunk(b"IHDR", &[0; 13]));
        invalid_dimensions.extend(png_chunk(b"IDAT", b"pixels"));
        invalid_dimensions.extend(png_chunk(b"IEND", b""));
        assert!(inspect_png(&invalid_dimensions).is_err());
    }

    #[test]
    fn strips_webp_exif_and_updates_size() {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend_from_slice(b"EXIF\x04\0\0\0data");
        source.extend(webp_chunk(b"VP8 ", &webp_lossy_data(1, 1, b"image")));
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        let (cleaned, findings) = clean_webp_with_options(&source, true).unwrap();
        assert_eq!(findings[0].count, 1);
        assert!(!cleaned.windows(4).any(|window| window == b"EXIF"));
        assert!(cleaned.windows(4).any(|window| window == b"VP8 "));
    }

    #[test]
    fn clears_webp_extended_metadata_flags() {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend_from_slice(b"VP8X\x0a\0\0\0\x0c\0\0\0\0\0\0\0\0\0");
        source.extend_from_slice(b"EXIF\x04\0\0\0data");
        source.extend(webp_chunk(b"VP8 ", &webp_lossy_data(1, 1, b"image")));
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        let (cleaned, findings) = clean_webp_with_options(&source, true).unwrap();
        assert_eq!(findings[0].count, 1);
        assert_eq!(cleaned[20] & 0x0c, 0);
        assert!(!cleaned.windows(4).any(|window| window == b"EXIF"));
    }

    #[test]
    fn rejects_invalid_webp_lengths() {
        assert!(inspect_webp(b"RIFF").is_err());
        assert!(inspect_webp(b"RIFF\x05\0\0\0WEBP").is_err());
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend_from_slice(b"EXIF\xff\xff\xff\x7f");
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&source).is_err());

        let mut dangling = b"RIFF\0\0\0\0WEBPtail".to_vec();
        let size = (dangling.len() - 8) as u32;
        dangling[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&dangling).is_err());

        let mut metadata_only = b"RIFF\0\0\0\0WEBP".to_vec();
        metadata_only.extend(webp_chunk(b"EXIF", b"data"));
        let size = (metadata_only.len() - 8) as u32;
        metadata_only[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&metadata_only).is_err());

        let mut misplaced_extended = b"RIFF\0\0\0\0WEBP".to_vec();
        misplaced_extended.extend(webp_chunk(b"VP8 ", b"image"));
        misplaced_extended.extend(webp_chunk(b"VP8X", &[0; 10]));
        let size = (misplaced_extended.len() - 8) as u32;
        misplaced_extended[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&misplaced_extended).is_err());

        let mut reserved_flag = b"RIFF\0\0\0\0WEBP".to_vec();
        reserved_flag.extend(webp_chunk(b"VP8X", &[0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        reserved_flag.extend(webp_chunk(b"VP8 ", b"image"));
        let size = (reserved_flag.len() - 8) as u32;
        reserved_flag[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&reserved_flag).is_err());

        let mut conflicting_payloads = b"RIFF\0\0\0\0WEBP".to_vec();
        conflicting_payloads.extend(webp_chunk(b"VP8 ", b"lossy"));
        conflicting_payloads.extend(webp_chunk(b"VP8L", b"lossless"));
        let size = (conflicting_payloads.len() - 8) as u32;
        conflicting_payloads[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_webp(&conflicting_payloads).is_err());
    }

    #[test]
    fn preserves_or_removes_jpeg_icc_profiles_explicitly() {
        let mut source = vec![0xff, 0xd8];
        source.extend(jpeg_segment(0xe2, b"ICC_PROFILE\0\x01\x01display-profile"));
        source.extend_from_slice(&[0xff, 0xd9]);

        let scanned = inspect_jpeg(&source).unwrap();
        assert_eq!(scanned[0].category, "color_profile");
        assert_eq!(scanned[0].severity, FindingSeverity::Informational);

        let (preserved, removed) = clean_jpeg_with_options(&source, true, true).unwrap();
        assert!(preserved.windows(11).any(|window| window == b"ICC_PROFILE"));
        assert!(removed.is_empty());
        verify_jpeg_cleaned(&preserved, true, true).unwrap();

        let (stripped, removed) = clean_jpeg_with_options(&source, true, false).unwrap();
        assert!(!stripped.windows(11).any(|window| window == b"ICC_PROFILE"));
        assert_eq!(removed[0].category, "color_profile");
        verify_jpeg_cleaned(&stripped, true, false).unwrap();
        assert!(verify_jpeg_cleaned(&source, true, false).is_err());
    }

    #[test]
    fn preserves_or_removes_png_icc_profiles_explicitly() {
        let mut source = png_start();
        source.extend(png_chunk(b"iCCP", b"Display\0\0profile"));
        finish_png(&mut source);

        let (preserved, removed) = clean_png_with_options(&source, true).unwrap();
        assert!(preserved.windows(4).any(|window| window == b"iCCP"));
        assert!(removed.is_empty());
        verify_png_cleaned(&preserved, true).unwrap();

        let (stripped, removed) = clean_png_with_options(&source, false).unwrap();
        assert!(!stripped.windows(4).any(|window| window == b"iCCP"));
        assert_eq!(removed[0].category, "color_profile");
        verify_png_cleaned(&stripped, false).unwrap();
        assert!(verify_png_cleaned(&source, false).is_err());
    }

    #[test]
    fn preserves_or_removes_webp_icc_profiles_and_feature_flag() {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend(webp_chunk(b"VP8X", &[0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        source.extend(webp_chunk(b"ICCP", b"profile"));
        source.extend(webp_chunk(b"VP8 ", &webp_lossy_data(1, 1, b"image")));
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());

        let (preserved, removed) = clean_webp_with_options(&source, true).unwrap();
        assert!(preserved.windows(4).any(|window| window == b"ICCP"));
        assert_eq!(preserved[20] & 0x20, 0x20);
        assert!(removed.is_empty());
        verify_webp_cleaned(&preserved, true).unwrap();

        let (stripped, removed) = clean_webp_with_options(&source, false).unwrap();
        assert!(!stripped.windows(4).any(|window| window == b"ICCP"));
        assert_eq!(stripped[20] & 0x20, 0);
        assert_eq!(removed[0].category, "color_profile");
        verify_webp_cleaned(&stripped, false).unwrap();
        assert!(verify_webp_cleaned(&source, false).is_err());
    }

    #[test]
    fn verification_allows_only_the_rebuilt_orientation_segment() {
        let cleaned = clean_jpeg_with_options(&jpeg_with_orientation(6), true, true)
            .unwrap()
            .0;
        verify_jpeg_cleaned(&cleaned, true, true).unwrap();
        assert!(verify_jpeg_cleaned(&jpeg_with_exif(), true, true).is_err());

        let orientation = jpeg_segments(&cleaned).unwrap()[0].2.clone();
        let mut duplicated = cleaned[..orientation.end].to_vec();
        duplicated.extend_from_slice(&cleaned[orientation]);
        duplicated.extend_from_slice(&[0xff, 0xd9]);
        assert!(verify_jpeg_cleaned(&duplicated, true, true).is_err());
    }
}
