use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    ops::Range,
};

use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
    Writer,
};
use zip::ZipArchive;

use super::{namespace_matches, read_utf8_bounded, MAX_ENTRIES, MAX_MANIFEST_BYTES};
use crate::error::{CleanError, Result};

const TYPES_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/package/2006/content-types";
const RELATIONSHIPS_NAMESPACE: &[u8] =
    b"http://schemas.openxmlformats.org/package/2006/relationships";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PropertyKind {
    Core,
    Extended,
    Custom,
}

impl PropertyKind {
    fn content_type(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "application/vnd.openxmlformats-package.core-properties+xml" => Some(Self::Core),
            "application/vnd.openxmlformats-officedocument.extended-properties+xml" => {
                Some(Self::Extended)
            }
            "application/vnd.openxmlformats-officedocument.custom-properties+xml" => {
                Some(Self::Custom)
            }
            _ => None,
        }
    }

    fn relationship(value: &str) -> Option<Self> {
        match value {
            "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" => Some(Self::Core),
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties"
            | "http://purl.oclc.org/ooxml/officeDocument/relationships/extendedProperties" => Some(Self::Extended),
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties"
            | "http://purl.oclc.org/ooxml/officeDocument/relationships/customProperties" => Some(Self::Custom),
            _ => None,
        }
    }

    fn root(self) -> (&'static str, &'static [u8]) {
        match self {
            Self::Core => (
                "coreProperties",
                b"http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
            ),
            Self::Extended => (
                "Properties",
                b"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
            ),
            Self::Custom => (
                "Properties",
                b"http://schemas.openxmlformats.org/officeDocument/2006/custom-properties",
            ),
        }
    }

    fn strict_namespace(self) -> Option<&'static [u8]> {
        match self {
            Self::Core => None,
            Self::Extended => Some(b"http://purl.oclc.org/ooxml/officeDocument/extendedProperties"),
            Self::Custom => Some(b"http://purl.oclc.org/ooxml/officeDocument/customProperties"),
        }
    }
}

fn invalid(message: &str) -> CleanError {
    CleanError::InvalidFormat(format!("Office 属性：{message}"))
}

struct Record {
    name: String,
    attributes: HashMap<String, String>,
    range: Range<usize>,
}

fn decoded_attributes(
    start: &BytesStart<'_>,
    reader: &NsReader<&[u8]>,
) -> Result<HashMap<String, String>> {
    let mut output = HashMap::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|_| invalid("XML 属性无效或重复"))?;
        if matches!(
            reader.resolver().resolve_attribute(attribute.key).0,
            ResolveResult::Unknown(_)
        ) {
            return Err(invalid("XML 属性命名空间未声明"));
        }
        let name =
            std::str::from_utf8(attribute.key.as_ref()).map_err(|_| invalid("XML 属性名称无效"))?;
        let value = attribute
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|_| invalid("XML 属性值无效"))?;
        output.insert(name.to_owned(), value.into_owned());
    }
    Ok(output)
}

fn records(xml: &str, root: &[u8], children: &[&[u8]], namespace: &[u8]) -> Result<Vec<Record>> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut depth = 0usize;
    let mut seen_root = false;
    let mut legacy_namespace = false;
    let mut output = Vec::new();
    let mut active_record = None;
    loop {
        let position = reader.buffer_position() as usize;
        let (resolved, event) = reader
            .read_resolved_event()
            .map_err(|_| invalid("包清单 XML 无效"))?;
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                if matches!(resolved, ResolveResult::Unknown(_)) {
                    return Err(invalid("包清单命名空间未声明"));
                }
                let matches_namespace = namespace_matches(&resolved, namespace);
                let unqualified = matches!(resolved, ResolveResult::Unbound);
                let mut attributes = decoded_attributes(start, &reader)?;
                if depth == 0 {
                    if seen_root || start.local_name().as_ref() != root {
                        return Err(invalid("包清单根元素无效"));
                    }
                    // Existing packages with unqualified legacy manifests remain readable.
                    legacy_namespace = unqualified;
                    if !legacy_namespace && !matches_namespace {
                        return Err(invalid("包清单命名空间无效"));
                    }
                    seen_root = true;
                } else if children.contains(&start.local_name().as_ref()) {
                    if depth != 1 || !(matches_namespace || legacy_namespace && unqualified) {
                        return Err(invalid("包清单记录位置或命名空间无效"));
                    }
                    if output.len() >= MAX_ENTRIES * 4 {
                        return Err(invalid("包清单记录过多"));
                    }
                    attributes.retain(|key, _| key != "xmlns" && !key.contains(':'));
                    output.push(Record {
                        name: std::str::from_utf8(start.local_name().as_ref())
                            .map_err(|_| invalid("包清单元素名称无效"))?
                            .to_owned(),
                        attributes,
                        range: position..reader.buffer_position() as usize,
                    });
                    if matches!(event, Event::Start(_)) {
                        active_record = Some(output.len() - 1);
                    }
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                if depth == 2 {
                    if let Some(index) = active_record.take() {
                        output[index].range.end = reader.buffer_position() as usize;
                    }
                }
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("包清单元素未正确闭合"))?;
            }
            Event::DocType(_) => return Err(invalid("包清单不可包含 DTD")),
            Event::GeneralRef(_) | Event::CData(_) => return Err(invalid("包清单包含非记录内容")),
            Event::Text(text) if text.iter().any(|byte| !byte.is_ascii_whitespace()) => {
                return Err(invalid("包清单包含非记录内容"))
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !seen_root || depth != 0 {
        return Err(invalid("包清单元素未正确闭合"));
    }
    Ok(output)
}

fn target_name(value: &str, relative: bool) -> Result<String> {
    if value.is_empty()
        || value.starts_with("//")
        || (!relative && !value.starts_with('/'))
        || value.bytes().any(|byte| {
            byte.is_ascii_whitespace()
                || byte.is_ascii_control()
                || matches!(byte, b'\\' | b'?' | b'#')
        })
        || value
            .trim_start_matches('/')
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
    {
        return Err(invalid("属性目标不是有效的包内部件 URI"));
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digits = bytes
                .get(index + 1..index + 3)
                .ok_or_else(|| invalid("属性 URI 转义无效"))?;
            let hex = |value: u8| (value as char).to_digit(16).map(|value| value as u8);
            let (Some(high), Some(low)) = (hex(digits[0]), hex(digits[1])) else {
                return Err(invalid("属性 URI 转义无效"));
            };
            let byte = high * 16 + low;
            if byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/' | b'\\')
                || byte.is_ascii_control()
            {
                return Err(invalid("属性 URI 包含不允许的转义"));
            }
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    std::str::from_utf8(&decoded).map_err(|_| invalid("属性 URI 编码无效"))?;
    let mut segments = Vec::new();
    for segment in value.trim_start_matches('/').split('/') {
        match segment {
            "." if relative => {}
            ".." if relative => {
                segments
                    .pop()
                    .ok_or_else(|| invalid("属性 URI 越过包根目录"))?;
            }
            "" | "." | ".." => return Err(invalid("属性 URI 路径无效")),
            _ if segment.ends_with('.') => return Err(invalid("属性 URI 路径不能以点结束")),
            _ => segments.push(segment),
        }
    }
    if segments.is_empty() {
        return Err(invalid("属性 URI 不指向部件"));
    }
    // OPC ZIP item names retain URI escaping; only the leading slash is omitted.
    Ok(segments.join("/").to_ascii_lowercase())
}

fn relative_target(value: &str, base: &str) -> Result<String> {
    if value.starts_with('/') || base.is_empty() {
        target_name(value, true)
    } else {
        // Validate the reference before adding the source directory so a scheme
        // cannot become a seemingly relative path after joining.
        if value
            .split('/')
            .next()
            .is_some_and(|part| part.contains(':'))
        {
            return Err(invalid("属性目标不是有效的包内部件 URI"));
        }
        target_name(&format!("{base}/{value}"), true)
    }
}

pub(super) fn remove_references(
    xml: &str,
    name: &str,
    removed: &HashSet<String>,
) -> Result<String> {
    let lower = name.replace('\\', "/").to_ascii_lowercase();
    let is_types = lower == "[content_types].xml";
    let (root, children, namespace): (&[u8], &[&[u8]], &[u8]) = if is_types {
        (b"Types", &[b"Default", b"Override"], TYPES_NAMESPACE)
    } else {
        (
            b"Relationships",
            &[b"Relationship"],
            RELATIONSHIPS_NAMESPACE,
        )
    };
    let base = if lower == "_rels/.rels" || is_types {
        ""
    } else if let Some((directory, _)) = lower.rsplit_once("/_rels/") {
        directory
    } else {
        return Err(invalid("包关系部件路径无效"));
    };
    let mut output = String::with_capacity(xml.len());
    let mut copied = 0;
    for record in records(xml, root, children, namespace)? {
        let target = if is_types {
            record
                .attributes
                .get("PartName")
                .map(|value| target_name(value, false))
        } else if record
            .attributes
            .get("TargetMode")
            .is_some_and(|mode| mode == "External")
        {
            None
        } else {
            record
                .attributes
                .get("Target")
                .map(|value| relative_target(value, base))
        };
        if target
            .transpose()?
            .is_some_and(|target| removed.contains(&target))
        {
            output.push_str(&xml[copied..record.range.start]);
            copied = record.range.end;
        }
    }
    output.push_str(&xml[copied..]);
    Ok(output)
}

fn insert_kind(
    parts: &mut HashMap<String, PropertyKind>,
    name: String,
    kind: PropertyKind,
) -> Result<()> {
    if parts
        .insert(name, kind)
        .is_some_and(|previous| previous != kind)
    {
        return Err(invalid("属性部件类型相互冲突"));
    }
    Ok(())
}

pub(super) fn discover(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    extension: &str,
) -> Result<HashMap<String, PropertyKind>> {
    let mut parts = HashMap::new();
    if !matches!(extension, "docx" | "xlsx" | "pptx") {
        return Ok(parts);
    }
    let names: HashSet<String> = archive
        .file_names()
        .map(|name| name.replace('\\', "/").to_ascii_lowercase())
        .collect();
    let types = read_utf8_bounded(
        &mut archive.by_name("[Content_Types].xml")?,
        MAX_MANIFEST_BYTES,
        "Office 内容类型清单",
    )?;
    let mut defaults = HashMap::new();
    let mut overrides = HashMap::new();
    for record in records(
        &types,
        b"Types",
        &[b"Default", b"Override"],
        TYPES_NAMESPACE,
    )? {
        let Some(content_type) = record.attributes.get("ContentType") else {
            continue;
        };
        let key = if record.name == "Default" {
            record
                .attributes
                .get("Extension")
                .ok_or_else(|| invalid("默认内容类型缺少扩展名"))?
                .to_ascii_lowercase()
        } else {
            target_name(
                record
                    .attributes
                    .get("PartName")
                    .ok_or_else(|| invalid("内容类型缺少部件名"))?,
                false,
            )?
        };
        let map = if record.name == "Default" {
            &mut defaults
        } else {
            &mut overrides
        };
        if key.is_empty() || map.insert(key, content_type.clone()).is_some() {
            return Err(invalid("内容类型声明无效或重复"));
        }
    }
    for name in &names {
        let content_type = overrides.get(name).or_else(|| {
            name.rsplit_once('.')
                .and_then(|(_, extension)| defaults.get(extension))
        });
        if let Some(kind) = content_type.and_then(|value| PropertyKind::content_type(value)) {
            insert_kind(&mut parts, name.clone(), kind)?;
        }
    }
    for (name, content_type) in &overrides {
        if PropertyKind::content_type(content_type).is_some() && !names.contains(name) {
            return Err(invalid("声明的属性部件不存在"));
        }
    }
    let relationships = match archive.by_name("_rels/.rels") {
        Ok(mut file) => read_utf8_bounded(&mut file, MAX_MANIFEST_BYTES, "Office 包关系清单")?,
        Err(zip::result::ZipError::FileNotFound) => return Ok(parts),
        Err(error) => return Err(error.into()),
    };
    let mut ids = HashSet::new();
    for record in records(
        &relationships,
        b"Relationships",
        &[b"Relationship"],
        RELATIONSHIPS_NAMESPACE,
    )? {
        if let Some(id) = record.attributes.get("Id") {
            if id.is_empty() || !ids.insert(id.clone()) {
                return Err(invalid("包关系 ID 无效或重复"));
            }
        }
        let Some(kind) = record
            .attributes
            .get("Type")
            .and_then(|value| PropertyKind::relationship(value))
        else {
            continue;
        };
        if !record.attributes.contains_key("Id") {
            return Err(invalid("属性关系缺少 ID"));
        }
        if record
            .attributes
            .get("TargetMode")
            .is_some_and(|mode| mode != "Internal")
        {
            return Err(invalid("属性关系不能指向包外部"));
        }
        let name = target_name(
            record
                .attributes
                .get("Target")
                .ok_or_else(|| invalid("属性关系缺少目标"))?,
            true,
        )?;
        if !names.contains(&name) {
            return Err(invalid("属性关系目标不存在"));
        }
        if let Some(content_type) = overrides.get(&name) {
            if PropertyKind::content_type(content_type) != Some(kind) {
                return Err(invalid("属性关系与内容类型相互冲突"));
            }
        }
        insert_kind(&mut parts, name, kind)?;
    }
    Ok(parts)
}

pub(super) fn scrub(xml: &str, kind: PropertyKind) -> Result<(String, usize)> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut root = None;
    let mut depth = 0usize;
    let mut payload = false;
    let mut declaration = None;
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|_| invalid("属性 XML 无效"))?;
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                if matches!(namespace, ResolveResult::Unknown(_)) {
                    return Err(invalid("属性 XML 命名空间未声明"));
                }
                let (local, default_namespace) = kind.root();
                let expected = kind
                    .strict_namespace()
                    .filter(|strict| namespace_matches(&namespace, strict))
                    .unwrap_or(default_namespace);
                let matches_namespace = namespace_matches(&namespace, expected);
                let attributes = decoded_attributes(start, &reader)?;
                if depth == 0 {
                    if root.is_some()
                        || start.local_name().as_ref() != local.as_bytes()
                        || !matches_namespace
                    {
                        return Err(invalid("属性部件的根元素或类型不匹配"));
                    }
                    let name = std::str::from_utf8(start.name().as_ref())
                        .map_err(|_| invalid("属性根元素名称无效"))?
                        .to_owned();
                    let binding = name
                        .split_once(':')
                        .map_or("xmlns".to_owned(), |(prefix, _)| format!("xmlns:{prefix}"));
                    payload |= attributes.keys().any(|key| key != &binding);
                    root = Some((name, expected));
                } else {
                    payload = true;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("属性 XML 元素未正确闭合"))?;
            }
            Event::Text(text) => {
                let text = std::str::from_utf8(text.as_ref())
                    .map_err(|_| invalid("属性文本不是 UTF-8"))?;
                quick_xml::escape::unescape(text).map_err(|_| invalid("属性文本转义无效"))?;
                if !text.trim().is_empty() {
                    if depth == 0 {
                        return Err(invalid("属性根元素外存在文本"));
                    }
                    payload = true;
                }
            }
            Event::CData(_) if depth == 0 => return Err(invalid("属性根元素外存在文本")),
            Event::Comment(_) | Event::CData(_) | Event::PI(_) => payload = true,
            Event::GeneralRef(reference) => {
                let name = std::str::from_utf8(reference.as_ref())
                    .map_err(|_| invalid("属性文本转义无效"))?;
                quick_xml::escape::unescape(&format!("&{name};"))
                    .map_err(|_| invalid("属性文本转义无效"))?;
                if depth == 0 {
                    return Err(invalid("属性根元素外存在文本"));
                }
                payload = true;
            }
            Event::DocType(_) => return Err(invalid("属性 XML 不可包含 DTD")),
            Event::Decl(decl) => {
                if root.is_some() || declaration.is_some() {
                    return Err(invalid("属性 XML 声明位置无效或重复"));
                }
                declaration = Some(decl.clone().into_owned());
            }
            Event::Eof => break,
        }
    }
    let (root, namespace) = root.ok_or_else(|| invalid("属性 XML 缺少根元素"))?;
    if depth != 0 {
        return Err(invalid("属性 XML 元素未正确闭合"));
    }
    if !payload {
        return Ok((xml.to_owned(), 0));
    }
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    if let Some(declaration) = declaration {
        writer.write_event(Event::Decl(declaration))?;
    }
    let attribute = root
        .split_once(':')
        .map_or("xmlns".to_owned(), |(prefix, _)| format!("xmlns:{prefix}"));
    let mut empty = BytesStart::new(root);
    empty.push_attribute((
        attribute.as_str(),
        std::str::from_utf8(namespace).expect("property namespace is ASCII"),
    ));
    writer.write_event(Event::Empty(empty))?;
    let output = String::from_utf8(writer.into_inner().into_inner())
        .map_err(|_| invalid("属性输出不是 UTF-8"))?;
    Ok((output, 1))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::*;

    const CORE_TYPE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
    const CORE_REL: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
    const CORE_XML: &str = r#"<c:coreProperties xmlns:c="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:d="http://purl.org/dc/elements/1.1/"><d:creator>Private author</d:creator></c:coreProperties>"#;

    fn package(types: &str, relationships: &str, entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [
            ("[Content_Types].xml", types.as_bytes()),
            ("_rels/.rels", relationships.as_bytes()),
            (
                "word/document.xml",
                b"<document>Visible text</document>".as_slice(),
            ),
        ]
        .into_iter()
        .chain(entries.iter().copied())
        {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn resolves_internal_uris_without_decoding_zip_item_names() {
        for (uri, expected) in [
            (
                "/Properties/Private%20Author.props",
                "properties/private%20author.props",
            ),
            (
                "properties/./folder/../private.props",
                "properties/private.props",
            ),
            ("properties/%E4%B8%AD.props", "properties/%e4%b8%ad.props"),
        ] {
            assert_eq!(target_name(uri, true).unwrap(), expected);
        }
        assert_eq!(
            relative_target("../private.props", "word/subdir").unwrap(),
            "word/private.props"
        );
        for uri in [
            "",
            "//host/file",
            "https://host/file",
            "../private.props",
            "/",
            "file\\part",
            "file#part",
            "file?part",
            "file name",
            "file%",
            "file%zz",
            "file%2Fpart",
            "file%5cpart",
            "file%2Eprops",
            "file%00",
            "file%ff",
            "file.",
        ] {
            assert!(target_name(uri, true).is_err(), "{uri}");
        }
        assert!(target_name("properties/./private.props", false).is_err());
        assert!(target_name("/properties/../private.props", false).is_err());
        assert!(relative_target("https://host/file", "word").is_err());
    }

    #[test]
    fn scrubs_all_property_payloads_and_preserves_the_root_namespace() {
        for kind in [
            PropertyKind::Core,
            PropertyKind::Extended,
            PropertyKind::Custom,
        ] {
            let (local, namespace) = kind.root();
            let namespace = std::str::from_utf8(namespace).unwrap();
            let source = format!(
                r#"<?xml version="1.0"?><!--Private comment--><p:{local} xmlns:p="{namespace}" xmlns:u="urn:Private namespace" secret="Private attribute"><?private value?><u:field name="Private name"><![CDATA[Private value]]>&amp;&#65;</u:field></p:{local}>"#
            );
            let (cleaned, count) = scrub(&source, kind).unwrap();
            assert_eq!(count, 1);
            assert!(!cleaned.contains("Private"));
            assert!(cleaned.contains(&format!("<p:{local} xmlns:p=\"{namespace}\"/>")));
            assert_eq!(scrub(&cleaned, kind).unwrap(), (cleaned, 0));
        }
    }

    #[test]
    fn preserves_strict_property_namespaces_and_recognizes_their_relationships() {
        for (kind, role) in [
            (PropertyKind::Extended, "extended"),
            (PropertyKind::Custom, "custom"),
        ] {
            let namespace = std::str::from_utf8(kind.strict_namespace().unwrap()).unwrap();
            let xml = format!(
                r#"<Properties xmlns="{namespace}"><field>Private author</field></Properties>"#
            );
            let relationships = format!(
                r#"<Relationships><Relationship Id="p" Type="http://purl.oclc.org/ooxml/officeDocument/relationships/{role}Properties" Target="properties/strict.props"/></Relationships>"#
            );
            let source = package(
                "<Types/>",
                &relationships,
                &[("properties/strict.props", xml.as_bytes())],
            );
            assert!(!super::super::inspect(&source, "docx").unwrap().is_empty());
            let (cleaned, _) = super::super::clean(&source, "docx").unwrap();
            assert!(super::super::inspect(&cleaned, "docx").unwrap().is_empty());
            let mut archive = ZipArchive::new(Cursor::new(&cleaned)).unwrap();
            let mut output = String::new();
            archive
                .by_name("properties/strict.props")
                .unwrap()
                .read_to_string(&mut output)
                .unwrap();
            assert_eq!(output, format!(r#"<Properties xmlns="{namespace}"/>"#));
        }
    }

    #[test]
    fn removes_private_unused_namespace_bindings_and_entity_only_payloads() {
        for suffix in [
            r#" xmlns:private="urn:Private namespace"/>"#,
            ">&amp;</c:coreProperties>",
            ">&#65;</c:coreProperties>",
        ] {
            let source = format!(
                r#"<c:coreProperties xmlns:c="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"{suffix}"#
            );
            let (cleaned, count) = scrub(&source, PropertyKind::Core).unwrap();
            assert_eq!(count, 1);
            assert_eq!(scrub(&cleaned, PropertyKind::Core).unwrap().1, 0);
            assert!(!cleaned.contains("Private"));
        }
    }

    #[test]
    fn refuses_malformed_or_misidentified_property_xml() {
        let header = r#"<c:coreProperties xmlns:c="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">"#;
        for suffix in [
            "<field>",
            "</wrong>",
            "<u:field/></c:coreProperties>",
            "<field u:attr='value'/></c:coreProperties>",
            "<field a='1' a='2'/></c:coreProperties>",
            "<field a='&unknown;'/></c:coreProperties>",
            "&unknown;</c:coreProperties>",
            "</c:coreProperties><another/>",
            "</c:coreProperties>text",
            "</c:coreProperties><![CDATA[text]]>",
            "<?xml version='1.0'?></c:coreProperties>",
        ] {
            assert!(
                scrub(&format!("{header}{suffix}"), PropertyKind::Core).is_err(),
                "{suffix}"
            );
        }
        assert!(scrub(
            &format!("<!DOCTYPE c [<!ENTITY private 'author'>]>{CORE_XML}"),
            PropertyKind::Core
        )
        .is_err());
        assert!(scrub(CORE_XML, PropertyKind::Custom).is_err());
        assert!(scrub(
            "<Properties xmlns='urn:unrelated'><creator>Visible</creator></Properties>",
            PropertyKind::Custom
        )
        .is_err());
    }

    #[test]
    fn discovers_default_types_and_escaped_targets_through_namespaced_manifests() {
        let types = format!(
            r#"<t:Types xmlns:t="http://schemas.openxmlformats.org/package/2006/content-types"><t:Default Extension="props" ContentType="{CORE_TYPE}"/></t:Types>"#
        );
        let relationships = format!(
            r#"<r:Relationships xmlns:r="http://schemas.openxmlformats.org/package/2006/relationships"><r:Relationship Id="p" Type="{CORE_REL}" Target="./Properties/Private%20Author.props"/></r:Relationships>"#
        );
        let source = package(
            &types,
            &relationships,
            &[("Properties/Private%20Author.props", CORE_XML.as_bytes())],
        );
        let (cleaned, _) = super::super::clean(&source, "docx").unwrap();
        assert!(super::super::inspect(&cleaned, "docx").unwrap().is_empty());
        let mut archive = ZipArchive::new(Cursor::new(&cleaned)).unwrap();
        assert!(archive.by_name("Properties/Private%20Author.props").is_ok());
        assert!(archive.by_name("Properties/Private Author.props").is_err());
    }

    #[test]
    fn refuses_ambiguous_missing_external_and_unreadable_properties() {
        let declaration =
            format!(r#"<Override PartName="/private.props" ContentType="{CORE_TYPE}"/>"#);
        let relationship =
            format!(r#"<Relationship Id="p" Type="{CORE_REL}" Target="private.props"/>"#);
        let types = format!("<Types>{declaration}</Types>");
        let rels = format!("<Relationships>{relationship}</Relationships>");
        for (types, rels, entries) in [
            (
                format!("<Types>{declaration}{declaration}</Types>"),
                rels.clone(),
                vec![("private.props", CORE_XML.as_bytes())],
            ),
            (types.clone(), rels.clone(), vec![]),
            (
                types.clone(),
                format!("<Relationships>{relationship}{relationship}</Relationships>"),
                vec![("private.props", CORE_XML.as_bytes())],
            ),
            (
                types.clone(),
                rels.replace("Target=", "TargetMode='External' Target="),
                vec![("private.props", CORE_XML.as_bytes())],
            ),
            (
                types.clone(),
                rels.clone(),
                vec![("private.props", b"\xff".as_slice())],
            ),
            (
                types.replace(CORE_TYPE, "application/xml"),
                rels.clone(),
                vec![("private.props", CORE_XML.as_bytes())],
            ),
            (
                types.clone(),
                rels.replace("private.props", "../private.props"),
                vec![("private.props", CORE_XML.as_bytes())],
            ),
        ] {
            let source = package(&types, &rels, &entries);
            assert!(super::super::inspect(&source, "docx").is_err());
            assert!(super::super::clean(&source, "docx").is_err());
        }
    }

    #[test]
    fn rejects_malformed_manifest_records_and_keeps_legacy_empty_manifests() {
        for xml in [
            "<Types xmlns='urn:unrelated'/>",
            "<Types><group><Override/></group></Types>",
            "<Types><x:Override/></Types>",
            "<Types><Override PartName='one' PartName='two'/></Types>",
            "<!DOCTYPE Types><Types/>",
            "<Types>private text</Types>",
            "<Types/ ><Types/>",
        ] {
            assert!(
                records(xml, b"Types", &[b"Override"], TYPES_NAMESPACE).is_err(),
                "{xml}"
            );
        }
        assert!(
            records("<Types/>", b"Types", &[b"Override"], TYPES_NAMESPACE)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn prunes_only_references_to_removed_parts_with_relative_and_rooted_targets() {
        let removed = HashSet::from([
            "word/comments.xml".to_owned(),
            "customxml/private.xml".to_owned(),
        ]);
        let keep = r#"<r:Relationship Id="visible-comments" Type="urn:comments" Target="comments-summary.xml"/><r:Relationship Id="external" TargetMode="External" Target="https://host/comments.xml"/>"#;
        let source = format!(
            r#"<r:Relationships xmlns:r="http://schemas.openxmlformats.org/package/2006/relationships"><r:Relationship Id="a" Target="./comments.xml"></r:Relationship>{keep}<r:Relationship Id="b" Target="../customXml/private.xml"/></r:Relationships>"#
        );
        assert_eq!(
            remove_references(&source, "word/_rels/document.xml.rels", &removed).unwrap(),
            format!(
                r#"<r:Relationships xmlns:r="http://schemas.openxmlformats.org/package/2006/relationships">{keep}</r:Relationships>"#
            )
        );
        let types = r#"<Types><Override PartName="/word/comments.xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/comments-summary.xml"/></Types>"#;
        assert_eq!(
            remove_references(types, "[Content_Types].xml", &removed).unwrap(),
            r#"<Types><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/comments-summary.xml"/></Types>"#
        );
    }

    #[test]
    fn retains_identified_properties_even_when_their_paths_resemble_removed_parts() {
        for name in [
            "customXml/private.props",
            "word/comments-private.props",
            "properties/rights.xml",
        ] {
            let types = format!(
                r#"<Types><Override PartName="/{name}" ContentType="{CORE_TYPE}"/><Override PartName="/word/comments.xml" ContentType="application/xml"/></Types>"#
            );
            let keep = format!(r#"<Relationship Id="p" Type="{CORE_REL}" Target="{name}"/>"#);
            let rels = format!(
                r#"<Relationships>{keep}<Relationship Id="comment" Type="urn:comments" Target="word/comments.xml"/></Relationships>"#
            );
            let source = package(
                &types,
                &rels,
                &[
                    (name, CORE_XML.as_bytes()),
                    ("word/comments.xml", b"Private note"),
                    ("payload/comments-summary.xml", b"<visible>Keep</visible>"),
                ],
            );
            let (cleaned, _) = super::super::clean(&source, "docx").unwrap();
            assert!(super::super::inspect(&cleaned, "docx").unwrap().is_empty());
            let mut archive = ZipArchive::new(Cursor::new(&cleaned)).unwrap();
            assert!(archive.by_name(name).is_ok());
            assert!(archive.by_name("word/comments.xml").is_err());
            let mut relationships = String::new();
            archive
                .by_name("_rels/.rels")
                .unwrap()
                .read_to_string(&mut relationships)
                .unwrap();
            assert_eq!(
                relationships,
                format!("<Relationships>{keep}</Relationships>")
            );
            let mut visible = String::new();
            archive
                .by_name("payload/comments-summary.xml")
                .unwrap()
                .read_to_string(&mut visible)
                .unwrap();
            assert_eq!(visible, "<visible>Keep</visible>");
        }
    }
}
