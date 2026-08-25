//! AVI is a RIFF file with an index, and the index is the problem. Some encoders
//! write `idx1` offsets relative to the `movi` list and some write them relative
//! to the file, so deleting a chunk ahead of the media desynchronises half the
//! players in the world.
//!
//! Nothing is deleted here. A private chunk is renamed to `JUNK` — RIFF's own
//! padding tag, which every parser is required to skip — and its payload is
//! zeroed. The file keeps its length, the index keeps its meaning, and the
//! metadata is gone.

use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

#[derive(Debug, Clone)]
struct Chunk {
    kind: [u8; 4],
    list_type: Option<[u8; 4]>,
    range: Range<usize>,
}

pub fn is_avi(data: &[u8]) -> bool {
    data.len() >= 12 && &data[..4] == b"RIFF" && matches!(&data[8..12], b"AVI " | b"AVIX")
}

fn chunks(data: &[u8], span: Range<usize>) -> Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    let mut offset = span.start;
    while offset + 8 <= span.end {
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8 + length + (length & 1))
            .filter(|value| *value <= span.end)
            .ok_or_else(|| invalid("AVI 数据块越界"))?;
        let kind: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
        let list_type = (kind == *b"LIST" || kind == *b"RIFF")
            .then(|| data.get(offset + 8..offset + 12))
            .flatten()
            .and_then(|bytes| bytes.try_into().ok());
        if matches!(&kind, b"LIST" | b"RIFF") && list_type.is_none() {
            return Err(invalid("AVI LIST 数据块缺少类型"));
        }
        chunks.push(Chunk {
            kind,
            list_type,
            range: offset..end,
        });
        offset = end;
    }
    if offset != span.end {
        return Err(invalid("AVI 数据块尾存在截断数据"));
    }
    Ok(chunks)
}

fn private(chunk: &Chunk) -> bool {
    if chunk.list_type == Some(*b"INFO") {
        return true;
    }
    matches!(
        &chunk.kind,
        b"IDIT"
            | b"ISFT"
            | b"IART"
            | b"ICMT"
            | b"ICOP"
            | b"ICRD"
            | b"INAM"
            | b"ISBJ"
            | b"strn"
            | b"_PMX"
            | b"XMP "
            | b"C2PA"
            | b"tdat"
            | b"Tdat"
    )
}

/// Lists whose children are structure rather than media. `movi` is skipped
/// outright: its children are video frames, and a frame that happens to spell
/// `IART` is still a frame.
fn descend(chunk: &Chunk) -> bool {
    matches!(
        chunk.list_type,
        Some(value) if matches!(&value, b"hdrl" | b"strl" | b"odml" | b"AVI " | b"AVIX")
    )
}

fn collect(data: &[u8], span: Range<usize>, depth: usize, found: &mut Vec<Chunk>) -> Result<()> {
    if depth > 8 {
        return Err(invalid("AVI LIST 嵌套超过安全上限"));
    }
    for chunk in chunks(data, span)? {
        if private(&chunk) {
            found.push(chunk);
        } else if descend(&chunk) {
            collect(
                data,
                chunk.range.start + 12..chunk.range.end,
                depth + 1,
                found,
            )?;
        }
    }
    Ok(())
}

fn private_chunks(data: &[u8]) -> Result<Vec<Chunk>> {
    if !is_avi(data) {
        return Err(invalid("不是有效 AVI"));
    }
    let mut found = Vec::new();
    let mut offset = 0usize;
    while data.get(offset..offset + 4) == Some(b"RIFF") {
        let header_end = offset
            .checked_add(12)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| invalid("AVI 顶层 RIFF 头不完整"))?;
        if !matches!(&data[offset + 8..header_end], b"AVI " | b"AVIX") {
            break;
        }
        let declared =
            u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8)
            .and_then(|start| start.checked_add(declared))
            .filter(|end| declared >= 4 && *end <= data.len())
            .ok_or_else(|| invalid("AVI RIFF 长度不匹配"))?;
        collect(data, offset + 12..end, 0, &mut found)?;
        offset = end;
    }
    // Bytes after the final complete AVI/AVIX form are outside the container.
    if data.len() > offset {
        found.push(Chunk {
            kind: *b"junk",
            list_type: None,
            range: offset..data.len(),
        });
    }
    Ok(found)
}

fn findings(chunks: &[Chunk]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let metadata = chunks.iter().filter(|chunk| chunk.kind != *b"C2PA").count();
    if metadata > 0 {
        findings.push(Finding {
            category: "video_metadata".into(),
            label: "AVI INFO 标签、拍摄时间与 XMP".into(),
            count: metadata,
            severity: FindingSeverity::Privacy,
        });
    }
    let provenance = chunks.iter().filter(|chunk| chunk.kind == *b"C2PA").count();
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "AVI C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    findings
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(&private_chunks(data)?))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let chunks = private_chunks(data)?;
    let trailing = chunks
        .iter()
        .find(|chunk| chunk.kind == *b"junk")
        .map(|chunk| chunk.range.start);
    let mut output = data[..trailing.unwrap_or(data.len())].to_vec();
    for chunk in &chunks {
        if chunk.range.start >= output.len() {
            continue;
        }
        output[chunk.range.start..chunk.range.start + 4].copy_from_slice(b"JUNK");
        let end = chunk.range.end.min(output.len());
        output[chunk.range.start + 8..end].fill(0);
    }
    Ok((output, findings(&chunks)))
}

pub fn verify_cleaned(data: &[u8]) -> Result<()> {
    let residual = private_chunks(data)?.len();
    if residual > 0 {
        return Err(CleanError::Verification(format!(
            "AVI 中仍发现 {residual} 项应移除的痕迹"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = kind.to_vec();
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    fn sample() -> Vec<u8> {
        let mut hdrl = b"hdrl".to_vec();
        hdrl.extend(chunk(b"avih", &[0; 16]));
        let mut strl = b"strl".to_vec();
        strl.extend(chunk(b"strh", &[0; 8]));
        strl.extend(chunk(b"strn", b"C:\\Users\\alice\\holiday.avi"));
        hdrl.extend(chunk(b"LIST", &strl));

        let mut info = b"INFO".to_vec();
        info.extend(chunk(b"IART", b"Alice Zhang"));
        info.extend(chunk(b"ISFT", b"CameraSuite 9"));

        let mut movi = b"movi".to_vec();
        movi.extend(chunk(b"00dc", b"FRAME-BYTES"));

        let mut body = b"AVI ".to_vec();
        body.extend(chunk(b"LIST", &hdrl));
        body.extend(chunk(b"LIST", &info));
        body.extend(chunk(b"LIST", &movi));
        body.extend(chunk(b"IDIT", b"Mon Jul 04 18:00:00 2021"));
        body.extend(chunk(b"idx1", &[0; 16]));
        chunk(b"RIFF", &body)
    }

    #[test]
    fn blanks_metadata_chunks_without_shifting_the_index() {
        let source = sample();
        assert!(is_avi(&source));
        assert_eq!(inspect(&source).unwrap()[0].count, 3);

        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert_eq!(removed[0].count, 3);
        assert!(!contains(&cleaned, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"CameraSuite 9"));
        assert!(!contains(&cleaned, b"holiday.avi"));
        assert!(!contains(&cleaned, b"Mon Jul 04"));
        assert!(contains(&cleaned, b"FRAME-BYTES"));
        assert!(contains(&cleaned, b"movi"));
        assert!(inspect(&cleaned).unwrap().is_empty());
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn leaves_frames_that_merely_look_like_tags_alone() {
        let mut movi = b"movi".to_vec();
        movi.extend(chunk(b"00dc", b"IARTAlice Zhang"));
        let mut body = b"AVI ".to_vec();
        body.extend(chunk(b"LIST", &movi));
        let source = chunk(b"RIFF", &body);
        assert!(inspect(&source).unwrap().is_empty());
        let (cleaned, _) = clean(&source).unwrap();
        assert!(contains(&cleaned, b"IARTAlice Zhang"));
    }

    #[test]
    fn drops_anything_stapled_past_the_riff_chunk() {
        let mut source = sample();
        let length = source.len();
        source.extend_from_slice(b"<x:xmpmeta>bob@example.test</x:xmpmeta>");
        assert_eq!(inspect(&source).unwrap()[0].count, 4);
        let (cleaned, _) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), length);
        assert!(!contains(&cleaned, b"bob@example.test"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn preserves_concatenated_open_dml_avix_segments() {
        let mut source = sample();
        let mut second_body = b"AVIX".to_vec();
        let mut movi = b"movi".to_vec();
        movi.extend(chunk(b"01dc", b"SECOND-SEGMENT-FRAME"));
        second_body.extend(chunk(b"LIST", &movi));
        let second = chunk(b"RIFF", &second_body);
        source.extend_from_slice(&second);

        let (cleaned, _) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert!(contains(&cleaned, b"SECOND-SEGMENT-FRAME"));
        assert!(contains(&cleaned, b"AVIX"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn removes_the_standard_riff_c2pa_chunk_as_provenance() {
        let mut body = b"AVI ".to_vec();
        body.extend(chunk(b"C2PA", b"manifest"));
        let source = chunk(b"RIFF", &body);
        let findings = inspect(&source).unwrap();
        assert_eq!(findings[0].category, "provenance");
        assert_eq!(findings[0].severity, FindingSeverity::Provenance);
        let (cleaned, _) = clean(&source).unwrap();
        assert!(!contains(&cleaned, b"manifest"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn rejects_containers_that_are_not_avi() {
        assert!(inspect(b"RIFF\0\0\0\0WAVE").is_err());
        assert!(inspect(b"RIFF").is_err());
        assert!(!is_avi(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn rejects_truncated_and_over_nested_list_layouts() {
        let mut truncated_body = b"AVI ".to_vec();
        truncated_body.extend_from_slice(b"tail");
        assert!(inspect(&chunk(b"RIFF", &truncated_body)).is_err());

        let mut nested = chunk(b"IART", b"private");
        for _ in 0..10 {
            let mut list = b"hdrl".to_vec();
            list.extend(nested);
            nested = chunk(b"LIST", &list);
        }
        let mut body = b"AVI ".to_vec();
        body.extend(nested);
        assert!(inspect(&chunk(b"RIFF", &body)).is_err());
    }
}
