use crate::models::{Finding, FindingSeverity};

fn is_private_use(code: u32) -> bool {
    matches!(code, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

fn is_invisible(code: u32) -> bool {
    matches!(code,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 |
        0x180B..=0x180E | 0x200B..=0x200F | 0x202A..=0x202E |
        0x2060..=0x206F | 0xFE00..=0xFE0F | 0xFEFF | 0xFFF9..=0xFFFB |
        0xE0000..=0xE007F | 0xE0100..=0xE01EF
    ) || is_private_use(code)
}

fn space_replacement(code: u32) -> bool {
    matches!(
        code,
        0x00A0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

fn emoji_glue(chars: &[char], index: usize) -> bool {
    let current = chars[index] as u32;
    if !matches!(current, 0x200D | 0xFE0F) || index == 0 {
        return false;
    }
    let previous = chars[index - 1] as u32;
    previous >= 0x1F000
        || (current == 0x200D && index + 1 < chars.len() && chars[index + 1] as u32 >= 0x1F000)
}

pub fn inspect(value: &str) -> Vec<Finding> {
    let chars: Vec<char> = value.chars().collect();
    let mut invisible = 0;
    let mut spaces = 0;
    for (index, character) in chars.iter().enumerate() {
        let code = *character as u32;
        if is_invisible(code) && !emoji_glue(&chars, index) {
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
    let output = chars
        .iter()
        .enumerate()
        .filter_map(|(index, character)| {
            let code = *character as u32;
            if is_invisible(code) && !emoji_glue(&chars, index) {
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
    fn strips_private_use_characters() {
        assert_eq!(clean("a\u{e000}b").0, "ab");
    }
}
