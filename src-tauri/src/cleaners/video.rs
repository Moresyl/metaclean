use std::ops::Range;

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_ATOM_DEPTH: usize = 32;
const ADOBE_XMP_UUID: [u8; 16] = [
    0xbe, 0x7a, 0xcf, 0xcb, 0x97, 0xa9, 0x42, 0xe8, 0x9c, 0x71, 0x99, 0x94, 0x91, 0xe3, 0xaf, 0xac,
];

#[derive(Debug, Clone)]
struct Atom {
    kind: [u8; 4],
    range: Range<usize>,
    header_len: usize,
}

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

fn parse_atom(data: &[u8], offset: usize, parent_end: usize) -> Result<Atom> {
    let header_end = offset
        .checked_add(8)
        .filter(|end| *end <= parent_end)
        .ok_or_else(|| invalid("MP4 原子头不完整"))?;
    let header = data
        .get(offset..header_end)
        .ok_or_else(|| invalid("MP4 原子头不完整"))?;
    let short_size = u32::from_be_bytes(header[..4].try_into().unwrap());
    let kind = header[4..8].try_into().unwrap();
    let (size, header_len) = match short_size {
        0 => (parent_end - offset, 8),
        1 => {
            let extended_end = offset
                .checked_add(16)
                .filter(|end| *end <= parent_end)
                .ok_or_else(|| invalid("MP4 扩展原子长度缺失"))?;
            let extended = data
                .get(header_end..extended_end)
                .ok_or_else(|| invalid("MP4 扩展原子长度缺失"))?;
            let size = usize::try_from(u64::from_be_bytes(extended.try_into().unwrap()))
                .map_err(|_| invalid("MP4 原子长度超出平台范围"))?;
            (size, 16)
        }
        value => (value as usize, 8),
    };
    let end = offset
        .checked_add(size)
        .filter(|end| size >= header_len && *end <= parent_end)
        .ok_or_else(|| invalid("MP4 原子长度越界"))?;
    if kind == *b"uuid" && size < header_len + 16 {
        return Err(invalid("MP4 UUID 原子缺少标识符"));
    }
    Ok(Atom {
        kind,
        range: offset..end,
        header_len,
    })
}

fn supported_brand(brand: &[u8]) -> bool {
    matches!(
        brand,
        b"isom"
            | b"iso2"
            | b"iso4"
            | b"iso5"
            | b"iso6"
            | b"mp41"
            | b"mp42"
            | b"avc1"
            | b"dash"
            | b"qt  "
            | b"M4A "
            | b"M4V "
            | b"MSNV"
            | b"F4A "
            | b"F4B "
            | b"F4P "
            | b"F4V "
            | b"3gp1"
            | b"3gp2"
            | b"3gp3"
            | b"3gp4"
            | b"3gp5"
            | b"3gp6"
            | b"3gp7"
            | b"3gp8"
            | b"3gp9"
            | b"3g2a"
            | b"3g2b"
    )
}

pub fn is_iso_media(data: &[u8]) -> bool {
    let Ok(atom) = parse_atom(data, 0, data.len()) else {
        return false;
    };
    if atom.kind != *b"ftyp" || atom.range.len() < atom.header_len + 8 {
        return false;
    }
    let payload = &data[atom.range.start + atom.header_len..atom.range.end];
    if !(payload.len() - 8).is_multiple_of(4) {
        return false;
    }
    supported_brand(&payload[..4])
        || payload
            .get(8..)
            .is_some_and(|brands| brands.chunks_exact(4).any(supported_brand))
}

fn container(kind: &[u8; 4]) -> bool {
    matches!(
        kind,
        b"moov"
            | b"trak"
            | b"mdia"
            | b"minf"
            | b"dinf"
            | b"edts"
            | b"moof"
            | b"traf"
            | b"mfra"
            | b"mvex"
            | b"tref"
    )
}

fn private_atom(atom: &Atom, data: &[u8]) -> bool {
    let kind = &atom.kind;
    if matches!(
        kind,
        b"udta"
            | b"meta"
            | b"ilst"
            | b"XMP_"
            | b"cprt"
            | b"loci"
            | b"name"
            | b"auth"
            | b"perf"
            | b"gnre"
            | b"albm"
            | b"yrrc"
            | b"titl"
            | b"dscp"
            | b"kywd"
            | b"rtng"
    ) || kind[0] == 0xa9
    {
        return true;
    }
    if kind != b"uuid" {
        return false;
    }
    let Some(user_type_start) = atom.range.start.checked_add(atom.header_len) else {
        return false;
    };
    let Some(user_type_end) = user_type_start.checked_add(ADOBE_XMP_UUID.len()) else {
        return false;
    };
    data.get(user_type_start..user_type_end) == Some(ADOBE_XMP_UUID.as_slice())
}

fn walk_children(
    data: &[u8],
    start: usize,
    end: usize,
    depth: usize,
    private: &mut Vec<Atom>,
) -> Result<()> {
    if depth > MAX_ATOM_DEPTH {
        return Err(invalid("MP4 原子嵌套超过 32 层安全上限"));
    }
    let mut offset = start;
    while offset < end {
        let atom = parse_atom(data, offset, end)?;
        if private_atom(&atom, data) {
            private.push(atom.clone());
        } else if container(&atom.kind) {
            walk_children(
                data,
                atom.range.start + atom.header_len,
                atom.range.end,
                depth + 1,
                private,
            )?;
        }
        offset = atom.range.end;
    }
    Ok(())
}

fn private_atoms(data: &[u8]) -> Result<Vec<Atom>> {
    if !is_iso_media(data) {
        return Err(invalid("不是受支持的 MP4 / QuickTime 容器"));
    }
    let mut private = Vec::new();
    let mut offset = 0;
    let mut has_moov = false;
    let mut has_mdat = false;
    while offset < data.len() {
        let atom = parse_atom(data, offset, data.len())?;
        has_moov |= atom.kind == *b"moov";
        has_mdat |= atom.kind == *b"mdat";
        if private_atom(&atom, data) {
            private.push(atom.clone());
        } else if container(&atom.kind) {
            walk_children(
                data,
                atom.range.start + atom.header_len,
                atom.range.end,
                1,
                &mut private,
            )?;
        }
        offset = atom.range.end;
    }
    if !has_moov || !has_mdat {
        return Err(invalid("MP4 缺少 moov 或 mdat 原子"));
    }
    Ok(private)
}

fn findings(count: usize) -> Vec<Finding> {
    if count == 0 {
        Vec::new()
    } else {
        vec![Finding {
            category: "video_metadata".into(),
            label: "MP4 / QuickTime 用户数据、XMP 与位置元数据".into(),
            count,
            severity: FindingSeverity::Privacy,
        }]
    }
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    Ok(findings(private_atoms(data)?.len()))
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let atoms = private_atoms(data)?;
    let mut output = data.to_vec();
    for atom in &atoms {
        output[atom.range.start + 4..atom.range.start + 8].copy_from_slice(b"free");
        output[atom.range.start + atom.header_len..atom.range.end].fill(0);
    }
    Ok((output, findings(atoms.len())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn sample() -> Vec<u8> {
        let ftyp = b"isom\0\0\0\0isommp42".to_vec();
        let metadata = atom(b"udta", b"author=Alice;location=Shanghai");
        let track_metadata = atom(b"meta", b"\0\0\0\0camera=iPhone");
        let track = atom(b"trak", &track_metadata);
        let mut moov_payload = atom(b"mvhd", &[0; 16]);
        moov_payload.extend(metadata);
        moov_payload.extend(track);
        let mut file = atom(b"ftyp", &ftyp);
        file.extend(atom(b"moov", &moov_payload));
        let mut uuid_payload = ADOBE_XMP_UUID.to_vec();
        uuid_payload.extend_from_slice(b"<xmp>creator@example.test</xmp>");
        file.extend(atom(b"uuid", &uuid_payload));
        file.extend(atom(b"mdat", b"\0\0\0\x01VIDEO-FRAMES"));
        file
    }

    #[test]
    fn removes_metadata_without_moving_media_bytes() {
        let source = sample();
        let (cleaned, removed) = clean(&source).unwrap();
        assert_eq!(removed[0].count, 3);
        assert_eq!(cleaned.len(), source.len());
        assert!(cleaned.ends_with(b"\0\0\0\x01VIDEO-FRAMES"));
        assert!(!cleaned.windows(5).any(|bytes| bytes == b"Alice"));
        assert!(!cleaned.windows(6).any(|bytes| bytes == b"camera"));
        assert!(!cleaned.windows(7).any(|bytes| bytes == b"creator"));
        assert!(inspect(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn accepts_video_brands_and_rejects_image_brands() {
        assert!(is_iso_media(&sample()));
        for brand in [
            b"F4A ", b"F4B ", b"F4P ", b"F4V ", b"3gp1", b"3gp2", b"3gp3", b"3gp4", b"3gp5",
            b"3gp6", b"3gp7", b"3gp8", b"3gp9", b"3g2a", b"3g2b",
        ] {
            let media = atom(
                b"ftyp",
                &[brand.as_slice(), b"\0\0\0\0", brand.as_slice()].concat(),
            );
            assert!(is_iso_media(&media), "{:?}", brand);
        }
        let heic = atom(b"ftyp", b"heic\0\0\0\0heicmif1");
        assert!(!is_iso_media(&heic));
        assert!(!is_iso_media(&atom(b"ftyp", b"isom\0\0\0\0x")));
    }

    #[test]
    fn rejects_truncated_or_incomplete_containers() {
        let mut truncated = atom(b"ftyp", b"isom\0\0\0\0isom");
        truncated.extend_from_slice(&100u32.to_be_bytes());
        truncated.extend_from_slice(b"moov");
        assert!(inspect(&truncated).is_err());
        assert!(inspect(&atom(b"ftyp", b"isom\0\0\0\0isom")).is_err());
    }
}
