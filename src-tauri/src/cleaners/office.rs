use std::{
    collections::HashSet,
    io::{self, Cursor, Read, Write},
    sync::OnceLock,
};

use quick_xml::{events::Event, name::ResolveResult, reader::NsReader, Reader, Writer};
use regex::Regex;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::{
    error::{CleanError, Result},
    models::{Finding, FindingSeverity},
};

const MAX_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED: u64 = 512 * 1024 * 1024;
const MAX_XML_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const SENSITIVE_XML_NAMES: &[&str] = &[
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

fn privacy_finding(label: &str, count: usize) -> Finding {
    Finding {
        category: "office_metadata".into(),
        label: label.into(),
        count,
        severity: FindingSeverity::Privacy,
    }
}

fn read_archive(data: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    if archive.len() > MAX_ENTRIES {
        return Err(CleanError::InvalidFormat(
            "Office 文件包含过多 ZIP 条目".into(),
        ));
    }
    let mut unique = HashSet::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name();
        let normalized = name.replace('\\', "/");
        let unsafe_name = normalized.starts_with('/')
            || normalized.split('/').any(|part| part == "..")
            || normalized
                .split('/')
                .next()
                .is_some_and(|part| part.contains(':'));
        if unsafe_name {
            return Err(CleanError::InvalidFormat(format!(
                "文档 ZIP 包含不安全路径：{name}"
            )));
        }
        if file.encrypted() {
            return Err(CleanError::InvalidFormat(format!(
                "文档 ZIP 包含无法检查的加密条目：{name}"
            )));
        }
        if !unique.insert(normalized.to_ascii_lowercase()) {
            return Err(CleanError::InvalidFormat(format!(
                "文档 ZIP 包含重复或大小写冲突条目：{name}"
            )));
        }
    }
    Ok(archive)
}

fn validate_container(data: &[u8], extension: &str) -> Result<()> {
    let mut archive = read_archive(data)?;
    if let Some(mimetype) = odf_mimetype(extension) {
        validate_mimetype(&mut archive, mimetype, true)?;
        reject_encrypted_odf(&mut archive)?;
        archive.by_name("content.xml")?;
        return Ok(());
    }
    match extension {
        "docx" => {
            archive.by_name("[Content_Types].xml")?;
            archive.by_name("word/document.xml")?;
        }
        "xlsx" => {
            archive.by_name("[Content_Types].xml")?;
            archive.by_name("xl/workbook.xml")?;
        }
        "pptx" => {
            archive.by_name("[Content_Types].xml")?;
            archive.by_name("ppt/presentation.xml")?;
        }
        "epub" => {
            validate_mimetype(&mut archive, "application/epub+zip", true)?;
            archive.by_name("META-INF/container.xml")?;
        }
        _ => return Err(CleanError::Unsupported(extension.into())),
    }
    Ok(())
}

fn odf_mimetype(extension: &str) -> Option<&'static str> {
    Some(match extension {
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "odg" => "application/vnd.oasis.opendocument.graphics",
        "odf" => "application/vnd.oasis.opendocument.formula",
        "odb" => "application/vnd.oasis.opendocument.database",
        "odm" => "application/vnd.oasis.opendocument.text-master",
        "ott" => "application/vnd.oasis.opendocument.text-template",
        "ots" => "application/vnd.oasis.opendocument.spreadsheet-template",
        "otp" => "application/vnd.oasis.opendocument.presentation-template",
        "otg" => "application/vnd.oasis.opendocument.graphics-template",
        _ => return None,
    })
}

fn is_odf_extension(extension: &str) -> bool {
    odf_mimetype(extension).is_some()
}

fn reject_encrypted_odf(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<()> {
    if !archive
        .file_names()
        .any(|name| name.eq_ignore_ascii_case("META-INF/manifest.xml"))
    {
        return Ok(());
    }
    let mut entry = archive.by_name("META-INF/manifest.xml")?;
    let manifest = read_utf8_bounded(&mut entry, MAX_MANIFEST_BYTES, "ODF 清单")?;
    if manifest.contains("encryption-data") {
        return Err(CleanError::InvalidFormat(
            "ODF 文档已加密，无法验证或清理其中的隐私元数据".into(),
        ));
    }
    Ok(())
}

fn validate_mimetype(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    expected: &str,
    must_be_first_and_stored: bool,
) -> Result<()> {
    let first_is_valid = if must_be_first_and_stored {
        let first = archive.by_index(0)?;
        first.name() == "mimetype" && first.compression() == zip::CompressionMethod::Stored
    } else {
        true
    };
    if !first_is_valid {
        return Err(CleanError::InvalidFormat(
            "文档 mimetype 必须是首个未压缩 ZIP 条目".into(),
        ));
    }
    let mut entry = archive.by_name("mimetype")?;
    let value = read_utf8_bounded(&mut entry, 256, "文档 mimetype")?;
    if value != expected {
        return Err(CleanError::InvalidFormat(format!(
            "文档 mimetype 不匹配：{value}"
        )));
    }
    Ok(())
}

fn read_utf8_bounded(reader: &mut impl Read, limit: u64, label: &str) -> Result<String> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(CleanError::InvalidFormat(format!("{label} 解压后过大")));
    }
    String::from_utf8(bytes)
        .map_err(|_| CleanError::InvalidFormat(format!("{label} 不是可安全处理的 UTF-8")))
}

pub fn is_supported_container(data: &[u8], extension: &str) -> bool {
    validate_container(data, extension).is_ok()
}

fn sensitive_xml_patterns() -> &'static [(Regex, Regex)] {
    static PATTERNS: OnceLock<Vec<(Regex, Regex)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        SENSITIVE_XML_NAMES
            .iter()
            .map(|name| {
                let name = regex::escape(name);
                (
                    Regex::new(&format!(
                        r"(?is)<(?:[\w-]+:)?{name}(?:\s[^>]*)?>.*?</(?:[\w-]+:)?{name}\s*>"
                    ))
                    .expect("sensitive XML element regex must compile"),
                    Regex::new(&format!(r"(?is)<(?:[\w-]+:)?{name}(?:\s[^>]*)?/\s*>"))
                        .expect("empty sensitive XML element regex must compile"),
                )
            })
            .collect()
    })
}

fn validate_xml(xml: &str, name: &str) -> Result<()> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_end_names = true;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return Ok(()),
            Ok(_) => {}
            Err(error) => {
                return Err(CleanError::InvalidFormat(format!(
                    "文档 XML 结构无效：{name}：{error}"
                )))
            }
        }
    }
}

fn sensitive_xml_count(xml: &str) -> usize {
    sensitive_xml_patterns()
        .iter()
        .map(|(pair, empty)| pair.find_iter(xml).count() + empty.find_iter(xml).count())
        .sum()
}

fn strip_sensitive_xml(xml: &str) -> (String, usize) {
    let mut output = xml.to_owned();
    let mut removed = 0;
    for (pair, empty) in sensitive_xml_patterns() {
        let count = pair.find_iter(&output).count() + empty.find_iter(&output).count();
        output = pair.replace_all(&output, "").into_owned();
        output = empty.replace_all(&output, "").into_owned();
        removed += count;
    }
    (output, removed)
}

const WORD_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT_WORD_NAMESPACE: &[u8] = b"http://purl.oclc.org/ooxml/wordprocessingml/main";

#[derive(Clone, Copy)]
enum WordRevisionElement {
    Drop,
    Unwrap,
}

fn namespace_matches(namespace: &ResolveResult<'_>, expected: &[u8]) -> bool {
    let ResolveResult::Bound(namespace) = namespace else {
        return false;
    };
    if namespace.as_ref() == expected {
        return true;
    }
    std::str::from_utf8(namespace.as_ref())
        .ok()
        .and_then(|value| quick_xml::escape::unescape(value).ok())
        .is_some_and(|value| value.as_bytes() == expected)
}

fn word_revision_element(
    namespace: &ResolveResult<'_>,
    local_name: &[u8],
) -> Option<WordRevisionElement> {
    if !namespace_matches(namespace, WORD_NAMESPACE)
        && !namespace_matches(namespace, STRICT_WORD_NAMESPACE)
    {
        return None;
    }
    if matches!(local_name, b"ins" | b"moveTo") {
        return Some(WordRevisionElement::Unwrap);
    }
    matches!(
        local_name,
        b"del"
            | b"moveFrom"
            | b"rPrChange"
            | b"pPrChange"
            | b"tblPrChange"
            | b"trPrChange"
            | b"tcPrChange"
            | b"sectPrChange"
            | b"numberingChange"
            | b"commentRangeStart"
            | b"commentRangeEnd"
            | b"commentReference"
            | b"moveFromRangeStart"
            | b"moveFromRangeEnd"
            | b"moveToRangeStart"
            | b"moveToRangeEnd"
    )
    .then_some(WordRevisionElement::Drop)
}

fn accept_word_revisions(xml: &str) -> Result<(String, usize)> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut writer = Writer::new(Cursor::new(Vec::with_capacity(xml.len())));
    let mut frames = Vec::new();
    let mut skipped_depth = 0usize;
    let mut removed = 0usize;

    loop {
        let (namespace, event) = reader.read_resolved_event().map_err(|error| {
            CleanError::InvalidFormat(format!("Word XML 命名空间无效：{error}"))
        })?;
        match event {
            Event::Start(start) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(CleanError::InvalidFormat(
                        "Word XML 使用了未声明的命名空间前缀".into(),
                    ));
                }
                if skipped_depth > 0 {
                    skipped_depth += 1;
                } else {
                    match word_revision_element(&namespace, start.local_name().as_ref()) {
                        Some(WordRevisionElement::Drop) => {
                            removed += 1;
                            skipped_depth = 1;
                        }
                        Some(WordRevisionElement::Unwrap) => {
                            removed += 1;
                            frames.push(false);
                        }
                        None => {
                            writer.write_event(Event::Start(start))?;
                            frames.push(true);
                        }
                    }
                }
            }
            Event::Empty(empty) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(CleanError::InvalidFormat(
                        "Word XML 使用了未声明的命名空间前缀".into(),
                    ));
                }
                if skipped_depth == 0 {
                    if word_revision_element(&namespace, empty.local_name().as_ref()).is_some() {
                        removed += 1;
                    } else {
                        writer.write_event(Event::Empty(empty))?;
                    }
                }
            }
            Event::End(end) => {
                if skipped_depth > 0 {
                    skipped_depth -= 1;
                } else if frames.pop().is_some_and(|write_end| write_end) {
                    writer.write_event(Event::End(end))?;
                }
            }
            Event::Eof => break,
            event if skipped_depth == 0 => writer.write_event(event)?,
            _ => {}
        }
    }

    if !frames.is_empty() || skipped_depth != 0 {
        return Err(CleanError::InvalidFormat(
            "Word XML 修订元素未正确闭合".into(),
        ));
    }
    let output = String::from_utf8(writer.into_inner().into_inner())
        .map_err(|_| CleanError::InvalidFormat("Word XML 不是 UTF-8 编码".into()))?;
    Ok((output, removed))
}

const ODF_OFFICE_NAMESPACE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const ODF_TEXT_NAMESPACE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const ODF_TABLE_NAMESPACE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:table:1.0";

#[derive(Clone, Copy)]
enum OdfPrivateElement {
    Annotation,
    Revision,
}

fn odf_private_element(
    namespace: &ResolveResult<'_>,
    local_name: &[u8],
) -> Option<OdfPrivateElement> {
    if namespace_matches(namespace, ODF_OFFICE_NAMESPACE)
        && matches!(local_name, b"annotation" | b"annotation-end")
    {
        return Some(OdfPrivateElement::Annotation);
    }
    if (namespace_matches(namespace, ODF_TEXT_NAMESPACE)
        || namespace_matches(namespace, ODF_TABLE_NAMESPACE))
        && matches!(
            local_name,
            b"tracked-changes" | b"change" | b"change-start" | b"change-end"
        )
        || namespace_matches(namespace, ODF_OFFICE_NAMESPACE) && local_name == b"change-info"
    {
        return Some(OdfPrivateElement::Revision);
    }
    None
}

fn strip_odf_private_xml(xml: &str) -> Result<(String, usize, usize)> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut writer = Writer::new(Cursor::new(Vec::with_capacity(xml.len())));
    let mut skipped_depth = 0usize;
    let mut annotations = 0usize;
    let mut revisions = 0usize;

    loop {
        let (namespace, event) = reader.read_resolved_event().map_err(|error| {
            CleanError::InvalidFormat(format!("OpenDocument XML 命名空间无效：{error}"))
        })?;
        match event {
            Event::Start(start) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(CleanError::InvalidFormat(
                        "OpenDocument XML 使用了未声明的命名空间前缀".into(),
                    ));
                }
                if skipped_depth > 0 {
                    skipped_depth += 1;
                } else if let Some(kind) =
                    odf_private_element(&namespace, start.local_name().as_ref())
                {
                    match kind {
                        OdfPrivateElement::Annotation => annotations += 1,
                        OdfPrivateElement::Revision => revisions += 1,
                    }
                    skipped_depth = 1;
                } else {
                    writer.write_event(Event::Start(start))?;
                }
            }
            Event::Empty(empty) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(CleanError::InvalidFormat(
                        "OpenDocument XML 使用了未声明的命名空间前缀".into(),
                    ));
                }
                if skipped_depth == 0 {
                    if let Some(kind) = odf_private_element(&namespace, empty.local_name().as_ref())
                    {
                        match kind {
                            OdfPrivateElement::Annotation => annotations += 1,
                            OdfPrivateElement::Revision => revisions += 1,
                        }
                    } else {
                        writer.write_event(Event::Empty(empty))?;
                    }
                }
            }
            Event::End(end) => {
                if skipped_depth > 0 {
                    skipped_depth -= 1;
                } else {
                    writer.write_event(Event::End(end))?;
                }
            }
            Event::Eof => break,
            event if skipped_depth == 0 => writer.write_event(event)?,
            _ => {}
        }
    }

    let output = String::from_utf8(writer.into_inner().into_inner())
        .map_err(|_| CleanError::InvalidFormat("OpenDocument XML 不是 UTF-8 编码".into()))?;
    Ok((output, annotations, revisions))
}

fn is_comment_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("word/comments")
        || lower.starts_with("word/people")
        || lower.starts_with("xl/comments")
        || lower.starts_with("xl/threadedcomments/")
        || lower.starts_with("xl/persons/")
        || lower.starts_with("ppt/comments/")
        || lower.starts_with("ppt/commentauthors")
        || lower.starts_with("ppt/authors/")
        || lower.starts_with("customxml/")
}

fn requires_xml_rewrite(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    is_package_document(&lower)
        || lower.starts_with("docprops/")
        || lower.ends_with("settings.xml")
        || lower == "meta.xml"
        || lower.starts_with("word/")
        || lower.ends_with(".rels")
        || lower == "[content_types].xml"
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

fn epub_elements() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| EPUB_TERMS.iter().map(|term| epub_element(term)).collect())
}

/// Both halves stop at the first `<`, so a reader tag can never swallow the
/// element that follows it — the alternation is leftmost-first, and a greedy
/// `.*?` here would eat the required `dcterms:modified` next door.
fn calibre_meta() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(
        r#"(?is)<meta[^>]*(?:calibre|sigil|kobo|epubcheck)[^>]*/\s*>|<meta[^>]*(?:calibre|sigil|kobo|epubcheck)[^>]*>[^<]*</meta\s*>"#,
    )
    .expect("EPUB reader metadata regex must compile"))
}

/// The one timestamp EPUB 3 refuses to live without. It cannot be deleted, so it
/// is pinned to the epoch, where it identifies nobody.
fn modified_meta() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?is)(<meta[^>]*dcterms:modified[^>]*>)[^<]*(</meta\s*>)"#)
            .expect("EPUB modified regex must compile")
    })
}

fn strip_epub_metadata(xml: &str) -> (String, usize) {
    let mut output = xml.to_owned();
    let mut removed = 0;
    for pattern in epub_elements() {
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

fn private_relationship_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"(?is)<(?:[\w-]+:)?(?:Relationship|Override)[^>]*(?:comments|customXml)[^>]*/>"#,
        )
        .expect("private Office relationship regex must compile")
    })
}

pub fn inspect(data: &[u8], extension: &str) -> Result<Vec<Finding>> {
    validate_container(data, extension)?;
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
        let lower = name.to_ascii_lowercase();
        if is_comment_part(&name) {
            comments += 1;
            continue;
        }
        if is_reader_part(&name) {
            publication += 1;
            continue;
        }
        if !lower.ends_with(".xml") && !is_package_document(&lower) {
            continue;
        }
        if file.size() > MAX_XML_BYTES {
            return Err(CleanError::InvalidFormat(format!(
                "文档 XML 解压后过大：{name}"
            )));
        }
        let xml = match read_utf8_bounded(&mut file, MAX_XML_BYTES, &format!("文档 XML：{name}"))
        {
            Ok(xml) => xml,
            Err(_) if !requires_xml_rewrite(&name) => continue,
            Err(error) => return Err(error),
        };
        validate_xml(&xml, &name)?;
        if is_package_document(&lower) {
            publication += strip_epub_metadata(&xml).1;
            continue;
        }
        if is_odf_extension(extension) && matches!(lower.as_str(), "content.xml" | "styles.xml") {
            let (_, found_comments, found_revisions) = strip_odf_private_xml(&xml)?;
            comments += found_comments;
            revisions += found_revisions;
        }
        if lower.starts_with("docprops/") || lower.ends_with("settings.xml") || lower == "meta.xml"
        {
            metadata += sensitive_xml_count(&xml);
        }
        if lower.starts_with("word/") {
            revisions += accept_word_revisions(&xml)?.1;
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

pub fn clean(data: &[u8], extension: &str) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect(data, extension)?;
    let mut archive = read_archive(data)?;
    let cursor = Cursor::new(Vec::with_capacity(data.len()));
    let mut writer = ZipWriter::new(cursor);
    let deflated =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let private_relation = private_relationship_pattern();
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_owned();
        let lower = name.to_ascii_lowercase();
        if is_comment_part(&name) || is_reader_part(&name) {
            continue;
        }
        if file.is_dir() {
            writer.add_directory(name, deflated)?;
            continue;
        }
        expanded = expanded.saturating_add(file.size());
        if expanded > MAX_UNCOMPRESSED {
            return Err(CleanError::InvalidFormat("Office 文件解压后过大".into()));
        }
        let rewrite =
            lower.ends_with(".xml") || lower.ends_with(".rels") || is_package_document(&lower);
        writer.start_file(&name, if name == "mimetype" { stored } else { deflated })?;
        if !rewrite {
            let expected = file.size();
            let copied = io::copy(&mut file.take(expected + 1), &mut writer)?;
            if copied != expected {
                return Err(CleanError::InvalidFormat(format!(
                    "文档 ZIP 条目的实际解压长度不匹配：{name}"
                )));
            }
            continue;
        }
        if file.size() > MAX_XML_BYTES {
            return Err(CleanError::InvalidFormat(format!(
                "文档 XML 解压后过大：{name}"
            )));
        }
        let xml = read_utf8_bounded(&mut file, MAX_XML_BYTES, &format!("文档 XML：{name}"))?;
        validate_xml(&xml, &name)?;
        let mut cleaned = xml;
        if is_package_document(&lower) {
            cleaned = strip_epub_metadata(&cleaned).0;
        }
        if is_odf_extension(extension) && matches!(lower.as_str(), "content.xml" | "styles.xml") {
            cleaned = strip_odf_private_xml(&cleaned)?.0;
        }
        if lower.starts_with("docprops/") || lower.ends_with("settings.xml") || lower == "meta.xml"
        {
            cleaned = strip_sensitive_xml(&cleaned).0;
        }
        if lower.starts_with("word/") {
            cleaned = accept_word_revisions(&cleaned)?.0;
        }
        if lower.ends_with(".rels") || lower == "[content_types].xml" {
            cleaned = private_relation.replace_all(&cleaned, "").into_owned();
        }
        validate_xml(&cleaned, &name)?;
        writer.write_all(cleaned.as_bytes())?;
    }
    Ok((writer.finish()?.into_inner(), findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_options() -> SimpleFileOptions {
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
    }

    #[test]
    fn bounded_xml_reader_rejects_oversized_and_non_utf8_content() {
        assert!(read_utf8_bounded(&mut Cursor::new(b"12345"), 4, "XML").is_err());
        assert!(read_utf8_bounded(&mut Cursor::new([0xff, 0xfe]), 4, "XML").is_err());
        assert_eq!(
            read_utf8_bounded(&mut Cursor::new(b"<x/>"), 4, "XML").unwrap(),
            "<x/>"
        );
    }

    fn sample_docx() -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer
            .write_all(br#"<Types><Override PartName="/word/document.xml"/></Types>"#)
            .unwrap();
        writer.start_file("docProps/core.xml", options).unwrap();
        writer
            .write_all(br#"<cp:coreProperties><dc:creator>Alice</dc:creator></cp:coreProperties>"#)
            .unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessing&#x6d;l/2006/main" xmlns:m="http://www.w3.org/1998/Math/MathML"><w:body><w:del><w:r>old</w:r></w:del><w:ins><w:r>new</w:r></w:ins><m:annotation>keep formula semantics</m:annotation></w:body></w:document>"#).unwrap();
        writer.start_file("word/comments.xml", options).unwrap();
        writer.write_all(b"secret").unwrap();
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn scans_and_cleans_docx() {
        let data = sample_docx();
        assert_eq!(inspect(&data, "docx").unwrap().len(), 3);
        let (cleaned, _) = clean(&data, "docx").unwrap();
        let mut zip = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        assert!(zip.by_name("word/comments.xml").is_err());
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(!xml.contains("old"));
        assert!(xml.contains("new"));
        assert!(xml.contains("keep formula semantics"));
        assert!(xml.contains("m:annotation"));
    }

    #[test]
    fn cleans_privacy_parts_even_when_optional_part_names_use_mixed_case() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer.write_all(b"<Types/>").unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(b"<document/>").unwrap();
        writer.start_file("DOCPROPS/core.xml", options).unwrap();
        writer.write_all(br#"<cp:coreProperties xmlns:cp="urn:core" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>Alice</dc:creator></cp:coreProperties>"#).unwrap();
        writer.start_file("WORD/header1.xml", options).unwrap();
        writer.write_all(br#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:del><w:r>old</w:r></w:del><w:ins><w:r>new</w:r></w:ins></w:hdr>"#).unwrap();
        let source = writer.finish().unwrap().into_inner();

        assert!(!inspect(&source, "docx").unwrap().is_empty());
        let (cleaned, _) = clean(&source, "docx").unwrap();
        let mut archive = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        let mut properties = String::new();
        archive
            .by_name("DOCPROPS/core.xml")
            .unwrap()
            .read_to_string(&mut properties)
            .unwrap();
        assert!(!properties.contains("Alice"));
        let mut header = String::new();
        archive
            .by_name("WORD/header1.xml")
            .unwrap()
            .read_to_string(&mut header)
            .unwrap();
        assert!(!header.contains("old"));
        assert!(header.contains("new"));
    }

    #[test]
    fn recognizes_legacy_and_threaded_comment_part_families() {
        for name in [
            "word/comments.xml",
            "word/people.xml",
            "xl/comments1.xml",
            "xl/threadedComments/threadedComment1.xml",
            "xl/persons/person.xml",
            "ppt/comments/comment1.xml",
            "ppt/commentAuthors.xml",
            "ppt/authors/author1.xml",
            "customXml/item1.xml",
        ] {
            assert!(is_comment_part(name), "missed {name}");
        }
        assert!(!is_comment_part("xl/worksheets/comments-summary.xml"));
    }

    #[test]
    fn counts_every_sensitive_xml_occurrence() {
        let xml = "<root><dc:creator>Alice</dc:creator><dc:creator>Bob</dc:creator><title/></root>";
        assert_eq!(sensitive_xml_count(xml), 3);
        let (cleaned, removed) = strip_sensitive_xml(xml);
        assert_eq!(removed, 3);
        assert!(!cleaned.contains("creator"));
        assert!(!cleaned.contains("title"));
    }

    #[test]
    fn removes_odt_generator() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", stored_options()).unwrap();
        writer
            .write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();
        writer.start_file("content.xml", options).unwrap();
        writer.write_all(br#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#).unwrap();
        writer.start_file("meta.xml", options).unwrap();
        writer.write_all(br#"<office:document-meta><meta:generator>Claude Writer</meta:generator><meta:editing-cycles>7</meta:editing-cycles></office:document-meta>"#).unwrap();
        let data = writer.finish().unwrap().into_inner();
        let (cleaned, findings) = clean(&data, "odt").unwrap();
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
    fn supports_the_open_document_family_and_resolves_annotations_and_revisions() {
        for (extension, mimetype) in [
            ("odt", "application/vnd.oasis.opendocument.text"),
            ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
            ("odp", "application/vnd.oasis.opendocument.presentation"),
            ("odg", "application/vnd.oasis.opendocument.graphics"),
            ("odf", "application/vnd.oasis.opendocument.formula"),
            ("odb", "application/vnd.oasis.opendocument.database"),
            ("odm", "application/vnd.oasis.opendocument.text-master"),
            ("ott", "application/vnd.oasis.opendocument.text-template"),
            (
                "ots",
                "application/vnd.oasis.opendocument.spreadsheet-template",
            ),
            (
                "otp",
                "application/vnd.oasis.opendocument.presentation-template",
            ),
            (
                "otg",
                "application/vnd.oasis.opendocument.graphics-template",
            ),
        ] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            let options = SimpleFileOptions::default();
            writer.start_file("mimetype", stored_options()).unwrap();
            writer.write_all(mimetype.as_bytes()).unwrap();
            writer.start_file("content.xml", options).unwrap();
            writer.write_all(br#"<o:document xmlns:o="urn:oasis:names:tc:opendocument:xmlns:offic&#x65;:1.0" xmlns:t="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:d="http://purl.org/dc/elements/1.1/" xmlns:m="http://www.w3.org/1998/Math/MathML"><t:tracked-changes><t:changed-region><t:deletion><t:p>old private text</t:p></t:deletion></t:changed-region></t:tracked-changes><t:p><t:change-start t:change-id="ct1"/>current text<t:change-end t:change-id="ct1"/><o:annotation><d:creator>Alice</d:creator><t:p>private note</t:p></o:annotation><o:annotation-end o:name="note1"/><m:annotation encoding="application/x-tex">keep formula semantics</m:annotation></t:p></o:document>"#).unwrap();
            let source = writer.finish().unwrap().into_inner();

            assert!(is_supported_container(&source, extension), "{extension}");
            let findings = inspect(&source, extension).unwrap();
            assert!(findings
                .iter()
                .any(|finding| finding.label == "批注与自定义 XML"));
            assert!(findings.iter().any(|finding| finding.label == "修订记录"));

            let (cleaned, _) = clean(&source, extension).unwrap();
            assert!(
                inspect(&cleaned, extension).unwrap().is_empty(),
                "{extension}"
            );
            let mut archive = ZipArchive::new(Cursor::new(cleaned)).unwrap();
            let mut content = String::new();
            archive
                .by_name("content.xml")
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            assert!(content.contains("current text"));
            assert!(!content.contains("old private text"));
            assert!(!content.contains("private note"));
            assert!(content.contains("keep formula semantics"));
            assert!(content.contains("m:annotation"));
            assert_eq!(
                archive.by_index(0).unwrap().compression(),
                zip::CompressionMethod::Stored
            );
        }
    }

    #[test]
    fn strips_epub_publication_data_but_keeps_what_the_spec_requires() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", stored_options()).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        writer
            .start_file("META-INF/container.xml", options)
            .unwrap();
        writer
            .write_all(br#"<container><rootfiles/></container>"#)
            .unwrap();
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

        let findings = inspect(&data, "epub").unwrap();
        assert!(findings
            .iter()
            .any(|finding| finding.category == "document_metadata"));

        let (cleaned, _) = clean(&data, "epub").unwrap();
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
        writer.start_file("mimetype", stored_options()).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        writer
            .start_file("META-INF/container.xml", options)
            .unwrap();
        writer
            .write_all(br#"<container><rootfiles/></container>"#)
            .unwrap();
        writer.start_file("content.opf", options).unwrap();
        writer.write_all(br#"<package><metadata><dc:creator>Bob</dc:creator><meta property="dcterms:modified">2024-01-01T00:00:00Z</meta></metadata></package>"#).unwrap();
        let data = writer.finish().unwrap().into_inner();
        let (cleaned, _) = clean(&data, "epub").unwrap();
        assert!(inspect(&cleaned, "epub").unwrap().is_empty());
    }

    #[test]
    fn cleans_revisions_and_comment_anchors_in_headers() {
        let xml = r#"<word:hdr xmlns:word="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><word:moveFrom><word:r><word:t>old</word:t></word:r></word:moveFrom><word:moveTo><word:r><word:t>new</word:t></word:r></word:moveTo><word:pPrChange><word:pPr/></word:pPrChange><word:commentRangeStart word:id="1"/><word:commentReference word:id="1"/></word:hdr>"#;
        let (cleaned, count) = accept_word_revisions(xml).unwrap();
        assert!(count >= 4);
        assert!(!cleaned.contains("old"));
        assert!(cleaned.contains("new"));
        assert!(!cleaned.contains("PrChange"));
        assert!(!cleaned.contains("comment"));
    }

    #[test]
    fn rejects_plain_zip_files_and_duplicate_members() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("notes.txt", options).unwrap();
        writer.write_all(b"not an Office document").unwrap();
        let plain = writer.finish().unwrap().into_inner();
        assert!(!is_supported_container(&plain, "docx"));
        assert!(inspect(&plain, "docx").is_err());

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("mimetype", stored_options()).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        writer.start_file("mimetypx", options).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        let mut duplicate = writer.finish().unwrap().into_inner();
        for offset in 0..=duplicate.len() - b"mimetypx".len() {
            if duplicate[offset..].starts_with(b"mimetypx") {
                duplicate[offset..offset + b"mimetype".len()].copy_from_slice(b"mimetype");
            }
        }
        assert!(inspect(&duplicate, "epub").is_err());
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_zip_member_names() {
        let options = SimpleFileOptions::default();
        for names in [
            ["word/document.xml", "WORD/DOCUMENT.XML"],
            ["../word/document.xml", "notes.txt"],
            ["C:/word/document.xml", "notes.txt"],
        ] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            for name in names {
                writer.start_file(name, options).unwrap();
                writer.write_all(b"content").unwrap();
            }
            let data = writer.finish().unwrap().into_inner();
            assert!(read_archive(&data).is_err(), "accepted {names:?}");
        }
    }

    #[test]
    fn rejects_encrypted_odf_metadata_instead_of_claiming_it_is_clean() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", stored_options()).unwrap();
        writer
            .write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();
        writer.start_file("content.xml", options).unwrap();
        writer.write_all(br#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>"#).unwrap();
        writer.start_file("META-INF/manifest.xml", options).unwrap();
        writer
            .write_all(b"<manifest:manifest><manifest:encryption-data/></manifest:manifest>")
            .unwrap();
        let data = writer.finish().unwrap().into_inner();

        let error = inspect(&data, "odt").unwrap_err().to_string();
        assert!(error.contains("已加密"));
    }

    #[test]
    fn keeps_epub_mimetype_first_and_uncompressed() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("mimetype", stored_options()).unwrap();
        writer.write_all(b"application/epub+zip").unwrap();
        writer
            .start_file("META-INF/container.xml", options)
            .unwrap();
        writer.write_all(b"<container/>").unwrap();
        writer.start_file("content.opf", options).unwrap();
        writer
            .write_all(br#"<package><metadata><dc:creator>Alice</dc:creator></metadata></package>"#)
            .unwrap();
        let source = writer.finish().unwrap().into_inner();

        let (cleaned, _) = clean(&source, "epub").unwrap();
        let mut archive = ZipArchive::new(Cursor::new(cleaned)).unwrap();
        let first = archive.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), zip::CompressionMethod::Stored);
    }

    #[test]
    fn rejects_unreadable_private_xml_instead_of_silently_passing_it() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer.write_all(b"<Types/>").unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(b"<w:document/>").unwrap();
        writer.start_file("docProps/core.xml", options).unwrap();
        writer.write_all(&[0xff, 0xfe, 0x00]).unwrap();
        let source = writer.finish().unwrap().into_inner();

        assert!(inspect(&source, "docx").is_err());
        assert!(clean(&source, "docx").is_err());
    }

    #[test]
    fn rejects_malformed_xml_and_never_writes_a_corrupted_document() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer.write_all(b"<Types/>").unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer
            .write_all(b"<w:document><w:body></w:document>")
            .unwrap();
        let source = writer.finish().unwrap().into_inner();

        assert!(inspect(&source, "docx").is_err());
        assert!(clean(&source, "docx").is_err());
    }
}
