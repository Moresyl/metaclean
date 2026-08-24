use std::io::Cursor;

use lopdf::{Dictionary, Document, Object};

use crate::{
    error::Result,
    models::{Finding, FindingSeverity},
};

use super::image;

fn embedded_image_findings(document: &Document) -> usize {
    document
        .objects
        .values()
        .filter_map(|object| match object {
            Object::Stream(stream)
                if stream
                    .dict
                    .get(b"Subtype")
                    .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Image")) =>
            {
                Some(stream)
            }
            _ => None,
        })
        .map(|stream| &stream.content)
        .filter(|content| {
            content.starts_with(&[0xff, 0xd8, 0xff])
                && image::inspect_jpeg(content).is_ok_and(|findings| !findings.is_empty())
        })
        .count()
}

pub fn inspect(data: &[u8]) -> Result<Vec<Finding>> {
    let document = Document::load_mem(data)?;
    let mut count = 0;
    if let Ok(Object::Dictionary(info)) = document.trailer.get(b"Info") {
        count += info.len();
    }
    if document.trailer.get(b"Info").is_ok() {
        count += 1;
    }
    for object in document.objects.values() {
        match object {
            Object::Stream(stream)
                if stream
                    .dict
                    .get(b"Type")
                    .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Metadata")) =>
            {
                count += 1
            }
            Object::Dictionary(dict) if dict.get(b"Metadata").is_ok() => count += 1,
            _ => {}
        }
    }
    count += embedded_image_findings(&document);
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
    let mut document = Document::load_mem(data)?;
    document.trailer.remove(b"Info");
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
        if let Object::Dictionary(dictionary) = object {
            dictionary.remove(b"Metadata");
            scrub_dictionary(dictionary);
        }
        if let Object::Stream(stream) = object {
            stream.dict.remove(b"Metadata");
            scrub_dictionary(&mut stream.dict);
            if stream
                .dict
                .get(b"Subtype")
                .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Image"))
            {
                let content = stream.content.clone();
                if content.starts_with(&[0xff, 0xd8, 0xff]) {
                    if let Ok((cleaned, removed)) =
                        image::clean_jpeg_with_options(&content, true, true)
                    {
                        if !removed.is_empty() && cleaned != content {
                            stream.set_content(cleaned);
                        }
                    }
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

fn scrub_dictionary(dictionary: &mut Dictionary) {
    for key in [
        b"Author".as_slice(),
        b"Creator",
        b"Producer",
        b"Title",
        b"Subject",
        b"Keywords",
        b"CreationDate",
        b"ModDate",
        b"PieceInfo",
        b"LastModified",
    ] {
        dictionary.remove(key);
    }
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
        let jpeg = vec![
            0xff, 0xd8, 0xff, 0xe1, 0, 10, b'E', b'x', b'i', b'f', 0, 0, 1, 2, 0xff, 0xd9,
        ];
        let mut document = Document::with_version("1.5");
        let stream = lopdf::Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 1,
                "Height" => 1,
                "Filter" => "DCTDecode",
            },
            jpeg,
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
                    if stream
                        .dict
                        .get(b"Subtype")
                        .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Image")) =>
                {
                    Some(stream)
                }
                _ => None,
            })
            .unwrap();
        let content = &image_stream.content;
        assert_eq!(content.as_slice(), &[0xff, 0xd8, 0xff, 0xd9]);
        assert!(image::inspect_jpeg(content).unwrap().is_empty());
    }
}
