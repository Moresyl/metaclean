use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

fn finding(category: &str, label: &str, count: usize) -> Vec<Finding> {
    if count == 0 {
        Vec::new()
    } else {
        vec![Finding {
            category: category.into(),
            label: label.into(),
            count,
            severity: FindingSeverity::Privacy,
        }]
    }
}

fn provenance_finding(label: &str, count: usize) -> Vec<Finding> {
    if count == 0 {
        Vec::new()
    } else {
        vec![Finding {
            category: "provenance".into(),
            label: label.into(),
            count,
            severity: FindingSeverity::Provenance,
        }]
    }
}

fn gif_sub_blocks_end(data: &[u8], mut offset: usize) -> Result<usize> {
    loop {
        let length = usize::from(
            *data
                .get(offset)
                .ok_or_else(|| CleanError::InvalidFormat("GIF 数据块长度缺失".into()))?,
        );
        offset += 1;
        if length == 0 {
            return Ok(offset);
        }
        offset = offset
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| CleanError::InvalidFormat("GIF 数据块越界".into()))?;
    }
}

fn gif_private_ranges(data: &[u8]) -> Result<Vec<(Range<usize>, bool)>> {
    if data.len() < 14 || !matches!(&data[..6], b"GIF87a" | b"GIF89a") {
        return Err(CleanError::InvalidFormat("不是有效 GIF".into()));
    }
    let packed = data[10];
    let global_table = if packed & 0x80 != 0 {
        3usize << (usize::from(packed & 0x07) + 1)
    } else {
        0
    };
    let mut offset = 13usize
        .checked_add(global_table)
        .filter(|value| *value <= data.len())
        .ok_or_else(|| CleanError::InvalidFormat("GIF 全局颜色表越界".into()))?;
    let mut private = Vec::new();
    loop {
        match data.get(offset).copied() {
            Some(0x3b) => {
                if offset + 1 != data.len() {
                    return Err(CleanError::InvalidFormat(
                        "GIF 结束标记后存在多余数据".into(),
                    ));
                }
                return Ok(private);
            }
            Some(0x2c) => {
                let descriptor_end = offset
                    .checked_add(10)
                    .filter(|value| *value <= data.len())
                    .ok_or_else(|| CleanError::InvalidFormat("GIF 图像描述符越界".into()))?;
                let local_packed = data[offset + 9];
                let local_table = if local_packed & 0x80 != 0 {
                    3usize << (usize::from(local_packed & 0x07) + 1)
                } else {
                    0
                };
                let image_data = descriptor_end
                    .checked_add(local_table + 1)
                    .filter(|value| *value <= data.len())
                    .ok_or_else(|| CleanError::InvalidFormat("GIF 局部颜色表越界".into()))?;
                offset = gif_sub_blocks_end(data, image_data)?;
            }
            Some(0x21) => {
                let start = offset;
                let label = *data
                    .get(offset + 1)
                    .ok_or_else(|| CleanError::InvalidFormat("GIF 扩展标签缺失".into()))?;
                if label == 0xf9 {
                    if data.get(offset + 2) != Some(&4) || data.get(offset + 7) != Some(&0) {
                        return Err(CleanError::InvalidFormat("GIF 图形控制扩展无效".into()));
                    }
                    offset += 8;
                    continue;
                }
                let first_length = usize::from(
                    *data
                        .get(offset + 2)
                        .ok_or_else(|| CleanError::InvalidFormat("GIF 扩展长度缺失".into()))?,
                );
                let first_start = offset + 3;
                let sub_blocks = first_start
                    .checked_add(first_length)
                    .filter(|value| *value <= data.len())
                    .ok_or_else(|| CleanError::InvalidFormat("GIF 扩展越界".into()))?;
                offset = gif_sub_blocks_end(data, sub_blocks)?;
                let xmp_application =
                    label == 0xff && data[first_start..sub_blocks].starts_with(b"XMP DataXMP");
                let c2pa_application =
                    label == 0xff && data[first_start..sub_blocks].starts_with(b"C2PA_GIF");
                if label == 0xfe || xmp_application || c2pa_application {
                    private.push((start..offset, c2pa_application));
                }
            }
            _ => return Err(CleanError::InvalidFormat("GIF 块标记无效".into())),
        }
    }
}

pub fn inspect_gif(data: &[u8]) -> Result<Vec<Finding>> {
    let ranges = gif_private_ranges(data)?;
    let mut findings = finding(
        "image_metadata",
        "GIF 注释 / XMP 元数据",
        ranges.iter().filter(|(_, provenance)| !provenance).count(),
    );
    let provenance = ranges.iter().filter(|(_, provenance)| *provenance).count();
    if provenance > 0 {
        findings.push(Finding {
            category: "provenance".into(),
            label: "GIF C2PA 来源标记".into(),
            count: provenance,
            severity: FindingSeverity::Provenance,
        });
    }
    Ok(findings)
}

pub fn clean_gif(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let ranges = gif_private_ranges(data)?;
    let findings = inspect_gif(data)?;
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for (range, _) in ranges {
        output.extend_from_slice(&data[cursor..range.start]);
        cursor = range.end;
    }
    output.extend_from_slice(&data[cursor..]);
    Ok((output, findings))
}

fn synchsafe_size(bytes: &[u8]) -> Result<usize> {
    if bytes.len() != 4 || bytes.iter().any(|byte| byte & 0x80 != 0) {
        return Err(CleanError::InvalidFormat("ID3v2 长度无效".into()));
    }
    Ok(bytes
        .iter()
        .fold(0usize, |value, byte| (value << 7) | usize::from(*byte)))
}

fn id3v2_end(data: &[u8]) -> Result<usize> {
    if !data.starts_with(b"ID3") || data.len() < 10 {
        return Err(CleanError::InvalidFormat("ID3v2 头部不完整".into()));
    }
    let version = data[3];
    let allowed_flags = match version {
        2 => 0xc0,
        3 => 0xe0,
        4 => 0xf0,
        _ => return Err(CleanError::InvalidFormat("不支持的 ID3v2 版本".into())),
    };
    if data[4] != 0 || data[5] & !allowed_flags != 0 {
        return Err(CleanError::InvalidFormat("ID3v2 版本或标志无效".into()));
    }
    let body_end = 10usize
        .checked_add(synchsafe_size(&data[6..10])?)
        .filter(|value| *value <= data.len())
        .ok_or_else(|| CleanError::InvalidFormat("ID3v2 标签越界".into()))?;
    if version != 4 || data[5] & 0x10 == 0 {
        return Ok(body_end);
    }
    let footer_end = body_end
        .checked_add(10)
        .ok_or_else(|| CleanError::InvalidFormat("ID3v2.4 标签尾越界".into()))?;
    let footer = data
        .get(body_end..footer_end)
        .ok_or_else(|| CleanError::InvalidFormat("ID3v2.4 标签尾缺失".into()))?;
    if &footer[..3] != b"3DI"
        || footer[3] != version
        || footer[4] != 0
        || footer[5] != data[5]
        || footer[6..10] != data[6..10]
    {
        return Err(CleanError::InvalidFormat("ID3v2.4 标签尾无效".into()));
    }
    Ok(footer_end)
}

fn mp3_frame_length(data: &[u8]) -> Option<usize> {
    if data.len() < 4
        || data[0] != 0xff
        || data[1] & 0xe0 != 0xe0
        || data[1] & 0x18 == 0x08
        || data[1] & 0x06 == 0
        || matches!(data[2] >> 4, 0 | 0x0f)
        || data[2] & 0x0c == 0x0c
        || data[3] & 0x03 == 0x02
    {
        return None;
    }
    const MPEG1_LAYER1: [usize; 15] = [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
    ];
    const MPEG1_LAYER2: [usize; 15] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
    ];
    const MPEG1_LAYER3: [usize; 15] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ];
    const MPEG2_LAYER1: [usize; 15] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
    ];
    const MPEG2_LAYER23: [usize; 15] =
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

    let version = (data[1] >> 3) & 0x03;
    let layer = (data[1] >> 1) & 0x03;
    let bitrate_index = usize::from(data[2] >> 4);
    let bitrate_kbps = match (version, layer) {
        (3, 3) => MPEG1_LAYER1[bitrate_index],
        (3, 2) => MPEG1_LAYER2[bitrate_index],
        (3, 1) => MPEG1_LAYER3[bitrate_index],
        (_, 3) => MPEG2_LAYER1[bitrate_index],
        (_, 1 | 2) => MPEG2_LAYER23[bitrate_index],
        _ => return None,
    };
    let sample_index = usize::from((data[2] >> 2) & 0x03);
    let sample_rate = [44_100usize, 48_000, 32_000].get(sample_index)?
        / match version {
            3 => 1,
            2 => 2,
            0 => 4,
            _ => return None,
        };
    let bitrate = bitrate_kbps.checked_mul(1_000)?;
    let padding = usize::from(data[2] & 0x02 != 0);
    match layer {
        3 => (12 * bitrate / sample_rate + padding).checked_mul(4),
        2 => Some(144 * bitrate / sample_rate + padding),
        1 if version == 3 => Some(144 * bitrate / sample_rate + padding),
        1 => Some(72 * bitrate / sample_rate + padding),
        _ => None,
    }
}

fn valid_mp3_frame_header(data: &[u8]) -> bool {
    mp3_frame_length(data).is_some_and(|length| length <= data.len())
}

fn apev2_start(data: &[u8], audio_start: usize, end: usize) -> Result<Option<usize>> {
    if end < 32 || &data[end - 32..end - 24] != b"APETAGEX" {
        return Ok(None);
    }
    let footer = end - 32;
    let version = u32::from_le_bytes(data[footer + 8..footer + 12].try_into().unwrap());
    let size = u32::from_le_bytes(data[footer + 12..footer + 16].try_into().unwrap()) as usize;
    let item_count =
        u32::from_le_bytes(data[footer + 16..footer + 20].try_into().unwrap()) as usize;
    let flags = u32::from_le_bytes(data[footer + 20..footer + 24].try_into().unwrap());
    if version != 2000
        || !(32..=end.saturating_sub(audio_start)).contains(&size)
        || flags & !0x8000_0000 != 0
        || data[footer + 24..end].iter().any(|byte| *byte != 0)
    {
        return Err(CleanError::InvalidFormat("APEv2 标签尾无效".into()));
    }
    let items_start = end - size;
    if item_count > (footer - items_start) / 11 {
        return Err(CleanError::InvalidFormat("APEv2 项目计数无效".into()));
    }

    let mut cursor = items_start;
    for _ in 0..item_count {
        let item_header = cursor
            .checked_add(8)
            .filter(|value| *value <= footer)
            .ok_or_else(|| CleanError::InvalidFormat("APEv2 项目头越界".into()))?;
        let value_size = u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        let item_flags = u32::from_le_bytes(data[cursor + 4..item_header].try_into().unwrap());
        if item_flags & !0x7 != 0 || item_flags & 0x6 == 0x6 {
            return Err(CleanError::InvalidFormat("APEv2 项目标志无效".into()));
        }
        let key_end = data[item_header..footer]
            .iter()
            .position(|byte| *byte == 0)
            .map(|position| item_header + position)
            .ok_or_else(|| CleanError::InvalidFormat("APEv2 项目键缺少结束符".into()))?;
        let key = &data[item_header..key_end];
        if !(2..=255).contains(&key.len())
            || key
                .iter()
                .any(|byte| !(0x20..=0x7e).contains(byte) || *byte == b'=')
        {
            return Err(CleanError::InvalidFormat("APEv2 项目键无效".into()));
        }
        cursor = key_end
            .checked_add(1)
            .and_then(|value| value.checked_add(value_size))
            .filter(|value| *value <= footer)
            .ok_or_else(|| CleanError::InvalidFormat("APEv2 项目值越界".into()))?;
    }
    if cursor != footer {
        return Err(CleanError::InvalidFormat(
            "APEv2 项目长度与标签不匹配".into(),
        ));
    }

    if flags & 0x8000_0000 == 0 {
        return Ok(Some(items_start));
    }
    let header = items_start
        .checked_sub(32)
        .filter(|value| *value >= audio_start)
        .ok_or_else(|| CleanError::InvalidFormat("APEv2 声明了不存在的标签头".into()))?;
    let header_flags = u32::from_le_bytes(data[header + 20..header + 24].try_into().unwrap());
    if &data[header..header + 8] != b"APETAGEX"
        || data[header + 8..header + 20] != data[footer + 8..footer + 20]
        || header_flags != flags | 0x2000_0000
        || data[header + 24..header + 32].iter().any(|byte| *byte != 0)
    {
        return Err(CleanError::InvalidFormat(
            "APEv2 标签头与标签尾不匹配".into(),
        ));
    }
    Ok(Some(header))
}

fn mp3_metadata_bounds(data: &[u8]) -> Result<(usize, usize, usize)> {
    let mut start = 0;
    let mut count = 0;
    if data.starts_with(b"ID3") {
        start = id3v2_end(data)?;
        count += 1;
    }
    let mut end = data.len();
    if end >= 128 && &data[end - 128..end - 125] == b"TAG" {
        end -= 128;
        count += 1;
    }
    if let Some(tag_start) = apev2_start(data, start, end)? {
        end = tag_start;
        count += 1;
    }
    let audio = data
        .get(start..end)
        .ok_or_else(|| CleanError::InvalidFormat("MP3 音频范围无效".into()))?;
    if !valid_mp3_frame_header(audio) {
        return Err(CleanError::InvalidFormat("MP3 缺少有效音频帧".into()));
    }
    Ok((start, end, count))
}

pub fn is_mp3(data: &[u8]) -> bool {
    mp3_metadata_bounds(data).is_ok()
}

pub fn inspect_mp3(data: &[u8]) -> Result<Vec<Finding>> {
    let (_, _, count) = mp3_metadata_bounds(data)?;
    Ok(finding("audio_metadata", "ID3 / APE 音频元数据", count))
}

pub fn clean_mp3(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let (start, end, count) = mp3_metadata_bounds(data)?;
    Ok((
        data[start..end].to_vec(),
        finding("audio_metadata", "ID3 / APE 音频元数据", count),
    ))
}

fn wav_chunks(data: &[u8]) -> Result<Vec<([u8; 4], Range<usize>)>> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(CleanError::InvalidFormat("不是有效 WAV".into()));
    }
    let declared = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize + 8;
    if declared != data.len() {
        return Err(CleanError::InvalidFormat("WAV RIFF 长度不匹配".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 12;
    while offset + 8 <= declared {
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8 + length + (length & 1))
            .filter(|value| *value <= declared)
            .ok_or_else(|| CleanError::InvalidFormat("WAV 数据块越界".into()))?;
        chunks.push((data[offset..offset + 4].try_into().unwrap(), offset..end));
        offset = end;
    }
    if offset != declared
        || !chunks.iter().any(|(kind, _)| kind == b"fmt ")
        || !chunks.iter().any(|(kind, _)| kind == b"data")
    {
        return Err(CleanError::InvalidFormat("WAV 缺少 fmt 或 data 块".into()));
    }
    Ok(chunks)
}

fn private_wav_chunk(kind: &[u8; 4], bytes: &[u8]) -> bool {
    matches!(
        kind,
        b"C2PA" | b"ID3 " | b"id3 " | b"XMP " | b"bext" | b"iXML" | b"axml" | b"cart" | b"DISP"
    ) || (kind == b"LIST" && bytes.get(8..12) == Some(b"INFO"))
}

pub fn inspect_wav(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = wav_chunks(data)?;
    let count = chunks
        .iter()
        .filter(|(kind, range)| kind != b"C2PA" && private_wav_chunk(kind, &data[range.clone()]))
        .count();
    let mut findings = finding("audio_metadata", "WAV INFO / XMP / 广播元数据", count);
    findings.extend(provenance_finding(
        "WAV C2PA 来源标记",
        chunks.iter().filter(|(kind, _)| kind == b"C2PA").count(),
    ));
    Ok(findings)
}

pub fn clean_wav(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let chunks = wav_chunks(data)?;
    let findings = inspect_wav(data)?;
    let mut output = data[..12].to_vec();
    for (kind, range) in chunks {
        if !private_wav_chunk(&kind, &data[range.clone()]) {
            output.extend_from_slice(&data[range]);
        }
    }
    let size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&size.to_le_bytes());
    Ok((output, findings))
}

fn aiff_chunks(data: &[u8]) -> Result<Vec<([u8; 4], Range<usize>)>> {
    if data.len() < 12 || &data[..4] != b"FORM" || !matches!(&data[8..12], b"AIFF" | b"AIFC") {
        return Err(CleanError::InvalidFormat("不是有效 AIFF".into()));
    }
    let declared = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize + 8;
    if declared != data.len() {
        return Err(CleanError::InvalidFormat("AIFF FORM 长度不匹配".into()));
    }
    let mut chunks = Vec::new();
    let mut offset = 12;
    while offset + 8 <= declared {
        let length = u32::from_be_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .and_then(|value| value.checked_add(length & 1))
            .filter(|value| *value <= declared)
            .ok_or_else(|| CleanError::InvalidFormat("AIFF 数据块越界".into()))?;
        chunks.push((data[offset..offset + 4].try_into().unwrap(), offset..end));
        offset = end;
    }
    let common: Vec<_> = chunks.iter().filter(|(kind, _)| kind == b"COMM").collect();
    let sound: Vec<_> = chunks.iter().filter(|(kind, _)| kind == b"SSND").collect();
    let common_minimum = if &data[8..12] == b"AIFC" { 23 } else { 18 };
    if offset != declared || common.len() != 1 || sound.len() != 1 {
        return Err(CleanError::InvalidFormat(
            "AIFF 必须且只能包含一个 COMM 和 SSND 块".into(),
        ));
    }
    let common = &common[0].1;
    let common_length =
        u32::from_be_bytes(data[common.start + 4..common.start + 8].try_into().unwrap()) as usize;
    if common_length < common_minimum
        || u16::from_be_bytes(
            data[common.start + 8..common.start + 10]
                .try_into()
                .unwrap(),
        ) == 0
    {
        return Err(CleanError::InvalidFormat("AIFF COMM 块无效".into()));
    }
    let sound = &sound[0].1;
    let sound_length =
        u32::from_be_bytes(data[sound.start + 4..sound.start + 8].try_into().unwrap()) as usize;
    if sound_length < 8 {
        return Err(CleanError::InvalidFormat("AIFF SSND 块无效".into()));
    }
    let sample_offset =
        u32::from_be_bytes(data[sound.start + 8..sound.start + 12].try_into().unwrap()) as usize;
    if sample_offset > sound_length - 8 {
        return Err(CleanError::InvalidFormat("AIFF SSND 音频偏移越界".into()));
    }
    if &data[8..12] == b"AIFC" {
        let valid_version = chunks.iter().filter(|(kind, _)| kind == b"FVER").count() == 1
            && chunks.iter().any(|(kind, range)| {
                kind == b"FVER"
                    && u32::from_be_bytes(
                        data[range.start + 4..range.start + 8].try_into().unwrap(),
                    ) == 4
                    && data.get(range.start + 8..range.start + 12)
                        == Some(&[0xa2, 0x80, 0x51, 0x40])
            });
        if !valid_version {
            return Err(CleanError::InvalidFormat(
                "AIFC 缺少有效 FVER 版本块".into(),
            ));
        }
    }
    Ok(chunks)
}

fn private_aiff_chunk(kind: &[u8; 4]) -> bool {
    matches!(
        kind,
        b"NAME" | b"AUTH" | b"(c) " | b"ANNO" | b"COMT" | b"APPL" | b"ID3 " | b"XMP " | b"C2PA"
    )
}

pub fn is_aiff(data: &[u8]) -> bool {
    aiff_chunks(data).is_ok()
}

pub fn inspect_aiff(data: &[u8]) -> Result<Vec<Finding>> {
    let chunks = aiff_chunks(data)?;
    let count = chunks
        .iter()
        .filter(|(kind, _)| kind != b"C2PA" && private_aiff_chunk(kind))
        .count();
    let mut findings = finding(
        "audio_metadata",
        "AIFF 名称 / 作者 / 注释 / ID3 元数据",
        count,
    );
    findings.extend(provenance_finding(
        "AIFF C2PA 来源标记",
        chunks.iter().filter(|(kind, _)| kind == b"C2PA").count(),
    ));
    Ok(findings)
}

pub fn clean_aiff(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let chunks = aiff_chunks(data)?;
    let findings = inspect_aiff(data)?;
    let mut output = data[..12].to_vec();
    for (kind, range) in chunks {
        if !private_aiff_chunk(&kind) {
            output.extend_from_slice(&data[range]);
        }
    }
    let size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&size.to_be_bytes());
    Ok((output, findings))
}

type FlacBlock = (u8, Range<usize>, Range<usize>);
type FlacBlocks = (Vec<FlacBlock>, usize, usize);

fn flac_start(data: &[u8]) -> Result<usize> {
    if data.starts_with(b"fLaC") {
        return Ok(0);
    }
    if !data.starts_with(b"ID3") {
        return Err(CleanError::InvalidFormat("不是有效 FLAC".into()));
    }
    let start = id3v2_end(data)?;
    let start = data
        .get(start..)
        .filter(|tail| tail.starts_with(b"fLaC"))
        .map(|_| start)
        .ok_or_else(|| CleanError::InvalidFormat("FLAC 的 ID3v2 前缀无效".into()))?;
    Ok(start)
}

pub fn is_flac(data: &[u8]) -> bool {
    flac_start(data).is_ok()
}

fn flac_blocks(data: &[u8]) -> Result<FlacBlocks> {
    let prefix = flac_start(data)?;
    let mut blocks = Vec::new();
    let mut offset = prefix + 4;
    loop {
        let header = *data
            .get(offset)
            .ok_or_else(|| CleanError::InvalidFormat("FLAC 元数据头缺失".into()))?;
        let kind = header & 0x7f;
        if kind == 0x7f {
            return Err(CleanError::InvalidFormat("FLAC 元数据类型无效".into()));
        }
        let length_bytes = data
            .get(offset + 1..offset + 4)
            .ok_or_else(|| CleanError::InvalidFormat("FLAC 元数据长度缺失".into()))?;
        let length = (usize::from(length_bytes[0]) << 16)
            | (usize::from(length_bytes[1]) << 8)
            | usize::from(length_bytes[2]);
        let end = offset
            .checked_add(4 + length)
            .filter(|value| *value <= data.len())
            .ok_or_else(|| CleanError::InvalidFormat("FLAC 元数据块越界".into()))?;
        blocks.push((kind, offset..end, offset + 4..end));
        offset = end;
        if header & 0x80 != 0 {
            break;
        }
    }
    if blocks
        .first()
        .is_none_or(|(kind, range, _)| *kind != 0 || range.len() != 38)
    {
        return Err(CleanError::InvalidFormat("FLAC 缺少有效 STREAMINFO".into()));
    }
    if data.len() < offset + 2 || data[offset] != 0xff || data[offset + 1] & 0xfe != 0xf8 {
        return Err(CleanError::InvalidFormat("FLAC 缺少音频帧".into()));
    }
    Ok((blocks, offset, prefix))
}

fn private_flac_block(kind: u8, payload: &[u8]) -> bool {
    matches!(kind, 4 | 6) || (kind == 2 && payload.starts_with(b"XMP "))
}

pub fn inspect_flac(data: &[u8]) -> Result<Vec<Finding>> {
    let (blocks, _, prefix) = flac_blocks(data)?;
    let count = blocks
        .iter()
        .filter(|(kind, _, payload)| private_flac_block(*kind, &data[payload.clone()]))
        .count()
        + usize::from(prefix > 0);
    Ok(finding(
        "audio_metadata",
        "FLAC ID3 / C2PA / 评论 / 封面 / XMP",
        count,
    ))
}

pub fn clean_flac(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let (blocks, audio_offset, prefix) = flac_blocks(data)?;
    let kept: Vec<_> = blocks
        .iter()
        .filter(|(kind, _, payload)| !private_flac_block(*kind, &data[payload.clone()]))
        .collect();
    let removed = blocks.len() - kept.len() + usize::from(prefix > 0);
    let mut output = b"fLaC".to_vec();
    for (index, (kind, range, _)) in kept.iter().enumerate() {
        let start = output.len();
        output.extend_from_slice(&data[range.clone()]);
        output[start] = *kind | if index + 1 == kept.len() { 0x80 } else { 0 };
    }
    output.extend_from_slice(&data[audio_offset..]);
    Ok((
        output,
        finding(
            "audio_metadata",
            "FLAC ID3 / C2PA / 评论 / 封面 / XMP",
            removed,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp3_audio() -> Vec<u8> {
        let mut frame = vec![0xff, 0xfb, 0x90, 0x64];
        frame.resize(417, 0);
        frame
    }

    #[test]
    fn removes_gif_comments_without_touching_image_data() {
        let mut source = b"GIF89a\x01\0\x01\0\0\0\0".to_vec();
        source.extend_from_slice(b"\x21\xfe\x06author\0");
        source.extend_from_slice(b"\x2c\0\0\0\0\x01\0\x01\0\0\x02\x02D\x01\0\x3b");
        let (cleaned, findings) = clean_gif(&source).unwrap();
        assert_eq!(findings[0].count, 1);
        assert!(!cleaned.windows(6).any(|bytes| bytes == b"author"));
        assert!(cleaned.ends_with(b"\x02\x02D\x01\0\x3b"));
    }

    #[test]
    fn removes_the_standard_c2pa_gif_application_extension() {
        let mut source = b"GIF89a\x01\0\x01\0\0\0\0".to_vec();
        source.extend_from_slice(b"\x21\xff\x0bC2PA_GIF\x01\0\0\x08manifest\0");
        source.extend_from_slice(b"\x2c\0\0\0\0\x01\0\x01\0\0\x02\x02D\x01\0\x3b");

        let findings = inspect_gif(&source).unwrap();
        assert_eq!(findings[0].category, "provenance");
        assert_eq!(findings[0].severity, FindingSeverity::Provenance);
        let (cleaned, _) = clean_gif(&source).unwrap();
        assert!(!cleaned.windows(8).any(|window| window == b"C2PA_GIF"));
        assert!(inspect_gif(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn removes_id3v2_id3v1_and_ape_tags() {
        let mut source = b"ID3\x04\0\0\0\0\0\x03tag".to_vec();
        source.extend(mp3_audio());
        let mut ape = vec![0u8; 32];
        ape[..8].copy_from_slice(b"APETAGEX");
        ape[8..12].copy_from_slice(&2000u32.to_le_bytes());
        ape[12..16].copy_from_slice(&32u32.to_le_bytes());
        source.extend(ape);
        let mut id3v1 = vec![0u8; 128];
        id3v1[..3].copy_from_slice(b"TAG");
        source.extend(id3v1);
        let (cleaned, findings) = clean_mp3(&source).unwrap();
        assert_eq!(cleaned, mp3_audio());
        assert_eq!(findings[0].count, 3);
    }

    #[test]
    fn validates_ape_items_and_matching_optional_header() {
        let mut item = 5u32.to_le_bytes().to_vec();
        item.extend_from_slice(&0u32.to_le_bytes());
        item.extend_from_slice(b"Title\0Alice");
        let size = (item.len() + 32) as u32;

        let mut footer = vec![0u8; 32];
        footer[..8].copy_from_slice(b"APETAGEX");
        footer[8..12].copy_from_slice(&2000u32.to_le_bytes());
        footer[12..16].copy_from_slice(&size.to_le_bytes());
        footer[16..20].copy_from_slice(&1u32.to_le_bytes());
        footer[20..24].copy_from_slice(&0x8000_0000u32.to_le_bytes());

        let mut header = footer.clone();
        header[20..24].copy_from_slice(&0xa000_0000u32.to_le_bytes());
        let mut source = mp3_audio();
        source.extend(header);
        source.extend(item);
        source.extend(footer);

        let (cleaned, findings) = clean_mp3(&source).unwrap();
        assert_eq!(cleaned, mp3_audio());
        assert_eq!(findings[0].count, 1);
    }

    #[test]
    fn validates_id3v24_footer_before_removing_it() {
        let mut source = b"ID3\x04\0\x10\0\0\0\0".to_vec();
        source.extend_from_slice(b"3DI\x04\0\x10\0\0\0\0");
        source.extend(mp3_audio());
        assert_eq!(clean_mp3(&source).unwrap().0, mp3_audio());

        source[10] = b'X';
        assert!(inspect_mp3(&source).is_err());
    }

    #[test]
    fn rejects_mp3_false_positives_and_invalid_metadata_headers() {
        assert!(!is_mp3(b"\xff\xfb"));
        assert!(!is_mp3(b"\xff\xfb\x90\x64audio"));
        assert!(!is_mp3(b"ID3\x04\0\x01\0\0\0\0\xff\xfb\x90\x64"));

        let mut source = mp3_audio();
        let mut fake_ape = vec![0u8; 32];
        fake_ape[..8].copy_from_slice(b"APETAGEX");
        fake_ape[8..12].copy_from_slice(&1000u32.to_le_bytes());
        fake_ape[12..16].copy_from_slice(&32u32.to_le_bytes());
        source.extend(fake_ape);
        assert!(inspect_mp3(&source).is_err());

        let mut impossible_items = mp3_audio();
        let mut footer = vec![0u8; 32];
        footer[..8].copy_from_slice(b"APETAGEX");
        footer[8..12].copy_from_slice(&2000u32.to_le_bytes());
        footer[12..16].copy_from_slice(&32u32.to_le_bytes());
        footer[16..20].copy_from_slice(&1u32.to_le_bytes());
        impossible_items.extend(footer);
        assert!(inspect_mp3(&impossible_items).is_err());
    }

    fn wav_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut chunk = kind.to_vec();
        chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        chunk.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            chunk.push(0);
        }
        chunk
    }

    #[test]
    fn removes_wav_metadata_and_repairs_riff_size() {
        let mut source = b"RIFF\0\0\0\0WAVE".to_vec();
        source.extend(wav_chunk(b"fmt ", &[1, 0, 1, 0]));
        source.extend(wav_chunk(b"LIST", b"INFOIARTAlice"));
        source.extend(wav_chunk(b"C2PA", b"manifest"));
        source.extend(wav_chunk(b"data", &[1, 2, 3, 4]));
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_le_bytes());
        let (cleaned, findings) = clean_wav(&source).unwrap();
        assert_eq!(
            findings.iter().map(|finding| finding.count).sum::<usize>(),
            2
        );
        assert!(findings.iter().any(|finding| {
            finding.category == "provenance" && finding.severity == FindingSeverity::Provenance
        }));
        assert!(!cleaned.windows(4).any(|bytes| bytes == b"LIST"));
        assert!(!cleaned.windows(4).any(|bytes| bytes == b"C2PA"));
        assert_eq!(
            u32::from_le_bytes(cleaned[4..8].try_into().unwrap()) as usize + 8,
            cleaned.len()
        );
    }

    #[test]
    fn removes_aiff_metadata_without_reencoding_audio() {
        fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut output = kind.to_vec();
            output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            output.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                output.push(0);
            }
            output
        }

        let mut source = b"FORM\0\0\0\0AIFF".to_vec();
        let mut common = [0; 18];
        common[1] = 1;
        source.extend(chunk(b"COMM", &common));
        source.extend(chunk(b"NAME", b"private recording"));
        source.extend(chunk(b"AUTH", b"Alice"));
        source.extend(chunk(b"SSND", b"\0\0\0\0\0\0\0\0AUDIO-SAMPLES"));
        let size = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&size.to_be_bytes());

        assert!(is_aiff(&source));
        assert_eq!(inspect_aiff(&source).unwrap()[0].count, 2);
        let (cleaned, findings) = clean_aiff(&source).unwrap();
        assert_eq!(findings[0].count, 2);
        assert!(inspect_aiff(&cleaned).unwrap().is_empty());
        assert!(cleaned.ends_with(b"AUDIO-SAMPLES\0"));
        assert_eq!(
            u32::from_be_bytes(cleaned[4..8].try_into().unwrap()) as usize + 8,
            cleaned.len()
        );
    }

    #[test]
    fn rejects_malformed_aiff_containers() {
        assert!(!is_aiff(b"FORM\0\0\0\x04AIFF"));
        assert!(clean_aiff(b"FORM\0\0\0\x04AIFF").is_err());

        let mut source = b"FORM\0\0\0\0AIFF".to_vec();
        source.extend_from_slice(b"COMM\0\0\0\x12\0\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
        source.extend_from_slice(b"SSND\0\0\0\x08\0\0\0\x01\0\0\0\0");
        let declared = (source.len() - 8) as u32;
        source[4..8].copy_from_slice(&declared.to_be_bytes());
        assert!(clean_aiff(&source).is_err());
    }

    #[test]
    fn rejects_truncated_media_containers() {
        assert!(inspect_gif(b"GIF89a").is_err());
        assert!(inspect_mp3(b"ID3\x04").is_err());
        assert!(inspect_wav(b"RIFF\0\0\0\0WAVE").is_err());
        assert!(inspect_flac(b"fLaC").is_err());
    }

    fn flac_block(kind: u8, last: bool, payload: &[u8]) -> Vec<u8> {
        let mut block = vec![kind | if last { 0x80 } else { 0 }];
        let length = payload.len() as u32;
        block.extend_from_slice(&length.to_be_bytes()[1..]);
        block.extend_from_slice(payload);
        block
    }

    #[test]
    fn removes_flac_comments_and_pictures_without_reencoding_audio() {
        let mut source = b"fLaC".to_vec();
        source.extend(flac_block(0, false, &[0; 34]));
        source.extend(flac_block(4, false, b"artist=Alice"));
        source.extend(flac_block(6, true, b"cover"));
        source.extend_from_slice(b"\xff\xf8audio");
        let (cleaned, findings) = clean_flac(&source).unwrap();
        assert_eq!(findings[0].count, 2);
        assert_eq!(cleaned[4], 0x80);
        assert!(cleaned.ends_with(b"\xff\xf8audio"));
        assert!(!cleaned.windows(6).any(|bytes| bytes == b"artist"));
    }

    #[test]
    fn removes_flac_id3_c2pa_prefix_without_touching_audio() {
        let geob = b"GEOB\0application/c2pa\0manifest";
        let mut source = b"ID3\x04\0\0".to_vec();
        source.extend_from_slice(&[0, 0, 0, geob.len() as u8]);
        source.extend_from_slice(geob);
        source.extend_from_slice(b"fLaC");
        source.extend(flac_block(0, true, &[0; 34]));
        source.extend_from_slice(b"\xff\xf8audio");

        assert!(is_flac(&source));
        let (cleaned, findings) = clean_flac(&source).unwrap();
        assert_eq!(findings[0].count, 1);
        assert!(cleaned.starts_with(b"fLaC"));
        assert!(cleaned.ends_with(b"\xff\xf8audio"));
        assert!(!cleaned.windows(4).any(|bytes| bytes == b"C2PA"));
    }

    #[test]
    fn rejects_invalid_flac_id3_headers_and_missing_frame_sync() {
        let mut invalid_id3 = b"ID3\x04\0\x01\0\0\0\0fLaC".to_vec();
        invalid_id3.extend(flac_block(0, true, &[0; 34]));
        invalid_id3.extend_from_slice(b"\xff\xf8audio");
        assert!(inspect_flac(&invalid_id3).is_err());

        let mut missing_sync = b"fLaC".to_vec();
        missing_sync.extend(flac_block(0, true, &[0; 34]));
        missing_sync.extend_from_slice(b"not audio");
        assert!(inspect_flac(&missing_sync).is_err());
    }
}
