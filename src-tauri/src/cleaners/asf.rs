//! ASF — the container behind WMV, WMA and .asf — is a flat list of objects,
//! each one a sixteen byte GUID followed by its own length. Titles, authors,
//! copyright lines and the whole `WM/` attribute space live in named objects
//! inside the header.
//!
//! The format defines a padding object precisely so that a writer can reserve
//! space it does not intend to use, and every reader is required to skip it. So
//! a metadata object is retired by stamping the padding GUID over its own and
//! blanking the body. The object count in the header stays honest, no length
//! changes, and the media object never moves.

use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const GUID: usize = 16;
const OBJECT_HEADER: usize = 24;
/// Children of the header object begin after its count and two reserved bytes.
const HEADER_PREAMBLE: usize = OBJECT_HEADER + 6;

const HEADER_OBJECT: [u8; GUID] = [
    0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62, 0xce, 0x6c,
];
const PADDING_OBJECT: [u8; GUID] = [
    0x74, 0xd4, 0x06, 0x18, 0xdf, 0xca, 0x09, 0x45, 0xa4, 0xba, 0x9a, 0xab, 0xcb, 0x96, 0xaa, 0xe8,
];
const CONTENT_DESCRIPTION: [u8; GUID] = [
    0x33, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62, 0xce, 0x6c,
];
const EXTENDED_CONTENT_DESCRIPTION: [u8; GUID] = [
    0x40, 0xa4, 0xd0, 0xd2, 0x07, 0xe3, 0xd2, 0x11, 0x97, 0xf0, 0x00, 0xa0, 0xc9, 0x5e, 0xa8, 0x50,
];
const METADATA_OBJECT: [u8; GUID] = [
    0xea, 0xcb, 0xf8, 0xc5, 0xaf, 0x5b, 0x77, 0x48, 0x84, 0x67, 0xaa, 0x8c, 0x44, 0xfa, 0x4c, 0xca,
];
const METADATA_LIBRARY_OBJECT: [u8; GUID] = [
    0x94, 0x1c, 0x23, 0x44, 0x98, 0x94, 0xd1, 0x49, 0xa1, 0x41, 0x1d, 0x13, 0x4e, 0x45, 0x70, 0x54,
];
const HEADER_EXTENSION_OBJECT: [u8; GUID] = [
    0xb5, 0x03, 0xbf, 0x5f, 0x2e, 0xa9, 0xcf, 0x11, 0x8e, 0xe3, 0x00, 0xc0, 0x0c, 0x20, 0x53, 0x65,
];
const HEADER_EXTENSION_PREAMBLE: usize = OBJECT_HEADER + GUID + 2 + 4;

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

pub fn is_asf(data: &[u8]) -> bool {
    data.len() > HEADER_PREAMBLE && data[..GUID] == HEADER_OBJECT
}

fn object_at(data: &[u8], offset: usize, end: usize) -> Result<([u8; GUID], Range<usize>)> {
    let header_end = offset
        .checked_add(OBJECT_HEADER)
        .filter(|value| *value <= end)
        .ok_or_else(|| invalid("ASF 对象头不完整"))?;
    let guid: [u8; GUID] = data[offset..offset + GUID].try_into().unwrap();
    let size = u64::from_le_bytes(data[offset + GUID..header_end].try_into().unwrap());
    let size = usize::try_from(size).map_err(|_| invalid("ASF 对象长度超出平台范围"))?;
    let stop = offset
        .checked_add(size)
        .filter(|value| size >= OBJECT_HEADER && *value <= end)
        .ok_or_else(|| invalid("ASF 对象长度越界"))?;
    Ok((guid, offset..stop))
}

fn private(guid: &[u8; GUID]) -> bool {
    matches!(
        *guid,
        CONTENT_DESCRIPTION
            | EXTENDED_CONTENT_DESCRIPTION
            | METADATA_OBJECT
            | METADATA_LIBRARY_OBJECT
    )
}

fn private_objects(data: &[u8]) -> Result<Vec<Range<usize>>> {
    if !is_asf(data) {
        return Err(invalid("不是有效 ASF / WMV"));
    }
    let (_, header) = object_at(data, 0, data.len())?;
    if header.len() < HEADER_PREAMBLE {
        return Err(invalid("ASF Header 对象过短"));
    }
    let object_count =
        u32::from_le_bytes(data[OBJECT_HEADER..OBJECT_HEADER + 4].try_into().unwrap());
    let object_count = usize::try_from(object_count).map_err(|_| invalid("ASF 对象数量过大"))?;
    let mut found = Vec::new();
    let mut offset = HEADER_PREAMBLE;
    for _ in 0..object_count {
        let (guid, range) = object_at(data, offset, header.end)?;
        if private(&guid) {
            found.push(range.clone());
        } else if guid == HEADER_EXTENSION_OBJECT {
            collect_header_extension(data, &range, &mut found)?;
        }
        offset = range.end;
    }
    if offset != header.end {
        return Err(invalid("ASF Header 对象数量或尾部长度不匹配"));
    }
    Ok(found)
}

fn collect_header_extension(
    data: &[u8],
    extension: &Range<usize>,
    found: &mut Vec<Range<usize>>,
) -> Result<()> {
    if extension.len() < HEADER_EXTENSION_PREAMBLE {
        return Err(invalid("ASF Header Extension 对象过短"));
    }
    let size_at = extension.start + OBJECT_HEADER + GUID + 2;
    let extension_size = u32::from_le_bytes(data[size_at..size_at + 4].try_into().unwrap());
    let extension_size =
        usize::try_from(extension_size).map_err(|_| invalid("ASF Header Extension 数据过大"))?;
    let mut offset = extension.start + HEADER_EXTENSION_PREAMBLE;
    let expected_end = offset
        .checked_add(extension_size)
        .filter(|value| *value == extension.end)
        .ok_or_else(|| invalid("ASF Header Extension 长度不匹配"))?;
    while offset < expected_end {
        let (guid, range) = object_at(data, offset, expected_end)?;
        if private(&guid) {
            found.push(range.clone());
        }
        offset = range.end;
    }
    Ok(())
}

fn findings(count: usize) -> Vec<Finding> {
    if count == 0 {
        Vec::new()
    } else {
        vec![Finding {
            category: "video_metadata".into(),
            label: "WMV / ASF 标题、作者与扩展属性".into(),
            count,
            severity: FindingSeverity::Privacy,
        }]
    }
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(private_objects(data)?.len()))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let objects = private_objects(data)?;
    let mut output = data.to_vec();
    for range in &objects {
        output[range.start..range.start + GUID].copy_from_slice(&PADDING_OBJECT);
        output[range.start + OBJECT_HEADER..range.end].fill(0);
    }
    Ok((output, findings(objects.len())))
}

pub fn verify_cleaned(data: &[u8]) -> Result<()> {
    let residual = private_objects(data)?.len();
    if residual > 0 {
        return Err(CleanError::Verification(format!(
            "ASF 中仍发现 {residual} 项应移除的痕迹"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(guid: &[u8; GUID], payload: &[u8]) -> Vec<u8> {
        let mut bytes = guid.to_vec();
        bytes.extend_from_slice(&((payload.len() + OBJECT_HEADER) as u64).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    fn sample() -> Vec<u8> {
        let description = object(&CONTENT_DESCRIPTION, b"\x0c\0\0\0\0\0Alice Zhang\0");
        let extended = object(&EXTENDED_CONTENT_DESCRIPTION, b"WM/ToolName=CameraSuite");
        let stream = object(&[0x11; GUID], b"STREAM-PROPERTIES");
        let mut payload = 3u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&[1, 2]);
        payload.extend(stream);
        payload.extend(description);
        payload.extend(extended);
        let mut file = object(&HEADER_OBJECT, &payload);
        file.extend(object(&[0x22; GUID], b"MEDIA-PACKETS"));
        file
    }

    #[test]
    fn retires_metadata_objects_into_padding() {
        let source = sample();
        assert!(is_asf(&source));
        assert_eq!(inspect(&source).unwrap()[0].count, 2);

        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert_eq!(removed[0].count, 2);
        assert!(!contains(&cleaned, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"CameraSuite"));
        assert!(contains(&cleaned, b"STREAM-PROPERTIES"));
        assert!(cleaned.ends_with(b"MEDIA-PACKETS"));
        assert!(contains(&cleaned, &PADDING_OBJECT));
        assert!(inspect(&cleaned).unwrap().is_empty());
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn rejects_containers_that_are_not_asf() {
        assert!(inspect(b"RIFF\0\0\0\0AVI ").is_err());
        assert!(!is_asf(&[0u8; 8]));
        let mut truncated = HEADER_OBJECT.to_vec();
        truncated.extend_from_slice(&9_000u64.to_le_bytes());
        truncated.extend_from_slice(&[0; 16]);
        assert!(inspect(&truncated).is_err());
    }

    #[test]
    fn removes_metadata_nested_in_the_header_extension() {
        let metadata = object(&METADATA_LIBRARY_OBJECT, b"WM/AlbumArtist=Alice");
        let mut extension_payload = vec![0u8; GUID];
        extension_payload.extend_from_slice(&6u16.to_le_bytes());
        extension_payload.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        extension_payload.extend(metadata);
        let extension = object(&HEADER_EXTENSION_OBJECT, &extension_payload);
        let stream = object(&[0x11; GUID], b"STREAM-PROPERTIES");
        let mut header_payload = 2u32.to_le_bytes().to_vec();
        header_payload.extend_from_slice(&[1, 2]);
        header_payload.extend(stream);
        header_payload.extend(extension);
        let source = object(&HEADER_OBJECT, &header_payload);

        assert_eq!(inspect(&source).unwrap()[0].count, 1);
        let (cleaned, _) = clean(&source).unwrap();
        assert!(!contains(&cleaned, b"AlbumArtist"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn rejects_header_object_count_mismatches() {
        let mut payload = 2u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&[1, 2]);
        payload.extend(object(&[0x11; GUID], b"one object only"));
        assert!(inspect(&object(&HEADER_OBJECT, &payload)).is_err());
    }
}
