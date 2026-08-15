use std::io::Cursor;

use lopdf::{Dictionary, Document, Object};

use crate::{
    error::Result,
    models::{Finding, FindingSeverity},
};

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
    Ok(if count > 0 {
        vec![Finding {
            category: "pdf_metadata".into(),
            label: "PDF 文档属性 / XMP".into(),
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
}
