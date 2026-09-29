use std::{borrow::Cow, io::Cursor};

use lopdf::{Dictionary, Document, LoadOptions, Object, Stream};

use crate::{
    error::Result,
    models::{Finding, FindingSeverity},
};

use super::image;

const MAX_PDF_DECOMPRESSED_STREAM_BYTES: usize = 64 * 1024 * 1024;
const MAX_PDF_OBJECT_DEPTH: usize = 64;

fn load_document(data: &[u8]) -> Result<Document> {
    Ok(Document::load_mem_with_options(
        data,
        LoadOptions::with_max_decompressed_size(MAX_PDF_DECOMPRESSED_STREAM_BYTES),
    )?)
}

fn jpeg_stream(stream: &Stream) -> Result<Option<Cow<'_, [u8]>>> {
    if !name_is(&stream.dict, b"Subtype", b"Image") {
        return Ok(None);
    }
    let Ok(filter) = stream.dict.get(b"Filter") else {
        return Ok(None);
    };
    let filters: Vec<&[u8]> = if let Ok(name) = filter.as_name() {
        vec![name]
    } else {
        filter
            .as_array()?
            .iter()
            .map(Object::as_name)
            .collect::<lopdf::Result<_>>()?
    };
    let dct = filters
        .iter()
        .filter(|name| matches!(**name, b"DCTDecode" | b"DCT"))
        .count();
    if dct == 0 {
        return Ok(None);
    }
    let content = if filters.len() == 1 {
        Cow::Borrowed(stream.content.as_slice())
    } else if filters.len() == 2
        && matches!(filters[0], b"ASCII85Decode" | b"A85")
        && matches!(filters[1], b"DCTDecode" | b"DCT")
    {
        Cow::Owned(decode_ascii85_jpeg(
            stream,
            MAX_PDF_DECOMPRESSED_STREAM_BYTES,
        )?)
    } else {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF JPEG 图像使用复合过滤器，无法证明其元数据已清理".into(),
        ));
    };
    if !content.starts_with(&[0xff, 0xd8, 0xff]) {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF DCT 图像流缺少 JPEG 签名".into(),
        ));
    }
    Ok(Some(content))
}

fn decode_ascii85_jpeg(stream: &Stream, limit: usize) -> Result<Vec<u8>> {
    // Only unwrap a known lossless transport filter. Do not decode JPEG pixels
    // or guess how non-default filter parameters map to a rewritten chain.
    let default_params = match stream.dict.get(b"DecodeParms") {
        Err(_) | Ok(Object::Null) => true,
        Ok(Object::Array(params)) => {
            params.len() == 2 && params.iter().all(|param| matches!(param, Object::Null))
        }
        _ => false,
    };
    if !default_params {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF ASCII85 JPEG 图像包含不支持的过滤器参数".into(),
        ));
    }
    // lopdf tolerates invalid characters and uses u32 arithmetic internally.
    // Validate complete groups first so malformed input cannot truncate data
    // silently or overflow that decoder's arithmetic.
    let invalid = || crate::error::CleanError::InvalidFormat("PDF ASCII85 图像编码无效".into());
    let encoded = stream.content.trim_ascii_end();
    let encoded = encoded.strip_suffix(b"~>").ok_or_else(invalid)?;
    let mut value = 0u64;
    let mut digits = 0;
    for &byte in encoded {
        match byte {
            b'z' if digits == 0 => continue,
            byte if byte.is_ascii_whitespace() => continue,
            b'!'..=b'u' => {
                value = value * 85 + u64::from(byte - b'!');
                digits += 1;
                if digits == 5 {
                    if value > u64::from(u32::MAX) {
                        return Err(invalid());
                    }
                    digits = 0;
                    value = 0;
                }
            }
            _ => return Err(invalid()),
        }
    }
    if digits == 1 {
        return Err(invalid());
    }
    if digits > 1 {
        for _ in digits..5 {
            value = value * 85 + 84;
        }
        if value > u64::from(u32::MAX) {
            return Err(invalid());
        }
    }
    let mut transport = stream.clone();
    transport.dict.set("Filter", "ASCII85Decode");
    transport.dict.remove(b"DecodeParms");
    Ok(transport.decompressed_content_with_limit(limit)?)
}

fn embedded_image_findings(document: &Document) -> Result<usize> {
    let mut count = 0;
    for object in document.objects.values() {
        let Object::Stream(stream) = object else {
            continue;
        };
        if let Some(content) = jpeg_stream(stream)? {
            if !image::inspect_jpeg(&content)?.is_empty() {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn name_is(dictionary: &Dictionary, key: &[u8], expected: &[u8]) -> bool {
    dictionary
        .get(key)
        .is_ok_and(|value| value.as_name().is_ok_and(|name| name == expected))
}

fn is_annotation(dictionary: &Dictionary) -> bool {
    name_is(dictionary, b"Type", b"Annot")
        || dictionary.get(b"Rect").is_ok() && dictionary.get(b"Subtype").is_ok()
}

fn annotation_metadata_keys(dictionary: &Dictionary) -> &'static [&'static [u8]] {
    if !is_annotation(dictionary) {
        &[]
    } else if name_is(dictionary, b"Subtype", b"Widget") {
        // A merged field/widget uses T as its logical field name, not an author.
        &[b"M", b"CreationDate"]
    } else {
        &[b"T", b"M", b"CreationDate"]
    }
}

fn private_dictionary_count(dictionary: &Dictionary, depth: usize) -> Result<usize> {
    if depth > MAX_PDF_OBJECT_DEPTH {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF 直接对象嵌套超过 64 层安全上限".into(),
        ));
    }
    let mut count = [b"Metadata".as_slice(), b"PieceInfo", b"LastModified"]
        .iter()
        .filter(|key| dictionary.get(key).is_ok())
        .count();
    count += annotation_metadata_keys(dictionary)
        .iter()
        .filter(|key| dictionary.get(key).is_ok())
        .count();
    for (key, value) in dictionary.iter() {
        if key == b"Params" {
            if let Ok(params) = value.as_dict() {
                count += [b"CreationDate".as_slice(), b"ModDate", b"CheckSum"]
                    .iter()
                    .filter(|name| params.get(name).is_ok())
                    .count();
            }
        }
        count += private_object_count(value, depth + 1)?;
    }
    Ok(count)
}

fn private_object_count(object: &Object, depth: usize) -> Result<usize> {
    if depth > MAX_PDF_OBJECT_DEPTH {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF 直接对象嵌套超过 64 层安全上限".into(),
        ));
    }
    match object {
        Object::Dictionary(dictionary) => private_dictionary_count(dictionary, depth + 1),
        Object::Stream(stream) => private_dictionary_count(&stream.dict, depth + 1),
        Object::Array(values) => values.iter().try_fold(0usize, |count, value| {
            Ok(count + private_object_count(value, depth + 1)?)
        }),
        _ => Ok(0),
    }
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let document = load_document(data)?;
    let mut count = 0;
    if document.trailer.get(b"Info").is_ok() {
        count += 1;
    }
    if document.trailer.get(b"ID").is_ok() {
        count += 1;
    }
    for object in document.objects.values() {
        count += private_object_count(object, 0)?;
        if let Object::Stream(stream) = object {
            if name_is(&stream.dict, b"Type", b"Metadata") {
                count += 1;
            }
        }
    }
    count += embedded_image_findings(&document)?;
    Ok(if count > 0 {
        vec![Finding {
            category: "pdf_metadata".into(),
            label: "PDF 文档属性 / XMP / 内嵌图片".into(),
            count,
            severity: FindingSeverity::Privacy,
        }]
    } else {
        Vec::new()
    })
}

pub fn clean(data: &[u8]) -> Result<(Vec<u8>, Vec<Finding>)> {
    let findings = inspect(data)?;
    let mut document = load_document(data)?;
    let info_id = document
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|value| value.as_reference().ok());
    document.trailer.remove(b"Info");
    document.trailer.remove(b"ID");
    if let Some(info_id) = info_id {
        document.objects.remove(&info_id);
    }
    let metadata_ids: Vec<_> = document
        .objects
        .iter()
        .filter_map(|(id, object)| match object {
            Object::Stream(stream)
                if stream
                    .dict
                    .get(b"Type")
                    .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Metadata")) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    for object in document.objects.values_mut() {
        scrub_object(object, 0)?;
        if let Object::Stream(stream) = object {
            if let Some(content) = jpeg_stream(stream)? {
                let (cleaned, removed) = image::clean_jpeg_with_options(&content, true, true)?;
                if !removed.is_empty() && cleaned.as_slice() != content.as_ref() {
                    if matches!(content, Cow::Owned(_)) {
                        stream.dict.set("Filter", "DCTDecode");
                        stream.dict.remove(b"DecodeParms");
                    }
                    stream.set_content(cleaned);
                }
            }
        }
    }
    for id in metadata_ids {
        document.objects.remove(&id);
    }
    document.prune_objects();
    document.renumber_objects();
    document.compress();
    let mut output = Cursor::new(Vec::new());
    document.save_to(&mut output)?;
    Ok((output.into_inner(), findings))
}

fn scrub_object(object: &mut Object, depth: usize) -> Result<()> {
    if depth > MAX_PDF_OBJECT_DEPTH {
        return Err(crate::error::CleanError::InvalidFormat(
            "PDF 直接对象嵌套超过 64 层安全上限".into(),
        ));
    }
    match object {
        Object::Dictionary(dictionary) => scrub_dictionary(dictionary, depth + 1)?,
        Object::Stream(stream) => scrub_dictionary(&mut stream.dict, depth + 1)?,
        Object::Array(values) => {
            for value in values {
                scrub_object(value, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn scrub_dictionary(dictionary: &mut Dictionary, depth: usize) -> Result<()> {
    for key in [b"Metadata".as_slice(), b"PieceInfo", b"LastModified"] {
        dictionary.remove(key);
    }
    for key in annotation_metadata_keys(dictionary) {
        dictionary.remove(key);
    }
    for (key, value) in dictionary.iter_mut() {
        if key == b"Params" {
            if let Ok(params) = value.as_dict_mut() {
                for name in [b"CreationDate".as_slice(), b"ModDate", b"CheckSum"] {
                    params.remove(name);
                }
            }
        }
        scrub_object(value, depth + 1)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    fn incremental_pdf_with_stale_info() -> Vec<u8> {
        let objects: &[&[u8]] = &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
            b"<< /Producer (Claude Opus) /Creator (Anthropic Claude) >>",
        ];
        let mut output = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(output.len());
            output.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            output.extend_from_slice(object);
            output.extend_from_slice(b"\nendobj\n");
        }
        let first_xref = output.len();
        output.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        output.extend_from_slice(
            format!(
                "trailer\n<< /Size 5 /Root 1 0 R /Info 4 0 R >>\nstartxref\n{first_xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        let second_xref = output.len();
        output.extend_from_slice(format!("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {first_xref} >>\nstartxref\n{second_xref}\n%%EOF\n").as_bytes());
        output
    }
    #[test]
    fn removes_info_and_rewrites_pdf() {
        let mut doc = Document::with_version("1.5");
        let id = doc.add_object(lopdf::dictionary! {"Author" => Object::string_literal("Alice")});
        doc.trailer.set("Info", id);
        let mut source = Vec::new();
        doc.save_to(&mut source).unwrap();
        let (cleaned, findings) = clean(&source).unwrap();
        assert!(!findings.is_empty());
        let result = Document::load_mem(&cleaned).unwrap();
        assert!(result.trailer.get(b"Info").is_err());
        assert!(!cleaned.windows(5).any(|window| window == b"Alice"));
    }

    #[test]
    fn preserves_bookmark_titles_while_removing_nested_identity_metadata() {
        let mut document = Document::with_version("1.7");
        let info_id = document.add_object(dictionary! {
            "Author" => Object::string_literal("Alice"),
            "Title" => Object::string_literal("Private document title"),
        });
        document.trailer.set("Info", info_id);
        document.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal("stable-id-one"),
                Object::string_literal("stable-id-two"),
            ]),
        );

        let bookmark_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Keep bookmark"),
        });
        let annotation_id = document.add_object(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Text",
            "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
            "T" => Object::string_literal("Reviewer Alice"),
            "M" => Object::string_literal("D:20260826010000+08'00'"),
            "Contents" => Object::string_literal("Keep comment body"),
        });
        let attachment_id = document.add_object(dictionary! {
            "Type" => "Filespec",
            "F" => Object::string_literal("report.txt"),
            "Params" => Object::Dictionary(dictionary! {
                "CreationDate" => Object::string_literal("D:20260826010000+08'00'"),
                "ModDate" => Object::string_literal("D:20260826010100+08'00'"),
                "CheckSum" => Object::string_literal("private-fingerprint"),
                "Size" => 42,
            }),
        });
        let root_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Bookmark" => bookmark_id,
            "Annotation" => annotation_id,
            "Attachment" => attachment_id,
        });
        document.trailer.set("Root", root_id);
        let mut source = Vec::new();
        document.save_to(&mut source).unwrap();

        assert!(!inspect(&source).unwrap().is_empty());
        let (cleaned, _) = clean(&source).unwrap();
        let result = Document::load_mem(&cleaned).unwrap();
        assert!(result.trailer.get(b"Info").is_err());
        assert!(result.trailer.get(b"ID").is_err());
        assert!(cleaned.windows(13).any(|value| value == b"Keep bookmark"));
        assert!(cleaned
            .windows(17)
            .any(|value| value == b"Keep comment body"));
        for object in result.objects.values() {
            if let Object::Dictionary(dictionary) = object {
                if is_annotation(dictionary) {
                    assert!(dictionary.get(b"T").is_err());
                    assert!(dictionary.get(b"M").is_err());
                }
                if let Ok(params) = dictionary.get(b"Params").and_then(Object::as_dict) {
                    assert!(params.get(b"CreationDate").is_err());
                    assert!(params.get(b"ModDate").is_err());
                    assert!(params.get(b"CheckSum").is_err());
                    assert!(params.get(b"Size").is_ok());
                }
            }
        }
        assert!(inspect(&cleaned).unwrap().is_empty());
    }

    #[test]
    fn preserves_merged_widget_field_names_and_values() {
        for explicit_type in [false, true] {
            let mut widget = dictionary! {
                "Subtype" => "Widget",
                "Rect" => vec![0.into(), 0.into(), 100.into(), 20.into()],
                "FT" => "Tx",
                "T" => Object::string_literal("reviewer_name"),
                "V" => Object::string_literal("Sample reviewer"),
                "M" => Object::string_literal("D:20260929000000Z"),
                "CreationDate" => Object::string_literal("D:20260928000000Z"),
            };
            if explicit_type {
                widget.set("Type", "Annot");
            }
            let mut document = Document::with_version("1.7");
            let field = document.add_object(widget);
            let root = document.add_object(dictionary! {
                "Type" => "Catalog",
                "AcroForm" => dictionary! { "Fields" => vec![Object::Reference(field)] },
            });
            document.trailer.set("Root", root);
            let mut source = Vec::new();
            document.save_to(&mut source).unwrap();
            let (cleaned, findings) = clean(&source).unwrap();
            assert!(!findings.is_empty());
            let result = load_document(&cleaned).unwrap();
            let fields = result
                .catalog()
                .unwrap()
                .get(b"AcroForm")
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"Fields")
                .unwrap()
                .as_array()
                .unwrap();
            let field = result
                .get_object(fields[0].as_reference().unwrap())
                .unwrap()
                .as_dict()
                .unwrap();
            assert_eq!(field.get(b"T").unwrap().as_str().unwrap(), b"reviewer_name");
            assert_eq!(
                field.get(b"V").unwrap().as_str().unwrap(),
                b"Sample reviewer"
            );
            assert!(field.get(b"M").is_err());
            assert!(field.get(b"CreationDate").is_err());
            assert!(inspect(&cleaned).unwrap().is_empty());
        }
    }

    #[test]
    fn drops_metadata_bytes_from_incremental_history() {
        let source = incremental_pdf_with_stale_info();
        assert!(source.windows(6).any(|window| window == b"Claude"));
        let (cleaned, _) = clean(&source).unwrap();
        assert!(!cleaned.windows(6).any(|window| window == b"Claude"));
        assert!(!cleaned.windows(9).any(|window| window == b"Anthropic"));
        Document::load_mem(&cleaned).unwrap();
    }

    #[test]
    fn strips_metadata_from_embedded_jpeg_streams() {
        for filter in [
            Object::Name(b"DCTDecode".to_vec()),
            Object::Array(vec!["ASCII85Decode".into(), "DCTDecode".into()]),
            Object::Array(vec!["A85".into(), "DCT".into()]),
        ] {
            let jpeg = vec![
                0xff, 0xd8, 0xff, 0xe1, 0, 10, b'E', b'x', b'i', b'f', 0, 0, 1, 2, 0xff, 0xd9,
            ];
            let mut document = Document::with_version("1.5");
            let content = if matches!(filter, Object::Array(_)) {
                b"s4IA1!\")/>Bk@>F!<W;S~>".to_vec()
            } else {
                jpeg
            };
            let stream = lopdf::Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => 1,
                    "Height" => 1,
                    "Filter" => filter,
                },
                content,
            );
            let image_id = document.add_object(stream);
            let root_id = document.add_object(dictionary! {
                "Type" => "Catalog",
                "EmbeddedImage" => image_id,
            });
            document.trailer.set("Root", root_id);
            let mut source = Vec::new();
            document.save_to(&mut source).unwrap();

            assert!(!inspect(&source).unwrap().is_empty());
            let (cleaned, _) = clean(&source).unwrap();
            let result = Document::load_mem(&cleaned).unwrap();
            let image_stream = result
                .objects
                .values()
                .find_map(|object| match object {
                    Object::Stream(stream)
                        if stream.dict.get(b"Subtype").is_ok_and(|value| {
                            value.as_name().is_ok_and(|name| name == b"Image")
                        }) =>
                    {
                        Some(stream)
                    }
                    _ => None,
                })
                .unwrap();
            let content = &image_stream.content;
            assert_eq!(content.as_slice(), &[0xff, 0xd8, 0xff, 0xd9]);
            assert!(image::inspect_jpeg(content).unwrap().is_empty());
            assert!(inspect(&cleaned).unwrap().is_empty());
            assert!(name_is(&image_stream.dict, b"Filter", b"DCTDecode"));
        }
    }

    #[test]
    fn bounds_ascii85_expansion_and_rejects_custom_parameters() {
        let mut stream = Stream::new(
            dictionary! { "Subtype" => "Image", "Filter" => vec![
                Object::Name(b"ASCII85Decode".to_vec()), Object::Name(b"DCTDecode".to_vec())
            ] },
            b"zzzzzzzzzz~>".to_vec(),
        );
        assert_eq!(decode_ascii85_jpeg(&stream, 40).unwrap().len(), 40);
        assert!(decode_ascii85_jpeg(&stream, 39).is_err());
        stream
            .dict
            .set("DecodeParms", vec![Object::Null, Object::Null]);
        assert!(decode_ascii85_jpeg(&stream, 40).is_ok());
        stream.dict.set(
            "DecodeParms",
            vec![
                Object::Null,
                Object::Dictionary(dictionary! {
                    "ColorTransform" => 0,
                }),
            ],
        );
        assert!(jpeg_stream(&stream).is_err());
        stream.dict.remove(b"DecodeParms");
        stream.dict.set(
            "Filter",
            vec![
                Object::Name(b"FlateDecode".to_vec()),
                Object::Name(b"DCTDecode".to_vec()),
            ],
        );
        assert!(jpeg_stream(&stream).is_err());
    }

    #[test]
    fn rejects_malformed_ascii85_without_panicking_or_truncating() {
        for encoded in ["s8W-\"~>", "uuuuu~>", "!~>", "!z~>", "z?x~>", "z", "uu~>"] {
            let stream = Stream::new(Dictionary::new(), encoded.as_bytes().to_vec());
            assert!(decode_ascii85_jpeg(&stream, 1024).is_err(), "{encoded}");
        }
        let stream = Stream::new(Dictionary::new(), b"z \n~>\r\n".to_vec());
        assert_eq!(decode_ascii85_jpeg(&stream, 4).unwrap(), vec![0; 4]);
    }

    #[test]
    fn refuses_ambiguous_or_malformed_embedded_dct_streams() {
        let malformed = lopdf::Stream::new(
            dictionary! {
                "Subtype" => "Image",
                "Filter" => "DCTDecode",
            },
            b"not-a-jpeg".to_vec(),
        );
        assert!(jpeg_stream(&malformed).is_err());

        let composite = lopdf::Stream::new(
            dictionary! {
                "Subtype" => "Image",
                "Filter" => Object::Array(vec![
                    Object::Name(b"ASCII85Decode".to_vec()),
                    Object::Name(b"DCTDecode".to_vec()),
                ]),
            },
            vec![0xff, 0xd8, 0xff, 0xd9],
        );
        assert!(jpeg_stream(&composite).is_err());

        let non_dct = lopdf::Stream::new(
            dictionary! {
                "Subtype" => "Image",
                "Filter" => "FlateDecode",
            },
            vec![0xff, 0xd8, 0xff, 0xd9],
        );
        assert!(jpeg_stream(&non_dct).unwrap().is_none());
    }

    #[test]
    fn rejects_direct_object_trees_beyond_the_depth_limit() {
        let mut object = Object::Null;
        for _ in 0..=MAX_PDF_OBJECT_DEPTH {
            object = Object::Array(vec![object]);
        }
        assert!(private_object_count(&object, 0).is_err());
        assert!(scrub_object(&mut object, 0).is_err());
    }
}
