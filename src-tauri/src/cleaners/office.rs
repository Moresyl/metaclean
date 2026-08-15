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
    let deletion = Regex::new(r"(?is)<(?:w:)?del(?:\s[^>]*)?>.*?</(?:w:)?del\s*>").unwrap();
    let insertion = Regex::new(r"(?is)<(?:w:)?ins(?:\s[^>]*)?>(.*?)</(?:w:)?ins\s*>").unwrap();
    let count = deletion.find_iter(xml).count() + insertion.find_iter(xml).count();
    let without_deletions = deletion.replace_all(xml, "");
    (
        insertion.replace_all(&without_deletions, "$1").into_owned(),
        count,
    )
}

fn is_comment_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("comments")
        || lower.contains("commentauthors")
        || lower.starts_with("customxml/")
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let mut archive = read_archive(data)?;
    let mut metadata = 0;
    let mut comments = 0;
    let mut revisions = 0;
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
        if !name.ends_with(".xml") {
            continue;
        }
        let mut xml = String::new();
        if file.read_to_string(&mut xml).is_err() {
            continue;
        }
        if name.starts_with("docProps/") || name.ends_with("settings.xml") {
            metadata += sensitive_xml_count(&xml);
        }
        if name.ends_with("document.xml") {
            revisions += Regex::new(r"(?is)<(?:w:)?(?:ins|del)(?:\s|>)")
                .unwrap()
                .find_iter(&xml)
                .count();
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
    Ok(findings)
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect(data)?;
    let mut archive = read_archive(data)?;
    let cursor = Cursor::new(Vec::with_capacity(data.len()));
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_owned();
        if is_comment_part(&name) {
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
        if name.ends_with(".xml") || name.ends_with(".rels") {
            if let Ok(xml) = std::str::from_utf8(&content) {
                let mut cleaned = xml.to_owned();
                if name.starts_with("docProps/") || name.ends_with("settings.xml") {
                    cleaned = strip_sensitive_xml(&cleaned).0;
                }
                if name.ends_with("document.xml") {
                    cleaned = accept_word_revisions(&cleaned).0;
                }
                if name.ends_with(".rels") || name == "[Content_Types].xml" {
                    let relation = Regex::new(
                        r#"(?is)<(?:Relationship|Override)[^>]*(?:comments|customXml)[^>]*/>"#,
                    )
                    .unwrap();
                    cleaned = relation.replace_all(&cleaned, "").into_owned();
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
}
