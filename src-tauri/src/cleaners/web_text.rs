use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use regex::Regex;
use std::sync::OnceLock;

use super::{bmp, heif, image, jxl, media, text};
use crate::models::{Finding, FindingSeverity};

const MAX_EMBEDDED_BYTES: usize = 16 * 1024 * 1024;
const MAX_EMBEDDED_DEPTH: usize = 4;

fn metadata_finding(count: usize) -> Finding {
    Finding {
        category: "document_metadata".into(),
        label: "作者 / 生成器 / AI 元数据".into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

fn html_patterns() -> &'static [Regex; 2] {
    static PATTERNS: OnceLock<[Regex; 2]> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            Regex::new(r#"(?is)<meta\b[^>]*(?:name|property)\s*=\s*["'](?:generator|author|ai[_-]?(?:generated|model)|c2pa)["'][^>]*>"#).unwrap(),
            Regex::new(r#"(?is)\sdata-(?:ai|llm|model|c2pa)[\w-]*\s*=\s*(?:"[^"]*"|'[^']*')"#).unwrap(),
        ]
    })
}

fn frontmatter(value: &str) -> Option<(usize, usize)> {
    if !value.starts_with("---\n") && !value.starts_with("---\r\n") {
        return None;
    }
    let start = value.find('\n')? + 1;
    let end = frontmatter_end_pattern().find(&value[start..])?.start() + start;
    Some((start, end))
}

fn frontmatter_end_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?m)^---[\t ]*\r?$").expect("frontmatter fence regex must compile")
    })
}

fn markdown_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?im)^\s*(?:generator|author|creator|last_modified_by|ai[_-]?(?:generated|model)|model|c2pa)\s*:.*(?:\r?\n|$)").unwrap()
    })
}

fn data_image_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"(?i)data:image/(?P<mime>[a-z0-9+.-]+)(?P<params>;[^\s\"')<>]+)?,(?P<payload>[A-Za-z0-9+/=\s%._~:-]+)"#,
        )
        .unwrap()
    })
}

fn svg_metadata_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?is)<metadata\b[^>]*>.*?</metadata\s*>")
            .expect("SVG metadata regex must compile")
    })
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_data_uri(payload: &str, base64: bool) -> Option<Vec<u8>> {
    let decoded = if base64 {
        let max_encoded = MAX_EMBEDDED_BYTES.div_ceil(3) * 4;
        let compact_length = payload
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .take(max_encoded + 1)
            .count();
        if compact_length > max_encoded {
            return None;
        }
        let mut compact = String::with_capacity(compact_length + 3);
        compact.extend(
            payload
                .chars()
                .filter(|character| !character.is_whitespace()),
        );
        while !compact.len().is_multiple_of(4) {
            compact.push('=');
        }
        BASE64.decode(compact).ok()?
    } else {
        if payload.len() > MAX_EMBEDDED_BYTES * 3 {
            return None;
        }
        let bytes = payload.as_bytes();
        let mut output = Vec::with_capacity(bytes.len().min(MAX_EMBEDDED_BYTES));
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                let high = decode_hex(*bytes.get(index + 1)?)?;
                let low = decode_hex(*bytes.get(index + 2)?)?;
                output.push((high << 4) | low);
                index += 3;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
            if output.len() > MAX_EMBEDDED_BYTES {
                return None;
            }
        }
        output
    };
    (!decoded.is_empty() && decoded.len() <= MAX_EMBEDDED_BYTES).then_some(decoded)
}

fn encode_data_uri(data: &[u8], base64: bool) -> String {
    if base64 {
        return BASE64.encode(data);
    }
    let mut output = String::with_capacity(data.len() * 3);
    for byte in data {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

fn privacy_count(findings: &[Finding]) -> usize {
    findings
        .iter()
        .filter(|finding| finding.severity != FindingSeverity::Informational)
        .map(|finding| finding.count)
        .sum()
}

fn first_non_whitespace(data: &[u8]) -> Option<u8> {
    data.iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
}

fn inspect_embedded_bytes(data: &[u8], mime: &str, depth: usize) -> usize {
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        image::inspect_jpeg(data).map_or(1, |items| privacy_count(&items))
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        image::inspect_png(data).map_or(1, |items| privacy_count(&items))
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        image::inspect_webp(data).map_or(1, |items| privacy_count(&items))
    } else if data.starts_with(b"\xff\x0a") || data.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n") {
        jxl::inspect(data).map_or(1, |items| privacy_count(&items))
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        media::inspect_gif(data).map_or(1, |items| privacy_count(&items))
    } else if data.starts_with(b"BM") {
        bmp::inspect(data).map_or(1, |items| privacy_count(&items))
    } else if heif::is_heif(data) {
        heif::inspect(data).map_or(1, |items| privacy_count(&items))
    } else if depth < MAX_EMBEDDED_DEPTH
        && (mime.contains("svg") || first_non_whitespace(data) == Some(b'<'))
    {
        std::str::from_utf8(data).map_or(1, |value| {
            inspect_with_depth(value, "svg", depth + 1)
                .iter()
                .map(|finding| finding.count)
                .sum()
        })
    } else {
        0
    }
}

fn clean_embedded_bytes(data: &[u8], mime: &str, depth: usize) -> Option<Vec<u8>> {
    let (cleaned, findings) = if data.starts_with(&[0xff, 0xd8, 0xff]) {
        image::clean_jpeg_with_options(data, true, true).ok()?
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        image::clean_png_with_options(data, true).ok()?
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        image::clean_webp_with_options(data, true).ok()?
    } else if jxl::is_jxl(data) {
        jxl::clean(data).ok()?
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        media::clean_gif(data).ok()?
    } else if bmp::is_bmp(data) {
        bmp::clean(data, true).ok()?
    } else if heif::is_heif(data) {
        heif::clean(data).ok()?
    } else if depth < MAX_EMBEDDED_DEPTH
        && (mime.contains("svg") || first_non_whitespace(data) == Some(b'<'))
    {
        let source = std::str::from_utf8(data).ok()?;
        let cleaned = clean_with_depth(source, "svg", depth + 1).0.into_bytes();
        return (cleaned != data).then_some(cleaned);
    } else {
        return None;
    };
    (privacy_count(&findings) > 0 && cleaned != data).then_some(cleaned)
}

fn inspect_embedded(value: &str, depth: usize) -> usize {
    data_image_pattern()
        .captures_iter(value)
        .filter_map(|capture| {
            let params = capture.name("params").map_or("", |value| value.as_str());
            let data = decode_data_uri(
                capture.name("payload")?.as_str(),
                params.to_ascii_lowercase().contains("base64"),
            )?;
            let count = inspect_embedded_bytes(&data, capture.name("mime")?.as_str(), depth);
            (count > 0).then_some(count)
        })
        .sum()
}

fn clean_embedded(value: &str, depth: usize) -> String {
    let pattern = data_image_pattern();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    for capture in pattern.captures_iter(value) {
        let whole = capture.get(0).unwrap();
        output.push_str(&value[cursor..whole.start()]);
        let params = capture.name("params").map_or("", |item| item.as_str());
        let encoded = capture.name("payload").unwrap().as_str();
        let base64 = params.to_ascii_lowercase().contains("base64");
        let replacement = decode_data_uri(encoded, base64)
            .and_then(|data| {
                clean_embedded_bytes(&data, capture.name("mime").unwrap().as_str(), depth)
            })
            .map(|cleaned| {
                format!(
                    "data:image/{}{params},{}",
                    capture.name("mime").unwrap().as_str(),
                    encode_data_uri(&cleaned, base64)
                )
            });
        output.push_str(replacement.as_deref().unwrap_or(whole.as_str()));
        cursor = whole.end();
    }
    output.push_str(&value[cursor..]);
    output
}

pub fn inspect(value: &str, extension: &str) -> Vec<Finding> {
    inspect_with_depth(value, extension, 0)
}

fn inspect_with_depth(value: &str, extension: &str, depth: usize) -> Vec<Finding> {
    let mut findings = text::inspect(value);
    let metadata = match extension {
        "html" | "htm" | "xhtml" => html_patterns()
            .iter()
            .map(|pattern| pattern.find_iter(value).count())
            .sum(),
        "svg" => svg_metadata_pattern().find_iter(value).count(),
        "md" | "markdown" => frontmatter(value)
            .map(|(start, end)| markdown_pattern().find_iter(&value[start..end]).count())
            .unwrap_or(0),
        _ => 0,
    };
    if metadata > 0 {
        findings.push(metadata_finding(metadata));
    }
    let embedded = inspect_embedded(value, depth);
    if embedded > 0 {
        findings.push(Finding {
            category: "embedded_image_metadata".into(),
            label: "嵌入图片元数据 / C2PA".into(),
            count: embedded,
            severity: FindingSeverity::Provenance,
        });
    }
    findings
}

pub fn clean(value: &str, extension: &str) -> (String, Vec<Finding>) {
    clean_with_depth(value, extension, 0)
}

fn clean_with_depth(value: &str, extension: &str, depth: usize) -> (String, Vec<Finding>) {
    let findings = inspect_with_depth(value, extension, depth);
    let mut output = value.to_owned();
    match extension {
        "html" | "htm" | "xhtml" => {
            for pattern in html_patterns() {
                output = pattern.replace_all(&output, "").into_owned();
            }
        }
        "svg" => output = svg_metadata_pattern().replace_all(&output, "").into_owned(),
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
    output = clean_embedded(&output, depth);
    let (output, _) = text::clean(&output);
    (output, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut output = (payload.len() as u32).to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output.extend_from_slice(&image::png_crc32(&output[4..]).to_be_bytes());
        output
    }

    fn private_png() -> Vec<u8> {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(png_chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]));
        png.extend(png_chunk(b"tEXt", b"Author\0Alice"));
        png.extend(png_chunk(b"IDAT", b"pixels"));
        png.extend(png_chunk(b"IEND", b""));
        png
    }

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

    #[test]
    fn does_not_treat_a_longer_yaml_value_as_the_closing_fence() {
        let source = "---\r\ntitle: Hello\r\n---not-a-fence\r\ngenerator: Claude\r\n---\r\nBody";
        let cleaned = clean(source, "md").0;
        assert!(cleaned.contains("---not-a-fence"));
        assert!(!cleaned.contains("Claude"));
        assert!(cleaned.ends_with("Body"));
    }

    #[test]
    fn cleans_metadata_inside_embedded_base64_images() {
        let png = private_png();
        let source = format!(
            "<img src=\"data:image/png;base64,{}\">",
            BASE64.encode(&png)
        );

        let findings = inspect(&source, "html");
        assert!(findings
            .iter()
            .any(|finding| finding.category == "embedded_image_metadata"));
        let cleaned = clean(&source, "html").0;
        assert_ne!(cleaned, source);
        let payload = data_image_pattern()
            .captures(&cleaned)
            .unwrap()
            .name("payload")
            .unwrap()
            .as_str();
        let decoded = BASE64.decode(payload).unwrap();
        assert!(!decoded.windows(4).any(|window| window == b"tEXt"));
        assert!(decoded.windows(4).any(|window| window == b"IEND"));
    }

    #[test]
    fn cleans_metadata_inside_embedded_jpeg_xl() {
        let jxl_box = |kind: &[u8; 4], payload: &[u8]| {
            let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
            bytes.extend_from_slice(kind);
            bytes.extend_from_slice(payload);
            bytes
        };
        let mut jxl = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
        jxl.extend(jxl_box(b"ftyp", b"jxl \0\0\0\0jxl "));
        jxl.extend(jxl_box(b"Exif", b"\0\0\0\0II*\0Alice"));
        jxl.extend(jxl_box(b"jxlc", b"\xff\x0aIMAGE-CODESTREAM"));
        let source = format!(
            "<img src=\"data:image/jxl;base64,{}\">",
            BASE64.encode(&jxl)
        );

        assert!(inspect(&source, "html")
            .iter()
            .any(|finding| finding.category == "embedded_image_metadata"));
        let cleaned = clean(&source, "html").0;
        let payload = data_image_pattern()
            .captures(&cleaned)
            .unwrap()
            .name("payload")
            .unwrap()
            .as_str();
        let decoded = BASE64.decode(payload).unwrap();
        assert_eq!(decoded.len(), jxl.len());
        assert!(!decoded.windows(4).any(|window| window == b"Exif"));
        jxl::verify_cleaned(&decoded).unwrap();
    }

    #[test]
    fn scans_and_cleans_embedded_images_after_the_hundredth_item() {
        let benign = "<img src=\"data:image/png;base64,AAAA\">";
        let mut source = benign.repeat(100);
        source.push_str(&format!(
            "<img src=\"data:image/png;base64,{}\">",
            BASE64.encode(private_png())
        ));

        assert!(inspect(&source, "html")
            .iter()
            .any(|finding| finding.category == "embedded_image_metadata"));
        let cleaned = clean(&source, "html").0;
        assert_ne!(cleaned, source);
        assert!(inspect(&cleaned, "html").is_empty());
    }

    #[test]
    fn rejects_embedded_payloads_that_exceed_the_decode_budget() {
        let oversized_base64 = "A".repeat(MAX_EMBEDDED_BYTES.div_ceil(3) * 4 + 1);
        assert!(decode_data_uri(&oversized_base64, true).is_none());

        let oversized_plain = "a".repeat(MAX_EMBEDDED_BYTES + 1);
        assert!(decode_data_uri(&oversized_plain, false).is_none());
    }

    #[test]
    fn treats_recognized_but_unverifiable_embedded_images_as_residual() {
        let mut corrupted = private_png();
        corrupted[20] ^= 1;
        let source = format!(
            "<img src=\"data:image/png;base64,{}\">",
            BASE64.encode(corrupted)
        );

        let findings = inspect(&source, "html");
        assert!(findings
            .iter()
            .any(|finding| finding.category == "embedded_image_metadata"));
        let cleaned = clean(&source, "html").0;
        assert_eq!(cleaned, source);
        assert!(!inspect(&cleaned, "html").is_empty());
    }
}
