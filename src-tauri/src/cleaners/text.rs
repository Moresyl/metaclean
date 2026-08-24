use crate::models::{Finding, FindingSeverity};

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

fn paired_control_indices(chars: &[char]) -> Vec<bool> {
    let mut preserved = vec![false; chars.len()];

    let mut flag = 0;
    while flag < chars.len() {
        if chars[flag] as u32 != 0x1F3F4 {
            flag += 1;
            continue;
        }
        let mut end = flag + 1;
        while end < chars.len() && matches!(chars[end] as u32, 0xE0020..=0xE007E) {
            end += 1;
        }
        if end > flag + 1 && end < chars.len() && chars[end] as u32 == 0xE007F {
            preserved[flag + 1..=end].fill(true);
            flag = end + 1;
        } else {
            flag += 1;
        }
    }

    let mut embeddings: Vec<(u32, usize)> = Vec::new();
    for (index, character) in chars.iter().enumerate() {
        match *character as u32 {
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

fn preserve_invisible(chars: &[char], index: usize, paired: &[bool]) -> bool {
    let code = chars[index] as u32;
    paired[index]
        || matches!(code, 0x061C | 0x200E | 0x200F | 0x2066..=0x2069)
        || emoji_glue(chars, index)
        || script_glue(chars, index)
        || cjk_variation(chars, index)
}

pub fn inspect(value: &str) -> Vec<Finding> {
    let chars: Vec<char> = value.chars().collect();
    let paired = paired_control_indices(&chars);
    let mut invisible = 0;
    let mut spaces = 0;
    for (index, character) in chars.iter().enumerate() {
        let code = *character as u32;
        if is_invisible(code) && !preserve_invisible(&chars, index, &paired) {
            invisible += 1;
        } else if space_replacement(code) {
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

pub fn clean(value: &str) -> (String, Vec<Finding>) {
    let findings = inspect(value);
    let chars: Vec<char> = value.chars().collect();
    let paired = paired_control_indices(&chars);
    let output = chars
        .iter()
        .enumerate()
        .filter_map(|(index, character)| {
            let code = *character as u32;
            if is_invisible(code) && !preserve_invisible(&chars, index, &paired) {
                None
            } else if space_replacement(code) {
                Some(' ')
            } else {
                Some(*character)
            }
        })
        .collect();
    (output, findings)
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
}
