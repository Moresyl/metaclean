//! Remove Ogg audio comments without repaginating or changing audio bytes.

use std::{collections::HashMap, ops::Range};

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_HEADER: usize = 16 * 1024 * 1024;

fn invalid() -> CleanError {
    CleanError::InvalidFormat("Invalid or unsupported Ogg audio structure".into())
}

fn word(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or_else(invalid)?
            .try_into()
            .unwrap(),
    ))
}

fn crc(page: &[u8]) -> u32 {
    let mut value = 0u32;
    for (index, byte) in page.iter().enumerate() {
        value ^= u32::from(if (22..26).contains(&index) { 0 } else { *byte }) << 24;
        for _ in 0..8 {
            value = if value & 0x8000_0000 != 0 {
                (value << 1) ^ 0x04c1_1db7
            } else {
                value << 1
            };
        }
    }
    value
}

#[derive(Default)]
struct Stream {
    codec: Option<Codec>,
    sequence: u32,
    packets: usize,
    continued: bool,
    ended: bool,
    header: Vec<u8>,
    fragments: Vec<Range<usize>>,
    packet_size: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Codec {
    Opus,
    Vorbis,
}

impl Stream {
    fn header_count(&self) -> usize {
        if self.codec == Some(Codec::Vorbis) {
            3
        } else {
            2
        }
    }
}

fn identify_codec(packet: &[u8]) -> Result<Codec> {
    if packet.starts_with(b"OpusHead") {
        identification(packet)?;
        return Ok(Codec::Opus);
    }
    if packet.len() != 30 || !packet.starts_with(b"\x01vorbis") {
        return Err(invalid());
    }
    let small_block = packet[28] & 15;
    let large_block = packet[28] >> 4;
    if word(packet, 7)? != 0
        || packet[11] == 0
        || word(packet, 12)? == 0
        || !(6..=13).contains(&small_block)
        || !(small_block..=13).contains(&large_block)
        || packet[29] != 1
    {
        return Err(invalid());
    }
    Ok(Codec::Vorbis)
}

struct Layout {
    pages: Vec<Range<usize>>,
    comments: Vec<(Vec<Range<usize>>, Vec<u8>)>,
    count: usize,
}

fn identification(packet: &[u8]) -> Result<()> {
    if packet.len() < 19 || !packet.starts_with(b"OpusHead") || packet[8] != 1 || packet[9] == 0 {
        return Err(invalid());
    }
    let channels = usize::from(packet[9]);
    match packet[18] {
        0 if channels <= 2 && packet.len() == 19 => Ok(()),
        1 | 255 if packet.len() == 21 + channels => {
            let streams = u16::from(packet[19]);
            let coupled = u16::from(packet[20]);
            if streams == 0
                || (packet[18] == 1 && channels > 8)
                || coupled > streams
                || streams + coupled > 255
                || packet[21..]
                    .iter()
                    .any(|value| *value != 255 && u16::from(*value) >= streams + coupled)
            {
                return Err(invalid());
            }
            Ok(())
        }
        _ => Err(invalid()),
    }
}

fn playback_gain(name: &[u8], value: &[u8], vorbis: bool) -> Result<bool> {
    let r128 = !vorbis
        && (name.eq_ignore_ascii_case(b"R128_TRACK_GAIN")
            || name.eq_ignore_ascii_case(b"R128_ALBUM_GAIN"));
    let replay_gain = vorbis
        && (name.eq_ignore_ascii_case(b"REPLAYGAIN_TRACK_GAIN")
            || name.eq_ignore_ascii_case(b"REPLAYGAIN_ALBUM_GAIN")
            || name.eq_ignore_ascii_case(b"REPLAYGAIN_REFERENCE_LOUDNESS"));
    let replay_peak = vorbis
        && (name.eq_ignore_ascii_case(b"REPLAYGAIN_TRACK_PEAK")
            || name.eq_ignore_ascii_case(b"REPLAYGAIN_ALBUM_PEAK"));
    if !r128 && !replay_gain && !replay_peak {
        return Ok(false);
    }
    let number = if replay_gain {
        value.strip_suffix(b" dB").unwrap_or(value)
    } else {
        value
    };
    let digits = if matches!(number.first(), Some(b'+' | b'-')) {
        &number[1..]
    } else {
        number
    };
    let valid = if r128 {
        value.len() <= 6
            && !digits.is_empty()
            && digits.iter().all(u8::is_ascii_digit)
            && std::str::from_utf8(number)
                .ok()
                .and_then(|value| value.parse::<i16>().ok())
                .is_some()
    } else {
        value.len() <= 32
            && digits.iter().any(u8::is_ascii_digit)
            && digits
                .iter()
                .all(|byte| byte.is_ascii_digit() || *byte == b'.')
            && std::str::from_utf8(number)
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value.is_finite() && (!replay_peak || value >= 0.0))
    };
    if !valid {
        return Err(invalid());
    }
    Ok(true)
}

fn clean_comments(packet: &[u8]) -> Result<(Vec<u8>, usize)> {
    let (prefix, vorbis): (&[u8], bool) = if packet.starts_with(b"OpusTags") {
        (b"OpusTags", false)
    } else if packet.starts_with(b"\x03vorbis") {
        (b"\x03vorbis", true)
    } else {
        return Err(invalid());
    };
    let vendor_size = word(packet, prefix.len())? as usize;
    let mut offset = (prefix.len() + 4)
        .checked_add(vendor_size)
        .filter(|end| *end <= packet.len())
        .ok_or_else(invalid)?;
    let comments = word(packet, offset)? as usize;
    offset += 4;
    if comments > (packet.len() - offset) / 4 {
        return Err(invalid());
    }
    let mut retained: Vec<&[u8]> = Vec::new();
    let mut gain_names = Vec::new();
    for _ in 0..comments {
        let size = word(packet, offset)? as usize;
        let start = offset + 4;
        offset = offset
            .checked_add(4)
            .and_then(|start| start.checked_add(size))
            .filter(|end| *end <= packet.len())
            .ok_or_else(invalid)?;
        let comment = &packet[start..offset];
        if let Some(separator) = comment.iter().position(|byte| *byte == b'=') {
            let name = &comment[..separator];
            if playback_gain(name, &comment[separator + 1..], vorbis)? {
                let normalized = name.to_ascii_uppercase();
                if gain_names.contains(&normalized) {
                    return Err(invalid());
                }
                gain_names.push(normalized);
                retained.push(comment);
            }
        }
    }
    // An odd extension marker requests preservation of unspecified binary data.
    // Refuse it until its semantics can be validated rather than erasing it.
    if vorbis {
        if packet.get(offset).is_none_or(|byte| byte & 1 == 0) {
            return Err(invalid());
        }
    } else if packet.get(offset).is_some_and(|byte| byte & 1 != 0) {
        return Err(invalid());
    }
    // Zero padding carries no private data; disposable padding is cleared.
    let padding_has_data = if vorbis {
        packet[offset] & !1 != 0 || packet[offset + 1..].iter().any(|byte| *byte != 0)
    } else {
        packet[offset..].iter().any(|byte| *byte != 0)
    };
    let count =
        usize::from(vendor_size > 0) + comments - retained.len() + usize::from(padding_has_data);
    let mut cleaned = prefix.to_vec();
    cleaned.extend_from_slice(&0u32.to_le_bytes());
    cleaned.extend_from_slice(&(retained.len() as u32).to_le_bytes());
    for comment in retained {
        cleaned.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        cleaned.extend_from_slice(comment);
    }
    if vorbis {
        cleaned.push(1);
    }
    cleaned.resize(packet.len(), 0);
    Ok((cleaned, count))
}

fn layout(data: &[u8]) -> Result<Layout> {
    let mut streams: HashMap<u32, Stream> = HashMap::new();
    let mut result = Layout {
        pages: Vec::new(),
        comments: Vec::new(),
        count: 0,
    };
    let mut offset = 0usize;
    while offset < data.len() {
        let header = data.get(offset..offset + 27).ok_or_else(invalid)?;
        if &header[..4] != b"OggS" || header[4] != 0 || header[5] & !7 != 0 {
            return Err(invalid());
        }
        let flags = header[5];
        let serial = word(header, 14)?;
        let sequence = word(header, 18)?;
        let lacing_end = offset + 27 + usize::from(header[26]);
        let lacing = data.get(offset + 27..lacing_end).ok_or_else(invalid)?;
        let length = lacing.iter().map(|byte| usize::from(*byte)).sum::<usize>();
        let end = lacing_end
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or_else(invalid)?;
        if crc(&data[offset..end]) != word(header, 22)? {
            return Err(invalid());
        }
        if flags & 2 != 0 {
            if sequence != 0
                || flags & 1 != 0
                || streams.contains_key(&serial)
                || streams.len() >= 64
            {
                return Err(invalid());
            }
            streams.insert(serial, Stream::default());
        }
        let stream = streams.get_mut(&serial).ok_or_else(invalid)?;
        if stream.ended || stream.sequence != sequence || stream.continued != (flags & 1 != 0) {
            return Err(invalid());
        }
        stream.sequence = sequence.wrapping_add(1);
        let mut cursor = lacing_end;
        for (index, segment) in lacing.iter().enumerate() {
            let next = cursor + usize::from(*segment);
            if stream.codec == Some(Codec::Vorbis)
                && stream.packets >= 3
                && stream.packet_size == 0
                && next > cursor
                && data[cursor] & 1 != 0
            {
                return Err(invalid());
            }
            stream.packet_size += usize::from(*segment);
            if stream.packets < stream.header_count() {
                if stream.packet_size > MAX_HEADER {
                    return Err(invalid());
                }
                stream.header.extend_from_slice(&data[cursor..next]);
                stream.fragments.push(cursor..next);
            }
            stream.continued = *segment == 255;
            if !stream.continued {
                if stream.packets < stream.header_count() {
                    let comment_can_share_page =
                        stream.codec == Some(Codec::Vorbis) && stream.packets == 1;
                    if (!comment_can_share_page && index + 1 != lacing.len())
                        || header[6..14] != [0; 8]
                    {
                        return Err(invalid());
                    }
                    if stream.packets == 0 {
                        if flags & 2 == 0 {
                            return Err(invalid());
                        }
                        stream.codec = Some(identify_codec(&stream.header)?);
                    } else if stream.packets == 1 {
                        let expected: &[u8] = if stream.codec == Some(Codec::Vorbis) {
                            b"\x03vorbis"
                        } else {
                            b"OpusTags"
                        };
                        if !stream.header.starts_with(expected) {
                            return Err(invalid());
                        }
                        let (cleaned, count) = clean_comments(&stream.header)?;
                        result.count += count;
                        result
                            .comments
                            .push((std::mem::take(&mut stream.fragments), cleaned));
                    } else if !stream.header.starts_with(b"\x05vorbis") || stream.header.len() <= 7
                    {
                        return Err(invalid());
                    }
                    stream.header.clear();
                    stream.fragments.clear();
                } else if stream.packet_size == 0 && stream.codec == Some(Codec::Opus) {
                    return Err(invalid());
                }
                stream.packets += 1;
                stream.packet_size = 0;
            }
            cursor = next;
        }
        if flags & 2 != 0 && stream.packets != 1 {
            return Err(invalid());
        }
        stream.ended = flags & 4 != 0;
        if stream.ended && (stream.continued || stream.packets <= stream.header_count()) {
            return Err(invalid());
        }
        result.pages.push(offset..end);
        offset = end;
    }
    if streams.is_empty() || streams.values().any(|stream| !stream.ended) {
        return Err(invalid());
    }
    Ok(result)
}

fn findings(count: usize) -> Vec<Finding> {
    if count == 0 {
        return Vec::new();
    }
    vec![Finding {
        category: "audio_metadata".into(),
        label: "Ogg audio comments and encoder metadata".into(),
        count,
        severity: FindingSeverity::Privacy,
    }]
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(layout(data)?.count))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let parsed = layout(data)?;
    let mut output = data.to_vec();
    for (fragments, packet) in parsed.comments {
        let mut position = 0;
        for fragment in fragments {
            for byte in &mut output[fragment] {
                *byte = packet[position];
                position += 1;
            }
        }
    }
    for page in parsed.pages {
        let checksum = crc(&output[page.clone()]);
        output[page.start + 22..page.start + 26].copy_from_slice(&checksum.to_le_bytes());
    }
    Ok((output, findings(parsed.count)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn comment_count(packet: &[u8]) -> Result<usize> {
        Ok(clean_comments(packet)?.1)
    }

    fn tags(comments: &[&str]) -> Vec<u8> {
        let mut packet = b"OpusTags".to_vec();
        packet.extend_from_slice(&0u32.to_le_bytes());
        packet.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for comment in comments {
            packet.extend_from_slice(&(comment.len() as u32).to_le_bytes());
            packet.extend_from_slice(comment.as_bytes());
        }
        packet
    }

    #[test]
    fn preserves_only_valid_playback_gain_comments() {
        let packet = tags(&[
            "ARTIST=Alice",
            "R128_TRACK_GAIN=-573",
            "r128_album_gain=+00111",
        ]);
        let (cleaned, count) = clean_comments(&packet).unwrap();
        assert_eq!(count, 1);
        assert_eq!(cleaned.len(), packet.len());
        assert!(cleaned
            .windows(b"r128_album_gain=+00111".len())
            .any(|value| value == b"r128_album_gain=+00111"));
        assert!(cleaned
            .windows(b"R128_TRACK_GAIN=-573".len())
            .any(|value| value == b"R128_TRACK_GAIN=-573"));
        assert!(!cleaned.windows(5).any(|value| value == b"Alice"));
        assert_eq!(clean_comments(&cleaned).unwrap(), (cleaned, 0));
        for invalid_value in ["", "+", " 1", "1 ", "32768", "-32769", "0000001", "1.0"] {
            assert!(clean_comments(&tags(&[&format!("R128_TRACK_GAIN={invalid_value}")])).is_err());
        }
        assert!(clean_comments(&tags(&["R128_TRACK_GAIN=1", "r128_track_gain=2"])).is_err());
    }

    fn page(sequence: u32, flags: u8, lacing: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut page = b"OggS\0".to_vec();
        page.push(flags);
        page.extend_from_slice(&0u64.to_le_bytes());
        page.extend_from_slice(&1u32.to_le_bytes());
        page.extend_from_slice(&sequence.to_le_bytes());
        page.extend_from_slice(&[0; 4]);
        page.push(lacing.len() as u8);
        page.extend_from_slice(lacing);
        page.extend_from_slice(payload);
        let checksum = crc(&page);
        page[22..26].copy_from_slice(&checksum.to_le_bytes());
        page
    }

    pub(crate) fn fixture(split: bool) -> Vec<u8> {
        let mut head = b"OpusHead\x01\x01".to_vec();
        head.extend_from_slice(&[0; 9]);
        let mut tags = b"OpusTags".to_vec();
        tags.extend_from_slice(&300u32.to_le_bytes());
        tags.extend_from_slice(&[b'v'; 300]);
        tags.extend_from_slice(&1u32.to_le_bytes());
        tags.extend_from_slice(&12u32.to_le_bytes());
        tags.extend_from_slice(b"ARTIST=Alice");
        let mut data = page(0, 2, &[19], &head);
        if split {
            data.extend(page(1, 0, &[255], &tags[..255]));
            data.extend(page(2, 1, &[(tags.len() - 255) as u8], &tags[255..]));
        } else {
            data.extend(page(1, 0, &[255, (tags.len() - 255) as u8], &tags));
        }
        data.extend(page(
            if split { 3 } else { 2 },
            4,
            &[3],
            &[0xf8, 0xff, 0xfe],
        ));
        data
    }

    pub(crate) fn vorbis_fixture(split: bool) -> Vec<u8> {
        let mut head = b"\x01vorbis".to_vec();
        head.extend_from_slice(&0u32.to_le_bytes());
        head.push(2);
        head.extend_from_slice(&48000u32.to_le_bytes());
        head.extend_from_slice(&[0; 12]);
        head.extend_from_slice(&[0xb8, 1]);
        let mut comments = b"\x03vorbis".to_vec();
        comments.extend_from_slice(&300u32.to_le_bytes());
        comments.extend_from_slice(&[b'v'; 300]);
        comments.extend_from_slice(&1u32.to_le_bytes());
        comments.extend_from_slice(&12u32.to_le_bytes());
        comments.extend_from_slice(b"ARTIST=Alice");
        comments.push(1);
        // Structural fixture; real codec setup is covered by external decoder tests.
        let setup = b"\x05vorbis\x01";
        let mut data = page(0, 2, &[30], &head);
        let tail = (comments.len() - 255) as u8;
        if split {
            data.extend(page(1, 0, &[255], &comments[..255]));
            data.extend(page(2, 1, &[tail, 8], &[&comments[255..], setup].concat()));
        } else {
            data.extend(page(
                1,
                0,
                &[255, tail, 8],
                &[comments.as_slice(), setup].concat(),
            ));
        }
        data.extend(page(if split { 3 } else { 2 }, 4, &[2], &[0, 42]));
        data
    }

    #[test]
    fn clears_vorbis_comments_sharing_pages_with_unchanged_setup() {
        for split in [false, true] {
            let original = vorbis_fixture(split);
            let (cleaned, findings) = clean(&original).unwrap();
            assert_eq!(findings[0].count, 2);
            assert_eq!(original.len(), cleaned.len());
            let layout = layout(&original).unwrap();
            let comment_ranges = &layout.comments[0].0;
            for index in 0..original.len() {
                let checksum = layout
                    .pages
                    .iter()
                    .any(|page| (page.start + 22..page.start + 26).contains(&index));
                if !checksum && !comment_ranges.iter().any(|range| range.contains(&index)) {
                    assert_eq!(original[index], cleaned[index]);
                }
            }
            assert!(inspect(&cleaned).unwrap().is_empty());
            assert_eq!(clean(&cleaned).unwrap().0, cleaned);
        }
    }

    #[test]
    fn validates_vorbis_identification_and_comment_framing() {
        let original = vorbis_fixture(false);
        let head = &original[28..58];
        assert!(identify_codec(head).is_ok());
        for (offset, value) in [(7, 1), (11, 0), (28, 0xb5), (28, 0x8b), (29, 0)] {
            let mut bad = head.to_vec();
            bad[offset] = value;
            assert!(identify_codec(&bad).is_err());
        }
        let mut packet = b"\x03vorbis".to_vec();
        packet.extend_from_slice(&[0; 8]);
        assert!(clean_comments(&packet).is_err());
        packet.push(0);
        assert!(clean_comments(&packet).is_err());
        packet[15] = 1;
        assert_eq!(clean_comments(&packet).unwrap(), (packet.clone(), 0));
        packet.extend_from_slice(b"hidden");
        let (cleaned, count) = clean_comments(&packet).unwrap();
        assert_eq!(count, 1);
        assert!(cleaned[16..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn preserves_bounded_numeric_vorbis_replaygain() {
        for (name, value) in [
            ("REPLAYGAIN_TRACK_GAIN", "-5.25 dB"),
            ("replaygain_album_gain", "+0.0"),
            ("REPLAYGAIN_TRACK_PEAK", "1.23"),
            ("REPLAYGAIN_ALBUM_PEAK", "0"),
            ("REPLAYGAIN_REFERENCE_LOUDNESS", "89.0 dB"),
        ] {
            let gain = format!("{name}={value}");
            let opus = tags(&["ARTIST=Alice", &gain]);
            let mut packet = b"\x03vorbis".to_vec();
            packet.extend_from_slice(&opus[8..]);
            packet.push(1);
            let (cleaned, count) = clean_comments(&packet).unwrap();
            assert_eq!(count, 1);
            assert!(cleaned
                .windows(gain.len())
                .any(|bytes| bytes == gain.as_bytes()));
            assert_eq!(clean_comments(&cleaned).unwrap(), (cleaned, 0));
        }
        for value in ["NaN", "inf", "-1", "1.2.3", "1e4", "", "Alice", "1 private"] {
            assert!(playback_gain(b"REPLAYGAIN_TRACK_PEAK", value.as_bytes(), true).is_err());
        }
    }

    #[test]
    fn refuses_vorbis_header_mismatches_even_with_valid_page_checksums() {
        let original = vorbis_fixture(false);
        let pages = layout(&original).unwrap().pages;
        let setup_start = pages[1].end - 8;
        let comment_start = pages[1].start + 30;
        let audio_start = pages[2].start + 28;
        for (offset, byte) in [(setup_start, 3), (comment_start, 5), (audio_start, 1)] {
            let mut corrupt = original.clone();
            corrupt[offset] = byte;
            let page = pages.iter().find(|page| page.contains(&offset)).unwrap();
            let checksum = crc(&corrupt[page.clone()]);
            corrupt[page.start + 22..page.start + 26].copy_from_slice(&checksum.to_le_bytes());
            assert!(inspect(&corrupt).is_err());
            assert!(clean(&corrupt).is_err());
        }
    }

    #[test]
    fn cleans_mixed_codec_chains_without_changing_either_audio_stream() {
        let first = fixture(false);
        let mut second = vorbis_fixture(true);
        for range in layout(&second).unwrap().pages {
            second[range.start + 14..range.start + 18].copy_from_slice(&2u32.to_le_bytes());
            let checksum = crc(&second[range.clone()]);
            second[range.start + 22..range.start + 26].copy_from_slice(&checksum.to_le_bytes());
        }
        let expected = [clean(&first).unwrap().0, clean(&second).unwrap().0].concat();
        let (cleaned, findings) = clean(&[first, second].concat()).unwrap();
        assert_eq!(findings[0].count, 4);
        assert_eq!(cleaned, expected);
        assert!(inspect(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn removes_comments_across_pages_without_changing_audio_or_offsets() {
        for split in [false, true] {
            let data = fixture(split);
            assert_eq!(inspect(&data).unwrap()[0].count, 2);
            let (cleaned, removed) = clean(&data).unwrap();
            assert_eq!(removed[0].count, 2);
            assert_eq!(data.len(), cleaned.len());
            assert_eq!(&data[data.len() - 31..], &cleaned[cleaned.len() - 31..]);
            assert!(!cleaned.windows(5).any(|bytes| bytes == b"Alice"));
            assert!(inspect(&cleaned).unwrap().is_empty());
            assert_eq!(clean(&cleaned).unwrap().0, cleaned);
        }
    }

    #[test]
    fn refuses_truncated_corrupted_or_reordered_pages() {
        let data = fixture(true);
        for end in 0..data.len() {
            assert!(inspect(&data[..end]).is_err());
        }
        for offset in 0..data.len() {
            let mut corrupt = data.clone();
            corrupt[offset] ^= 1;
            assert!(inspect(&corrupt).is_err());
        }
        let mut wrong_sequence = data.clone();
        let first = layout(&data).unwrap().pages[1].clone();
        wrong_sequence[first.start + 18] = 7;
        let checksum = crc(&wrong_sequence[first.clone()]);
        wrong_sequence[first.start + 22..first.start + 26].copy_from_slice(&checksum.to_le_bytes());
        assert!(clean(&wrong_sequence).is_err());
    }

    #[test]
    fn bounds_comment_lengths_and_refuses_preserved_extensions() {
        let mut tags = b"OpusTags".to_vec();
        tags.extend_from_slice(&0u32.to_le_bytes());
        tags.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(comment_count(&tags).unwrap(), 0);
        tags.extend_from_slice(&[0, 2, 3]);
        assert_eq!(comment_count(&tags).unwrap(), 1);
        tags[16] = 1;
        assert!(comment_count(&tags).is_err());
        tags.truncate(16);
        tags[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(comment_count(&tags).is_err());
        tags[8..12].fill(0);
        tags[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(comment_count(&tags).is_err());
    }

    #[test]
    fn validates_channel_mapping_and_identification() {
        let mut head = b"OpusHead\x01\x02".to_vec();
        head.extend_from_slice(&[0; 9]);
        assert!(identification(&head).is_ok());
        head[18] = 255;
        head.extend_from_slice(&[1, 1, 0, 1]);
        assert!(identification(&head).is_ok());
        head[22] = 2;
        assert!(identification(&head).is_err());
        head[22] = 255;
        assert!(identification(&head).is_ok());
        head[19] = 0;
        assert!(identification(&head).is_err());
        head[8] = 2;
        assert!(identification(&head).is_err());
        let mut surround = b"OpusHead\x01\x09".to_vec();
        surround.extend_from_slice(&[0; 8]);
        surround.extend_from_slice(&[1, 9, 0]);
        surround.extend(0..9);
        assert!(identification(&surround).is_err());
        surround[18] = 255;
        assert!(identification(&surround).is_ok());
    }

    #[test]
    fn cleans_chained_streams_and_refuses_duplicate_serials() {
        let first = fixture(false);
        let mut second = first.clone();
        for range in layout(&first).unwrap().pages {
            second[range.start + 14..range.start + 18].copy_from_slice(&2u32.to_le_bytes());
            let checksum = crc(&second[range.clone()]);
            second[range.start + 22..range.start + 26].copy_from_slice(&checksum.to_le_bytes());
        }
        let chained = [first.clone(), second].concat();
        assert_eq!(inspect(&chained).unwrap()[0].count, 4);
        assert!(inspect(&clean(&chained).unwrap().0).unwrap().is_empty());
        assert!(clean(&[first.clone(), first].concat()).is_err());
    }
}
