use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};
use std::ops::Range;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_TRAILER: [u8; 4] = [0; 4];
type JpegSegment<'a> = (u8, &'a [u8], Range<usize>);

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

pub fn inspect_jpeg(data: &[u8]) -> Result<Vec<Finding>> {
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
        }
        match marker {
            0xE1 if payload.starts_with(b"Exif\0\0") => exif += 1,
            0xE1 => xmp += 1,
            0xE2 if is_jpeg_icc_profile(marker, payload) => color_profiles += 1,
            0xEB => provenance += 1,
            marker if is_private_jpeg_marker(marker) => comments += 1,
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
    let findings = inspect_jpeg(data)?
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
        let remove = is_private_jpeg_marker(marker)
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload));
        if remove {
            output.extend_from_slice(&data[cursor..range.start]);
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
        if is_private_jpeg_marker(marker)
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload))
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
    matches!(kind, b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"caBX") || *kind == PNG_TRAILER
}

pub fn inspect_png(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = png_chunks(data)?;
    let metadata = chunks
        .iter()
        .filter(|(kind, _)| {
            matches!(kind, b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt") || *kind == PNG_TRAILER
        })
        .count();
    let provenance = chunks.iter().filter(|(kind, _)| kind == b"caBX").count();
    let color_profiles = chunks.iter().filter(|(kind, _)| kind == b"iCCP").count();
    let mut findings = Vec::new();
    if metadata > 0 {
        findings.push(finding("PNG 文本 / EXIF 元数据", metadata));
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
    let count = chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"EXIF" | b"XMP "))
        .count();
    let provenance = chunks.iter().filter(|(kind, _)| kind == b"C2PA").count();
    let color_profiles = chunks.iter().filter(|(kind, _)| kind == b"ICCP").count();
    let mut findings = Vec::new();
    if count > 0 {
        findings.push(finding("WebP EXIF / XMP 元数据", count));
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

fn webp_chunks(data: &[u8]) -> Result<Vec<([u8; 4], std::ops::Range<usize>)>> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err(CleanError::InvalidFormat("不是有效 WebP".into()));
    }
    let declared = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize + 8;
    if declared != data.len() {
        return Err(CleanError::InvalidFormat("WebP RIFF 长度不匹配".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 12;
    while offset + 8 <= declared {
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8 + length + (length & 1))
            .ok_or_else(|| CleanError::InvalidFormat("WebP 块长度溢出".into()))?;
        if end > declared {
            return Err(CleanError::InvalidFormat("WebP 块越界".into()));
        }
        chunks.push((data[offset..offset + 4].try_into().unwrap(), offset..end));
        offset = end;
    }
    if offset != declared {
        return Err(CleanError::InvalidFormat("WebP 块尾存在截断数据".into()));
    }
    if !chunks
        .iter()
        .any(|(kind, _)| matches!(kind, b"VP8 " | b"VP8L" | b"ANMF"))
    {
        return Err(CleanError::InvalidFormat("WebP 缺少图像数据块".into()));
    }
    let extended: Vec<_> = chunks.iter().filter(|(kind, _)| kind == b"VP8X").collect();
    if extended.len() > 1 || extended.iter().any(|(_, range)| range.len() != 18) {
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
        if matches!(&kind, b"EXIF" | b"XMP " | b"C2PA")
            || (!preserve_color_profile && &kind == b"ICCP")
        {
            continue;
        }
        let start = output.len();
        output.extend_from_slice(&data[range]);
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
    if webp_chunks(data)?.iter().any(|(kind, _)| {
        matches!(kind, b"EXIF" | b"XMP " | b"C2PA") || (!preserve_color_profile && kind == b"ICCP")
    }) {
        return Err(CleanError::Verification(
            "WebP 中仍存在应移除的元数据块".into(),
        ));
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
        source.extend(webp_chunk(b"VP8 ", b"image"));
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
        source.extend(webp_chunk(b"VP8 ", b"image"));
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
        source.extend(webp_chunk(b"VP8 ", b"image"));
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
