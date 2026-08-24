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

fn gif_private_ranges(data: &[u8]) -> Result<Vec<Range<usize>>> {
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
                if label == 0xfe || xmp_application {
                    private.push(start..offset);
                }
            }
            _ => return Err(CleanError::InvalidFormat("GIF 块标记无效".into())),
        }
    }
}

pub fn inspect_gif(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(finding(
        "image_metadata",
        "GIF 注释 / XMP 元数据",
        gif_private_ranges(data)?.len(),
    ))
}

pub fn clean_gif(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let ranges = gif_private_ranges(data)?;
    let findings = finding("image_metadata", "GIF 注释 / XMP 元数据", ranges.len());
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for range in ranges {
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

fn mp3_metadata_bounds(data: &[u8]) -> Result<(usize, usize, usize)> {
    let mut start = 0;
    let mut count = 0;
    if data.starts_with(b"ID3") {
        if data.len() < 10 {
            return Err(CleanError::InvalidFormat("ID3v2 头部不完整".into()));
        }
        let footer = if data[5] & 0x10 != 0 { 10 } else { 0 };
        start = 10usize
            .checked_add(synchsafe_size(&data[6..10])?)
            .and_then(|value| value.checked_add(footer))
            .filter(|value| *value <= data.len())
            .ok_or_else(|| CleanError::InvalidFormat("ID3v2 标签越界".into()))?;
        count += 1;
    }
    let mut end = data.len();
    if end >= 128 && &data[end - 128..end - 125] == b"TAG" {
        end -= 128;
        count += 1;
    }
    if end >= 32 && &data[end - 32..end - 24] == b"APETAGEX" {
        let size = u32::from_le_bytes(data[end - 20..end - 16].try_into().unwrap()) as usize;
        if !(32..=end - start).contains(&size) {
            return Err(CleanError::InvalidFormat("APEv2 标签长度无效".into()));
        }
        end -= size;
        count += 1;
    }
    let audio = data
        .get(start..end)
        .ok_or_else(|| CleanError::InvalidFormat("MP3 音频范围无效".into()))?;
    if audio.len() < 2 || audio[0] != 0xff || audio[1] & 0xe0 != 0xe0 {
        return Err(CleanError::InvalidFormat("MP3 缺少有效音频帧".into()));
    }
    Ok((start, end, count))
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
    let count = wav_chunks(data)?
        .iter()
        .filter(|(kind, range)| private_wav_chunk(kind, &data[range.clone()]))
        .count();
    Ok(finding(
        "audio_metadata",
        "WAV INFO / XMP / 广播元数据",
        count,
    ))
}

pub fn clean_wav(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let chunks = wav_chunks(data)?;
    let count = chunks
        .iter()
        .filter(|(kind, range)| private_wav_chunk(kind, &data[range.clone()]))
        .count();
    let mut output = data[..12].to_vec();
    for (kind, range) in chunks {
        if !private_wav_chunk(&kind, &data[range.clone()]) {
            output.extend_from_slice(&data[range]);
        }
    }
    let size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&size.to_le_bytes());
    Ok((
        output,
        finding("audio_metadata", "WAV INFO / XMP / 广播元数据", count),
    ))
}

type FlacBlock = (u8, Range<usize>, Range<usize>);
type FlacBlocks = (Vec<FlacBlock>, usize, usize);

fn flac_start(data: &[u8]) -> Result<usize> {
    if data.starts_with(b"fLaC") {
        return Ok(0);
    }
    if !data.starts_with(b"ID3") || data.len() < 10 {
        return Err(CleanError::InvalidFormat("不是有效 FLAC".into()));
    }
    let footer = if data[5] & 0x10 != 0 { 10 } else { 0 };
    let start = 10usize
        .checked_add(synchsafe_size(&data[6..10])?)
        .and_then(|value| value.checked_add(footer))
        .filter(|value| {
            data.get(*value..)
                .is_some_and(|tail| tail.starts_with(b"fLaC"))
        })
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
    if data.len() < offset + 2 {
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
    fn removes_id3v2_id3v1_and_ape_tags() {
        let mut source = b"ID3\x04\0\0\0\0\0\x03tag\xff\xfb\x90\x64audio".to_vec();
        let mut ape = vec![0u8; 32];
        ape[..8].copy_from_slice(b"APETAGEX");
        ape[12..16].copy_from_slice(&32u32.to_le_bytes());
        source.extend(ape);
        let mut id3v1 = vec![0u8; 128];
        id3v1[..3].copy_from_slice(b"TAG");
        source.extend(id3v1);
        let (cleaned, findings) = clean_mp3(&source).unwrap();
        assert_eq!(cleaned, b"\xff\xfb\x90\x64audio");
        assert_eq!(findings[0].count, 3);
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
        assert_eq!(findings[0].count, 2);
        assert!(!cleaned.windows(4).any(|bytes| bytes == b"LIST"));
        assert!(!cleaned.windows(4).any(|bytes| bytes == b"C2PA"));
        assert_eq!(
            u32::from_le_bytes(cleaned[4..8].try_into().unwrap()) as usize + 8,
            cleaned.len()
        );
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
}
