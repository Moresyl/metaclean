use regex::Regex;

use super::text;
use crate::models::{Finding, FindingSeverity};

fn metadata_finding(count: usize) -> Finding {
    Finding {
        category: "document_metadata".into(),
        label: "作者 / 生成器 / AI 元数据".into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

fn html_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r#"(?is)<meta\b[^>]*(?:name|property)\s*=\s*["'](?:generator|author|ai[_-]?(?:generated|model)|c2pa)["'][^>]*>"#).unwrap(),
        Regex::new(r#"(?is)\sdata-(?:ai|llm|model|c2pa)[\w-]*\s*=\s*(?:"[^"]*"|'[^']*')"#).unwrap(),
    ]
}

fn frontmatter(value: &str) -> Option<(usize, usize)> {
    if !value.starts_with("---\n") && !value.starts_with("---\r\n") {
        return None;
    }
    let start = value.find('\n')? + 1;
    let end = value[start..].find("\n---")? + start;
    Some((start, end))
}

fn markdown_pattern() -> Regex {
    Regex::new(r"(?im)^\s*(?:generator|author|creator|last_modified_by|ai[_-]?(?:generated|model)|model|c2pa)\s*:.*(?:\r?\n|$)").unwrap()
}

pub fn inspect(value: &str, extension: &str) -> Vec<Finding> {
    let mut findings = text::inspect(value);
    let metadata = match extension {
        "html" | "htm" | "xhtml" => html_patterns()
            .iter()
            .map(|pattern| pattern.find_iter(value).count())
            .sum(),
        "svg" => Regex::new(r"(?is)<metadata\b[^>]*>.*?</metadata\s*>")
            .unwrap()
            .find_iter(value)
            .count(),
        "md" | "markdown" => frontmatter(value)
            .map(|(start, end)| markdown_pattern().find_iter(&value[start..end]).count())
            .unwrap_or(0),
        _ => 0,
    };
    if metadata > 0 {
        findings.push(metadata_finding(metadata));
    }
    findings
}

pub fn clean(value: &str, extension: &str) -> (String, Vec<Finding>) {
    let findings = inspect(value, extension);
    let mut output = value.to_owned();
    match extension {
        "html" | "htm" | "xhtml" => {
            for pattern in html_patterns() {
                output = pattern.replace_all(&output, "").into_owned();
            }
        }
        "svg" => {
            output = Regex::new(r"(?is)<metadata\b[^>]*>.*?</metadata\s*>")
                .unwrap()
                .replace_all(&output, "")
                .into_owned()
        }
        "md" | "markdown" => {
            if let Some((start, end)) = frontmatter(&output) {
                let cleaned = markdown_pattern()
                    .replace_all(&output[start..end], "")
                    .into_owned();
                output.replace_range(start..end, &cleaned);
            }
        }
        _ => {}
    }
    let (output, _) = text::clean(&output);
    (output, findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removes_html_generator_and_ai_attributes() {
        let source =
            r#"<meta name="generator" content="ChatGPT"><body data-ai-model="gpt">ok</body>"#;
        let (cleaned, findings) = clean(source, "html");
        assert!(!cleaned.contains("ChatGPT"));
        assert!(!cleaned.contains("data-ai"));
        assert_eq!(findings[0].count, 2);
        assert!(!clean(source, "xhtml").0.contains("ChatGPT"));
    }
    #[test]
    fn removes_svg_metadata_but_preserves_drawing() {
        assert_eq!(
            clean("<svg><metadata>author</metadata><circle/></svg>", "svg").0,
            "<svg><circle/></svg>"
        );
    }
    #[test]
    fn removes_sensitive_frontmatter_only() {
        let cleaned = clean("---\ntitle: Hello\ngenerator: Claude\n---\nBody", "md").0;
        assert!(cleaned.contains("title: Hello"));
        assert!(!cleaned.contains("Claude"));
    }
}
