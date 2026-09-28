use crate::models::{Finding, FindingSeverity};
use std::borrow::Cow;

fn is_private_use(code: u32) -> bool {
    matches!(code, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

fn is_noncharacter(code: u32) -> bool {
    matches!(code, 0xFDD0..=0xFDEF) || code & 0xFFFE == 0xFFFE
}

fn is_reserved_ignorable(code: u32) -> bool {
    matches!(
        code,
        0x2065 | 0xE0000 | 0xFFF0..=0xFFF8 | 0xE0080..=0xE00FF | 0xE01F0..=0xE0FFF
    )
}

fn is_layout_control(code: u32) -> bool {
    matches!(code, 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A)
}

fn is_invisible(code: u32) -> bool {
    matches!(code,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 |
        0x180B..=0x180F | 0x200B..=0x200F | 0x202A..=0x202E |
        0x2060..=0x206F | 0xFE00..=0xFE0F | 0xFEFF | 0xFFF9..=0xFFFB |
        0x3164 | 0xFFA0 | 0xE0000..=0xE007F | 0xE0100..=0xE01EF
    ) || is_private_use(code)
        || is_noncharacter(code)
        || is_reserved_ignorable(code)
        || is_layout_control(code)
}

fn space_replacement(code: u32) -> bool {
    matches!(
        code,
        0x00A0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

fn emoji_glue(chars: &[char], index: usize) -> bool {
    let current = chars[index] as u32;
    if !matches!(current, 0x200D | 0xFE0E | 0xFE0F) || index == 0 {
        return false;
    }
    let previous = chars[index - 1] as u32;
    let is_emoji_base = |code: u32| {
        matches!(code,
            0x2190..=0x27BF | 0x2B00..=0x2BFF | 0x1F000..=0x1FAFF |
            0x0023 | 0x002A | 0x0030..=0x0039 | 0x00A9 | 0x00AE |
            0x203C | 0x2049 | 0x2122 | 0x2139 | 0x2934 | 0x2935 |
            0x3030 | 0x303D | 0x3297 | 0x3299
        )
    };
    is_emoji_base(previous)
        || (current == 0x200D
            && index + 1 < chars.len()
            && (is_emoji_base(chars[index + 1] as u32) || previous == 0xFE0F))
}

fn script_glue(chars: &[char], index: usize) -> bool {
    let current = chars[index] as u32;
    let in_range = |code: u32, start: u32, end: u32| (start..=end).contains(&code);
    if index > 0 && index + 1 < chars.len() && matches!(current, 0x200C | 0x200D) {
        let previous = chars[index - 1] as u32;
        let next = chars[index + 1] as u32;
        return [(0x0590, 0x08FF), (0x0900, 0x0DFF), (0x1780, 0x18AF)]
            .iter()
            .any(|&(start, end)| in_range(previous, start, end) && in_range(next, start, end));
    }
    if index == 0 || index + 1 >= chars.len() {
        return false;
    }
    let previous = chars[index - 1] as u32;
    let next = chars[index + 1] as u32;
    match current {
        0x180B..=0x180F => in_range(previous, 0x1800, 0x18AF) || in_range(next, 0x1800, 0x18AF),
        0x17B4 | 0x17B5 => in_range(previous, 0x1780, 0x17FF) || in_range(next, 0x1780, 0x17FF),
        0x115F | 0x1160 => in_range(previous, 0x1100, 0x11FF) || in_range(next, 0x1100, 0x11FF),
        0x3164 => in_range(previous, 0x3131, 0x318E) || in_range(next, 0x3131, 0x318E),
        0xFFA0 => in_range(previous, 0xFFA1, 0xFFDC) || in_range(next, 0xFFA1, 0xFFDC),
        0x13430..=0x1343F => {
            in_range(previous, 0x13000, 0x143FF) || in_range(next, 0x13000, 0x143FF)
        }
        0x1BCA0..=0x1BCA3 => {
            in_range(previous, 0x1BC00, 0x1BCA3) || in_range(next, 0x1BC00, 0x1BCA3)
        }
        0x1D173..=0x1D17A => {
            in_range(previous, 0x1D100, 0x1D1FF) || in_range(next, 0x1D100, 0x1D1FF)
        }
        _ => false,
    }
}

fn cjk_variation(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let current = chars[index] as u32;
    if !matches!(current, 0xFE00..=0xFE0D | 0xE0100..=0xE01EF) {
        return false;
    }
    matches!(chars[index - 1] as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

fn paired_control_indices(value: &str) -> Vec<bool> {
    if !value.chars().any(|character| {
        matches!(
            character as u32,
            0x1F3F4 | 0x202A | 0x202B | 0x202D | 0x202E
        )
    }) {
        return Vec::new();
    }
    let mut preserved = vec![false; value.chars().count()];
    let mut flag = None;
    let mut embeddings: Vec<(u32, usize)> = Vec::new();
    for (index, character) in value.chars().enumerate() {
        match character as u32 {
            0x1F3F4 => flag = Some(index),
            0xE0020..=0xE007E => {}
            0xE007F => {
                if let Some(start) = flag.take() {
                    if index > start + 1 {
                        preserved[start + 1..=index].fill(true);
                    }
                }
            }
            _ => flag = None,
        }
        match character as u32 {
            code @ (0x202A | 0x202B | 0x202D | 0x202E) => embeddings.push((code, index)),
            0x202C => {
                if let Some((opener, start)) = embeddings.pop() {
                    if matches!(opener, 0x202A | 0x202B) {
                        preserved[start] = true;
                        preserved[index] = true;
                    }
                }
            }
            _ => {}
        }
    }
    preserved
}

fn preserve_invisible(chars: &[char], index: usize, paired: bool) -> bool {
    let code = chars[index] as u32;
    paired
        || matches!(code, 0x061C | 0x200E | 0x200F | 0x2066..=0x2069)
        || emoji_glue(chars, index)
        || script_glue(chars, index)
        || cjk_variation(chars, index)
}

// Keep only the original neighbors. Removing a character must not change the
// context used to classify the following character.
fn classified_characters<'a>(
    value: &'a str,
    paired: &'a [bool],
) -> impl Iterator<Item = (usize, char, bool)> + 'a {
    let mut characters = value.char_indices().enumerate().peekable();
    let mut previous = None;
    std::iter::from_fn(move || {
        let (index, (offset, character)) = characters.next()?;
        let next = characters.peek().map(|(_, (_, character))| *character);
        let window = [previous.unwrap_or('\0'), character, next.unwrap_or('\0')];
        let start = usize::from(previous.is_none());
        let end = if next.is_some() { 3 } else { 2 };
        let remove = !character.is_ascii()
            && is_invisible(character as u32)
            && !preserve_invisible(
                &window[start..end],
                1 - start,
                paired.get(index).copied().unwrap_or(false),
            );
        previous = Some(character);
        Some((offset, character, remove))
    })
}

fn findings_for(value: &str, paired: &[bool]) -> Vec<Finding> {
    let mut invisible = 0;
    let mut spaces = 0;
    for (_, character, remove) in classified_characters(value, paired) {
        let code = character as u32;
        if remove {
            invisible += 1;
        } else if !character.is_ascii() && space_replacement(code) {
            spaces += 1;
        }
    }
    let mut findings = Vec::new();
    if invisible > 0 {
        findings.push(Finding {
            category: "unicode".into(),
            label: "不可见 Unicode 字符".into(),
            count: invisible,
            severity: FindingSeverity::Privacy,
        });
    }
    if spaces > 0 {
        findings.push(Finding {
            category: "unicode_space".into(),
            label: "异常空白字符".into(),
            count: spaces,
            severity: FindingSeverity::Informational,
        });
    }
    findings
}

pub fn clean_cow(value: &str) -> (Cow<'_, str>, Vec<Finding>) {
    if value.is_ascii() {
        return (Cow::Borrowed(value), Vec::new());
    }
    let paired = paired_control_indices(value);
    let findings = findings_for(value, &paired);
    if findings.is_empty() {
        return (Cow::Borrowed(value), findings);
    }
    let mut output = String::with_capacity(value.len());
    let mut copied_until = 0;
    for (offset, character, remove) in classified_characters(value, &paired) {
        let replace_space = !character.is_ascii() && space_replacement(character as u32);
        if remove || replace_space {
            output.push_str(&value[copied_until..offset]);
            if replace_space {
                output.push(' ');
            }
            copied_until = offset + character.len_utf8();
        }
    }
    output.push_str(&value[copied_until..]);
    (Cow::Owned(output), findings)
}

#[cfg(test)]
pub fn clean(value: &str) -> (String, Vec<Finding>) {
    let (output, findings) = clean_cow(value);
    (output.into_owned(), findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_invisible_and_normalizes_spaces() {
        let (cleaned, findings) = clean("a\u{200b}b\u{00a0}c");
        assert_eq!(cleaned, "ab c");
        assert_eq!(findings.iter().map(|item| item.count).sum::<usize>(), 2);
    }

    #[test]
    fn cleans_a_long_single_line_without_truncation() {
        let mut source = "a".repeat(1024 * 1024);
        source.insert(source.len() / 2, '\u{200b}');
        let (cleaned, findings) = clean(&source);
        assert_eq!(cleaned.len(), 1024 * 1024);
        assert!(cleaned.bytes().all(|byte| byte == b'a'));
        assert_eq!(findings.iter().map(|item| item.count).sum::<usize>(), 1);
    }

    #[test]
    fn preserves_emoji_joiners() {
        let source = "👨\u{200d}👩\u{200d}👧";
        assert_eq!(clean(source).0, source);
    }

    #[test]
    fn preserves_emoji_variation_selectors() {
        for source in ["❤️", "✈️"] {
            assert_eq!(clean(source).0, source);
        }
    }

    #[test]
    fn preserves_joiners_inside_complex_scripts() {
        let source = "می\u{200c}روم";
        assert_eq!(clean(source).0, source);
    }

    #[test]
    fn strips_private_use_characters() {
        assert_eq!(clean("a\u{e000}b").0, "ab");
    }

    #[test]
    fn strips_reserved_ignorables_and_all_noncharacters() {
        for code in [
            0x2065, 0xFFF0, 0xFFF8, 0xE0000, 0xE0080, 0xE00FF, 0xE01F0, 0xE0FFF,
        ] {
            let source = format!("a{}b", char::from_u32(code).unwrap());
            assert_eq!(clean(&source).0, "ab", "U+{code:04X}");
        }
        let noncharacters = (0xFDD0..=0xFDEF)
            .chain((0..=0x10).flat_map(|plane| [plane << 16 | 0xFFFE, plane << 16 | 0xFFFF]));
        assert_eq!(noncharacters.count(), 66);
        for code in (0xFDD0..=0xFDEF)
            .chain((0..=0x10).flat_map(|plane| [plane << 16 | 0xFFFE, plane << 16 | 0xFFFF]))
        {
            let source = format!("a{}b", char::from_u32(code).unwrap());
            assert_eq!(clean(&source).0, "ab", "U+{code:04X}");
        }
    }

    #[test]
    fn preserves_contextual_script_controls_and_strips_floating_ones() {
        for source in [
            "\u{1820}\u{180f}\u{1821}",
            "\u{3131}\u{3164}\u{314f}",
            "\u{ffa1}\u{ffa0}\u{ffc2}",
            "\u{13079}\u{13430}\u{130a7}",
            "\u{1bc02}\u{1bca0}\u{1bc03}",
            "\u{1d158}\u{1d173}\u{1d158}",
        ] {
            assert_eq!(clean(source).0, source);
        }
        for code in [0x180F, 0x3164, 0xFFA0, 0x13430, 0x1BCA0, 0x1D173] {
            let source = format!("a{}b", char::from_u32(code).unwrap());
            assert_eq!(clean(&source).0, "ab", "U+{code:04X}");
        }
    }

    #[test]
    fn preserves_complete_flags_cjk_variants_and_directional_text() {
        for source in [
            "\u{1f3f4}\u{e0067}\u{e0062}\u{e007f}",
            "\u{4e00}\u{e0100}",
            "\u{2067}مرحبا\u{2069}",
            "\u{202b}مرحبا\u{202c}",
            "↔️",
        ] {
            assert_eq!(clean(source).0, source);
        }
        assert_eq!(clean("a\u{e0067}b").0, "ab");
        assert_eq!(clean("a\u{202e}b").0, "ab");
    }

    #[test]
    fn preserves_original_neighbors_when_removing_adjacent_controls() {
        assert_eq!(clean("❤\u{200b}\u{fe0f}").0, "❤");
        assert_eq!(clean("一\u{200b}\u{e0100}").0, "一");
        assert_eq!(clean("\u{180f}\u{1820}").0, "\u{1820}");
        assert_eq!(clean("\u{1820}\u{180f}").0, "\u{1820}");
        assert_eq!(clean("❤\u{fe0f}").0, "❤\u{fe0f}");
        assert_eq!(clean("\u{fe0f}❤").0, "❤");
    }

    #[test]
    fn pairs_nested_embeddings_without_preserving_overrides_or_orphans() {
        let source = "\u{202a}a\u{202e}b\u{202c}c\u{202c}\u{202c}";
        let (output, findings) = clean(source);
        assert_eq!(output, "\u{202a}abc\u{202c}");
        assert_eq!(findings[0].count, 3);
        assert_eq!(clean("\u{202a}a\u{202b}b\u{202c}").0, "a\u{202b}b\u{202c}");
    }

    #[test]
    fn rejects_incomplete_flag_tags_and_keeps_later_complete_flags() {
        let flag = "\u{1f3f4}";
        let tag = "\u{e0067}";
        let end = "\u{e007f}";
        for source in [
            format!("{flag}{end}"),
            format!("{flag}{tag}"),
            format!("{flag}{tag}a{end}"),
        ] {
            let expected = if source.contains('a') {
                format!("{flag}a")
            } else {
                flag.to_owned()
            };
            assert_eq!(clean(&source).0, expected);
        }
        let source = format!("{flag}{tag}{flag}{tag}{end}");
        assert_eq!(clean(&source).0, format!("{flag}{flag}{tag}{end}"));
        assert!(matches!(
            clean_cow(&format!("{flag}{tag}{end}")).0,
            Cow::Borrowed(_)
        ));
    }
}
