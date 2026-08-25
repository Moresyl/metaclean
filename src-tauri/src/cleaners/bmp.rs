//! BMP has no metadata standard, which is exactly why it leaks. The two
//! reserved words in the file header are scratch space that editors write ID
//! numbers into, a version 5 header can carry a whole embedded ICC profile, and
//! because the format never says the file ends at the last pixel, plenty of
//! tools staple EXIF or XMP onto the tail where no viewer will ever show it.

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const FILE_HEADER: usize = 14;
const RESERVED: usize = 6;
const OFF_BITS: usize = 10;
const MIN_DIB: usize = 12;
/// `bV5CSType` when the profile is embedded in the file rather than named.
const PROFILE_EMBEDDED: u32 = 0x4d42_4544;
/// `LCS_sRGB`, the neutral colour space to fall back to once it is gone.
const PROFILE_SRGB: u32 = 0x7352_4742;

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(message.into())
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(at..at + 4)
            .ok_or_else(|| invalid("BMP 头部越界"))?
            .try_into()
            .unwrap(),
    ))
}

pub fn is_bmp(data: &[u8]) -> bool {
    layout(data).is_ok()
}

#[derive(Debug)]
struct Layout {
    /// Where the pixels stop and the appended junk begins.
    pixels_end: usize,
    profile: Option<(usize, usize)>,
    reserved: bool,
}

fn layout(data: &[u8]) -> Result<Layout> {
    if data.len() < FILE_HEADER + MIN_DIB || !data.starts_with(b"BM") {
        return Err(invalid("不是有效 BMP"));
    }
    let off_bits = u32_at(data, OFF_BITS)? as usize;
    let dib = u32_at(data, FILE_HEADER)? as usize;
    if dib < MIN_DIB
        || FILE_HEADER + dib > data.len()
        || off_bits < FILE_HEADER + dib
        || off_bits > data.len()
    {
        return Err(invalid("BMP 信息头无效"));
    }

    // A twelve byte core header keeps its dimensions in sixteen bit fields;
    // every later version widened them to thirty two.
    let (width, height, planes, bits, compression, declared) = if dib == MIN_DIB {
        let read = |at: usize| -> Result<i32> {
            Ok(u16::from_le_bytes(
                data.get(at..at + 2)
                    .ok_or_else(|| invalid("BMP 信息头越界"))?
                    .try_into()
                    .unwrap(),
            ) as i32)
        };
        (
            read(FILE_HEADER + 4)?,
            read(FILE_HEADER + 6)?,
            u16::from_le_bytes(data[FILE_HEADER + 8..FILE_HEADER + 10].try_into().unwrap()),
            u32::from(u16::from_le_bytes(
                data[FILE_HEADER + 10..FILE_HEADER + 12].try_into().unwrap(),
            )),
            0,
            0usize,
        )
    } else {
        (
            u32_at(data, FILE_HEADER + 4)? as i32,
            u32_at(data, FILE_HEADER + 8)? as i32,
            u16::from_le_bytes(
                data.get(FILE_HEADER + 12..FILE_HEADER + 14)
                    .ok_or_else(|| invalid("BMP 信息头越界"))?
                    .try_into()
                    .unwrap(),
            ),
            u32::from(u16::from_le_bytes(
                data.get(FILE_HEADER + 14..FILE_HEADER + 16)
                    .ok_or_else(|| invalid("BMP 信息头越界"))?
                    .try_into()
                    .unwrap(),
            )),
            u32_at(data, FILE_HEADER + 16)?,
            u32_at(data, FILE_HEADER + 20)? as usize,
        )
    };
    if width <= 0
        || height == 0
        || planes != 1
        || !(matches!(bits, 1 | 4 | 8 | 16 | 24 | 32) || bits == 0 && matches!(compression, 4 | 5))
        || !matches!(compression, 0..=6)
        || (compression == 1 && bits != 8)
        || (compression == 2 && bits != 4)
        || (matches!(compression, 3 | 6) && !matches!(bits, 16 | 32))
        || (matches!(compression, 1 | 2) && (declared == 0 || height < 0))
        || (matches!(compression, 4 | 5) && declared == 0)
    {
        return Err(invalid("BMP 尺寸、位深或压缩信息无效"));
    }

    let rows = height.unsigned_abs() as usize;
    let stride = (u64::from(width.unsigned_abs())
        .saturating_mul(u64::from(bits))
        .saturating_add(31)
        / 32)
        .saturating_mul(4) as usize;
    // A compressed bitmap has no predictable stride, so trust its own count.
    let measured_pixels = stride
        .checked_mul(rows)
        .ok_or_else(|| invalid("BMP 像素尺寸溢出"))?;
    let pixels = if matches!(compression, 1 | 2 | 4 | 5) {
        declared
    } else {
        measured_pixels
    };
    let pixels_end = off_bits
        .checked_add(pixels)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| invalid("BMP 像素数据越界"))?;

    // Only the version 4 and 5 headers describe a colour profile, and only
    // version 5 can embed one; the offset is measured from the info header.
    let profile = (dib >= 124)
        .then(|| -> Result<Option<(usize, usize)>> {
            if u32_at(data, FILE_HEADER + 56)? != PROFILE_EMBEDDED {
                return Ok(None);
            }
            let at = u32_at(data, FILE_HEADER + 112)? as usize;
            let size = u32_at(data, FILE_HEADER + 116)? as usize;
            let start = FILE_HEADER + at;
            let end = start
                .checked_add(size)
                .filter(|end| size > 0 && *end <= data.len())
                .ok_or_else(|| invalid("BMP ICC 配置文件越界"))?;
            if start < FILE_HEADER + dib || start < pixels_end && off_bits < end {
                return Err(invalid("BMP ICC 配置文件与结构或像素重叠"));
            }
            Ok(Some((start, size)))
        })
        .transpose()?
        .flatten();

    Ok(Layout {
        pixels_end,
        profile,
        reserved: data[RESERVED..RESERVED + 4] != [0, 0, 0, 0],
    })
}

/// Everything past the pixels that is not the colour profile we agreed to keep.
fn trailing(data: &[u8], layout: &Layout, keep_profile: bool) -> usize {
    let end = match layout.profile {
        Some((start, size)) if keep_profile => layout.pixels_end.max(start + size),
        _ => layout.pixels_end,
    };
    data.len().saturating_sub(end)
}

fn findings(data: &[u8], layout: &Layout) -> Vec<Finding> {
    let mut findings = Vec::new();
    let appended = trailing(data, layout, true);
    let private_profile_gap = layout.profile.is_some_and(|(start, _)| {
        start > layout.pixels_end && data[layout.pixels_end..start].iter().any(|byte| *byte != 0)
    });
    let count =
        usize::from(appended > 0) + usize::from(layout.reserved) + usize::from(private_profile_gap);
    if count > 0 {
        findings.push(Finding {
            category: "image_metadata".into(),
            label: "BMP 保留字段与尾部附加数据".into(),
            count,
            severity: FindingSeverity::Privacy,
        });
    }
    if layout.profile.is_some() {
        findings.push(Finding {
            category: "color_profile".into(),
            label: "ICC 色彩配置文件".into(),
            count: 1,
            severity: FindingSeverity::Informational,
        });
    }
    findings
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let layout = layout(data)?;
    Ok(findings(data, &layout))
}

pub fn clean(data: &[u8], preserve_color_profile: bool) -> Result<(Vec<u8>, Vec<Finding>)> {
    let layout = layout(data)?;
    let removed = findings(data, &layout);
    let keep = preserve_color_profile && layout.profile.is_some();
    let end = data.len() - trailing(data, &layout, keep);
    let mut output = data[..end].to_vec();
    output[RESERVED..RESERVED + 4].fill(0);
    if let Some((start, size)) = layout.profile {
        if !keep {
            if start + size <= output.len() {
                output[start..start + size].fill(0);
            }
            output[FILE_HEADER + 56..FILE_HEADER + 60].copy_from_slice(&PROFILE_SRGB.to_le_bytes());
            output[FILE_HEADER + 112..FILE_HEADER + 120].fill(0);
        } else if start > layout.pixels_end {
            output[layout.pixels_end..start].fill(0);
        }
    }
    let size = u32::try_from(output.len()).map_err(|_| invalid("BMP 体积超出格式上限"))?;
    output[2..6].copy_from_slice(&size.to_le_bytes());
    Ok((output, removed))
}

pub fn verify_cleaned(data: &[u8], preserve_color_profile: bool) -> Result<()> {
    let layout = layout(data)?;
    if trailing(data, &layout, preserve_color_profile) > 0 || layout.reserved {
        return Err(CleanError::Verification("BMP 中仍残留附加数据".into()));
    }
    if preserve_color_profile
        && layout.profile.is_some_and(|(start, _)| {
            start > layout.pixels_end
                && data[layout.pixels_end..start].iter().any(|byte| *byte != 0)
        })
    {
        return Err(CleanError::Verification(
            "BMP ICC 前仍存在非零尾部间隙".into(),
        ));
    }
    if layout.profile.is_some() != preserve_color_profile && layout.profile.is_some() {
        return Err(CleanError::Verification(
            "BMP 中仍残留 ICC 色彩配置文件".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains(data: &[u8], needle: &[u8]) -> bool {
        data.windows(needle.len()).any(|window| window == needle)
    }

    /// Four pixels of twenty four bit colour behind a plain info header.
    fn sample() -> Vec<u8> {
        let pixels = vec![0x11u8; 4 * 4];
        let mut dib = 40u32.to_le_bytes().to_vec();
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.resize(40, 0);
        let off_bits = (FILE_HEADER + dib.len()) as u32;
        let mut file = b"BM".to_vec();
        file.extend_from_slice(&((off_bits as usize + pixels.len()) as u32).to_le_bytes());
        file.extend_from_slice(&[0, 0, 0, 0]);
        file.extend_from_slice(&off_bits.to_le_bytes());
        file.extend(dib);
        file.extend(pixels);
        file
    }

    #[test]
    fn drops_metadata_stapled_to_the_tail() {
        let mut source = sample();
        source.extend_from_slice(b"Exif\0\0alice@example.test");
        assert!(is_bmp(&source));
        assert!(!inspect(&source).unwrap().is_empty());

        let (cleaned, removed) = clean(&source, true).unwrap();
        assert_eq!(removed[0].count, 1);
        assert_eq!(cleaned.len(), sample().len());
        assert!(!contains(&cleaned, b"alice@example.test"));
        assert_eq!(u32_at(&cleaned, 2).unwrap() as usize, cleaned.len());
        assert!(inspect(&cleaned).unwrap().is_empty());
        verify_cleaned(&cleaned, true).unwrap();
    }

    #[test]
    fn clears_the_reserved_words_editors_scribble_in() {
        let mut source = sample();
        source[RESERVED..RESERVED + 4].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(inspect(&source).unwrap()[0].count, 1);
        let (cleaned, _) = clean(&source, true).unwrap();
        assert_eq!(&cleaned[RESERVED..RESERVED + 4], &[0, 0, 0, 0]);
        verify_cleaned(&cleaned, true).unwrap();
    }

    #[test]
    fn honours_the_colour_profile_choice_for_version_five_headers() {
        let pixels = vec![0x22u8; 4 * 4];
        let private_gap = b"PRIVATE-GAP";
        let profile = b"ICC-PROFILE-BODY".to_vec();
        let mut dib = 124u32.to_le_bytes().to_vec();
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.resize(124, 0);
        dib[56..60].copy_from_slice(&PROFILE_EMBEDDED.to_le_bytes());
        let off_bits = FILE_HEADER + dib.len();
        // The profile offset is measured from the start of the info header.
        let profile_at = (dib.len() + pixels.len() + private_gap.len()) as u32;
        dib[112..116].copy_from_slice(&profile_at.to_le_bytes());
        dib[116..120].copy_from_slice(&(profile.len() as u32).to_le_bytes());
        let mut source = b"BM".to_vec();
        source.extend_from_slice(&0u32.to_le_bytes());
        source.extend_from_slice(&[0, 0, 0, 0]);
        source.extend_from_slice(&(off_bits as u32).to_le_bytes());
        source.extend(dib);
        source.extend(pixels);
        source.extend(private_gap);
        source.extend(profile);

        assert!(inspect(&source)
            .unwrap()
            .iter()
            .any(|finding| finding.category == "color_profile"));

        let (kept, _) = clean(&source, true).unwrap();
        assert!(contains(&kept, b"ICC-PROFILE-BODY"));
        assert!(!contains(&kept, private_gap));
        verify_cleaned(&kept, true).unwrap();

        let (stripped, _) = clean(&source, false).unwrap();
        assert!(!contains(&stripped, b"ICC-PROFILE-BODY"));
        assert_eq!(u32_at(&stripped, FILE_HEADER + 56).unwrap(), PROFILE_SRGB);
        verify_cleaned(&stripped, false).unwrap();
    }

    #[test]
    fn rejects_files_it_cannot_measure() {
        assert!(inspect(b"BM").is_err());
        assert!(inspect(b"\x89PNG\r\n\x1a\n").is_err());
        assert!(!is_bmp(b"II\x2a\x00\x08\0\0\0"));
    }

    #[test]
    fn rejects_headers_that_could_make_cleanup_truncate_structure() {
        let mut invalid_offset = sample();
        invalid_offset[OFF_BITS..OFF_BITS + 4].copy_from_slice(&(FILE_HEADER as u32).to_le_bytes());
        assert!(inspect(&invalid_offset).is_err());

        let mut invalid_planes = sample();
        invalid_planes[FILE_HEADER + 12..FILE_HEADER + 14].fill(0);
        assert!(inspect(&invalid_planes).is_err());

        let mut unmeasured_compressed = sample();
        unmeasured_compressed[FILE_HEADER + 16..FILE_HEADER + 20]
            .copy_from_slice(&1u32.to_le_bytes());
        assert!(inspect(&unmeasured_compressed).is_err());
    }
}
