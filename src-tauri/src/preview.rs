use quick_xml::{events::Event, Reader};
use serde::Serialize;

use crate::error::{CleanError, Result};

const MAX_FIELDS: usize = 128;
const MAX_VALUE_CHARS: usize = 2048;
const MAX_XML_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataField {
    source: String,
    name: String,
    value: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPreview {
    pub fields: Vec<MetadataField>,
    pub truncated: bool,
}

impl MetadataPreview {
    pub fn push(&mut self, source: &str, name: &str, value: &str) {
        if value.trim().is_empty() {
            return;
        }
        if self.fields.len() >= MAX_FIELDS {
            self.truncated = true;
            return;
        }
        let value = value.trim();
        self.truncated |= value.chars().count() > MAX_VALUE_CHARS
            || source.chars().count() > 256
            || name.chars().count() > 256;
        self.fields.push(MetadataField {
            source: source.chars().take(256).collect(),
            name: name.chars().take(256).collect(),
            value: value.chars().take(MAX_VALUE_CHARS).collect(),
        });
    }

    /// Read only explicitly identified metadata parts, never document body XML.
    pub fn xml(&mut self, source: &str, xml: &str) -> Result<()> {
        if xml.len() > MAX_XML_BYTES {
            return Err(CleanError::InvalidFormat(
                "Metadata preview XML is too large".into(),
            ));
        }
        let mut reader = Reader::from_str(xml);
        reader.config_mut().check_end_names = true;
        let mut stack: Vec<(String, String)> = Vec::new();
        loop {
            let event = reader.read_event().map_err(|_| invalid_xml())?;
            let empty = matches!(&event, Event::Empty(_));
            match event {
                Event::Start(start) | Event::Empty(start) => {
                    if stack.len() >= 64 {
                        return Err(invalid_xml());
                    }
                    let mut name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
                    for attr in start.attributes() {
                        let attr = attr.map_err(|_| invalid_xml())?;
                        let key = String::from_utf8_lossy(attr.key.as_ref());
                        if key == "xmlns" || key.starts_with("xmlns:") {
                            continue;
                        }
                        let value = attr
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|_| invalid_xml())?;
                        self.push(source, &format!("{name}/@{key}"), &value);
                        if key == "name" {
                            name =
                                format!("{name}[{}]", value.chars().take(128).collect::<String>());
                        }
                    }
                    if !empty {
                        stack.push((name, String::new()));
                    }
                }
                Event::Text(text) => {
                    let decoded = text.decode().map_err(|_| invalid_xml())?;
                    let decoded =
                        quick_xml::escape::unescape(&decoded).map_err(|_| invalid_xml())?;
                    append_text(&mut stack, &decoded, &mut self.truncated);
                }
                Event::CData(text) => {
                    let decoded = text.decode().map_err(|_| invalid_xml())?;
                    append_text(&mut stack, &decoded, &mut self.truncated);
                }
                Event::GeneralRef(reference) => {
                    let encoded = format!("&{};", String::from_utf8_lossy(reference.as_ref()));
                    let value = quick_xml::escape::unescape(&encoded).map_err(|_| invalid_xml())?;
                    append_text(&mut stack, &value, &mut self.truncated);
                }
                Event::End(_) => {
                    let (name, value) = stack.pop().ok_or_else(invalid_xml)?;
                    let mut names: Vec<&str> = stack
                        .iter()
                        .skip(1)
                        .map(|(name, _)| name.as_str())
                        .collect();
                    names.push(&name);
                    self.push(source, &names.join("/"), &value);
                }
                Event::DocType(_) => return Err(invalid_xml()),
                Event::Eof => break,
                _ => {}
            }
        }
        if !stack.is_empty() {
            return Err(invalid_xml());
        }
        Ok(())
    }
}

fn invalid_xml() -> CleanError {
    CleanError::InvalidFormat("Invalid metadata preview XML".into())
}

fn append_text(stack: &mut [(String, String)], text: &str, truncated: &mut bool) {
    if let Some((_, value)) = stack.last_mut() {
        let remaining = MAX_VALUE_CHARS.saturating_sub(value.chars().count());
        *truncated |= text.chars().count() > remaining;
        value.extend(text.chars().take(remaining));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn metadata_preview_decodes_text_and_bounds_output() {
        let mut preview = MetadataPreview::default();
        preview.xml("properties", "<properties><empty/><creator>张三 &amp; 李四</creator><title><![CDATA[<Report>]]></title></properties>").unwrap();
        assert_eq!(preview.fields.len(), 2);
        assert_eq!(preview.fields[0].value, "张三 & 李四");
        assert_eq!(preview.fields[1].value, "<Report>");
        preview.push("test", "long", &"中".repeat(3000));
        assert_eq!(preview.fields[2].value.chars().count(), 2048);
        for _ in 0..200 {
            preview.push("test", "author", "private");
        }
        assert_eq!(preview.fields.len(), 128);
        assert!(preview.truncated);
    }

    #[test]
    fn metadata_preview_rejects_dtd_malformed_and_deep_xml() {
        for xml in [
            "<!DOCTYPE x [<!ENTITY secret SYSTEM 'file:///secret'>]><x>&secret;</x>".to_owned(),
            "<x><a>broken</x>".to_owned(),
            "<x>".repeat(65) + &"</x>".repeat(65),
        ] {
            assert!(MetadataPreview::default().xml("test", &xml).is_err());
        }
    }

    #[test]
    fn metadata_preview_reads_relocated_office_properties_without_body_text() {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, value) in [
            (
                "[Content_Types].xml",
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/hidden/attributes.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#,
            ),
            (
                "word/document.xml",
                "<document>PRIVATE BODY MUST NOT APPEAR</document>",
            ),
            (
                "hidden/attributes.xml",
                r#"<c:coreProperties xmlns:c="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:d="http://purl.org/dc/elements/1.1/"><d:creator>张三</d:creator><d:title>试验标题</d:title></c:coreProperties>"#,
            ),
        ] {
            archive
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(value.as_bytes()).unwrap();
        }
        let data = archive.finish().unwrap().into_inner();
        let preview = crate::cleaners::office::preview(&data, "docx").unwrap();
        assert!(preview.fields.iter().any(|field| field.value == "张三"));
        assert!(preview.fields.iter().any(|field| field.value == "试验标题"));
        let json = serde_json::to_string(&preview).unwrap();
        assert!(!json.contains("PRIVATE BODY"));
        let (cleaned, _) = crate::cleaners::office::clean(&data, "docx").unwrap();
        assert!(crate::cleaners::office::preview(&cleaned, "docx")
            .unwrap()
            .fields
            .is_empty());
    }

    #[test]
    fn metadata_preview_reads_pdf_unicode_info_and_xmp() {
        let mut document = lopdf::Document::with_version("1.7");
        let info = document.add_object(lopdf::dictionary! {
            "Author" => lopdf::text_string("张三"),
            "Title" => lopdf::Object::string_literal("Report"),
        });
        document.trailer.set("Info", info);
        let metadata = document.add_object(lopdf::Stream::new(
            lopdf::dictionary! {"Type" => "Metadata", "Subtype" => "XML"},
            b"<xmp><creator>Alice &amp; Bob</creator></xmp>".to_vec(),
        ));
        let root =
            document.add_object(lopdf::dictionary! {"Type" => "Catalog", "Metadata" => metadata});
        document.trailer.set("Root", root);
        let mut data = Vec::new();
        document.save_to(&mut data).unwrap();
        let preview = crate::cleaners::pdf::preview(&data).unwrap();
        assert!(preview.fields.iter().any(|field| field.value == "张三"));
        assert!(preview
            .fields
            .iter()
            .any(|field| field.value == "Alice & Bob"));
        assert!(crate::cleaners::pdf::preview(b"not a PDF").is_err());
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("preview.pdf");
        std::fs::write(&path, &data).unwrap();
        assert_eq!(
            serde_json::to_string(&crate::engine::preview_metadata(&path).unwrap()).unwrap(),
            serde_json::to_string(&preview).unwrap()
        );
        assert_eq!(std::fs::read(&path).unwrap(), data);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn metadata_preview_does_not_write_files_or_expose_unsupported_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private.txt");
        std::fs::write(&path, b"private body").unwrap();
        assert!(crate::engine::preview_metadata(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"private body");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
        assert!(crate::engine::preview_metadata(&temp.path().join("missing.pdf")).is_err());
    }

    #[test]
    fn metadata_preview_reads_odf_metadata_but_not_document_content() {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(
                "mimetype",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        archive
            .write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();
        for (name, value) in [
            ("content.xml", "<body>DO NOT PREVIEW DOCUMENT BODY</body>"),
            (
                "meta.xml",
                r#"<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/"><office:meta><dc:creator>ODF author</dc:creator><dc:title>ODF title</dc:title></office:meta></office:document-meta>"#,
            ),
        ] {
            archive
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(value.as_bytes()).unwrap();
        }
        let data = archive.finish().unwrap().into_inner();
        let result = crate::cleaners::office::preview(&data, "odt").unwrap();
        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].value, "ODF author");
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("DOCUMENT BODY"));
    }
}
