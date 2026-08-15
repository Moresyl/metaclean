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

pub fn inspect_jpeg(data: &[u8]) -> Result<Vec<Finding>> {
    let segments = jpeg_segments(data)?;
    let mut exif = 0;
    let mut xmp = 0;
    let mut provenance = 0;
    let mut comments = 0;
    for (marker, payload, _) in segments {
        match marker {
            0xE1 if payload.starts_with(b"Exif\0\0") => exif += 1,
            0xE1 => xmp += 1,
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
    Ok(findings)
}

fn jpeg_segments(data: &[u8]) -> Result<Vec<(u8, &[u8], std::ops::Range<usize>)>> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return Err(CleanError::InvalidFormat("不是有效 JPEG".into()));
    }
    let mut result = Vec::new();
    let mut offset = 2;
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
    Ok(result)
}

pub fn clean_jpeg(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_jpeg(data)?;
    let segments = jpeg_segments(data)?;
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for (marker, payload, range) in segments {
        let remove = matches!(marker, 0xEB | 0xED | 0xFE)
            || marker == 0xE1
                && (payload.starts_with(b"Exif\0\0")
                    || payload
                        .windows(3)
                        .any(|window| window.eq_ignore_ascii_case(b"xmp")));
        if remove {
            output.extend_from_slice(&data[cursor..range.start]);
            cursor = range.end;
        }
    }
    output.extend_from_slice(&data[cursor..]);
    Ok((output, findings))
}

fn png_chunks(data: &[u8]) -> Result<Vec<([u8; 4], std::ops::Range<usize>)>> {
    if !data.starts_with(PNG_SIGNATURE) {
        return Err(CleanError::InvalidFormat("不是有效 PNG".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 8;
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
            break;
        }
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
    Ok(findings)
}

pub fn clean_png(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_png(data)?;
    let chunks = png_chunks(data)?;
    let mut output = Vec::with_capacity(data.len());
    output.extend_from_slice(PNG_SIGNATURE);
    for (kind, range) in chunks {
        if !is_private_png_chunk(&kind) {
            output.extend_from_slice(&data[range]);
        }
    }
    Ok((output, findings))
}

pub fn inspect_webp(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = webp_chunks(data)?;
    let count = chunks
        .iter()
        .filter(|(kind, _)| matches!(kind, b"EXIF" | b"XMP " | b"C2PA"))
        .count();
    Ok(if count > 0 {
        vec![finding("WebP EXIF / XMP / C2PA 元数据", count)]
    } else {
        Vec::new()
    })
}

fn webp_chunks(data: &[u8]) -> Result<Vec<([u8; 4], std::ops::Range<usize>)>> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err(CleanError::InvalidFormat("不是有效 WebP".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 12;
    while offset + 8 <= data.len() {
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8 + length + (length & 1))
            .ok_or_else(|| CleanError::InvalidFormat("WebP 块长度溢出".into()))?;
        if end > data.len() {
            return Err(CleanError::InvalidFormat("WebP 块越界".into()));
        }
        chunks.push((data[offset..offset + 4].try_into().unwrap(), offset..end));
        offset = end;
    }
    Ok(chunks)
}

pub fn clean_webp(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect_webp(data)?;
    let chunks = webp_chunks(data)?;
    let mut output = Vec::with_capacity(data.len());
    output.extend_from_slice(&data[..12]);
    for (kind, range) in chunks {
        if matches!(&kind, b"EXIF" | b"XMP " | b"C2PA") {
            continue;
        }
        let start = output.len();
        output.extend_from_slice(&data[range]);
        if &kind == b"VP8X" && output.len() >= start + 9 {
            output[start + 8] &= !(0x08 | 0x04);
        }
    }
    let riff_size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Ok((output, findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_with_exif() -> Vec<u8> {
        vec![
            0xff, 0xd8, 0xff, 0xe1, 0, 10, b'E', b'x', b'i', b'f', 0, 0, 1, 2, 0xff, 0xd9,
        ]
    }
    #[test]
    fn strips_jpeg_exif_segment() {
        let source = jpeg_with_exif();
        let (cleaned, findings) = clean_jpeg(&source).unwrap();
        assert_eq!(cleaned, vec![0xff, 0xd8, 0xff, 0xd9]);
        assert_eq!(findings[0].count, 1);
    }
    #[test]
    fn rejects_truncated_png() {
        assert!(inspect_png(b"\x89PNG\r\n\x1a\n\0").is_ok());
    }
}
