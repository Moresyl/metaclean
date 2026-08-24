use std::io::{Cursor, Read, Write};

use regex::Regex;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED: u64 = 512 * 1024 * 1024;

fn privacy_finding(label: &str, count: usize) -> Finding {
    Finding {
        category: "office_metadata".into(),
        label: label.into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

fn read_archive(data: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>> {
    let archive = ZipArchive::new(Cursor::new(data))?;
    if archive.len() > MAX_ENTRIES {
        return Err(CleanError::InvalidFormat(
            "Office 文件包含过多 ZIP 条目".into(),
        ));
    }
    let total: u64 = archive.file_names().count() as u64;
    if total > MAX_ENTRIES as u64 {
        return Err(CleanError::InvalidFormat("Office ZIP 条目超限".into()));
    }
    Ok(archive)
}

fn sensitive_xml_count(xml: &str) -> usize {
    let names = [
        "creator",
        "lastModifiedBy",
        "created",
        "modified",
        "title",
        "subject",
        "keywords",
        "description",
        "Application",
        "Company",
        "Manager",
        "Template",
        "TotalTime",
        "trackRevisions",
        "generator",
        "editing-duration",
        "editing-cycles",
    ];
    names
        .iter()
        .filter(|name| {
            Regex::new(&format!(
                r"(?is)<(?:[\w-]+:)?{}(?:\s[^>]*)?>.*?</(?:[\w-]+:)?{}\s*>",
                regex::escape(name),
                regex::escape(name)
            ))
            .unwrap()
            .is_match(xml)
        })
        .count()
}

fn strip_sensitive_xml(xml: &str) -> (String, usize) {
    let mut output = xml.to_owned();
    let mut removed = 0;
    for name in [
        "creator",
        "lastModifiedBy",
        "created",
        "modified",
        "title",
        "subject",
        "keywords",
        "description",
        "Application",
        "Company",
        "Manager",
        "Template",
        "TotalTime",
        "trackRevisions",
        "generator",
        "editing-duration",
        "editing-cycles",
    ] {
        let pair = Regex::new(&format!(
            r"(?is)<(?:[\w-]+:)?{}(?:\s[^>]*)?>.*?</(?:[\w-]+:)?{}\s*>",
            regex::escape(name),
            regex::escape(name)
        ))
        .unwrap();
        let empty = Regex::new(&format!(
            r"(?is)<(?:[\w-]+:)?{}(?:\s[^>]*)?/\s*>",
            regex::escape(name)
        ))
        .unwrap();
        let count = pair.find_iter(&output).count() + empty.find_iter(&output).count();
        output = pair.replace_all(&output, "").into_owned();
        output = empty.replace_all(&output, "").into_owned();
        removed += count;
    }
    (output, removed)
}

fn accept_word_revisions(xml: &str) -> (String, usize) {
    let deletion =
        Regex::new(r"(?is)<(?:w:)?(?:del|moveFrom)(?:\s[^>]*)?>.*?</(?:w:)?(?:del|moveFrom)\s*>")
            .unwrap();
    let insertion =
        Regex::new(r"(?is)<(?:w:)?(?:ins|moveTo)(?:\s[^>]*)?>(.*?)</(?:w:)?(?:ins|moveTo)\s*>")
            .unwrap();
    let property_change = Regex::new(r"(?is)<(?:w:)?(?:rPr|pPr|tblPr|trPr|tcPr|sectPr|numbering)Change(?:\s[^>]*)?>.*?</(?:w:)?(?:rPr|pPr|tblPr|trPr|tcPr|sectPr|numbering)Change\s*>").unwrap();
    let anchors = Regex::new(r"(?is)<(?:w:)?(?:commentRangeStart|commentRangeEnd|commentReference|moveFromRangeStart|moveFromRangeEnd|moveToRangeStart|moveToRangeEnd)(?:\s[^>]*)?/\s*>").unwrap();
    let count = deletion.find_iter(xml).count()
        + insertion.find_iter(xml).count()
        + property_change.find_iter(xml).count()
        + anchors.find_iter(xml).count();
    let output = deletion.replace_all(xml, "");
    let output = insertion.replace_all(&output, "$1");
    let output = property_change.replace_all(&output, "");
    (anchors.replace_all(&output, "").into_owned(), count)
}

fn is_comment_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("comments")
        || lower.contains("commentauthors")
        || lower.starts_with("xl/persons/")
        || lower.starts_with("customxml/")
}

fn document_finding(label: &str, count: usize) -> Finding {
    Finding {
        category: "document_metadata".into(),
        label: label.into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

/// An EPUB is a zip too, so it rides the same plumbing. Its package document
/// carries the Dublin Core block, and readers leave their own sediment beside
/// it: Calibre bookmarks, Apple purchase receipts, Adobe rights files.
fn is_reader_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("calibre_bookmarks.txt")
        || lower.ends_with("itunesmetadata.plist")
        || lower.ends_with("itunesartwork")
        || lower.ends_with("com.apple.ibooks.display-options.xml")
        || lower.ends_with("rights.xml")
        || lower.ends_with(".sigil")
}

/// Dublin Core terms that name a person or a moment. `title`, `identifier` and
/// `language` stay: the specification requires them, and an EPUB missing one is
/// a broken file rather than a private one.
const EPUB_TERMS: &[&str] = &[
    "creator",
    "contributor",
    "publisher",
    "description",
    "subject",
    "rights",
    "date",
    "source",
    "coverage",
];

fn epub_element(name: &str) -> Regex {
    Regex::new(&format!(
        r"(?is)<(?:[\w-]+:)?{}(?:\s[^>]*)?>.*?</(?:[\w-]+:)?{}\s*>|<(?:[\w-]+:)?{}(?:\s[^>]*)?/\s*>",
        regex::escape(name),
        regex::escape(name),
        regex::escape(name)
    ))
    .expect("EPUB element regex must compile")
}

/// Both halves stop at the first `<`, so a reader tag can never swallow the
/// element that follows it — the alternation is leftmost-first, and a greedy
/// `.*?` here would eat the required `dcterms:modified` next door.
fn calibre_meta() -> Regex {
    Regex::new(
        r#"(?is)<meta[^>]*(?:calibre|sigil|kobo|epubcheck)[^>]*/\s*>|<meta[^>]*(?:calibre|sigil|kobo|epubcheck)[^>]*>[^<]*</meta\s*>"#,
    )
    .expect("EPUB reader metadata regex must compile")
}

/// The one timestamp EPUB 3 refuses to live without. It cannot be deleted, so it
/// is pinned to the epoch, where it identifies nobody.
fn modified_meta() -> Regex {
    Regex::new(r#"(?is)(<meta[^>]*dcterms:modified[^>]*>)[^<]*(</meta\s*>)"#)
        .expect("EPUB modified regex must compile")
}

fn strip_epub_metadata(xml: &str) -> (String, usize) {
    let mut output = xml.to_owned();
    let mut removed = 0;
    for term in EPUB_TERMS {
        let pattern = epub_element(term);
        removed += pattern.find_iter(&output).count();
        output = pattern.replace_all(&output, "").into_owned();
    }
    let calibre = calibre_meta();
    removed += calibre.find_iter(&output).count();
    output = calibre.replace_all(&output, "").into_owned();
    let modified = modified_meta();
    removed += modified
        .find_iter(&output)
        .filter(|found| !found.as_str().contains("1970-01-01T00:00:00Z"))
        .count();
    output = modified
        .replace_all(&output, "${1}1970-01-01T00:00:00Z${2}")
        .into_owned();
    (output, removed)
}

fn is_package_document(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".opf")
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let mut archive = read_archive(data)?;
    let mut metadata = 0;
    let mut comments = 0;
    let mut revisions = 0;
    let mut publication = 0;
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        expanded = expanded.saturating_add(file.size());
        if expanded > MAX_UNCOMPRESSED {
            return Err(CleanError::InvalidFormat("Office 文件解压后过大".into()));
        }
        let name = file.name().to_owned();
        if is_comment_part(&name) {
            comments += 1;
            continue;
        }
        if is_reader_part(&name) {
            publication += 1;
            continue;
        }
        if !name.ends_with(".xml") && !is_package_document(&name) {
            continue;
        }
        let mut xml = String::new();
        if file.read_to_string(&mut xml).is_err() {
            continue;
        }
        if is_package_document(&name) {
            publication += strip_epub_metadata(&xml).1;
            continue;
        }
        if name.starts_with("docProps/") || name.ends_with("settings.xml") || name == "meta.xml" {
            metadata += sensitive_xml_count(&xml);
        }
        if name.starts_with("word/") {
            revisions += accept_word_revisions(&xml).1;
        }
    }
    let mut findings = Vec::new();
    if metadata > 0 {
        findings.push(privacy_finding("文档属性与作者信息", metadata));
    }
    if comments > 0 {
        findings.push(privacy_finding("批注与自定义 XML", comments));
    }
    if revisions > 0 {
        findings.push(privacy_finding("修订记录", revisions));
    }
    if publication > 0 {
        findings.push(document_finding("出版信息与阅读器残留", publication));
    }
    Ok(findings)
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect(data)?;
    let mut archive = read_archive(data)?;
    let cursor = Cursor::new(Vec::with_capacity(data.len()));
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let private_relation =
        Regex::new(r#"(?is)<(?:Relationship|Override)[^>]*(?:comments|customXml)[^>]*/>"#)
            .expect("private Office relationship regex must compile");
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_owned();
        if is_comment_part(&name) || is_reader_part(&name) {
            continue;
        }
        if file.is_dir() {
            writer.add_directory(name, options)?;
            continue;
        }
        expanded = expanded.saturating_add(file.size());
        if expanded > MAX_UNCOMPRESSED {
            return Err(CleanError::InvalidFormat("Office 文件解压后过大".into()));
        }
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        if name.ends_with(".xml") || name.ends_with(".rels") || is_package_document(&name) {
            if let Ok(xml) = std::str::from_utf8(&content) {
                let mut cleaned = xml.to_owned();
                if is_package_document(&name) {
                    cleaned = strip_epub_metadata(&cleaned).0;
                }
                if name.starts_with("docProps/")
                    || name.ends_with("settings.xml")
                    || name == "meta.xml"
                {
                    cleaned = strip_sensitive_xml(&cleaned).0;
                }
                if name.starts_with("word/") {
                    cleaned = accept_word_revisions(&cleaned).0;
                }
                if name.ends_with(".rels") || name == "[Content_Types].xml" {
                    cleaned = private_relation.replace_all(&cleaned, "").into_owned();
                }
                content = cleaned.into_bytes();
            }
        }
        writer.start_file(name, options)?;
        writer.write_all(&content)?;
    }
    Ok((writer.finish()?.into_inner(), findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_docx() -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("docProps/core.xml", options).unwrap();
        writer
            .write_all(br#"<cp:coreProperties><dc:creator>Alice</dc:creator></cp:coreProperties>"#)
            .unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(br#"<w:document><w:body><w:del><w:r>old</w:r></w:del><w:ins><w:r>new</w:r></w:ins></w:body></w:document>"#).unwrap();
        writer.start_file("word/comments.xml", options).unwrap();
        writer.write_all(b"secret").unwrap();
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn scans_and_cleans_docx() {
        let data = sample_docx();
        assert_eq!(inspect(&data).unwrap().len(), 3);
        let (cleaned, _) = clean(&data).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        assert!(zip.by_name("word/comments.xml").is_err());
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(!xml.contains("old"));
        assert!(xml.contains("new"));
    }

    #[test]
    fn removes_odt_generator() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", options).unwrap();
        writer
            .write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();
        writer.start_file("meta.xml", options).unwrap();
        writer.write_all(br#"<office:document-meta><meta:generator>Claude Writer</meta:generator><meta:editing-cycles>7</meta:editing-cycles></office:document-meta>"#).unwrap();
        let data = writer.finish().unwrap().into_inner();
        let (cleaned, findings) = clean(&data).unwrap();
        assert!(!findings.is_empty());
        let mut archive = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        let mut meta = String::new();
        archive
            .by_name("meta.xml")
            .unwrap()
            .read_to_string(&mut meta)
            .unwrap();
        assert!(!meta.contains("Claude"));
        assert!(!meta.contains("editing-cycles"));
    }

    #[test]
    fn strips_epub_publication_data_but_keeps_what_the_spec_requires() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", options).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        writer.start_file("OEBPS/content.opf", options).unwrap();
        writer
            .write_all(
                br#"<package unique-identifier="bookid"><metadata><dc:title>Notes</dc:title><dc:identifier id="bookid">urn:uuid:1234</dc:identifier><dc:language>zh</dc:language><dc:creator>Alice Zhang</dc:creator><dc:publisher>Home Press</dc:publisher><dc:date>2024-03-09</dc:date><meta name="calibre:timestamp" content="2024-03-09T11:00:00"/><meta property="dcterms:modified">2024-03-09T11:00:00Z</meta></metadata></package>"#,
            )
            .unwrap();
        writer
            .start_file("META-INF/calibre_bookmarks.txt", options)
            .unwrap();
        writer.write_all(b"last read position: alice").unwrap();
        let data = writer.finish().unwrap().into_inner();

        let findings = inspect(&data).unwrap();
        assert!(findings
            .iter()
            .any(|finding| finding.category == "document_metadata"));

        let (cleaned, _) = clean(&data).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        assert!(archive.by_name("META-INF/calibre_bookmarks.txt").is_err());
        let mut opf = String::new();
        archive
            .by_name("OEBPS/content.opf")
            .unwrap()
            .read_to_string(&mut opf)
            .unwrap();
        assert!(!opf.contains("Alice Zhang"));
        assert!(!opf.contains("Home Press"));
        assert!(!opf.contains("2024-03-09"));
        assert!(!opf.contains("calibre"));
        // Title, identifier and language are load-bearing; the timestamp is
        // pinned rather than removed because EPUB 3 demands one.
        assert!(opf.contains("<dc:title>Notes</dc:title>"));
        assert!(opf.contains("urn:uuid:1234"));
        assert!(opf.contains("<dc:language>zh</dc:language>"));
        assert!(opf.contains("1970-01-01T00:00:00Z"));
    }

    #[test]
    fn re_inspecting_a_cleaned_epub_reports_nothing_left() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("content.opf", options).unwrap();
        writer.write_all(br#"<package><metadata><dc:creator>Bob</dc:creator><meta property="dcterms:modified">2024-01-01T00:00:00Z</meta></metadata></package>"#).unwrap();
        let data = writer.finish().unwrap().into_inner();
        let (cleaned, _) = clean(&data).unwrap();
        assert!(inspect(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn cleans_revisions_and_comment_anchors_in_headers() {
        let xml = r#"<w:hdr><w:moveFrom><w:r><w:t>old</w:t></w:r></w:moveFrom><w:moveTo><w:r><w:t>new</w:t></w:r></w:moveTo><w:pPrChange><w:pPr/></w:pPrChange><w:commentRangeStart w:id="1"/><w:commentReference w:id="1"/></w:hdr>"#;
        let (cleaned, count) = accept_word_revisions(xml);
        assert!(count >= 4);
        assert!(!cleaned.contains("old"));
        assert!(cleaned.contains("new"));
        assert!(!cleaned.contains("PrChange"));
        assert!(!cleaned.contains("comment"));
    }
}
