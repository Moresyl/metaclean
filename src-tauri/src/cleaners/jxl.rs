//! JPEG XL can be a bare codestream or an ISOBMFF-style box container. Privacy
//! metadata only lives in container boxes, so the cleaner retires those boxes
//! in place as `free` boxes. Box sizes and all following offsets stay stable.

use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const CONTAINER_SIGNATURE: &[u8] = b"\0\0\0\x0cJXL \r\n\x87\n";

#[derive(Debug, Clone)]
struct BoxNode {
    kind: [u8; 4],
    range: Range<usize>,
    header: usize,
}

impl BoxNode {
    fn payload(&self) -> Range<usize> {
        self.range.start + self.header..self.range.end
    }
}

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

fn parse_box(data: &[u8], offset: usize) -> Result<BoxNode> {
    let short_end = offset
        .checked_add(8)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| invalid("JPEG XL 盒头不完整"))?;
    let size = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
    let kind = data[offset + 4..short_end].try_into().unwrap();
    let (length, header) = match size {
        0 => (data.len() - offset, 8),
        1 => {
            let extended_end = offset
                .checked_add(16)
                .filter(|end| *end <= data.len())
                .ok_or_else(|| invalid("JPEG XL 扩展盒长度缺失"))?;
            let length = usize::try_from(u64::from_be_bytes(
                data[short_end..extended_end].try_into().unwrap(),
            ))
            .map_err(|_| invalid("JPEG XL 盒长度超出平台范围"))?;
            (length, 16)
        }
        value => (value as usize, 8),
    };
    let end = offset
        .checked_add(length)
        .filter(|end| length >= header && *end <= data.len())
        .ok_or_else(|| invalid("JPEG XL 盒长度越界"))?;
    Ok(BoxNode {
        kind,
        range: offset..end,
        header,
    })
}

fn container_boxes(data: &[u8]) -> Result<Option<Vec<BoxNode>>> {
    if data.starts_with(b"\xff\x0a") {
        if data.len() == 2 {
            return Err(invalid("JPEG XL 裸码流不完整"));
        }
        return Ok(None);
    }
    if !data.starts_with(CONTAINER_SIGNATURE) {
        return Err(invalid("不是有效 JPEG XL"));
    }
    let mut boxes = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let node = parse_box(data, offset)?;
        if node.range.end == offset {
            return Err(invalid("JPEG XL 盒长度为零"));
        }
        offset = node.range.end;
        boxes.push(node);
    }
    if boxes
        .first()
        .is_none_or(|node| node.kind != *b"JXL " || &data[node.payload()] != b"\r\n\x87\n")
    {
        return Err(invalid("JPEG XL 容器签名盒无效"));
    }
    let file_type = boxes
        .get(1)
        .ok_or_else(|| invalid("JPEG XL 缺少文件类型盒"))?;
    let brand = &data[file_type.payload()];
    if file_type.kind != *b"ftyp"
        || brand.len() < 8
        || !(brand.len() - 8).is_multiple_of(4)
        || (&brand[..4] != b"jxl " && !brand[8..].chunks_exact(4).any(|value| value == b"jxl "))
    {
        return Err(invalid("JPEG XL 文件类型盒无效"));
    }

    let complete: Vec<_> = boxes.iter().filter(|node| node.kind == *b"jxlc").collect();
    let partial: Vec<_> = boxes.iter().filter(|node| node.kind == *b"jxlp").collect();
    if boxes
        .iter()
        .filter(|node| node.kind == *b"brob")
        .any(|node| node.payload().len() <= 4)
    {
        return Err(invalid("JPEG XL Brotli 盒负载不完整"));
    }
    if complete.len() > 1 || (!complete.is_empty() && !partial.is_empty()) {
        return Err(invalid("JPEG XL 码流盒组合无效"));
    }
    if let Some(node) = complete.first() {
        if data[node.payload()].len() <= 2 || !data[node.payload()].starts_with(b"\xff\x0a") {
            return Err(invalid("JPEG XL 码流签名无效"));
        }
    } else {
        if partial.is_empty() {
            return Err(invalid("JPEG XL 缺少码流盒"));
        }
        let mut indexes = Vec::with_capacity(partial.len());
        for node in partial {
            let payload = node.payload();
            if payload.len() <= 4 {
                return Err(invalid("JPEG XL 分片码流索引缺失"));
            }
            indexes.push(u32::from_be_bytes(
                data[payload.start..payload.start + 4].try_into().unwrap(),
            ));
        }
        if indexes.iter().enumerate().any(|(index, value)| {
            (*value & 0x7fff_ffff) != index as u32
                || (*value & 0x8000_0000 != 0) != (index + 1 == indexes.len())
        }) {
            return Err(invalid("JPEG XL 分片码流索引无效"));
        }
        let first = boxes
            .iter()
            .find(|node| {
                let payload = node.payload();
                node.kind == *b"jxlp"
                    && u32::from_be_bytes(
                        data[payload.start..payload.start + 4].try_into().unwrap(),
                    ) & 0x7fff_ffff
                        == 0
            })
            .ok_or_else(|| invalid("JPEG XL 缺少首个码流分片"))?;
        if !data[first.payload().start + 4..first.range.end].starts_with(b"\xff\x0a") {
            return Err(invalid("JPEG XL 分片码流签名无效"));
        }
    }
    Ok(Some(boxes))
}

fn private_kind(data: &[u8], node: &BoxNode) -> Option<([u8; 4], bool)> {
    let kind = if node.kind == *b"brob" {
        let payload = node.payload();
        data.get(payload.start..payload.start + 4)?
            .try_into()
            .ok()?
    } else {
        node.kind
    };
    matches!(&kind, b"Exif" | b"xml " | b"jbrd" | b"jumb").then_some((kind, kind == *b"jumb"))
}

pub fn is_jxl(data: &[u8]) -> bool {
    container_boxes(data).is_ok()
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let Some(boxes) = container_boxes(data)? else {
        return Ok(Vec::new());
    };
    let mut metadata = 0;
    let mut provenance = 0;
    for node in &boxes {
        if let Some((_, is_provenance)) = private_kind(data, node) {
            if is_provenance {
                provenance += 1;
            } else {
                metadata += 1;
            }
        }
    }
    let mut findings = Vec::new();
    if metadata > 0 {
        findings.push(Finding {
            category: "image_metadata".into(),
            label: "JPEG XL EXIF / XMP / JPEG 重建数据".into(),
            count: metadata,
            severity: FindingSeverity::Privacy,
        });
    }
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "JPEG XL JUMBF / C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    Ok(findings)
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect(data)?;
    let Some(boxes) = container_boxes(data)? else {
        return Ok((data.to_vec(), findings));
    };
    let mut output = data.to_vec();
    for node in boxes {
        if private_kind(data, &node).is_some() {
            output[node.range.start + 4..node.range.start + 8].copy_from_slice(b"free");
            output[node.payload()].fill(0);
        }
    }
    Ok((output, findings))
}

pub fn verify_cleaned(data: &[u8]) -> Result<()> {
    if inspect(data)?.is_empty() {
        Ok(())
    } else {
        Err(CleanError::Verification(
            "JPEG XL 中仍存在应移除的元数据盒".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jxl_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn sample() -> Vec<u8> {
        let mut bytes = CONTAINER_SIGNATURE.to_vec();
        bytes.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        bytes.extend(jxl_box(b"Exif", b"\0\0\0\0II*\0private"));
        bytes.extend(jxl_box(b"xml ", b"<x:xmpmeta>Alice</x:xmpmeta>"));
        bytes.extend(jxl_box(b"jumb", b"c2pa manifest"));
        bytes.extend(jxl_box(b"brob", b"Exifcompressed-private-data"));
        bytes.extend(jxl_box(b"jbrd", b"jpeg reconstruction data"));
        bytes.extend(jxl_box(b"jxlc", b"\xff\x0aIMAGE-CODESTREAM"));
        bytes
    }

    #[test]
    fn retires_metadata_boxes_without_moving_the_codestream() {
        let source = sample();
        let boxes = container_boxes(&source).unwrap().unwrap();
        let image = boxes.iter().find(|node| node.kind == *b"jxlc").unwrap();
        let image_before = source[image.range.clone()].to_vec();
        let findings = inspect(&source).unwrap();
        assert_eq!(
            findings.iter().map(|finding| finding.count).sum::<usize>(),
            5
        );

        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert_eq!(removed, findings);
        assert_eq!(&cleaned[image.range.clone()], image_before);
        assert_eq!(
            cleaned.windows(4).filter(|value| *value == b"free").count(),
            5
        );
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn accepts_bare_codestreams_and_rejects_malformed_containers() {
        let raw = b"\xff\x0aIMAGE-CODESTREAM";
        assert!(is_jxl(raw));
        assert_eq!(clean(raw).unwrap().0, raw);

        let mut truncated = sample();
        truncated.pop();
        assert!(!is_jxl(&truncated));
        assert!(clean(&truncated).is_err());

        let mut mixed = CONTAINER_SIGNATURE.to_vec();
        mixed.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        mixed.extend(jxl_box(b"jxlc", b"\xff\x0aFULL"));
        mixed.extend(jxl_box(b"jxlp", b"\x80\0\0\0\xff\x0aPART"));
        assert!(inspect(&mixed).is_err());

        let mut partial = CONTAINER_SIGNATURE.to_vec();
        partial.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        partial.extend(jxl_box(b"jxlp", b"\x80\0\0\0\xff\x0aONLY-PART"));
        assert!(is_jxl(&partial));

        let mut two_parts = CONTAINER_SIGNATURE.to_vec();
        two_parts.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        two_parts.extend(jxl_box(b"jxlp", b"\0\0\0\0\xff\x0aFIRST"));
        two_parts.extend(jxl_box(b"jxlp", b"\x80\0\0\x01SECOND"));
        assert!(is_jxl(&two_parts));
        assert_eq!(clean(&two_parts).unwrap().0, two_parts);

        assert!(!is_jxl(b"\xff\x0a"));

        let mut missing_fragment = CONTAINER_SIGNATURE.to_vec();
        missing_fragment.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        missing_fragment.extend(jxl_box(b"jxlp", b"\0\0\0\0\xff\x0aFIRST"));
        missing_fragment.extend(jxl_box(b"jxlp", b"\x80\0\0\x02LAST"));
        assert!(inspect(&missing_fragment).is_err());

        let mut out_of_order = CONTAINER_SIGNATURE.to_vec();
        out_of_order.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        out_of_order.extend(jxl_box(b"jxlp", b"\0\0\0\x01SECOND"));
        out_of_order.extend(jxl_box(b"jxlp", b"\x80\0\0\0\xff\x0aFIRST"));
        assert!(inspect(&out_of_order).is_err());

        let mut short_brob = CONTAINER_SIGNATURE.to_vec();
        short_brob.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        short_brob.extend(jxl_box(b"brob", b"Exif"));
        short_brob.extend(jxl_box(b"jxlc", b"\xff\x0aIMAGE"));
        assert!(inspect(&short_brob).is_err());

        let mut uneven_brands = CONTAINER_SIGNATURE.to_vec();
        uneven_brands.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0x"));
        uneven_brands.extend(jxl_box(b"jxlc", b"\xff\x0aIMAGE"));
        assert!(inspect(&uneven_brands).is_err());
    }
}
