//! Matroska and WebM are EBML: every element is a variable-width identifier, a
//! variable-width length, and a body. Rewriting one means rewriting every byte
//! offset in the cue index behind it, which is why so many tools refuse the
//! format outright.
//!
//! They do not have to. EBML defines `Void` for exactly this situation — an
//! element readers are required to skip — and its length field may be written
//! wider than strictly necessary. So a tag block is retired by stamping the
//! one-byte `Void` identifier over its own and widening the length to absorb the
//! difference. The element occupies the same bytes it always did, the cues stay
//! true, and the metadata is gone.
//!
//! Strings that the specification insists on keeping are not voided but
//! zero-filled, which EBML reads back as an empty string.

use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_DEPTH: usize = 8;
const VOID: u8 = 0xec;

const SEGMENT: u64 = 0x1853_8067;
const SEEK_HEAD: u64 = 0x114d_9b74;
const SEEK: u64 = 0x4dbb;
const SEEK_ID: u64 = 0x53ab;
const INFO: u64 = 0x1549_a966;
const TRACKS: u64 = 0x1654_ae6b;
const TRACK_ENTRY: u64 = 0xae;
const TAGS: u64 = 0x1254_c367;
const ATTACHMENTS: u64 = 0x1941_a469;
const DATE_UTC: u64 = 0x4461;

/// UTF-8 strings the specification marks mandatory. Emptying them is legal;
/// deleting them is not.
const BLANKED: &[u64] = &[
    0x4d80, // MuxingApp
    0x5741, // WritingApp
    0x7ba9, // Title
    0x536e, // TrackEntry Name
];

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

pub fn is_matroska(data: &[u8]) -> bool {
    data.len() > 8 && data.starts_with(&[0x1a, 0x45, 0xdf, 0xa3])
}

/// Element identifiers keep their marker bit — they are compared as raw bytes —
/// while lengths strip it to reveal the number underneath.
fn read_vint(data: &[u8], at: usize, keep_marker: bool) -> Option<(u64, usize, bool)> {
    let first = *data.get(at)?;
    if first == 0 {
        return None;
    }
    let width = first.leading_zeros() as usize + 1;
    if width > 8 || at + width > data.len() {
        return None;
    }
    // At the widest encoding the marker consumes the whole leading byte, so it
    // contributes no value bits at all.
    let mask = if width == 8 { 0 } else { 0xffu8 >> width };
    let mut value = u64::from(if keep_marker { first } else { first & mask });
    let mut unknown = first & mask == mask;
    for byte in &data[at + 1..at + width] {
        value = (value << 8) | u64::from(*byte);
        unknown &= *byte == 0xff;
    }
    Some((value, width, unknown))
}

fn write_vint(value: u64, width: usize) -> Option<Vec<u8>> {
    if width == 0 || width > 8 || value >= (1u64 << (7 * width)) - 1 {
        return None;
    }
    let mut bytes = value.to_be_bytes()[8 - width..].to_vec();
    bytes[0] |= 1 << (8 - width);
    Some(bytes)
}

#[derive(Debug, Clone)]
struct Element {
    id: u64,
    header: usize,
    range: Range<usize>,
}

impl Element {
    fn body(&self) -> Range<usize> {
        self.range.start + self.header..self.range.end
    }
}

fn elements(data: &[u8], span: Range<usize>) -> Result<Vec<Element>> {
    let mut found = Vec::new();
    let mut offset = span.start;
    while offset < span.end {
        let (id, id_len, _) =
            read_vint(data, offset, true).ok_or_else(|| invalid("MKV 元素标识无效"))?;
        let (size, size_len, unknown) =
            read_vint(data, offset + id_len, false).ok_or_else(|| invalid("MKV 元素长度无效"))?;
        let header = id_len + size_len;
        // A streaming writer may leave a length unknown; such an element runs to
        // the end of whatever contains it.
        let end = if unknown {
            span.end
        } else {
            offset
                .checked_add(header)
                .and_then(|start| {
                    usize::try_from(size)
                        .ok()
                        .and_then(|len| start.checked_add(len))
                })
                .filter(|end| *end <= span.end)
                .ok_or_else(|| invalid("MKV 元素长度越界"))?
        };
        found.push(Element {
            id,
            header,
            range: offset..end,
        });
        if unknown {
            break;
        }
        offset = end;
    }
    Ok(found)
}

#[derive(Debug, Default)]
struct Plan {
    void: Vec<Element>,
    blank: Vec<Range<usize>>,
}

impl Plan {
    fn total(&self) -> usize {
        self.void.len() + self.blank.len()
    }
}

fn descend(id: u64) -> bool {
    matches!(id, SEGMENT | INFO | TRACKS | TRACK_ENTRY | SEEK_HEAD | SEEK)
}

/// A seek entry that advertises where the tags live is itself a pointer to
/// nothing once they are gone.
fn stale_seek(data: &[u8], element: &Element) -> bool {
    let Ok(children) = elements(data, element.body()) else {
        return false;
    };
    children.iter().any(|child| {
        child.id == SEEK_ID
            && matches!(
                data.get(child.body()),
                Some(bytes)
                    if bytes == 0x1254_c367u32.to_be_bytes()
                        || bytes == 0x1941_a469u32.to_be_bytes()
            )
    })
}

fn collect(data: &[u8], span: Range<usize>, depth: usize, plan: &mut Plan) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(invalid("Matroska 元素嵌套超过安全上限"));
    }
    for element in elements(data, span)? {
        if matches!(element.id, TAGS | ATTACHMENTS | DATE_UTC) {
            plan.void.push(element);
            continue;
        }
        if element.id == SEEK && stale_seek(data, &element) {
            plan.void.push(element);
            continue;
        }
        if BLANKED.contains(&element.id) {
            let body = element.body();
            if !body.is_empty() && data[body.clone()].iter().any(|byte| *byte != 0) {
                plan.blank.push(body);
            }
            continue;
        }
        if descend(element.id) {
            collect(data, element.body(), depth + 1, plan)?;
        }
    }
    Ok(())
}

fn plan(data: &[u8]) -> Result<Plan> {
    if !is_matroska(data) {
        return Err(invalid("不是有效 Matroska / WebM"));
    }
    let top = elements(data, 0..data.len())?;
    if !top.iter().any(|element| element.id == SEGMENT) {
        return Err(invalid("Matroska 缺少 Segment 元素"));
    }
    let mut plan = Plan::default();
    for element in top {
        if descend(element.id) {
            collect(data, element.body(), 1, &mut plan)?;
        }
    }
    Ok(plan)
}

fn findings(count: usize) -> Vec<Finding> {
    if count == 0 {
        Vec::new()
    } else {
        vec![Finding {
            category: "video_metadata".into(),
            label: "Matroska 标签、附件与制作工具信息".into(),
            count,
            severity: FindingSeverity::Privacy,
        }]
    }
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(plan(data)?.total()))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let plan = plan(data)?;
    let mut output = data.to_vec();
    for element in &plan.void {
        // `Void` costs one byte of identifier, so the length field inherits
        // whatever width the original identifier gave up.
        let width = element.header - 1;
        let body = element.range.len() - element.header;
        let Some(length) = write_vint(body as u64, width) else {
            return Err(invalid("MKV 元素过长，无法改写为 Void"));
        };
        output[element.range.start] = VOID;
        output[element.range.start + 1..element.range.start + element.header]
            .copy_from_slice(&length);
        output[element.range.start + element.header..element.range.end].fill(0);
    }
    for range in &plan.blank {
        output[range.clone()].fill(0);
    }
    Ok((output, findings(plan.total())))
}

pub fn verify_cleaned(data: &[u8]) -> Result<()> {
    let residual = plan(data)?.total();
    if residual > 0 {
        return Err(CleanError::Verification(format!(
            "Matroska 中仍发现 {residual} 项应移除的痕迹"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(id: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut bytes = id.to_vec();
        bytes.extend(write_vint(payload.len() as u64, 4).unwrap());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    fn sample() -> Vec<u8> {
        let mut info = element(&[0x2a, 0xd7, 0xb1], &[0x0f, 0x42, 0x40]); // TimestampScale
        info.extend(element(&[0x4d, 0x80], b"libebml v1.4.2"));
        info.extend(element(&[0x57, 0x41], b"mkvmerge on alice-laptop"));
        info.extend(element(&[0x7b, 0xa9], b"Holiday in Lisbon"));
        info.extend(element(&[0x44, 0x61], &[0, 0, 0, 0, 0, 0, 0, 7]));

        let mut track = element(&[0xd7], &[1]); // TrackNumber
        track.extend(element(&[0x53, 0x6e], b"C:/Users/alice/clip.mkv"));
        let tracks = element(&[0x16, 0x54, 0xae, 0x6b], &element(&[0xae], &track));

        let tags = element(
            &[0x12, 0x54, 0xc3, 0x67],
            b"<SimpleTag>ARTIST=Alice Zhang</SimpleTag>",
        );
        let attachments = element(&[0x19, 0x41, 0xa4, 0x69], b"cover.jpg alice@example.test");

        let mut seek = element(&[0x53, 0xab], &0x1254_c367u32.to_be_bytes());
        seek.extend(element(&[0x53, 0xac], &[0x10]));
        let seek_head = element(&[0x11, 0x4d, 0x9b, 0x74], &element(&[0x4d, 0xbb], &seek));

        let cluster = element(&[0x1f, 0x43, 0xb6, 0x75], b"FRAME-BYTES-ARTIST=Bob");

        let mut segment = seek_head;
        segment.extend(element(&[0x15, 0x49, 0xa9, 0x66], &info));
        segment.extend(tracks);
        segment.extend(cluster);
        segment.extend(tags);
        segment.extend(attachments);

        let mut file = element(&[0x1a, 0x45, 0xdf, 0xa3], b"\x42\x82\x88matroska");
        file.extend(element(&[0x18, 0x53, 0x80, 0x67], &segment));
        file
    }

    #[test]
    fn voids_tags_and_attachments_without_moving_a_byte() {
        let source = sample();
        assert!(is_matroska(&source));
        assert_eq!(inspect(&source).unwrap()[0].count, 8);

        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert_eq!(removed[0].count, 8);
        assert!(!contains(&cleaned, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"alice@example.test"));
        assert!(!contains(&cleaned, b"mkvmerge"));
        assert!(!contains(&cleaned, b"Holiday in Lisbon"));
        assert!(!contains(&cleaned, b"clip.mkv"));
        // Frames are never walked, so a payload that happens to spell a tag
        // survives exactly as recorded.
        assert!(contains(&cleaned, b"FRAME-BYTES-ARTIST=Bob"));
        assert!(inspect(&cleaned).unwrap().is_empty());
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn the_replacement_is_a_well_formed_void_element() {
        let (cleaned, _) = clean(&sample()).unwrap();
        let top = elements(&cleaned, 0..cleaned.len()).unwrap();
        let segment = top.iter().find(|item| item.id == SEGMENT).unwrap();
        let children = elements(&cleaned, segment.body()).unwrap();
        // Every element still parses, and the two tag blocks now read as Void.
        assert_eq!(
            children
                .iter()
                .filter(|item| item.id == u64::from(VOID))
                .count(),
            2
        );
        assert!(children.iter().all(|item| item.range.end <= cleaned.len()));
        assert!(!children.iter().any(|item| item.id == TAGS));
    }

    #[test]
    fn mandatory_strings_survive_as_empty_rather_than_disappearing() {
        let (cleaned, _) = clean(&sample()).unwrap();
        let top = elements(&cleaned, 0..cleaned.len()).unwrap();
        let segment = top.iter().find(|item| item.id == SEGMENT).unwrap();
        let info = elements(&cleaned, segment.body())
            .unwrap()
            .into_iter()
            .find(|item| item.id == INFO)
            .unwrap();
        let children = elements(&cleaned, info.body()).unwrap();
        for id in [0x4d80u64, 0x5741] {
            let element = children.iter().find(|item| item.id == id).unwrap();
            assert!(cleaned[element.body()].iter().all(|byte| *byte == 0));
        }
        // TimestampScale is structure, not metadata, and is left alone.
        let scale = children.iter().find(|item| item.id == 0x2ad7b1).unwrap();
        assert_eq!(&cleaned[scale.body()], &[0x0f, 0x42, 0x40]);
    }

    #[test]
    fn variable_width_integers_round_trip() {
        for width in 1..=8usize {
            let value = (1u64 << (7 * width)) - 2;
            let encoded = write_vint(value, width).unwrap();
            assert_eq!(encoded.len(), width);
            let (decoded, decoded_width, unknown) = read_vint(&encoded, 0, false).unwrap();
            assert_eq!((decoded, decoded_width), (value, width));
            assert!(!unknown);
        }
        // All ones is reserved for "length not yet known".
        assert!(write_vint(0x7f, 1).is_none());
        assert!(read_vint(&[0xff], 0, false).unwrap().2);
        assert!(read_vint(&[0x00], 0, false).is_none());
    }

    #[test]
    fn rejects_containers_that_are_not_matroska() {
        assert!(inspect(b"RIFF\0\0\0\0AVI ").is_err());
        assert!(!is_matroska(b"\x1aE\xdf"));
        assert!(inspect(&element(&[0x1a, 0x45, 0xdf, 0xa3], b"\x42\x82\x88matroska")).is_err());
    }

    #[test]
    fn rejects_elements_nested_beyond_the_audit_limit() {
        let mut nested = element(&[0x12, 0x54, 0xc3, 0x67], b"private");
        for _ in 0..10 {
            nested = element(&[0xae], &nested);
        }
        let mut file = element(&[0x1a, 0x45, 0xdf, 0xa3], b"\x42\x82\x88matroska");
        file.extend(element(&[0x18, 0x53, 0x80, 0x67], &nested));
        assert!(inspect(&file).is_err());
    }
}
