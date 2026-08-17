use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

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

pub fn inspect_jpeg(data: &[u8]) -> Result<Vec<Finding>> {
    let segments = jpeg_segments(data)?;
    let mut exif = 0;
    let mut xmp = 0;
    let mut provenance = 0;
    let mut comments = 0;
    let mut color_profiles = 0;
    for (marker, payload, _) in segments {
        match marker {
            0xE1 if payload.starts_with(b"Exif\0\0") => exif += 1,
            0xE1 => xmp += 1,
            0xE2 if is_jpeg_icc_profile(marker, payload) => color_profiles += 1,
            0xEB => provenance += 1,
            0xED | 0xFE => comments += 1,
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
        findings.push(finding("图片注释或 IPTC", comments));
    }
    if color_profiles > 0 {
        findings.push(color_profile_finding(color_profiles));
    }
    Ok(findings)
}

fn jpeg_segments(data: &[u8]) -> Result<Vec<(u8, &[u8], std::ops::Range<usize>)>> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return Err(CleanError::InvalidFormat("不是有效 JPEG".into()));
    }
    let mut result = Vec::new();
    let mut offset = 2;
    let mut terminated = false;
    while offset + 1 < data.len() {
        if data[offset] != 0xFF {
            break;
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
        if marker == 0xDA || marker == 0xD9 {
            terminated = true;
            break;
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
        result.push((
            marker,
            &data[offset + 2..offset + length],
            start..offset + length,
        ));
        offset += length;
    }
    if !terminated {
        return Err(CleanError::InvalidFormat(
            "JPEG 缺少图像数据或结束标记".into(),
        ));
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
        .filter(|finding| finding.category != "color_profile" || !preserve_color_profile)
        .collect();
    let segments = jpeg_segments(data)?;
    let orientation = preserve_orientation
        .then(|| jpeg_orientation(&segments))
        .flatten();
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for (marker, payload, range) in segments {
        let remove = matches!(marker, 0xE1 | 0xEB | 0xED | 0xFE)
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload));
        if remove {
            output.extend_from_slice(&data[cursor..range.start]);
            cursor = range.end;
        }
    }
    output.extend_from_slice(&data[cursor..]);
    if let Some(value) = orientation {
        let segment = orientation_segment(value);
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
    let mut orientation_segments = 0;
    for (marker, payload, _) in segments {
        if marker == 0xE1 && preserve_orientation && is_minimal_orientation(payload) {
            orientation_segments += 1;
            continue;
        }
        if matches!(marker, 0xE1 | 0xEB | 0xED | 0xFE)
            || (!preserve_color_profile && is_jpeg_icc_profile(marker, payload))
        {
            return Err(CleanError::Verification(
                "JPEG 中仍存在应移除的元数据段".into(),
            ));
        }
    }
    if orientation_segments > 1 {
        return Err(CleanError::Verification(
            "JPEG 中存在多个保留的方向段".into(),
        ));
    }
    Ok(())
}

fn is_minimal_orientation(payload: &[u8]) -> bool {
    let Some(orientation) = payload.strip_prefix(b"Exif\0\0").and_then(tiff_orientation) else {
        return false;
    };
    orientation_segment(orientation).get(4..) == Some(payload)
}

fn jpeg_orientation(segments: &[(u8, &[u8], std::ops::Range<usize>)]) -> Option<u16> {
    segments.iter().find_map(|(marker, payload, _)| {
        if *marker != 0xE1 || !payload.starts_with(b"Exif\0\0") {
            return None;
        }
        tiff_orientation(&payload[6..])
    })
}

fn tiff_orientation(tiff: &[u8]) -> Option<u16> {
    if tiff.len() < 8 {
        return None;
    }
    let little_endian = match &tiff[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
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
    if read_u16(&tiff[2..])? != 42 {
        return None;
    }
    let ifd_offset = usize::try_from(read_u32(&tiff[4..])?).ok()?;
    let count = usize::from(read_u16(tiff.get(ifd_offset..)?)?);
    for index in 0..count {
        let start = ifd_offset.checked_add(2 + index * 12)?;
        let entry = tiff.get(start..start + 12)?;
        if read_u16(entry)? == 0x0112 && read_u16(&entry[2..])? == 3 && read_u32(&entry[4..])? == 1
        {
            let value = read_u16(&entry[8..])?;
            return (1..=8).contains(&value).then_some(value);
        }
    }
    None
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
    while offset + 12 <= data.len() {
        let length = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12 + length)
            .ok_or_else(|| CleanError::InvalidFormat("PNG 块长度溢出".into()))?;
        if end > data.len() {
            return Err(CleanError::InvalidFormat("PNG 块越界".into()));
        }
        let kind: [u8; 4] = data[offset + 4..offset + 8].try_into().unwrap();
        chunks.push((kind, offset..end));
        offset = end;
        if &kind == b"IEND" {
            has_iend = true;
            break;
        }
    }
    if !has_iend {
        return Err(CleanError::InvalidFormat("PNG 缺少 IEND 块".into()));
    }
    Ok(chunks)
}

fn is_private_png_chunk(kind: &[u8; 4]) -> bool {
    matches!(
        kind,
        b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"caBX" | b"c2pa" | b"jumb" | b"jumd"
    )
}

pub fn inspect_png(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = png_chunks(data)?;
    let metadata = chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt"))
        .count();
    let provenance = chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"caBX" | b"c2pa" | b"jumb" | b"jumd"))
        .count();
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
        .filter(|(kind, _)| matches!(kind, b"EXIF" | b"XMP " | b"C2PA"))
        .count();
    let color_profiles = chunks.iter().filter(|(kind, _)| kind == b"ICCP").count();
    let mut findings = Vec::new();
    if count > 0 {
        findings.push(finding("WebP EXIF / XMP / C2PA 元数据", count));
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
        source.extend(jpeg_segment(0xed, b"iptc"));
        source.extend_from_slice(&[0xff, 0xd9]);
        let findings = inspect_jpeg(&source).unwrap();
        assert_eq!(findings.len(), 4);
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
        assert_eq!(jpeg_orientation(&segments), Some(6));
        assert!(!cleaned.windows(3).any(|window| window == b"gps"));
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn removes_orientation_when_disabled() {
        let source = jpeg_with_orientation(6);
        let (cleaned, _) = clean_jpeg_with_options(&source, false, true).unwrap();
        assert_eq!(cleaned, vec![0xff, 0xd8, 0xff, 0xd9]);
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
        output.extend_from_slice(&[0, 0, 0, 0]);
        output
    }

    #[test]
    fn strips_png_text_chunks() {
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend(png_chunk(b"tEXt", b"Author\0Alice"));
        source.extend(png_chunk(b"IEND", b""));
        let (cleaned, findings) = clean_png_with_options(&source, true).unwrap();
        assert_eq!(findings[0].count, 1);
        assert!(!cleaned.windows(4).any(|window| window == b"tEXt"));
    }

    #[test]
    fn reports_and_removes_png_provenance_chunks() {
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend(png_chunk(b"caBX", b"claim"));
        source.extend(png_chunk(b"IEND", b""));
        let findings = inspect_png(&source).unwrap();
        assert_eq!(findings[0].category, "provenance");
        let (cleaned, _) = clean_png_with_options(&source, true).unwrap();
        assert!(!cleaned.windows(4).any(|window| window == b"caBX"));
    }

    #[test]
    fn rejects_png_chunks_that_cross_the_container_boundary() {
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend_from_slice(&100u32.to_be_bytes());
        source.extend_from_slice(b"tEXt");
        source.extend_from_slice(b"short");
        assert!(inspect_png(&source).is_err());
    }

    #[test]
    fn strips_webp_exif_and_updates_size() {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend_from_slice(b"EXIF\x04\0\0\0data");
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        let (cleaned, findings) = clean_webp_with_options(&source, true).unwrap();
        assert_eq!(findings[0].count, 1);
        assert_eq!(cleaned, b"RIFF\x04\0\0\0WEBP");
    }

    #[test]
    fn clears_webp_extended_metadata_flags() {
        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend_from_slice(b"VP8X\x0a\0\0\0\x0c\0\0\0\0\0\0\0\0\0");
        source.extend_from_slice(b"EXIF\x04\0\0\0data");
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
        let mut source = PNG_SIGNATURE.to_vec();
        source.extend(png_chunk(b"iCCP", b"Display\0\0profile"));
        source.extend(png_chunk(b"IEND", b""));

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
        fn webp_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut output = kind.to_vec();
            output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            output.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                output.push(0);
            }
            output
        }

        let mut source = b"RIFF\0\0\0\0WEBP".to_vec();
        source.extend(webp_chunk(b"VP8X", &[0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        source.extend(webp_chunk(b"ICCP", b"profile"));
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
