//! CIPA MPF indexes name complete JPEG images, including secondary views and
//! gain maps. Clean each indexed image and rebuild sizes and relative offsets.

use super::{jpeg_segments, JpegSegment};
use crate::{
    error::{CleanError, Result},
    models::Finding,
};
use std::ops::Range;

fn invalid() -> CleanError {
    CleanError::InvalidFormat("JPEG MPF 索引、属性或图像范围无效".into())
}

#[derive(Clone, Copy)]
struct Endian(bool);

impl Endian {
    fn u16(self, bytes: &[u8]) -> Result<u16> {
        let bytes = bytes.get(..2).ok_or_else(invalid)?.try_into().unwrap();
        Ok(if self.0 {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    }

    fn u32(self, bytes: &[u8]) -> Result<u32> {
        let bytes = bytes.get(..4).ok_or_else(invalid)?.try_into().unwrap();
        Ok(if self.0 {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }

    fn put_u16(self, bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&if self.0 {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        });
    }

    fn put_u32(self, bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&if self.0 {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        });
    }
}

#[derive(Clone)]
struct Field {
    tag: u16,
    kind: u16,
    count: u32,
    value: Vec<u8>,
}

struct Metadata {
    endian: Endian,
    directories: Vec<Vec<Field>>,
    private_count: usize,
}

#[derive(Clone, Copy)]
struct Entry {
    attributes: u32,
    size: u32,
    offset: u32,
    dependencies: [u16; 2],
}

impl Metadata {
    fn parse(payload: &[u8]) -> Result<Self> {
        let data = payload.strip_prefix(b"MPF\0").ok_or_else(invalid)?;
        let endian = match data.get(..4) {
            Some(b"II\x2a\0") => Endian(true),
            Some(b"MM\0\x2a") => Endian(false),
            _ => return Err(invalid()),
        };
        let mut offset = endian.u32(data.get(4..).ok_or_else(invalid)?)? as usize;
        let mut directories = Vec::new();
        let mut used: Vec<Range<usize>> = std::iter::once(0..8).collect();
        let mut directory_ranges = used.clone();
        let mut value_ranges = Vec::new();
        let mut private_count = 0;
        while offset != 0 {
            if offset < 8 || directories.len() == 2 {
                return Err(invalid());
            }
            let count = usize::from(endian.u16(data.get(offset..).ok_or_else(invalid)?)?);
            let end = offset.checked_add(6 + count * 12).ok_or_else(invalid)?;
            if end > data.len()
                || directory_ranges
                    .iter()
                    .any(|range| offset < range.end && end > range.start)
            {
                return Err(invalid());
            }
            directory_ranges.push(offset..end);
            used.push(offset..end);
            let mut fields = Vec::new();
            for index in 0..count {
                let entry = &data[offset + 2 + index * 12..offset + 14 + index * 12];
                let tag = endian.u16(entry)?;
                if fields.iter().any(|field: &Field| field.tag == tag) {
                    return Err(invalid());
                }
                let kind = endian.u16(&entry[2..])?;
                let count = endian.u32(&entry[4..])?;
                let width = match kind {
                    1 | 2 | 6 | 7 => 1usize,
                    3 | 8 => 2,
                    4 | 9 | 11 | 13 => 4,
                    5 | 10 | 12 => 8,
                    _ => return Err(invalid()),
                };
                let length = (count as usize).checked_mul(width).ok_or_else(invalid)?;
                let value = if length <= 4 {
                    if entry[8 + length..].iter().any(|byte| *byte != 0) {
                        private_count += 1;
                    }
                    entry[8..8 + length].to_vec()
                } else {
                    let start = endian.u32(&entry[8..])? as usize;
                    let end = start.checked_add(length).ok_or_else(invalid)?;
                    let value = data.get(start..end).ok_or_else(invalid)?.to_vec();
                    value_ranges.push(start..end);
                    used.push(start..end);
                    value
                };
                fields.push(Field {
                    tag,
                    kind,
                    count,
                    value,
                });
            }
            offset = endian.u32(&data[end - 4..end])? as usize;
            directories.push(fields);
        }
        if directories.is_empty()
            || value_ranges.iter().any(|value| {
                directory_ranges
                    .iter()
                    .any(|directory| value.start < directory.end && value.end > directory.start)
            })
        {
            return Err(invalid());
        }
        let indexed = directories[0]
            .iter()
            .any(|field| matches!(field.tag, 0xb001 | 0xb002));
        if !indexed && directories.len() != 1 {
            return Err(invalid());
        }
        for (index, fields) in directories.iter_mut().enumerate() {
            let is_index = indexed && index == 0;
            for field in fields.iter() {
                let expected = if is_index {
                    match field.tag {
                        0xb000 => Some((7, 4)),
                        0xb001 | 0xb004 => Some((4, 1)),
                        0xb002 => Some((7, field.count)),
                        _ => None,
                    }
                } else {
                    match field.tag {
                        0xb000 => Some((7, 4)),
                        0xb101 | 0xb201 | 0xb204 => Some((4, 1)),
                        0xb202 | 0xb203 | 0xb206 => Some((5, 1)),
                        0xb205 | 0xb207..=0xb20d => Some((10, 1)),
                        _ => None,
                    }
                };
                if let Some((kind, count)) = expected {
                    if field.kind != kind
                        || field.count != count
                        || field.tag == 0xb000 && field.value != b"0100"
                        || matches!(kind, 5 | 10) && endian.u32(&field.value[4..])? == 0
                    {
                        return Err(invalid());
                    }
                } else {
                    private_count += 1;
                }
            }
            fields.retain(|field| {
                if is_index {
                    matches!(field.tag, 0xb000..=0xb002 | 0xb004)
                } else {
                    matches!(field.tag, 0xb000 | 0xb101 | 0xb201..=0xb20d)
                }
            });
            fields.sort_by_key(|field| field.tag);
        }
        // Referenced values and directory records are accounted for separately;
        // nonzero orphan data must not survive reconstruction.
        used.sort_by_key(|range| range.start);
        let mut cursor = 0;
        let mut orphan = false;
        for range in used {
            if range.start > cursor {
                orphan |= data[cursor..range.start].iter().any(|byte| *byte != 0);
            }
            cursor = cursor.max(range.end);
        }
        orphan |= data[cursor..].iter().any(|byte| *byte != 0);
        private_count += usize::from(orphan);
        let metadata = Self {
            endian,
            directories,
            private_count,
        };
        if indexed {
            if !metadata.directories[0]
                .iter()
                .any(|field| field.tag == 0xb000)
            {
                return Err(invalid());
            }
            metadata.entries()?;
        }
        Ok(metadata)
    }

    fn entries(&self) -> Result<Option<Vec<Entry>>> {
        let fields = &self.directories[0];
        let count = fields.iter().find(|field| field.tag == 0xb001);
        let table = fields.iter().find(|field| field.tag == 0xb002);
        let (Some(count), Some(table)) = (count, table) else {
            return if count.is_none() && table.is_none() {
                Ok(None)
            } else {
                Err(invalid())
            };
        };
        let count = self.endian.u32(&count.value)? as usize;
        if count == 0 || count.checked_mul(16) != Some(table.value.len()) {
            return Err(invalid());
        }
        table
            .value
            .chunks_exact(16)
            .map(|bytes| {
                let entry = Entry {
                    attributes: self.endian.u32(bytes)?,
                    size: self.endian.u32(&bytes[4..])?,
                    offset: self.endian.u32(&bytes[8..])?,
                    dependencies: [
                        self.endian.u16(&bytes[12..])?,
                        self.endian.u16(&bytes[14..])?,
                    ],
                };
                if entry.size < 4
                    || entry.attributes & 0x0700_0000 != 0
                    || entry
                        .dependencies
                        .iter()
                        .any(|index| usize::from(*index) > count)
                {
                    return Err(invalid());
                }
                Ok(entry)
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    }

    fn set_entries(&mut self, entries: &[Entry]) {
        let field = self.directories[0]
            .iter_mut()
            .find(|field| field.tag == 0xb002)
            .unwrap();
        field.value.clear();
        for entry in entries {
            self.endian.put_u32(&mut field.value, entry.attributes);
            self.endian.put_u32(&mut field.value, entry.size);
            self.endian.put_u32(&mut field.value, entry.offset);
            for dependency in entry.dependencies {
                self.endian.put_u16(&mut field.value, dependency);
            }
        }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let mut data = if self.endian.0 {
            b"II\x2a\0".to_vec()
        } else {
            b"MM\0\x2a".to_vec()
        };
        self.endian.put_u32(&mut data, 8);
        let mut directory_offset = 8;
        let mut value_offset = 8 + self
            .directories
            .iter()
            .map(|fields| 6 + fields.len() * 12)
            .sum::<usize>();
        let mut values = Vec::new();
        for (index, fields) in self.directories.iter().enumerate() {
            self.endian.put_u16(&mut data, fields.len() as u16);
            for field in fields {
                self.endian.put_u16(&mut data, field.tag);
                self.endian.put_u16(&mut data, field.kind);
                self.endian.put_u32(&mut data, field.count);
                if field.value.len() <= 4 {
                    data.extend_from_slice(&field.value);
                    data.resize(data.len() + 4 - field.value.len(), 0);
                } else {
                    self.endian.put_u32(&mut data, value_offset as u32);
                    values.extend_from_slice(&field.value);
                    if values.len() % 2 != 0 {
                        values.push(0);
                    }
                    value_offset = 8
                        + self
                            .directories
                            .iter()
                            .map(|fields| 6 + fields.len() * 12)
                            .sum::<usize>()
                        + values.len();
                }
            }
            directory_offset += 6 + fields.len() * 12;
            self.endian.put_u32(
                &mut data,
                if index + 1 < self.directories.len() {
                    directory_offset as u32
                } else {
                    0
                },
            );
        }
        data.extend(values);
        let mut payload = b"MPF\0".to_vec();
        payload.extend(data);
        if payload.len() + 2 > usize::from(u16::MAX) {
            return Err(invalid());
        }
        Ok(payload)
    }
}

pub(super) fn privacy_count(payload: &[u8]) -> Result<usize> {
    Ok(Metadata::parse(payload)?.private_count)
}

pub(super) fn clean_metadata(payload: &[u8]) -> Result<Vec<u8>> {
    Metadata::parse(payload)?.serialize()
}

fn extension<'a>(segments: &'a [JpegSegment<'a>]) -> Result<Option<(&'a [u8], Range<usize>)>> {
    let mut found = None;
    for (marker, payload, range) in segments {
        if *marker == 0xe2 && payload.starts_with(b"MPF\0") {
            if found.is_some() {
                return Err(invalid());
            }
            found = Some((*payload, range.clone()));
        }
    }
    Ok(found)
}

pub(super) struct Container {
    frames: Vec<Range<usize>>,
    gaps: bool,
}

pub(super) fn parse(data: &[u8]) -> Result<Option<Container>> {
    let segments = jpeg_segments(data)?;
    let Some((payload, range)) = extension(&segments)? else {
        return Ok(None);
    };
    let metadata = Metadata::parse(payload)?;
    let Some(entries) = metadata.entries()? else {
        return Ok(None);
    };
    let base = range.end - payload.len() + 4;
    let first_end = segments
        .last()
        .filter(|segment| segment.0 == 0)
        .map_or(data.len(), |segment| segment.2.start);
    let mut frames = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let start = if index == 0 {
            if entry.offset != 0 || entry.size as usize != first_end {
                return Err(invalid());
            }
            0
        } else {
            if entry.offset == 0 {
                return Err(invalid());
            }
            base.checked_add(entry.offset as usize)
                .ok_or_else(invalid)?
        };
        let end = start.checked_add(entry.size as usize).ok_or_else(invalid)?;
        let frame = data.get(start..end).ok_or_else(invalid)?;
        let frame_segments = jpeg_segments(frame)?;
        if frame_segments.last().is_some_and(|segment| segment.0 == 0) {
            return Err(invalid());
        }
        if index != 0 {
            if let Some((payload, _)) = extension(&frame_segments)? {
                if Metadata::parse(payload)?.entries()?.is_some() {
                    return Err(invalid());
                }
            }
        }
        frames.push(start..end);
    }
    let mut sorted = frames.clone();
    sorted.sort_by_key(|range| range.start);
    let mut end = 0;
    let mut gaps = false;
    for frame in sorted {
        if frame.start < end {
            return Err(invalid());
        }
        gaps |= frame.start != end;
        end = frame.end;
    }
    gaps |= end != data.len();
    super::gainmap::validate_container(data, &frames)?;
    Ok(Some(Container { frames, gaps }))
}

fn merge(findings: &mut Vec<Finding>, additions: Vec<Finding>) {
    for addition in additions {
        if let Some(existing) = findings.iter_mut().find(|item| {
            item.category == addition.category
                && item.label == addition.label
                && item.severity == addition.severity
        }) {
            existing.count += addition.count;
        } else {
            findings.push(addition);
        }
    }
}

impl Container {
    pub(super) fn inspect(&self, data: &[u8]) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for frame in &self.frames {
            merge(
                &mut findings,
                super::inspect_single_jpeg(&data[frame.clone()])?,
            );
        }
        if self.gaps {
            findings.push(super::finding("JPEG 多图间隙 / 尾部应用数据", 1));
        }
        Ok(findings)
    }

    pub(super) fn clean(
        &self,
        data: &[u8],
        orientation: bool,
        profile: bool,
    ) -> Result<(Vec<u8>, Vec<Finding>)> {
        let findings = self
            .inspect(data)?
            .into_iter()
            .filter(|finding| match finding.category.as_str() {
                "color_profile" => !profile,
                "image_orientation" => !orientation,
                _ => true,
            })
            .collect();
        let mut frames = self
            .frames
            .iter()
            .map(|frame| {
                super::clean_single_jpeg(&data[frame.clone()], orientation, profile)
                    .map(|result| result.0)
            })
            .collect::<Result<Vec<_>>>()?;
        super::gainmap::rewrite_container(data, &self.frames, &mut frames)?;
        let (mut metadata, payload_range) = {
            let segments = jpeg_segments(&frames[0])?;
            let (payload, range) = extension(&segments)?.ok_or_else(invalid)?;
            (
                Metadata::parse(payload)?,
                range.end - payload.len()..range.end,
            )
        };
        let base = payload_range.start + 4;
        let mut entries = metadata.entries()?.ok_or_else(invalid)?;
        let mut offset = 0usize;
        let mut order = (0..frames.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| self.frames[*index].start);
        for &index in &order {
            let entry = &mut entries[index];
            let frame = &frames[index];
            entry.size = u32::try_from(frame.len()).map_err(|_| invalid())?;
            entry.offset = if index == 0 {
                0
            } else {
                u32::try_from(offset.checked_sub(base).ok_or_else(invalid)?)
                    .map_err(|_| invalid())?
            };
            offset = offset.checked_add(frame.len()).ok_or_else(invalid)?;
        }
        metadata.set_entries(&entries);
        let payload = metadata.serialize()?;
        if payload.len() != payload_range.len() {
            return Err(invalid());
        }
        frames[0][payload_range].copy_from_slice(&payload);
        let mut output = Vec::with_capacity(offset);
        for index in order {
            output.extend_from_slice(&frames[index]);
        }
        Ok((output, findings))
    }

    pub(super) fn verify(&self, data: &[u8], orientation: bool, profile: bool) -> Result<()> {
        if self.gaps {
            return Err(CleanError::Verification(
                "JPEG 多图间仍存在未索引数据".into(),
            ));
        }
        for frame in &self.frames {
            super::verify_single_jpeg(&data[frame.clone()], orientation, profile)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaners::image::{clean_jpeg_with_options, inspect_jpeg, verify_jpeg_cleaned};

    const ENTROPY: &[u8] = &[
        0xff, 0xda, 0, 8, 1, 1, 0, 0, 0x3f, 0, 0x21, 0xff, 0, 0x32, 0xff, 0xd9,
    ];

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, marker];
        bytes.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn image_prefix() -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend(segment(0xe1, b"Exif\0\0private author"));
        bytes.extend(super::super::orientation_segment(6));
        bytes.extend(segment(0xe2, b"ICC_PROFILE\0\x01\x01profile"));
        bytes
    }

    // Build the on-disk index directly, independent of Metadata::serialize.
    fn fixture(little: bool, private: bool) -> (Vec<u8>, usize) {
        let endian = Endian(little);
        let fields = if private { 4 } else { 3 };
        let table_offset = 8 + 6 + fields * 12;
        let mut payload = b"MPF\0".to_vec();
        payload.extend_from_slice(if little { b"II\x2a\0" } else { b"MM\0\x2a" });
        endian.put_u32(&mut payload, 8);
        endian.put_u16(&mut payload, fields as u16);
        for (tag, kind, count, value) in [
            (0xb000, 7, 4, b"0100".to_vec()),
            (
                0xb001,
                4,
                1,
                if little {
                    2u32.to_le_bytes()
                } else {
                    2u32.to_be_bytes()
                }
                .to_vec(),
            ),
            (
                0xb002,
                7,
                32,
                if little {
                    (table_offset as u32).to_le_bytes()
                } else {
                    (table_offset as u32).to_be_bytes()
                }
                .to_vec(),
            ),
        ] {
            endian.put_u16(&mut payload, tag);
            endian.put_u16(&mut payload, kind);
            endian.put_u32(&mut payload, count);
            payload.extend(value);
        }
        if private {
            endian.put_u16(&mut payload, 0xb003);
            endian.put_u16(&mut payload, 7);
            endian.put_u32(&mut payload, 66);
            endian.put_u32(&mut payload, (table_offset + 32) as u32);
        }
        endian.put_u32(&mut payload, 0);
        let first = image_prefix();
        let second = [image_prefix(), ENTROPY.to_vec()].concat();
        let first_size =
            first.len() + 4 + payload.len() + 32 + usize::from(private) * 66 + ENTROPY.len();
        let base = first.len() + 8;
        let gap = if private {
            b"private gap".as_slice()
        } else {
            b""
        };
        for (attributes, size, offset, dependency) in [
            (0xa003_0000, first_size, 0, 2),
            (0x4005_0000, second.len(), first_size + gap.len() - base, 0),
        ] {
            endian.put_u32(&mut payload, attributes);
            endian.put_u32(&mut payload, size as u32);
            endian.put_u32(&mut payload, offset as u32);
            endian.put_u16(&mut payload, dependency);
            endian.put_u16(&mut payload, 0);
        }
        if private {
            payload.extend_from_slice(&[b'X'; 66]);
        }
        let table = base + table_offset;
        let mut bytes = first;
        bytes.extend(segment(0xe2, &payload));
        bytes.extend_from_slice(ENTROPY);
        bytes.extend_from_slice(gap);
        bytes.extend(second);
        if private {
            bytes.extend_from_slice(b"private trailer");
        }
        (bytes, table)
    }

    #[test]
    fn cleans_all_indexed_images_and_relocates_both_endian_indexes() {
        for little in [false, true] {
            let (source, _) = fixture(little, true);
            let before = parse(&source).unwrap().unwrap();
            assert_eq!(before.frames.len(), 2);
            assert_eq!(
                inspect_jpeg(&source)
                    .unwrap()
                    .iter()
                    .find(|item| item.label == "EXIF / GPS 元数据")
                    .unwrap()
                    .count,
                2
            );
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
            for orientation in [false, true] {
                for profile in [false, true] {
                    let (output, findings) =
                        clean_jpeg_with_options(&source, orientation, profile).unwrap();
                    let after = parse(&output).unwrap().unwrap();
                    assert_eq!(after.frames.len(), 2);
                    assert!(!after.gaps);
                    assert!(output.len() < source.len());
                    assert!(!output.windows(7).any(|bytes| bytes == b"private"));
                    assert!(!output.windows(33).any(|bytes| bytes == [b'X'; 33]));
                    assert_eq!(
                        findings
                            .iter()
                            .find(|item| item.label == "EXIF / GPS 元数据")
                            .unwrap()
                            .count,
                        2
                    );
                    for frame in &after.frames {
                        let bytes = &output[frame.clone()];
                        assert!(bytes.ends_with(ENTROPY));
                        assert_eq!(
                            super::super::inspect_single_jpeg(bytes)
                                .unwrap()
                                .iter()
                                .any(|item| item.category == "image_orientation"),
                            orientation
                        );
                        assert_eq!(
                            super::super::inspect_single_jpeg(bytes)
                                .unwrap()
                                .iter()
                                .any(|item| item.category == "color_profile"),
                            profile
                        );
                    }
                    let segments = jpeg_segments(&output).unwrap();
                    let (payload, _) = extension(&segments).unwrap().unwrap();
                    let metadata = Metadata::parse(payload).unwrap();
                    let entries = metadata.entries().unwrap().unwrap();
                    assert_eq!(entries[0].attributes, 0xa003_0000);
                    assert_eq!(entries[0].dependencies, [2, 0]);
                    assert_eq!(entries[1].attributes, 0x4005_0000);
                    assert_eq!(metadata.private_count, 0);
                    verify_jpeg_cleaned(&output, orientation, profile).unwrap();
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_frame_ranges_formats_and_dependencies_before_cleaning() {
        for little in [false, true] {
            let (source, table) = fixture(little, false);
            let endian = Endian(little);
            for (position, value) in [
                (4, 4),
                (8, 1),
                (20, u32::MAX),
                (24, 0),
                (24, u32::MAX),
                (16, 0x0100_0000),
                (12, 3),
            ] {
                let mut invalid_source = source.clone();
                let mut replacement = Vec::new();
                endian.put_u32(&mut replacement, value);
                invalid_source[table + position..table + position + 4]
                    .copy_from_slice(&replacement);
                assert!(inspect_jpeg(&invalid_source).is_err());
                assert!(clean_jpeg_with_options(&invalid_source, true, true).is_err());
                assert!(verify_jpeg_cleaned(&invalid_source, true, true).is_err());
            }
            let mut truncated = source;
            truncated.pop();
            assert!(inspect_jpeg(&truncated).is_err());
        }
    }

    #[test]
    fn rejects_corrupt_ifd_links_counts_versions_and_aliases() {
        let (source, _) = fixture(true, false);
        let segments = jpeg_segments(&source).unwrap();
        let (payload, range) = extension(&segments).unwrap().unwrap();
        let payload_start = range.end - payload.len();
        for (position, value) in [
            (8, 0),
            (8, 4),
            (8, u32::MAX),
            (50, 8),
            (46, 8),
            (18, u32::MAX),
        ] {
            let mut bytes = source.clone();
            bytes[payload_start + position..payload_start + position + 4]
                .copy_from_slice(&value.to_le_bytes());
            assert!(inspect_jpeg(&bytes).is_err());
            assert!(clean_jpeg_with_options(&bytes, true, true).is_err());
        }
        let mut duplicate = source.clone();
        duplicate[payload_start + 26..payload_start + 28].copy_from_slice(&0xb000u16.to_le_bytes());
        assert!(inspect_jpeg(&duplicate).is_err());
        let mut version = source;
        version[payload_start + 22] = b'9';
        assert!(inspect_jpeg(&version).is_err());
    }

    #[test]
    fn handles_single_byte_index_corruption_without_panicking_or_emitting_invalid_candidates() {
        for little in [false, true] {
            let (source, _) = fixture(little, true);
            for index in 0..source.len() {
                let mut corrupted = source.clone();
                corrupted[index] ^= 0xff;
                let result = std::panic::catch_unwind(|| {
                    let _ = inspect_jpeg(&corrupted);
                    if let Ok((cleaned, _)) = clean_jpeg_with_options(&corrupted, true, true) {
                        verify_jpeg_cleaned(&cleaned, true, true).unwrap();
                    }
                });
                assert!(result.is_ok(), "Corruption at byte {index} panicked");
            }
        }
    }

    #[test]
    fn retains_signed_view_geometry_and_removes_attribute_identity_and_orphans() {
        // Attribute-only extensions occur in secondary frames. The signed yaw
        // value is rendering geometry, while an arbitrary string field is private.
        let mut payload = b"MPF\0MM\0\x2a\0\0\0\x08\0\x03".to_vec();
        for (tag, kind, count, value) in [
            (0xb000u16, 7u16, 4u32, *b"0100"),
            (0xb20b, 10, 1, 50u32.to_be_bytes()),
            (0xc001, 2, 8, 58u32.to_be_bytes()),
        ] {
            payload.extend_from_slice(&tag.to_be_bytes());
            payload.extend_from_slice(&kind.to_be_bytes());
            payload.extend_from_slice(&count.to_be_bytes());
            payload.extend_from_slice(&value);
        }
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&(-30i32).to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(b"private\0orphan");
        let source = [vec![0xff, 0xd8], segment(0xe2, &payload), ENTROPY.to_vec()].concat();
        assert_eq!(privacy_count(&payload).unwrap(), 2);
        assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        let output = clean_jpeg_with_options(&source, true, true).unwrap().0;
        let segments = jpeg_segments(&output).unwrap();
        let (cleaned, _) = extension(&segments).unwrap().unwrap();
        let metadata = Metadata::parse(cleaned).unwrap();
        assert_eq!(
            metadata.directories[0][1].value,
            [(-30i32).to_be_bytes(), 1u32.to_be_bytes()].concat()
        );
        assert_eq!(metadata.private_count, 0);
        assert!(output.ends_with(ENTROPY));
        verify_jpeg_cleaned(&output, true, true).unwrap();
    }
}
