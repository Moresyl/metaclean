//! HEIC, HEIF, AVIF and Canon's CR3 are all ISO base media files, but they are
//! not videos, and the rules that make `video.rs` safe would gut them. In a
//! video the top-level `meta` box is a place to hang a title; in a HEIF it holds
//! `iinf`, `iloc` and `iprp` — the item table that says where the picture is.
//! Deleting it deletes the image.
//!
//! So this walker works at item granularity. It reads the item table, finds the
//! items whose payload is an EXIF block, an XMP packet or a C2PA manifest,
//! zeroes those bytes where they lie in `mdat`, and sets their extent length to
//! zero so every reader now sees an empty item. Nothing moves, every other
//! item's offset survives, and the picture is untouched.

use std::{collections::HashSet, ops::Range};

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_DEPTH: usize = 24;
const ADOBE_XMP_UUID: [u8; 16] = [
    0xbe, 0x7a, 0xcf, 0xcb, 0x97, 0xa9, 0x42, 0xe8, 0x9c, 0x71, 0x99, 0x94, 0x91, 0xe3, 0xaf, 0xac,
];
const C2PA_UUID: [u8; 16] = [
    0xd8, 0xfe, 0xc3, 0xd6, 0x1b, 0x0e, 0x48, 0x3c, 0x92, 0x97, 0x58, 0x28, 0x87, 0x7e, 0xc4, 0x81,
];
/// Canon parks four bare TIFF directories inside this private box: IFD0, the
/// Exif IFD, the MakerNote and GPS.
const CANON_UUID: [u8; 16] = [
    0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

#[derive(Debug, Clone)]
struct Node {
    kind: [u8; 4],
    range: Range<usize>,
    box_header: usize,
    header: usize,
}

impl Node {
    fn payload(&self) -> Range<usize> {
        self.range.start + self.header..self.range.end
    }
}

fn parse_box(data: &[u8], offset: usize, parent_end: usize) -> Result<Node> {
    let header_end = offset
        .checked_add(8)
        .filter(|end| *end <= parent_end && *end <= data.len())
        .ok_or_else(|| invalid("HEIF 盒头不完整"))?;
    let short = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
    let kind: [u8; 4] = data[offset + 4..header_end].try_into().unwrap();
    let (size, header) = match short {
        0 => (parent_end - offset, 8),
        1 => {
            let extended_end = offset
                .checked_add(16)
                .filter(|end| *end <= parent_end && *end <= data.len())
                .ok_or_else(|| invalid("HEIF 扩展盒长度缺失"))?;
            let size = usize::try_from(u64::from_be_bytes(
                data[header_end..extended_end].try_into().unwrap(),
            ))
            .map_err(|_| invalid("HEIF 盒长度超出平台范围"))?;
            (size, 16)
        }
        value => (value as usize, 8),
    };
    let box_header = header;
    let header = if kind == *b"uuid" {
        box_header + 16
    } else {
        box_header
    };
    let end = offset
        .checked_add(size)
        .filter(|end| size >= header && *end <= parent_end)
        .ok_or_else(|| invalid("HEIF 盒长度越界"))?;
    Ok(Node {
        kind,
        range: offset..end,
        box_header,
        header,
    })
}

fn siblings(data: &[u8], span: Range<usize>) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut offset = span.start;
    while offset < span.end {
        let node = parse_box(data, offset, span.end)?;
        offset = node.range.end;
        nodes.push(node);
    }
    Ok(nodes)
}

fn user_type(data: &[u8], node: &Node) -> Option<[u8; 16]> {
    (node.kind == *b"uuid")
        .then(|| data.get(node.range.start + node.header - 16..node.range.start + node.header))
        .flatten()
        .and_then(|bytes| bytes.try_into().ok())
}

fn supported_brand(brand: &[u8]) -> bool {
    matches!(
        brand,
        b"heic"
            | b"heix"
            | b"heim"
            | b"heis"
            | b"hevc"
            | b"hevx"
            | b"hevm"
            | b"hevs"
            | b"mif1"
            | b"mif2"
            | b"msf1"
            | b"miaf"
            | b"avif"
            | b"avis"
            | b"crx "
    )
}

pub fn is_heif(data: &[u8]) -> bool {
    let Ok(node) = parse_box(data, 0, data.len()) else {
        return false;
    };
    if node.kind != *b"ftyp" || node.range.len() < node.header + 8 {
        return false;
    }
    let payload = &data[node.payload()];
    if !(payload.len() - 8).is_multiple_of(4) {
        return false;
    }
    supported_brand(&payload[..4])
        || payload
            .get(8..)
            .is_some_and(|brands| brands.chunks_exact(4).any(supported_brand))
}

pub fn is_canon_raw(data: &[u8]) -> bool {
    let Ok(node) = parse_box(data, 0, data.len()) else {
        return false;
    };
    if node.kind != *b"ftyp" || node.range.len() < node.header + 8 {
        return false;
    }
    let payload = &data[node.payload()];
    (payload.len() - 8).is_multiple_of(4) && payload[..4] == *b"crx "
}

// --- item table -------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Exif,
    Xmp,
    Provenance,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    end: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], span: Range<usize>) -> Self {
        Self {
            data,
            at: span.start,
            end: span.end,
        }
    }

    fn take(&mut self, width: usize) -> Result<&'a [u8]> {
        let next = self
            .at
            .checked_add(width)
            .filter(|value| *value <= self.end)
            .ok_or_else(|| invalid("HEIF 项目表越界"))?;
        let slice = &self.data[self.at..next];
        self.at = next;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Widths in `iloc` are declared as 0, 4 or 8 bytes; zero means the field is
    /// absent and reads as zero.
    fn sized(&mut self, width: usize) -> Result<u64> {
        match width {
            0 => Ok(0),
            4 => Ok(u64::from(self.u32()?)),
            8 => Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap())),
            _ => Err(invalid("HEIF 项目表字段宽度无效")),
        }
    }

    fn string(&mut self) -> Result<&'a [u8]> {
        let start = self.at;
        while self.at < self.end {
            if self.data[self.at] == 0 {
                let text = &self.data[start..self.at];
                self.at += 1;
                return Ok(text);
            }
            self.at += 1;
        }
        Err(invalid("HEIF 项目名称缺少结束符"))
    }
}

/// `iinf` lists what each item is. Only version 2 and later name the type with a
/// four-character code; the older layout predates HEIF and never carries EXIF.
fn item_types(data: &[u8], iinf: &Node) -> Result<Vec<(u32, Item)>> {
    let mut reader = Reader::new(data, iinf.payload());
    let version = reader.u8()?;
    if version > 1 {
        return Err(invalid("HEIF iinf 版本无效"));
    }
    reader.take(3)?;
    let count = if version == 0 {
        u32::from(reader.u16()?)
    } else {
        reader.u32()?
    };
    let mut items = Vec::new();
    let mut ids = HashSet::new();
    let mut offset = reader.at;
    for _ in 0..count {
        let entry = parse_box(data, offset, iinf.range.end)?;
        offset = entry.range.end;
        if entry.kind != *b"infe" {
            continue;
        }
        let mut reader = Reader::new(data, entry.payload());
        let version = reader.u8()?;
        reader.take(3)?;
        if version < 2 {
            continue;
        }
        if version > 3 {
            return Err(invalid("HEIF infe 版本无效"));
        }
        let id = if version == 2 {
            u32::from(reader.u16()?)
        } else {
            reader.u32()?
        };
        if !ids.insert(id) {
            return Err(invalid("HEIF 项目信息包含重复 ID"));
        }
        reader.u16()?;
        let kind: [u8; 4] = reader.take(4)?.try_into().unwrap();
        reader.string()?;
        let classified = match &kind {
            b"Exif" => Some(Item::Exif),
            b"jumb" | b"c2pa" => Some(Item::Provenance),
            b"mime" => {
                let content = reader.string()?;
                let content = String::from_utf8_lossy(content).to_ascii_lowercase();
                if content.contains("c2pa") {
                    Some(Item::Provenance)
                } else if content.contains("rdf+xml") || content.contains("xmp") {
                    Some(Item::Xmp)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(item) = classified {
            items.push((id, item));
        }
    }
    Ok(items)
}

#[derive(Debug)]
struct Extent {
    /// Where the payload actually lives, once base and construction are applied.
    data: Range<usize>,
    /// The length field inside `iloc`, so cleaning can zero it in place.
    length_field: Range<usize>,
}

#[derive(Debug)]
struct Located {
    id: u32,
    extents: Vec<Extent>,
    locatable: bool,
}

fn item_locations(
    data: &[u8],
    iloc: &Node,
    idat: Option<&Node>,
    media: &[Range<usize>],
) -> Result<Vec<Located>> {
    let mut reader = Reader::new(data, iloc.payload());
    let version = reader.u8()?;
    if version > 2 {
        return Err(invalid("HEIF iloc 版本无效"));
    }
    reader.take(3)?;
    let sizes = reader.u8()?;
    let (offset_size, length_size) = (usize::from(sizes >> 4), usize::from(sizes & 0x0f));
    let sizes = reader.u8()?;
    let (base_size, index_size) = (usize::from(sizes >> 4), usize::from(sizes & 0x0f));
    let count = if version < 2 {
        u32::from(reader.u16()?)
    } else {
        reader.u32()?
    };
    let mut located = Vec::new();
    let mut ids = HashSet::new();
    for _ in 0..count {
        let id = if version < 2 {
            u32::from(reader.u16()?)
        } else {
            reader.u32()?
        };
        if !ids.insert(id) {
            return Err(invalid("HEIF 位置表包含重复 ID"));
        }
        let construction_field = if version == 0 { 0 } else { reader.u16()? };
        if construction_field & 0xfff0 != 0 {
            return Err(invalid("HEIF iloc 构造方式保留位非零"));
        }
        let construction = construction_field & 0x0f;
        let data_reference = reader.u16()?;
        let base = reader.sized(base_size)?;
        let extent_count = reader.u16()?;
        let mut extents = Vec::new();
        let inline_storage = (construction == 1)
            .then(|| idat.map(Node::payload))
            .flatten();
        let locatable = data_reference == 0
            && construction <= 1
            && (construction == 0 || inline_storage.is_some());
        for _ in 0..extent_count {
            if index_size > 0 && version > 0 {
                reader.sized(index_size)?;
            }
            let offset = reader.sized(offset_size)?;
            let length_at = reader.at;
            let length = reader.sized(length_size)?;
            let length_field = length_at..reader.at;
            if !locatable {
                continue;
            }
            let origin = match inline_storage.as_ref() {
                Some(storage) => storage.start as u64,
                None => 0,
            };
            let Some(start) = origin
                .checked_add(base)
                .and_then(|value| value.checked_add(offset))
                .and_then(|value| usize::try_from(value).ok())
            else {
                return Err(invalid("HEIF 项目偏移超出平台范围"));
            };
            let Some(end) = usize::try_from(length)
                .ok()
                .and_then(|width| start.checked_add(width))
                .filter(|end| *end <= data.len())
            else {
                return Err(invalid("HEIF 项目范围越界"));
            };
            let inside_storage = inline_storage.as_ref().map_or_else(
                || {
                    media
                        .iter()
                        .any(|span| start >= span.start && end <= span.end)
                },
                |span| start >= span.start && end <= span.end,
            );
            if length > 0 && !inside_storage {
                return Err(invalid("HEIF 项目范围不在 mdat / idat 载荷内"));
            }
            extents.push(Extent {
                data: start..end,
                length_field,
            });
        }
        located.push(Located {
            id,
            extents,
            locatable,
        });
    }
    Ok(located)
}

// --- survey -----------------------------------------------------------------

#[derive(Debug, Default)]
struct Plan {
    /// Boxes to rename to `free` and blank: stray XMP, C2PA and user data.
    retire: Vec<Node>,
    /// Item payloads to blank, with the `iloc` length fields that describe them.
    blank: Vec<Range<usize>>,
    lengths: Vec<Range<usize>>,
    /// Canon parks whole TIFF directories in a private box; those get the same
    /// tag-level treatment an ordinary raw negative gets.
    directories: Vec<Range<usize>>,
    exif: usize,
    provenance: usize,
}

fn retired(kind: &[u8; 4], user: Option<[u8; 16]>) -> bool {
    if matches!(
        kind,
        b"udta" | b"ilst" | b"XMP_" | b"cprt" | b"loci" | b"CMT3" | b"CMT4"
    ) || kind[0] == 0xa9
    {
        return true;
    }
    matches!(user, Some(value) if value == ADOBE_XMP_UUID || value == C2PA_UUID)
}

fn collect(data: &[u8], span: Range<usize>, depth: usize, plan: &mut Plan) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(invalid("HEIF 盒嵌套超过安全上限"));
    }
    for node in siblings(data, span)? {
        let user = user_type(data, &node);
        if retired(&node.kind, user) {
            plan.provenance += 1;
            plan.retire.push(node);
            continue;
        }
        if matches!(&node.kind, b"CMT1" | b"CMT2") {
            plan.directories.push(node.payload());
            continue;
        }
        if matches!(&node.kind, b"moov" | b"trak" | b"mdia" | b"minf" | b"udta")
            || user == Some(CANON_UUID)
        {
            collect(data, node.payload(), depth + 1, plan)?;
        }
    }
    Ok(())
}

fn plan(data: &[u8]) -> Result<Plan> {
    if !is_heif(data) {
        return Err(invalid("不是受支持的 HEIF / AVIF 容器"));
    }
    let top = siblings(data, 0..data.len())?;
    let media: Vec<_> = top
        .iter()
        .filter(|node| node.kind == *b"mdat")
        .map(Node::payload)
        .collect();
    let mut plan = Plan::default();
    let mut anchored = false;

    let metadata_boxes: Vec<_> = top.iter().filter(|node| node.kind == *b"meta").collect();
    if metadata_boxes.len() > 1 {
        return Err(invalid("HEIF 包含重复的顶层 meta 盒"));
    }
    if let Some(meta) = metadata_boxes.first() {
        // A top-level `meta` is a full box, so four bytes of version and flags
        // sit between the header and the first child.
        let inner = meta.payload().start + 4..meta.range.end;
        let children = siblings(data, inner)?;
        for kind in [b"iinf", b"iloc", b"idat"] {
            if children.iter().filter(|node| node.kind == *kind).count() > 1 {
                return Err(invalid("HEIF meta 盒包含重复的关键子盒"));
            }
        }
        let idat = children.iter().find(|node| node.kind == *b"idat");
        let types = match children.iter().find(|node| node.kind == *b"iinf") {
            Some(iinf) => item_types(data, iinf)?,
            None => Vec::new(),
        };
        if let Some(iloc) = children.iter().find(|node| node.kind == *b"iloc") {
            anchored = true;
            let located = item_locations(data, iloc, idat, &media)?;
            for (id, item) in &types {
                let Some(entry) = located.iter().find(|entry| entry.id == *id) else {
                    return Err(invalid("HEIF 元数据项目缺少位置条目"));
                };
                if !entry.locatable {
                    return Err(invalid("HEIF 元数据项目使用不支持的外部或派生存储"));
                }
                let bytes: usize = entry.extents.iter().map(|extent| extent.data.len()).sum();
                if bytes == 0 {
                    continue;
                }
                let overlaps_other_item = entry.extents.iter().any(|extent| {
                    located.iter().any(|other| {
                        other.id != *id
                            && other.extents.iter().any(|candidate| {
                                extent.data.start < candidate.data.end
                                    && candidate.data.start < extent.data.end
                            })
                    })
                });
                if overlaps_other_item {
                    return Err(invalid("HEIF 元数据范围与其他项目载荷重叠"));
                }
                match item {
                    Item::Exif => plan.exif += 1,
                    Item::Xmp | Item::Provenance => plan.provenance += 1,
                }
                for extent in &entry.extents {
                    plan.blank.push(extent.data.clone());
                    plan.lengths.push(extent.length_field.clone());
                }
            }
        } else if !types.is_empty() {
            return Err(invalid("HEIF 元数据项目缺少 iloc 位置表"));
        }
        for node in children {
            if retired(&node.kind, user_type(data, &node)) {
                plan.provenance += 1;
                plan.retire.push(node);
            }
        }
    }

    for node in &top {
        let user = user_type(data, node);
        if retired(&node.kind, user) {
            plan.provenance += 1;
            plan.retire.push(node.clone());
        } else if node.kind == *b"moov" {
            anchored = true;
            collect(data, node.payload(), 1, &mut plan)?;
        }
    }

    for span in &plan.directories {
        plan.exif += super::tiff::inspect_tiff(&data[span.clone()], true)
            .map(|findings| findings.iter().map(|finding| finding.count).sum::<usize>())
            .unwrap_or(0);
    }

    if !anchored {
        return Err(invalid("HEIF 缺少项目表或影片盒"));
    }
    Ok(plan)
}

fn findings(plan: &Plan) -> Vec<Finding> {
    let mut findings = Vec::new();
    if plan.exif > 0 {
        findings.push(Finding {
            category: "image_metadata".into(),
            label: "EXIF / GPS 元数据".into(),
            count: plan.exif,
            severity: FindingSeverity::Privacy,
        });
    }
    if plan.provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "XMP / C2PA 来源标记".into(),
            count: plan.provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    findings
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(&plan(data)?))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let plan = plan(data)?;
    let mut output = data.to_vec();
    for node in &plan.retire {
        output[node.range.start + 4..node.range.start + 8].copy_from_slice(b"free");
        output[node.range.start + node.box_header..node.range.end].fill(0);
    }
    for range in &plan.blank {
        output[range.clone()].fill(0);
    }
    // An item whose extent length is zero holds nothing, whatever the item table
    // still claims it is.
    for range in &plan.lengths {
        output[range.clone()].fill(0);
    }
    for span in &plan.directories {
        if let Ok((cleaned, _)) =
            super::tiff::clean_tiff_with_options(&data[span.clone()], true, true, true)
        {
            if cleaned.len() == span.len() {
                output[span.clone()].copy_from_slice(&cleaned);
            }
        }
    }
    Ok((output, findings(&plan)))
}

pub fn verify_cleaned(data: &[u8]) -> Result<()> {
    let residual: usize = inspect(data)?.iter().map(|finding| finding.count).sum();
    if residual > 0 {
        return Err(CleanError::Verification(format!(
            "HEIF 中仍发现 {residual} 项应移除的痕迹"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn wrap_extended(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = 1u32.to_be_bytes().to_vec();
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(&((payload.len() + 16) as u64).to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn infe(id: u16, kind: &[u8; 4], name: &str, content: Option<&str>) -> Vec<u8> {
        let mut payload = vec![2u8, 0, 0, 0];
        payload.extend_from_slice(&id.to_be_bytes());
        payload.extend_from_slice(&0u16.to_be_bytes());
        payload.extend_from_slice(kind);
        payload.extend_from_slice(name.as_bytes());
        payload.push(0);
        if let Some(content) = content {
            payload.extend_from_slice(content.as_bytes());
            payload.push(0);
        }
        wrap(b"infe", &payload)
    }

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    /// A minimal but honest HEIC: an image item, an EXIF item and an XMP item,
    /// all three located by `iloc` inside one `mdat`.
    fn sample() -> Vec<u8> {
        let ftyp = wrap(b"ftyp", b"heic\0\0\0\0mif1heic");

        let mut iinf_payload = vec![0u8, 0, 0, 0];
        iinf_payload.extend_from_slice(&3u16.to_be_bytes());
        iinf_payload.extend(infe(1, b"hvc1", "image", None));
        iinf_payload.extend(infe(2, b"Exif", "exif", None));
        iinf_payload.extend(infe(3, b"mime", "xmp", Some("application/rdf+xml")));
        let iinf = wrap(b"iinf", &iinf_payload);

        let payloads: [&[u8]; 3] = [
            b"PIXELS-PIXELS-PIXELS",
            b"\0\0\0\x06Exif\0\0GPS 31.2304,121.4737",
            b"<x:xmpmeta>alice@example.test</x:xmpmeta>",
        ];

        // Lay the file out once with placeholder offsets to learn where `mdat`
        // lands, then write the real ones.
        let build = |mdat_at: usize| -> Vec<u8> {
            let mut iloc_payload = vec![0u8, 0, 0, 0, 0x44, 0x00];
            iloc_payload.extend_from_slice(&3u16.to_be_bytes());
            let mut cursor = mdat_at + 8;
            for (index, payload) in payloads.iter().enumerate() {
                iloc_payload.extend_from_slice(&((index as u16) + 1).to_be_bytes());
                iloc_payload.extend_from_slice(&0u16.to_be_bytes());
                iloc_payload.extend_from_slice(&1u16.to_be_bytes());
                iloc_payload.extend_from_slice(&(cursor as u32).to_be_bytes());
                iloc_payload.extend_from_slice(&(payload.len() as u32).to_be_bytes());
                cursor += payload.len();
            }
            let iloc = wrap(b"iloc", &iloc_payload);
            let mut meta_payload = vec![0u8, 0, 0, 0];
            meta_payload.extend(wrap(b"hdlr", b"\0\0\0\0\0\0\0\0pict"));
            meta_payload.extend(iinf.clone());
            meta_payload.extend(iloc);
            let mut file = ftyp.clone();
            file.extend(wrap(b"meta", &meta_payload));
            file.extend(wrap(b"mdat", &payloads.concat()));
            file
        };
        let probe = build(0);
        let mdat_at = probe.len() - payloads.concat().len() - 8;
        build(mdat_at)
    }

    fn item_without_local_storage(construction: u16) -> Vec<u8> {
        let ftyp = wrap(b"ftyp", b"heic\0\0\0\0mif1heic");

        let mut iinf_payload = vec![0u8, 0, 0, 0];
        iinf_payload.extend_from_slice(&1u16.to_be_bytes());
        iinf_payload.extend(infe(1, b"Exif", "exif", None));

        let mut iloc_payload = vec![1u8, 0, 0, 0, 0x44, 0x00];
        iloc_payload.extend_from_slice(&1u16.to_be_bytes());
        iloc_payload.extend_from_slice(&1u16.to_be_bytes());
        iloc_payload.extend_from_slice(&construction.to_be_bytes());
        iloc_payload.extend_from_slice(&0u16.to_be_bytes());
        iloc_payload.extend_from_slice(&1u16.to_be_bytes());
        iloc_payload.extend_from_slice(&0u32.to_be_bytes());
        iloc_payload.extend_from_slice(&4u32.to_be_bytes());

        let mut meta_payload = vec![0u8, 0, 0, 0];
        meta_payload.extend(wrap(b"iinf", &iinf_payload));
        meta_payload.extend(wrap(b"iloc", &iloc_payload));
        let mut file = ftyp;
        file.extend(wrap(b"meta", &meta_payload));
        file
    }

    fn metadata_item_without_iloc() -> Vec<u8> {
        let mut iinf_payload = vec![0u8, 0, 0, 0];
        iinf_payload.extend_from_slice(&1u16.to_be_bytes());
        iinf_payload.extend(infe(1, b"Exif", "exif", None));
        let mut meta_payload = vec![0u8, 0, 0, 0];
        meta_payload.extend(wrap(b"iinf", &iinf_payload));
        let mut file = wrap(b"ftyp", b"heic\0\0\0\0mif1heic");
        file.extend(wrap(b"meta", &meta_payload));
        file
    }

    fn iloc_payload(data: &[u8]) -> Range<usize> {
        let top = siblings(data, 0..data.len()).unwrap();
        let meta = top.iter().find(|node| node.kind == *b"meta").unwrap();
        siblings(data, meta.payload().start + 4..meta.range.end)
            .unwrap()
            .into_iter()
            .find(|node| node.kind == *b"iloc")
            .unwrap()
            .payload()
    }

    #[test]
    fn blanks_exif_and_xmp_items_without_touching_the_picture() {
        let source = sample();
        assert!(is_heif(&source));
        let before = inspect(&source).unwrap();
        assert_eq!(before.iter().map(|finding| finding.count).sum::<usize>(), 2);

        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert_eq!(
            removed.iter().map(|finding| finding.count).sum::<usize>(),
            2
        );
        assert!(contains(&cleaned, b"PIXELS-PIXELS-PIXELS"));
        assert!(!contains(&cleaned, b"31.2304"));
        assert!(!contains(&cleaned, b"alice@example.test"));
        assert!(inspect(&cleaned).unwrap().is_empty());
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn retires_stray_provenance_boxes() {
        let mut source = sample();
        let mut uuid_payload = ADOBE_XMP_UUID.to_vec();
        uuid_payload.extend_from_slice(b"<x:xmpmeta>bob@example.test</x:xmpmeta>");
        source.extend(wrap(b"uuid", &uuid_payload));

        assert_eq!(
            inspect(&source)
                .unwrap()
                .iter()
                .map(|finding| finding.count)
                .sum::<usize>(),
            3
        );
        let (cleaned, _) = clean(&source).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert!(!contains(&cleaned, b"bob@example.test"));
        assert!(contains(&cleaned, b"free"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn preserves_extended_box_length_when_retiring_uuid_metadata() {
        let mut source = sample();
        let box_at = source.len();
        let mut uuid_payload = ADOBE_XMP_UUID.to_vec();
        uuid_payload.extend_from_slice(b"<xmp>extended-box@example.test</xmp>");
        source.extend(wrap_extended(b"uuid", &uuid_payload));

        let (cleaned, _) = clean(&source).unwrap();
        assert_eq!(&cleaned[box_at..box_at + 4], &1u32.to_be_bytes());
        assert_eq!(&cleaned[box_at + 4..box_at + 8], b"free");
        assert_eq!(
            &cleaned[box_at + 8..box_at + 16],
            &source[box_at + 8..box_at + 16]
        );
        assert!(cleaned[box_at + 16..].iter().all(|byte| *byte == 0));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn rejects_item_extents_outside_declared_local_storage() {
        assert!(inspect(&item_without_local_storage(0)).is_err());
        assert!(inspect(&item_without_local_storage(1)).is_err());
        assert!(inspect(&item_without_local_storage(2)).is_err());
        assert!(inspect(&metadata_item_without_iloc()).is_err());
    }

    #[test]
    fn rejects_duplicate_ids_and_metadata_overlapping_image_payloads() {
        const HEADER: usize = 8;
        const ITEM_WIDTH: usize = 14;
        const OFFSET_IN_ITEM: usize = 6;

        let mut duplicate = sample();
        let iloc = iloc_payload(&duplicate);
        let third_id = iloc.start + HEADER + 2 * ITEM_WIDTH;
        duplicate[third_id..third_id + 2].copy_from_slice(&2u16.to_be_bytes());
        assert!(inspect(&duplicate).is_err());

        let mut missing_location = sample();
        let iloc = iloc_payload(&missing_location);
        let exif_id = iloc.start + HEADER + ITEM_WIDTH;
        missing_location[exif_id..exif_id + 2].copy_from_slice(&99u16.to_be_bytes());
        assert!(inspect(&missing_location).is_err());

        let mut overlapping = sample();
        let iloc = iloc_payload(&overlapping);
        let image_offset = iloc.start + HEADER + OFFSET_IN_ITEM;
        let exif_offset = iloc.start + HEADER + ITEM_WIDTH + OFFSET_IN_ITEM;
        let value: [u8; 4] = overlapping[image_offset..image_offset + 4]
            .try_into()
            .unwrap();
        overlapping[exif_offset..exif_offset + 4].copy_from_slice(&value);
        assert!(inspect(&overlapping).is_err());
    }

    #[test]
    fn cleans_the_tiff_directories_canon_hides_in_a_private_box() {
        let ftyp = wrap(b"ftyp", b"crx \0\0\0\0crx isom");
        // CMT1 is a bare TIFF directory: model, capture time, photographer.
        let mut directory = 3u16.to_le_bytes().to_vec();
        let values_at = 8 + 2 + 3 * 12 + 4;
        let mut values = Vec::new();
        for (tag, text) in [
            (0x0110u16, "EOS R5"),
            (0x0132u16, "2024:02:11 07:30:00"),
            (0x013bu16, "Alice Zhang"),
        ] {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            directory.extend_from_slice(&tag.to_le_bytes());
            directory.extend_from_slice(&2u16.to_le_bytes());
            directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            directory.extend_from_slice(&((values_at + values.len()) as u32).to_le_bytes());
            values.extend(bytes);
        }
        directory.extend_from_slice(&0u32.to_le_bytes());
        let mut cmt1 = b"II\x2a\x00".to_vec();
        cmt1.extend_from_slice(&8u32.to_le_bytes());
        cmt1.extend(directory);
        cmt1.extend(values);

        let mut canon = CANON_UUID.to_vec();
        canon.extend(wrap(b"CMT1", &cmt1));
        canon.extend(wrap(b"CMT3", b"MakerNote serial SN-4471"));
        canon.extend(wrap(b"CMT4", b"GPS 22.5431,114.0579"));
        let moov = wrap(b"moov", &wrap(b"uuid", &canon));

        let mut file = ftyp;
        file.extend(moov);
        file.extend(wrap(b"mdat", b"RAW-SENSOR-DATA"));

        assert!(is_canon_raw(&file));
        assert!(!inspect(&file).unwrap().is_empty());
        let (cleaned, _) = clean(&file).unwrap();
        assert_eq!(cleaned.len(), file.len());
        assert!(!contains(&cleaned, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"2024:02:11"));
        assert!(!contains(&cleaned, b"SN-4471"));
        assert!(!contains(&cleaned, b"22.5431"));
        // The model survives, because a raw decoder cannot dispatch without it.
        assert!(contains(&cleaned, b"EOS R5"));
        assert!(cleaned.ends_with(b"RAW-SENSOR-DATA"));
        verify_cleaned(&cleaned).unwrap();
    }

    #[test]
    fn rejects_videos_and_malformed_containers() {
        assert!(!is_heif(&wrap(b"ftyp", b"isom\0\0\0\0isommp42")));
        assert!(!is_heif(&wrap(b"ftyp", b"heic\0\0\0\0x")));
        assert!(inspect(&wrap(b"ftyp", b"heic\0\0\0\0mif1heic")).is_err());
        assert!(inspect(b"not-a-box").is_err());

        let mut duplicate_meta = sample();
        let top = siblings(&duplicate_meta, 0..duplicate_meta.len()).unwrap();
        let meta = top.iter().find(|node| node.kind == *b"meta").unwrap();
        let bytes = duplicate_meta[meta.range.clone()].to_vec();
        duplicate_meta.extend(bytes);
        assert!(inspect(&duplicate_meta).is_err());
    }
}
