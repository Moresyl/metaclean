//! A TIFF file is a linked list of image file directories. Every entry is
//! twelve bytes wide, and any value longer than four bytes is parked elsewhere
//! in the file behind an absolute offset. Raw negatives from every major camera
//! vendor are that same container wearing a private magic word, so one careful
//! walker covers TIFF, DNG, CR2, NEF, ARW, ORF, RW2, PEF, SRW and their
//! siblings.
//!
//! Cleaning never moves a byte. Entries are deleted by compacting the directory
//! in place and rewriting the next-directory pointer behind them, so a
//! directory only ever shrinks. Every strip, tile and preview offset in the
//! file therefore stays valid, which is the whole reason it is safe to strip a
//! raw negative rather than refuse it.

use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const ENTRY: usize = 12;
const MAX_DIRECTORIES: usize = 64;
const MAX_DEPTH: u8 = 4;

const EXIF_POINTER: u16 = 0x8769;
const GPS_POINTER: u16 = 0x8825;
const INTEROP_POINTER: u16 = 0xa005;
const SUB_IFDS: u16 = 0x014a;
const ICC_PROFILE: u16 = 0x8773;
const ORIENTATION: u16 = 0x0112;

/// Tags that name a person, a place, or an editing history.
const IDENTITY_TAGS: &[u16] = &[
    0x010d, // DocumentName
    0x010e, // ImageDescription
    0x0131, // Software
    0x013b, // Artist
    0x8298, // Copyright
    0x9286, // UserComment
    0x9c9b, // XPTitle
    0x9c9c, // XPComment
    0x9c9d, // XPAuthor
    0x9c9e, // XPKeywords
    0x9c9f, // XPSubject
    0xa430, // CameraOwnerName
    0xc68b, // OriginalRawFileName
    0xc68c, // OriginalRawFileData
    0xc71a, // PreviewApplicationName
    0xc71b, // PreviewApplicationVersion
    0xc71c, // PreviewSettingsName
    0xc71d, // PreviewSettingsDigest
];

/// Tags that pin the file to a moment.
const TIME_TAGS: &[u16] = &[
    0x0132, // DateTime
    0x9003, // DateTimeOriginal
    0x9004, // DateTimeDigitized
    0x9010, // OffsetTime
    0x9011, // OffsetTimeOriginal
    0x9012, // OffsetTimeDigitized
    0x9290, // SubSecTime
    0x9291, // SubSecTimeOriginal
    0x9292, // SubSecTimeDigitized
    0xc763, // TimeCodes
];

/// Tags that identify the individual device rather than the model. A raw
/// decoder needs to know it is looking at a Canon sensor; it never needs the
/// serial number engraved on that particular body.
const IDENTITY_DEVICE_TAGS: &[u16] = &[
    0x927c, // MakerNote — carries serial, owner and often GPS for most vendors
    0xa420, // ImageUniqueID
    0xa431, // BodySerialNumber
    0xa435, // LensSerialNumber
    0xc62f, // CameraSerialNumber
    0xc634, // DNGPrivateData — the original MakerNote, smuggled into DNG
];

/// Tags naming the camera model. Raw decoders dispatch on these, so they only
/// go when the file is an ordinary TIFF that any decoder can render blind.
const MODEL_TAGS: &[u16] = &[
    0x010f, // Make
    0x0110, // Model
    0xa433, // LensMake
    0xa434, // LensModel
];

/// Whole metadata standards riding along inside a TIFF tag.
const SIDECAR_TAGS: &[u16] = &[
    0x02bc, // XMP
    0x83bb, // IPTC / NAA
    0x8649, // Photoshop image resources
    0x935c, // ImageSourceData — the full Photoshop layer stack
    0xc4a5, // PrintIM
    0xc6d2, // PanasonicTitle
    0xc6d3, // PanasonicTitle2
];

/// Offset/length tag pairs that address actual picture bytes. Anything they
/// point at is off limits to the scrubber.
const PAYLOAD_TAGS: &[(u16, u16)] = &[
    (0x0111, 0x0117), // StripOffsets / StripByteCounts
    (0x0144, 0x0145), // TileOffsets / TileByteCounts
    (0x0201, 0x0202), // JPEGInterchangeFormat / JPEGInterchangeFormatLength
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trace {
    Location,
    Time,
    Identity,
    Device,
    Sidecar,
    Profile,
    Silent,
}

#[derive(Debug, Clone, Copy)]
struct Policy {
    raw: bool,
    keep_orientation: bool,
    keep_profile: bool,
}

impl Policy {
    fn inspection(raw: bool) -> Self {
        Self {
            raw,
            keep_orientation: true,
            keep_profile: false,
        }
    }
}

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

struct Reader<'a> {
    data: &'a [u8],
    little_endian: bool,
}

impl<'a> Reader<'a> {
    fn u16(&self, at: usize) -> Result<u16> {
        let bytes: [u8; 2] = self
            .data
            .get(at..at + 2)
            .ok_or_else(|| invalid("TIFF 目录越界"))?
            .try_into()
            .unwrap();
        Ok(if self.little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    }

    fn u32(&self, at: usize) -> Result<u32> {
        let bytes: [u8; 4] = self
            .data
            .get(at..at + 4)
            .ok_or_else(|| invalid("TIFF 目录越界"))?
            .try_into()
            .unwrap();
        Ok(if self.little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }
}

fn type_size(kind: u16) -> usize {
    match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 => 8,
        _ => 0,
    }
}

/// The header only tells us where the first directory lives and which way round
/// the integers are. The magic word is 42 for TIFF and DNG, and a vendor
/// constant for Olympus and Panasonic; every one of them keeps the offset in
/// bytes four through eight.
fn header(data: &[u8]) -> Result<(bool, u32)> {
    let little_endian = match data.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Err(invalid("不是有效 TIFF")),
    };
    let reader = Reader {
        data,
        little_endian,
    };
    let magic = reader.u16(2)?;
    if magic == 0x2b {
        return Err(CleanError::Unsupported("暂不支持 BigTIFF".into()));
    }
    if !matches!(magic, 42 | 0x4f52 | 0x5352 | 0x0055) {
        return Err(invalid("不是有效 TIFF"));
    }
    Ok((little_endian, reader.u32(4)?))
}

pub fn is_tiff(data: &[u8]) -> bool {
    header(data).is_ok()
}

pub fn is_raf(data: &[u8]) -> bool {
    data.len() > 92 && data.starts_with(b"FUJIFILMCCD-RAW ")
}

fn classify(tag: u16, policy: &Policy) -> Option<Trace> {
    if tag == GPS_POINTER {
        return Some(Trace::Location);
    }
    if TIME_TAGS.contains(&tag) {
        return Some(Trace::Time);
    }
    if IDENTITY_TAGS.contains(&tag) {
        return Some(Trace::Identity);
    }
    if IDENTITY_DEVICE_TAGS.contains(&tag) {
        return Some(Trace::Device);
    }
    if MODEL_TAGS.contains(&tag) {
        return (!policy.raw).then_some(Trace::Device);
    }
    if SIDECAR_TAGS.contains(&tag) {
        return Some(Trace::Sidecar);
    }
    if tag == ICC_PROFILE {
        return (!policy.keep_profile).then_some(Trace::Profile);
    }
    if tag == ORIENTATION {
        return (!policy.keep_orientation).then_some(Trace::Silent);
    }
    None
}

#[derive(Debug)]
struct DirectoryEdit {
    at: usize,
    original: usize,
    kept: Vec<[u8; ENTRY]>,
    next: u32,
}

#[derive(Debug, Default)]
struct Survey {
    edits: Vec<DirectoryEdit>,
    scrub: Vec<Range<usize>>,
    protected: Vec<Range<usize>>,
    traces: Vec<Trace>,
}

impl Survey {
    fn count(&self, trace: Trace) -> usize {
        self.traces.iter().filter(|value| **value == trace).count()
    }

    fn removals(&self) -> usize {
        self.traces
            .iter()
            .filter(|trace| **trace != Trace::Silent)
            .count()
    }
}

struct Entry {
    tag: u16,
    bytes: [u8; ENTRY],
    external: Option<Range<usize>>,
    inline: u32,
    items: usize,
}

fn read_entries(reader: &Reader, at: usize, count: usize) -> Result<Vec<Entry>> {
    (0..count)
        .map(|index| {
            let base = at + 2 + index * ENTRY;
            let tag = reader.u16(base)?;
            let kind = reader.u16(base + 2)?;
            let items = reader.u32(base + 4)? as usize;
            let inline = reader.u32(base + 8)?;
            let width = type_size(kind).saturating_mul(items);
            let external = (width > 4)
                .then(|| {
                    let start = inline as usize;
                    start
                        .checked_add(width)
                        .filter(|end| *end <= reader.data.len())
                        .map(|end| start..end)
                })
                .flatten();
            Ok(Entry {
                tag,
                bytes: reader.data[base..base + ENTRY].try_into().unwrap(),
                external,
                inline,
                items,
            })
        })
        .collect()
}

/// Everything a directory addresses that must survive: the picture itself, and
/// any preview JPEG hanging off a thumbnail directory.
fn collect_payload(reader: &Reader, entries: &[Entry], protected: &mut Vec<Range<usize>>) {
    for (offsets, lengths) in PAYLOAD_TAGS {
        let Some(offset_entry) = entries.iter().find(|entry| entry.tag == *offsets) else {
            continue;
        };
        let Some(length_entry) = entries.iter().find(|entry| entry.tag == *lengths) else {
            continue;
        };
        let read = |entry: &Entry, index: usize| -> Option<usize> {
            if entry.items <= 1 {
                return Some(entry.inline as usize);
            }
            let base = entry.external.as_ref()?.start;
            let width = entry.external.as_ref()?.len() / entry.items;
            match width {
                2 => reader.u16(base + index * 2).ok().map(usize::from),
                4 => reader
                    .u32(base + index * 4)
                    .ok()
                    .map(|value| value as usize),
                _ => None,
            }
        };
        for index in 0..offset_entry.items.min(length_entry.items) {
            let (Some(start), Some(length)) =
                (read(offset_entry, index), read(length_entry, index))
            else {
                continue;
            };
            if let Some(range) = start
                .checked_add(length)
                .filter(|end| *end <= reader.data.len())
                .map(|end| start..end)
            {
                protected.push(range);
            }
        }
    }
}

/// Walk a directory and everything it points at, recording what must go.
fn walk(
    reader: &Reader,
    at: usize,
    depth: u8,
    policy: &Policy,
    survey: &mut Survey,
    seen: &mut Vec<usize>,
) -> Result<u32> {
    if depth > MAX_DEPTH || seen.contains(&at) {
        return Ok(0);
    }
    if seen.len() >= MAX_DIRECTORIES {
        return Err(invalid("TIFF 目录数量超过安全上限"));
    }
    seen.push(at);

    let count = usize::from(reader.u16(at)?);
    let end = at
        .checked_add(2 + count * ENTRY + 4)
        .filter(|value| *value <= reader.data.len())
        .ok_or_else(|| invalid("TIFF 目录越界"))?;
    survey.protected.push(at..end);

    let entries = read_entries(reader, at, count)?;
    collect_payload(reader, &entries, &mut survey.protected);

    let mut kept = Vec::with_capacity(count);
    let mut children = Vec::new();
    for entry in &entries {
        match classify(entry.tag, policy) {
            Some(trace) => {
                survey.traces.push(trace);
                if trace == Trace::Location {
                    collect_directory_span(reader, entry.inline as usize, &mut survey.scrub);
                }
                if let Some(range) = entry.external.clone() {
                    survey.scrub.push(range);
                }
            }
            None => {
                kept.push(entry.bytes);
                if let Some(range) = entry.external.clone() {
                    survey.protected.push(range);
                }
                match entry.tag {
                    EXIF_POINTER | INTEROP_POINTER => children.push(entry.inline as usize),
                    SUB_IFDS => children.extend(sub_directories(reader, entry)),
                    _ => {}
                }
            }
        }
    }

    let next = reader.u32(at + 2 + count * ENTRY)?;
    survey.edits.push(DirectoryEdit {
        at,
        original: count,
        kept,
        next,
    });

    for child in children {
        walk(reader, child, depth + 1, policy, survey, seen)?;
    }
    Ok(next)
}

fn sub_directories(reader: &Reader, entry: &Entry) -> Vec<usize> {
    if entry.items <= 1 {
        return vec![entry.inline as usize];
    }
    let Some(range) = entry.external.clone() else {
        return Vec::new();
    };
    (0..entry.items)
        .filter_map(|index| reader.u32(range.start + index * 4).ok())
        .map(|value| value as usize)
        .collect()
}

/// A GPS directory is orphaned the moment its pointer entry goes, but the
/// coordinates are still sitting there in plain sight. Wipe the directory and
/// every value it addresses.
fn collect_directory_span(reader: &Reader, at: usize, scrub: &mut Vec<Range<usize>>) {
    let Ok(count) = reader.u16(at) else {
        return;
    };
    let count = usize::from(count);
    let Some(end) = at
        .checked_add(2 + count * ENTRY + 4)
        .filter(|value| *value <= reader.data.len())
    else {
        return;
    };
    if let Ok(entries) = read_entries(reader, at, count) {
        scrub.extend(entries.into_iter().filter_map(|entry| entry.external));
    }
    scrub.push(at..end);
}

fn survey(data: &[u8], policy: Policy) -> Result<Survey> {
    let (little_endian, first) = header(data)?;
    let reader = Reader {
        data,
        little_endian,
    };
    let mut survey = Survey::default();
    let mut seen = Vec::new();
    let mut next = first;
    while next != 0 {
        next = walk(&reader, next as usize, 0, &policy, &mut survey, &mut seen)?;
    }
    if survey.edits.is_empty() {
        return Err(invalid("TIFF 缺少图像目录"));
    }
    Ok(survey)
}

fn overlaps(range: &Range<usize>, other: &Range<usize>) -> bool {
    range.start < other.end && other.start < range.end
}

fn findings(survey: &Survey) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut push = |category: &str, label: &str, count: usize, severity: FindingSeverity| {
        if count > 0 {
            findings.push(Finding {
                category: category.into(),
                label: label.into(),
                count,
                severity,
            });
        }
    };
    push(
        "image_metadata",
        "EXIF / GPS 元数据",
        survey.count(Trace::Location) + survey.count(Trace::Time) + survey.count(Trace::Device),
        FindingSeverity::Privacy,
    );
    push(
        "image_metadata",
        "作者、说明与编辑历史",
        survey.count(Trace::Identity),
        FindingSeverity::Privacy,
    );
    push(
        "provenance",
        "XMP / IPTC 来源标记",
        survey.count(Trace::Sidecar),
        FindingSeverity::Provenance,
    );
    push(
        "color_profile",
        "ICC 色彩配置文件",
        survey.count(Trace::Profile),
        FindingSeverity::Informational,
    );
    findings
}

pub fn inspect_tiff(data: &[u8], raw: bool) -> Result<Vec<Finding>> {
    Ok(findings(&survey(data, Policy::inspection(raw))?))
}

pub fn clean_tiff_with_options(
    data: &[u8],
    raw: bool,
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    let survey = survey(
        data,
        Policy {
            raw,
            keep_orientation: preserve_orientation,
            keep_profile: preserve_color_profile,
        },
    )?;
    let little_endian = data[0] == b'I';
    let mut output = data.to_vec();

    for edit in &survey.edits {
        let kept = edit.kept.len();
        let mut cursor = edit.at + 2;
        let count = u16::try_from(kept).map_err(|_| invalid("TIFF 目录条目过多"))?;
        output[edit.at..edit.at + 2].copy_from_slice(&if little_endian {
            count.to_le_bytes()
        } else {
            count.to_be_bytes()
        });
        for entry in &edit.kept {
            output[cursor..cursor + ENTRY].copy_from_slice(entry);
            cursor += ENTRY;
        }
        output[cursor..cursor + 4].copy_from_slice(&if little_endian {
            edit.next.to_le_bytes()
        } else {
            edit.next.to_be_bytes()
        });
        cursor += 4;
        let tail = edit.at + 2 + edit.original * ENTRY + 4;
        output[cursor..tail].fill(0);
    }

    // Deleting the entry is what removes the metadata; zeroing the value it
    // pointed at is what stops a hex editor from finding it anyway. Skip any
    // range that a surviving structure also claims, so a malformed offset can
    // never eat the picture.
    for range in &survey.scrub {
        if survey.protected.iter().any(|guard| overlaps(range, guard)) {
            continue;
        }
        output[range.clone()].fill(0);
    }

    Ok((output, findings(&survey)))
}

pub fn verify_tiff_cleaned(
    data: &[u8],
    raw: bool,
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<()> {
    let survey = survey(
        data,
        Policy {
            raw,
            keep_orientation: preserve_orientation,
            keep_profile: preserve_color_profile,
        },
    )?;
    let residual = survey.removals();
    if residual > 0 {
        return Err(CleanError::Verification(format!(
            "TIFF 中仍发现 {residual} 项应移除的痕迹"
        )));
    }
    Ok(())
}

// --- Fujifilm RAF -----------------------------------------------------------
//
// A RAF is not a TIFF. It is a fixed header naming three byte ranges: an
// embedded JPEG preview, a sensor description, and the sensor data itself. The
// preview carries a complete EXIF block — GPS, serial number, timestamps, the
// lot — which is why cleaning it matters and why refusing the format leaves the
// worst of the leak in place.
//
// The preview is cleaned as an ordinary JPEG and written back at its original
// offset. Only the length field moves; every other offset in the header still
// addresses the byte it always did.

const RAF_JPEG_OFFSET: usize = 84;
const RAF_JPEG_LENGTH: usize = 88;

fn raf_preview(data: &[u8]) -> Result<Range<usize>> {
    if !is_raf(data) {
        return Err(invalid("不是有效 RAF"));
    }
    let start = u32::from_be_bytes(
        data[RAF_JPEG_OFFSET..RAF_JPEG_OFFSET + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let length = u32::from_be_bytes(
        data[RAF_JPEG_LENGTH..RAF_JPEG_LENGTH + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let end = start
        .checked_add(length)
        .filter(|value| *value <= data.len() && length > 4)
        .ok_or_else(|| invalid("RAF 预览图越界"))?;
    if !data[start..end].starts_with(&[0xff, 0xd8, 0xff]) {
        return Err(invalid("RAF 预览图不是 JPEG"));
    }
    Ok(start..end)
}

pub fn inspect_raf(data: &[u8]) -> Result<Vec<Finding>> {
    let preview = raf_preview(data)?;
    super::image::inspect_jpeg(&data[preview])
}

pub fn clean_raf_with_options(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<(Vec<u8>, Vec<Finding>)> {
    let preview = raf_preview(data)?;
    let (cleaned, removed) = super::image::clean_jpeg_with_options(
        &data[preview.clone()],
        preserve_orientation,
        preserve_color_profile,
    )?;
    if cleaned.len() > preview.len() {
        return Err(invalid("RAF 预览图清理后变大"));
    }
    let mut output = data.to_vec();
    let end = preview.start + cleaned.len();
    output[preview.start..end].copy_from_slice(&cleaned);
    output[end..preview.end].fill(0);
    let length = u32::try_from(cleaned.len()).map_err(|_| invalid("RAF 预览图过大"))?;
    output[RAF_JPEG_LENGTH..RAF_JPEG_LENGTH + 4].copy_from_slice(&length.to_be_bytes());
    Ok((output, removed))
}

pub fn verify_raf_cleaned(
    data: &[u8],
    preserve_orientation: bool,
    preserve_color_profile: bool,
) -> Result<()> {
    let preview = raf_preview(data)?;
    super::image::verify_jpeg_cleaned(&data[preview], preserve_orientation, preserve_color_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Builder {
        entries: Vec<(u16, u16, u32, Vec<u8>)>,
    }

    impl Builder {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        fn inline(mut self, tag: u16, kind: u16, items: u32, value: u32) -> Self {
            self.entries
                .push((tag, kind, items, value.to_le_bytes().to_vec()));
            self
        }

        fn ascii(mut self, tag: u16, text: &str) -> Self {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            self.entries.push((tag, 2, bytes.len() as u32, bytes));
            self
        }

        /// Header, then the directory, then every out-of-line value packed
        /// behind it — the layout a real encoder produces.
        fn build(self) -> Vec<u8> {
            let count = self.entries.len();
            let directory_at = 8usize;
            let values_at = directory_at + 2 + count * ENTRY + 4;
            let mut directory = (count as u16).to_le_bytes().to_vec();
            let mut values = Vec::new();
            for (tag, kind, items, payload) in &self.entries {
                directory.extend_from_slice(&tag.to_le_bytes());
                directory.extend_from_slice(&kind.to_le_bytes());
                directory.extend_from_slice(&items.to_le_bytes());
                let width = type_size(*kind) * (*items as usize);
                if width > 4 {
                    directory.extend_from_slice(&((values_at + values.len()) as u32).to_le_bytes());
                    values.extend_from_slice(payload);
                } else {
                    let mut cell = payload.clone();
                    cell.resize(4, 0);
                    directory.extend_from_slice(&cell);
                }
            }
            directory.extend_from_slice(&0u32.to_le_bytes());
            let mut file = b"II\x2a\x00".to_vec();
            file.extend_from_slice(&(directory_at as u32).to_le_bytes());
            file.extend(directory);
            file.extend(values);
            file
        }
    }

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    fn sample() -> Vec<u8> {
        Builder::new()
            .inline(0x0100, 3, 1, 64) // ImageWidth
            .inline(0x0101, 3, 1, 64) // ImageLength
            .inline(ORIENTATION, 3, 1, 6)
            .ascii(0x010f, "Canon")
            .ascii(0x0132, "2019:04:01 09:15:00")
            .ascii(0x013b, "Alice Zhang")
            .ascii(0x8298, "(c) Alice Zhang")
            .ascii(0x02bc, "<x:xmpmeta>alice@example.test</x:xmpmeta>")
            .inline(0x0111, 4, 1, 4) // StripOffsets -> the magic word itself
            .inline(0x0117, 4, 1, 4) // StripByteCounts
            .build()
    }

    #[test]
    fn strips_author_time_and_sidecar_metadata_without_moving_bytes() {
        let source = sample();
        let (cleaned, removed) = clean_tiff_with_options(&source, false, true, true).unwrap();
        assert_eq!(cleaned.len(), source.len());
        assert!(!removed.is_empty());
        assert!(contains(&source, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"Alice Zhang"));
        assert!(!contains(&cleaned, b"2019:04:01"));
        assert!(!contains(&cleaned, b"alice@example.test"));
        assert!(inspect_tiff(&cleaned, false).unwrap().is_empty());
        verify_tiff_cleaned(&cleaned, false, true, true).unwrap();
    }

    #[test]
    fn keeps_rendering_tags_that_a_decoder_needs() {
        let source = sample();
        let (cleaned, _) = clean_tiff_with_options(&source, true, true, true).unwrap();
        // A raw negative keeps the camera model so a decoder can dispatch, and
        // orientation so the picture is not silently rotated.
        assert!(contains(&cleaned, b"Canon"));
        let (_, first) = header(&cleaned).unwrap();
        let reader = Reader {
            data: &cleaned,
            little_endian: true,
        };
        let count = usize::from(reader.u16(first as usize).unwrap());
        let tags: Vec<u16> = (0..count)
            .map(|index| reader.u16(first as usize + 2 + index * ENTRY).unwrap())
            .collect();
        assert!(tags.contains(&ORIENTATION));
        assert!(tags.contains(&0x0100));
        assert!(tags.contains(&0x0111));
        assert!(!tags.contains(&0x013b));
        // An ordinary TIFF has no decoder to protect, so the model goes too.
        let (plain, _) = clean_tiff_with_options(&source, false, true, true).unwrap();
        assert!(!contains(&plain, b"Canon"));
    }

    #[test]
    fn drops_orientation_only_when_the_caller_asks() {
        let source = sample();
        let (kept, _) = clean_tiff_with_options(&source, false, true, true).unwrap();
        let (dropped, _) = clean_tiff_with_options(&source, false, false, true).unwrap();
        let reader = |data: &[u8]| -> Vec<u16> {
            let (_, first) = header(data).unwrap();
            let reader = Reader {
                data,
                little_endian: true,
            };
            let count = usize::from(reader.u16(first as usize).unwrap());
            (0..count)
                .map(|index| reader.u16(first as usize + 2 + index * ENTRY).unwrap())
                .collect()
        };
        assert!(reader(&kept).contains(&ORIENTATION));
        assert!(!reader(&dropped).contains(&ORIENTATION));
    }

    #[test]
    fn erases_the_gps_directory_the_pointer_left_behind() {
        // IFD0 points at a GPS directory holding a latitude reference.
        let mut file = b"II\x2a\x00".to_vec();
        file.extend_from_slice(&8u32.to_le_bytes());
        let gps_at = 8 + 2 + ENTRY + 4;
        let mut directory = 1u16.to_le_bytes().to_vec();
        directory.extend_from_slice(&GPS_POINTER.to_le_bytes());
        directory.extend_from_slice(&4u16.to_le_bytes());
        directory.extend_from_slice(&1u32.to_le_bytes());
        directory.extend_from_slice(&(gps_at as u32).to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        file.extend(directory);
        let values_at = gps_at + 2 + ENTRY + 4;
        let mut gps = 1u16.to_le_bytes().to_vec();
        gps.extend_from_slice(&0x0001u16.to_le_bytes());
        gps.extend_from_slice(&2u16.to_le_bytes());
        gps.extend_from_slice(&8u32.to_le_bytes());
        gps.extend_from_slice(&(values_at as u32).to_le_bytes());
        gps.extend_from_slice(&0u32.to_le_bytes());
        file.extend(gps);
        file.extend_from_slice(b"31.2304\0");

        assert!(contains(&file, b"31.2304"));
        let (cleaned, removed) = clean_tiff_with_options(&file, false, true, true).unwrap();
        assert_eq!(cleaned.len(), file.len());
        assert!(!contains(&cleaned, b"31.2304"));
        assert_eq!(removed.iter().map(|item| item.count).sum::<usize>(), 1);
        assert!(inspect_tiff(&cleaned, false).unwrap().is_empty());
    }

    #[test]
    fn never_scrubs_bytes_the_picture_still_needs() {
        // A malformed file whose XMP tag points straight at the strip data.
        let mut file = b"II\x2a\x00".to_vec();
        file.extend_from_slice(&8u32.to_le_bytes());
        let payload_at = 8 + 2 + 3 * ENTRY + 4;
        let mut directory = 3u16.to_le_bytes().to_vec();
        for (tag, kind, items, value) in [
            (0x0111u16, 4u16, 1u32, payload_at as u32),
            (0x0117u16, 4u16, 1u32, 16u32),
            (0x02bcu16, 1u16, 16u32, payload_at as u32),
        ] {
            directory.extend_from_slice(&tag.to_le_bytes());
            directory.extend_from_slice(&kind.to_le_bytes());
            directory.extend_from_slice(&items.to_le_bytes());
            directory.extend_from_slice(&value.to_le_bytes());
        }
        directory.extend_from_slice(&0u32.to_le_bytes());
        file.extend(directory);
        file.extend_from_slice(b"PICTURE-BYTES!!!");

        let (cleaned, _) = clean_tiff_with_options(&file, false, true, true).unwrap();
        assert!(contains(&cleaned, b"PICTURE-BYTES!!!"));
        assert!(inspect_tiff(&cleaned, false).unwrap().is_empty());
    }

    #[test]
    fn honours_the_colour_profile_choice() {
        let source = Builder::new()
            .inline(0x0100, 3, 1, 8)
            .ascii(ICC_PROFILE, "ICC-PROFILE-BODY")
            .build();
        let profile = |findings: &[Finding]| {
            findings
                .iter()
                .any(|finding| finding.category == "color_profile")
        };
        assert!(profile(&inspect_tiff(&source, false).unwrap()));
        let (kept, _) = clean_tiff_with_options(&source, false, true, true).unwrap();
        assert!(contains(&kept, b"ICC-PROFILE-BODY"));
        verify_tiff_cleaned(&kept, false, true, true).unwrap();
        let (stripped, _) = clean_tiff_with_options(&source, false, true, false).unwrap();
        assert!(!contains(&stripped, b"ICC-PROFILE-BODY"));
        verify_tiff_cleaned(&stripped, false, true, false).unwrap();
        assert!(verify_tiff_cleaned(&kept, false, true, false).is_err());
    }

    #[test]
    fn follows_exif_and_sub_directories() {
        // IFD0 -> ExifIFD holding DateTimeOriginal and a serial number.
        let mut file = b"II\x2a\x00".to_vec();
        file.extend_from_slice(&8u32.to_le_bytes());
        let exif_at = 8 + 2 + ENTRY + 4;
        let mut directory = 1u16.to_le_bytes().to_vec();
        directory.extend_from_slice(&EXIF_POINTER.to_le_bytes());
        directory.extend_from_slice(&4u16.to_le_bytes());
        directory.extend_from_slice(&1u32.to_le_bytes());
        directory.extend_from_slice(&(exif_at as u32).to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        file.extend(directory);
        let values_at = exif_at + 2 + 2 * ENTRY + 4;
        let mut exif = 2u16.to_le_bytes().to_vec();
        exif.extend_from_slice(&0x9003u16.to_le_bytes());
        exif.extend_from_slice(&2u16.to_le_bytes());
        exif.extend_from_slice(&20u32.to_le_bytes());
        exif.extend_from_slice(&(values_at as u32).to_le_bytes());
        exif.extend_from_slice(&0xa431u16.to_le_bytes());
        exif.extend_from_slice(&2u16.to_le_bytes());
        exif.extend_from_slice(&9u32.to_le_bytes());
        exif.extend_from_slice(&((values_at + 20) as u32).to_le_bytes());
        exif.extend_from_slice(&0u32.to_le_bytes());
        file.extend(exif);
        file.extend_from_slice(b"2021:07:04 18:00:00\0");
        file.extend_from_slice(b"SN-91827\0");

        assert_eq!(
            inspect_tiff(&file, true)
                .unwrap()
                .iter()
                .map(|finding| finding.count)
                .sum::<usize>(),
            2
        );
        let (cleaned, _) = clean_tiff_with_options(&file, true, true, true).unwrap();
        assert!(!contains(&cleaned, b"2021:07:04"));
        assert!(!contains(&cleaned, b"SN-91827"));
        assert!(inspect_tiff(&cleaned, true).unwrap().is_empty());
    }

    #[test]
    fn rejects_containers_it_cannot_reason_about() {
        assert!(inspect_tiff(b"II", false).is_err());
        assert!(inspect_tiff(b"\x89PNG\r\n\x1a\n", false).is_err());
        assert!(inspect_tiff(b"II\x2b\x00\x08\0\0\0", false).is_err());
        assert!(!is_tiff(b"BM\0\0\0\0"));
        assert!(is_tiff(&sample()));
    }

    #[test]
    fn cleans_the_fujifilm_preview_instead_of_refusing_the_file() {
        let mut preview = vec![0xff, 0xd8, 0xff];
        preview.extend_from_slice(&[0xe1]);
        let exif = b"Exif\0\0Alice-was-here".to_vec();
        preview.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        preview.extend_from_slice(&exif);
        preview.extend_from_slice(&[0xff, 0xda, 0x00, 0x02]);
        preview.extend_from_slice(b"SCAN");
        preview.extend_from_slice(&[0xff, 0xd9]);

        let mut file = b"FUJIFILMCCD-RAW ".to_vec();
        file.resize(RAF_JPEG_OFFSET, 0);
        file.extend_from_slice(&(96u32).to_be_bytes());
        file.extend_from_slice(&(preview.len() as u32).to_be_bytes());
        file.resize(96, 0);
        file.extend_from_slice(&preview);
        file.extend_from_slice(b"CFA-SENSOR-DATA");

        assert!(is_raf(&file));
        assert!(!inspect_raf(&file).unwrap().is_empty());
        let (cleaned, removed) = clean_raf_with_options(&file, true, true).unwrap();
        assert_eq!(cleaned.len(), file.len());
        assert!(!removed.is_empty());
        assert!(!contains(&cleaned, b"Alice-was-here"));
        assert!(cleaned.ends_with(b"CFA-SENSOR-DATA"));
        assert!(inspect_raf(&cleaned).unwrap().is_empty());
        verify_raf_cleaned(&cleaned, true, true).unwrap();
    }
}
