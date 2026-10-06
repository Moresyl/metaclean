//! Retain only defined HDR rendering values from JPEG XMP. Namespace bindings,
//! numeric precision, channel order and indexed image associations are significant.

use super::jpeg_segments;
use crate::error::{CleanError, Result};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use std::{collections::BTreeMap, ops::Range};

const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const META: &str = "adobe:ns:meta/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const HDR: &str = "http://ns.adobe.com/hdr-gain-map/1.0/";
const CONTAINER: &str = "http://ns.google.com/photos/1.0/container/";
const ITEM: &str = "http://ns.google.com/photos/1.0/container/item/";
const FIELDS: &[&str] = &[
    "Version",
    "BaseRenditionIsHDR",
    "GainMapMin",
    "GainMapMax",
    "Gamma",
    "OffsetSDR",
    "OffsetHDR",
    "HDRCapacityMin",
    "HDRCapacityMax",
];

fn invalid() -> CleanError {
    CleanError::InvalidFormat("JPEG HDR XMP 显示参数或图像关联无效".into())
}

#[derive(Default)]
struct Node {
    namespace: String,
    name: String,
    attributes: Vec<(String, String, String)>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn is(&self, namespace: &str, name: &str) -> bool {
        self.namespace == namespace && self.name == name
    }

    fn attribute(&self, namespace: &str, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find_map(|(ns, key, value)| (ns == namespace && key == name).then_some(value.as_str()))
    }

    fn scalar(&self) -> Result<String> {
        if !self.children.is_empty() || !self.attributes.is_empty() {
            return Err(invalid());
        }
        Ok(self.text.trim().to_owned())
    }
}

fn namespace(value: ResolveResult<'_>) -> Result<String> {
    match value {
        ResolveResult::Bound(ns) => {
            let text = std::str::from_utf8(ns.as_ref()).map_err(|_| invalid())?;
            quick_xml::escape::unescape(text)
                .map(|value| value.into_owned())
                .map_err(|_| invalid())
        }
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Unknown(_) => Err(invalid()),
    }
}

fn xml_tree(xml: &str) -> Result<Node> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut nodes = 0usize;
    loop {
        let (resolved, event) = reader.read_resolved_event().map_err(|_| invalid())?;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                nodes += 1;
                if nodes > 4096 || stack.len() >= 64 {
                    return Err(invalid());
                }
                let mut node = Node {
                    namespace: namespace(resolved)?,
                    name: std::str::from_utf8(start.local_name().as_ref())
                        .map_err(|_| invalid())?
                        .to_owned(),
                    ..Node::default()
                };
                for attribute in start.attributes() {
                    let attribute = attribute.map_err(|_| invalid())?;
                    if attribute.key.as_ref() == b"xmlns"
                        || attribute.key.as_ref().starts_with(b"xmlns:")
                    {
                        continue;
                    }
                    let (ns, key) = reader.resolver().resolve_attribute(attribute.key);
                    let ns = namespace(ns)?;
                    let key = std::str::from_utf8(key.as_ref())
                        .map_err(|_| invalid())?
                        .to_owned();
                    if node
                        .attributes
                        .iter()
                        .any(|(previous_ns, previous_key, _)| {
                            *previous_ns == ns && *previous_key == key
                        })
                    {
                        return Err(invalid());
                    }
                    let value = attribute
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|_| invalid())?
                        .into_owned();
                    node.attributes.push((ns, key, value));
                }
                if matches!(event, Event::Empty(_)) {
                    append_node(node, &mut stack, &mut root)?;
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(invalid)?;
                append_node(node, &mut stack, &mut root)?;
            }
            Event::Text(text) => {
                let value = std::str::from_utf8(text.as_ref()).map_err(|_| invalid())?;
                append_text(value, &mut stack)?;
            }
            Event::CData(text) => {
                let value = std::str::from_utf8(text.as_ref()).map_err(|_| invalid())?;
                append_text(value, &mut stack)?;
            }
            Event::GeneralRef(reference) => {
                let name = std::str::from_utf8(reference.as_ref()).map_err(|_| invalid())?;
                let encoded = format!("&{name};");
                let value = quick_xml::escape::unescape(&encoded).map_err(|_| invalid())?;
                append_text(&value, &mut stack)?;
            }
            Event::DocType(_) => return Err(invalid()),
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {}
        }
    }
    if !stack.is_empty() {
        return Err(invalid());
    }
    root.ok_or_else(invalid)
}

fn append_node(node: Node, stack: &mut [Node], root: &mut Option<Node>) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err(invalid());
    }
    Ok(())
}

fn append_text(value: &str, stack: &mut [Node]) -> Result<()> {
    if let Some(node) = stack.last_mut() {
        node.text.push_str(value);
    } else if !value.trim().is_empty() {
        return Err(invalid());
    }
    Ok(())
}

#[derive(Clone)]
struct Value {
    values: Vec<String>,
    array: bool,
}

impl Value {
    fn scalar(value: &str) -> Self {
        Self {
            values: vec![value.trim().to_owned()],
            array: false,
        }
    }

    fn parse(node: &Node) -> Result<Self> {
        if node.children.is_empty() {
            return Ok(Self::scalar(&node.scalar()?));
        }
        if !node.text.trim().is_empty()
            || !node.attributes.is_empty()
            || node.children.len() != 1
            || !node.children[0].is(RDF, "Seq")
        {
            return Err(invalid());
        }
        let sequence = &node.children[0];
        if !sequence.text.trim().is_empty()
            || !sequence.attributes.is_empty()
            || !matches!(sequence.children.len(), 1 | 3)
        {
            return Err(invalid());
        }
        let values = sequence
            .children
            .iter()
            .map(|child| {
                if !child.is(RDF, "li") {
                    return Err(invalid());
                }
                child.scalar()
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            values,
            array: true,
        })
    }

    fn numbers(&self) -> Result<Vec<f64>> {
        self.values
            .iter()
            .map(|value| {
                if value.is_empty()
                    || value
                        .bytes()
                        .any(|byte| !matches!(byte, b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E'))
                {
                    return Err(invalid());
                }
                let number = value.parse::<f64>().map_err(|_| invalid())?;
                if !number.is_finite() {
                    return Err(invalid());
                }
                Ok(number)
            })
            .collect()
    }
}

struct Item {
    semantic: String,
    length: Option<usize>,
    padding: usize,
}

pub(super) struct Metadata {
    fields: BTreeMap<String, Value>,
    items: Vec<Item>,
}

fn integer(value: &str) -> Result<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    value.parse().map_err(|_| invalid())
}

fn item_property(node: &Node, name: &str) -> Result<Option<String>> {
    let mut value = node.attribute(ITEM, name).map(str::to_owned);
    for child in node.children.iter().filter(|child| child.is(ITEM, name)) {
        if value.is_some() {
            return Err(invalid());
        }
        value = Some(child.scalar()?);
    }
    Ok(value)
}

fn directory(node: &Node) -> Result<Vec<Item>> {
    if node.children.len() != 1
        || !node.children[0].is(RDF, "Seq")
        || !node.text.trim().is_empty()
        || !node.attributes.is_empty()
    {
        return Err(invalid());
    }
    let sequence = &node.children[0];
    if !sequence.text.trim().is_empty() || !sequence.attributes.is_empty() {
        return Err(invalid());
    }
    let mut items = Vec::new();
    for entry in &sequence.children {
        if !entry.is(RDF, "li")
            || entry.children.len() != 1
            || !entry.children[0].is(CONTAINER, "Item")
            || !entry.text.trim().is_empty()
        {
            return Err(invalid());
        }
        let node = &entry.children[0];
        if !node.text.trim().is_empty()
            || item_property(node, "Mime")?.as_deref() != Some("image/jpeg")
        {
            return Err(invalid());
        }
        let semantic = item_property(node, "Semantic")?.ok_or_else(invalid)?;
        if !matches!(
            semantic.as_str(),
            "Primary" | "GainMap" | "Depth" | "Confidence"
        ) {
            return Err(invalid());
        }
        let length = item_property(node, "Length")?
            .map(|value| integer(&value))
            .transpose()?;
        let padding = item_property(node, "Padding")?
            .map(|value| integer(&value))
            .transpose()?
            .unwrap_or(0);
        items.push(Item {
            semantic,
            length,
            padding,
        });
    }
    if items.len() < 2
        || items[0].semantic != "Primary"
        || items
            .iter()
            .skip(1)
            .any(|item| item.semantic == "Primary" || item.length.is_none())
        || items
            .iter()
            .filter(|item| item.semantic == "GainMap")
            .count()
            != 1
    {
        return Err(invalid());
    }
    Ok(items)
}

pub(super) fn parse(payload: &[u8]) -> Result<Option<Metadata>> {
    let Some(xml) = payload.strip_prefix(XMP) else {
        return Ok(None);
    };
    let tree = xml_tree(std::str::from_utf8(xml).map_err(|_| invalid())?)?;
    let rdf = if tree.is(META, "xmpmeta") {
        if tree.children.len() != 1 || !tree.children[0].is(RDF, "RDF") {
            return Err(invalid());
        }
        &tree.children[0]
    } else if tree.is(RDF, "RDF") {
        &tree
    } else {
        return Ok(None);
    };
    let mut metadata = Metadata {
        fields: BTreeMap::new(),
        items: Vec::new(),
    };
    let mut item_node = None;
    for description in rdf
        .children
        .iter()
        .filter(|node| node.is(RDF, "Description"))
    {
        for (ns, name, value) in &description.attributes {
            if ns == HDR && FIELDS.contains(&name.as_str()) {
                metadata.insert(name, Value::scalar(value))?;
            }
        }
        for child in &description.children {
            if child.namespace == HDR && FIELDS.contains(&child.name.as_str()) {
                metadata.insert(&child.name, Value::parse(child)?)?;
            } else if child.is(CONTAINER, "Directory") && item_node.replace(child).is_some() {
                return Err(invalid());
            }
        }
    }
    if metadata.fields.is_empty() {
        return Ok(None);
    }
    metadata.validate()?;
    if let Some(node) = item_node {
        metadata.items = directory(node)?;
    }
    Ok(Some(metadata))
}

impl Metadata {
    fn insert(&mut self, name: &str, value: Value) -> Result<()> {
        if self.fields.insert(name.to_owned(), value).is_some() {
            return Err(invalid());
        }
        Ok(())
    }

    fn number(&self, name: &str, default: f64, channel: usize) -> Result<f64> {
        if let Some(value) = self.fields.get(name) {
            let numbers = value.numbers()?;
            Ok(numbers[if numbers.len() == 1 { 0 } else { channel }])
        } else {
            Ok(default)
        }
    }

    fn rendering(&self) -> bool {
        self.fields.contains_key("GainMapMax")
    }

    fn validate(&self) -> Result<()> {
        let version = self.fields.get("Version").ok_or_else(invalid)?;
        if version.array || version.values != ["1.0"] {
            return Err(invalid());
        }
        if let Some(value) = self.fields.get("BaseRenditionIsHDR") {
            if value.array || !matches!(value.values[0].as_str(), "True" | "False") {
                return Err(invalid());
            }
        }
        let numeric = self
            .fields
            .iter()
            .filter(|(name, _)| !matches!(name.as_str(), "Version" | "BaseRenditionIsHDR"))
            .collect::<Vec<_>>();
        if numeric.is_empty() {
            return Ok(());
        }
        if !self.rendering() || !self.fields.contains_key("HDRCapacityMax") {
            return Err(invalid());
        }
        for (name, value) in numeric {
            if name.starts_with("HDRCapacity") && value.array {
                return Err(invalid());
            }
            value.numbers()?;
        }
        for channel in 0..3 {
            if self.number("GainMapMin", 0.0, channel)? > self.number("GainMapMax", 0.0, channel)?
                || self.number("Gamma", 1.0, channel)? <= 0.0
                || self.number("OffsetSDR", 1.0 / 64.0, channel)? < 0.0
                || self.number("OffsetHDR", 1.0 / 64.0, channel)? < 0.0
            {
                return Err(invalid());
            }
        }
        let min = self.number("HDRCapacityMin", 0.0, 0)?;
        if min < 0.0 || self.number("HDRCapacityMax", 0.0, 0)? <= min {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn serialize(&self) -> Result<Vec<u8>> {
        let mut xml = format!(
            "<x:xmpmeta xmlns:x=\"{META}\"><rdf:RDF xmlns:rdf=\"{RDF}\"><rdf:Description rdf:about=\"\" xmlns:hdrgm=\"{HDR}\""
        );
        if !self.items.is_empty() {
            xml.push_str(&format!(
                " xmlns:Container=\"{CONTAINER}\" xmlns:Item=\"{ITEM}\""
            ));
        }
        for name in FIELDS {
            if let Some(value) = self.fields.get(*name).filter(|value| !value.array) {
                xml.push_str(&format!(" hdrgm:{name}=\"{}\"", value.values[0]));
            }
        }
        xml.push('>');
        for name in FIELDS {
            if let Some(value) = self.fields.get(*name).filter(|value| value.array) {
                xml.push_str(&format!("<hdrgm:{name}><rdf:Seq>"));
                for channel in &value.values {
                    xml.push_str(&format!("<rdf:li>{channel}</rdf:li>"));
                }
                xml.push_str(&format!("</rdf:Seq></hdrgm:{name}>"));
            }
        }
        if !self.items.is_empty() {
            xml.push_str("<Container:Directory><rdf:Seq>");
            for (index, item) in self.items.iter().enumerate() {
                xml.push_str(&format!(
                    "<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"image/jpeg\" Item:Semantic=\"{}\"", item.semantic,
                ));
                if index != 0 {
                    xml.push_str(&format!(
                        " Item:Length=\"{}\"",
                        item.length.ok_or_else(invalid)?
                    ));
                }
                if item.padding != 0 {
                    xml.push_str(&format!(" Item:Padding=\"{}\"", item.padding));
                }
                xml.push_str("/></rdf:li>");
            }
            xml.push_str("</rdf:Seq></Container:Directory>");
        }
        xml.push_str("</rdf:Description></rdf:RDF></x:xmpmeta>");
        if XMP.len() + xml.len() > u16::MAX as usize - 2 {
            return Err(invalid());
        }
        Ok([XMP, xml.as_bytes()].concat())
    }

    fn associations(&self, frames: &[Range<usize>]) -> Result<Vec<usize>> {
        if self.items.is_empty() {
            return Ok(Vec::new());
        }
        let first = frames.first().ok_or_else(invalid)?;
        if first.start != 0
            || self.items[0]
                .length
                .is_some_and(|length| length != 0 && length != first.len())
        {
            return Err(invalid());
        }
        let mut cursor = first
            .end
            .checked_add(self.items[0].padding)
            .ok_or_else(invalid)?;
        let mut previous = 0;
        let mut associations = vec![0];
        for item in self.items.iter().skip(1) {
            let length = item.length.ok_or_else(invalid)?;
            let index = if length == 0 {
                if item.padding != 0 {
                    return Err(invalid());
                }
                previous
            } else {
                let end = cursor.checked_add(length).ok_or_else(invalid)?;
                let index = frames
                    .iter()
                    .position(|frame| frame.start == cursor && frame.end == end)
                    .ok_or_else(invalid)?;
                cursor = end.checked_add(item.padding).ok_or_else(invalid)?;
                index
            };
            if item.semantic == "GainMap" && index == 0 {
                return Err(invalid());
            }
            previous = index;
            associations.push(index);
        }
        Ok(associations)
    }
}

fn image_metadata(data: &[u8]) -> Result<Option<(Metadata, Range<usize>)>> {
    let mut found = None;
    for (marker, payload, range) in jpeg_segments(data)? {
        if marker == 0xe1 {
            if let Some(metadata) = parse(payload)? {
                if found.is_some() {
                    return Err(invalid());
                }
                found = Some((metadata, range));
            }
        }
    }
    Ok(found)
}

pub(super) fn validate_single(data: &[u8]) -> Result<()> {
    if image_metadata(data)?.is_some_and(|(metadata, _)| !metadata.items.is_empty()) {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_container(data: &[u8], frames: &[Range<usize>]) -> Result<()> {
    for (index, frame) in frames.iter().enumerate() {
        let Some((metadata, _)) = image_metadata(&data[frame.clone()])? else {
            continue;
        };
        if index != 0 && !metadata.items.is_empty() {
            return Err(invalid());
        }
        let associations = metadata.associations(frames)?;
        for (item, associated) in metadata.items.iter().zip(associations) {
            if item.semantic != "GainMap" {
                continue;
            }
            let gainmap = &data[frames[associated].clone()];
            let xmp = image_metadata(gainmap)?.is_some_and(|(metadata, _)| metadata.rendering());
            let iso = jpeg_segments(gainmap)?.iter().any(|(marker, payload, _)| {
                *marker == 0xe2
                    && payload.starts_with(super::JPEG_GAINMAP_NAMESPACE)
                    && payload.len() > super::JPEG_GAINMAP_NAMESPACE.len() + 4
            });
            if !xmp && !iso {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

pub(super) fn rewrite_container(
    data: &[u8],
    ranges: &[Range<usize>],
    frames: &mut [Vec<u8>],
) -> Result<()> {
    let Some((mut metadata, _)) = image_metadata(&data[ranges[0].clone()])? else {
        return Ok(());
    };
    if metadata.items.is_empty() {
        return Ok(());
    }
    let associations = metadata.associations(ranges)?;
    // Every indexed JPEG remains in physical order, while MP entry order is retained.
    // Shared resources keep their zero lengths; primary length is implicit.
    let mut order = (0..ranges.len()).collect::<Vec<_>>();
    order.sort_by_key(|index| ranges[*index].start);
    let mut starts = vec![0usize; frames.len()];
    let mut cursor = 0usize;
    for index in order {
        starts[index] = cursor;
        cursor = cursor
            .checked_add(frames[index].len())
            .ok_or_else(invalid)?;
    }
    let mut resources = vec![(0, 0)];
    for (item_index, (item, &index)) in metadata.items.iter_mut().zip(&associations).enumerate() {
        if item.length.is_some_and(|length| length != 0) {
            item.length = Some(frames[index].len());
            if item_index != 0 {
                resources.push((item_index, index));
            }
        }
        item.padding = 0;
    }
    for pair in resources.windows(2) {
        let (item, index) = pair[0];
        let (_, next) = pair[1];
        metadata.items[item].padding = starts[next]
            .checked_sub(starts[index] + frames[index].len())
            .ok_or_else(invalid)?;
    }
    metadata.items[0].length = None;
    let (_, range) = image_metadata(&frames[0])?.ok_or_else(invalid)?;
    let payload = metadata.serialize()?;
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend(payload);
    frames[0].splice(range, segment);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaners::image::{clean_jpeg_with_options, inspect_jpeg, verify_jpeg_cleaned};

    fn packet(attributes: &str, children: &str) -> Vec<u8> {
        let xml = format!(
            "<x:xmpmeta xmlns:x='{META}'><r:RDF xmlns:r='{RDF}'><r:Description xmlns:h='{HDR}' xmlns:c='{CONTAINER}' xmlns:i='{ITEM}' {attributes}>{children}</r:Description></r:RDF></x:xmpmeta>"
        );
        [XMP, xml.as_bytes()].concat()
    }

    fn image(payload: &[u8], private: bool, sample: u8) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend([0xff, 0xe1]);
        bytes.extend(((payload.len() + 2) as u16).to_be_bytes());
        bytes.extend(payload);
        if private {
            bytes.extend([0xff, 0xfe, 0, 9]);
            bytes.extend(b"private");
        }
        bytes.extend([0xff, 0xda, 0, 8, 1, 1, 0, 0, 0x3f, 0, sample, 0xff, 0xd9]);
        bytes
    }

    fn rejected(payload: &[u8]) {
        let bytes = image(payload, false, 0x21);
        assert!(inspect_jpeg(&bytes).is_err());
        assert!(clean_jpeg_with_options(&bytes, true, true).is_err());
        assert!(verify_jpeg_cleaned(&bytes, true, true).is_err());
    }

    #[test]
    fn preserves_namespace_aliases_entities_and_ordered_channel_precision() {
        let source = packet(
            "h:Version='1.0' h:HDRCapacityMax='4.0' h:OffsetSDR='&#49;.5625e-2'",
            "<h:GainMapMax><r:Seq><r:li>3.0000000000000001</r:li><r:li>2.5</r:li><r:li>4e0</r:li></r:Seq></h:GainMapMax><h:Gamma><![CDATA[1.0]]></h:Gamma><h:Unknown>private</h:Unknown><r:other>private</r:other>",
        );
        let metadata = parse(&source).unwrap().unwrap();
        assert_eq!(
            metadata.fields["GainMapMax"].values,
            ["3.0000000000000001", "2.5", "4e0"]
        );
        let clean = metadata.serialize().unwrap();
        let text = std::str::from_utf8(&clean).unwrap();
        assert!(text.contains("OffsetSDR=\"1.5625e-2\""));
        assert!(text.contains(
            "<rdf:li>3.0000000000000001</rdf:li><rdf:li>2.5</rdf:li><rdf:li>4e0</rdf:li>"
        ));
        assert!(!text.contains("private"));
        assert_eq!(parse(&clean).unwrap().unwrap().serialize().unwrap(), clean);
        let fake = packet("xmlns:f='urn:private' f:Version='1.0' f:GainMapMax='4'", "");
        assert!(parse(&fake).unwrap().is_none());
        let escaped_ns = source
            .windows(HDR.len())
            .position(|bytes| bytes == HDR.as_bytes())
            .unwrap();
        let mut escaped = source.clone();
        escaped.splice(escaped_ns..escaped_ns + 1, b"&#104;".iter().copied());
        assert_eq!(
            parse(&escaped).unwrap().unwrap().serialize().unwrap(),
            clean
        );
    }

    #[test]
    fn rejects_ambiguous_invalid_and_nonfinite_display_parameters() {
        for attributes in [
            "h:Version='2.0'",
            "h:GainMapMax='3' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:GainMapMax='3'",
            "h:Version='1.0' h:GainMapMax='NaN' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:GainMapMax='1e999' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:GainMapMin='4' h:GainMapMax='3' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:GainMapMax='3' h:HDRCapacityMax='4' h:Gamma='0'",
            "h:Version='1.0' h:GainMapMax='3' h:HDRCapacityMax='4' h:OffsetHDR='-1'",
            "h:Version='1.0' h:GainMapMax='3' h:HDRCapacityMin='4' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:GainMapMax='3' h:HDRCapacityMin='-1' h:HDRCapacityMax='4'",
            "h:Version='1.0' h:BaseRenditionIsHDR='private'",
            "h:Version='1.0' xmlns:a='http://ns.adobe.com/hdr-gain-map/1.0/' a:Version='1.0'",
        ] {
            rejected(&packet(attributes, ""));
        }
        for children in [
            "<h:Version>1.0</h:Version>",
            "<h:GainMapMax><r:Seq><r:li>1</r:li><r:li>2</r:li></r:Seq></h:GainMapMax>",
            "<h:GainMapMax><r:Bag><r:li>2</r:li></r:Bag></h:GainMapMax>",
            "<h:HDRCapacityMax><r:Seq><r:li>4</r:li></r:Seq></h:HDRCapacityMax>",
            "<h:GainMapMax r:resource='private'/>",
        ] {
            rejected(&packet("h:Version='1.0'", children));
        }
        let duplicate = [image(&packet("h:Version='1.0'", ""), false, 0x21)].concat();
        let mut bytes = duplicate.clone();
        let segment = jpeg_segments(&duplicate).unwrap()[0].2.clone();
        bytes.splice(2..2, duplicate[segment].iter().copied());
        assert!(inspect_jpeg(&bytes).is_err());
        assert!(clean_jpeg_with_options(&bytes, true, true).is_err());
    }

    #[test]
    fn rejects_unsafe_or_unbounded_xml_and_multiple_roots() {
        for xml in [
            "<!DOCTYPE x [<!ENTITY secret SYSTEM 'file:///private'>]><x/>",
            "<x/><x/>",
            "<x><y></x>",
            "<undeclared:Description/>",
            "<x>&unknown;</x>",
            "<x a='1' a='2'/>",
            "<x/>outside",
            "",
        ] {
            rejected(&[XMP, xml.as_bytes()].concat());
        }
        let deep = format!("{}{}", "<x>".repeat(65), "</x>".repeat(65));
        rejected(&[XMP, deep.as_bytes()].concat());
        let many = format!("<x>{}</x>", "<y/>".repeat(4096));
        rejected(&[XMP, many.as_bytes()].concat());
        let ordinary = packet("xmlns:p='urn:private' p:author='private'", "");
        let bytes = image(&ordinary, false, 0x21);
        let output = clean_jpeg_with_options(&bytes, true, true).unwrap().0;
        assert!(!output.windows(7).any(|bytes| bytes == b"private"));
        verify_jpeg_cleaned(&output, true, true).unwrap();
    }

    fn item(semantic: &str, length: Option<usize>, padding: usize) -> String {
        format!(
            "<r:li r:parseType='Resource'><c:Item i:Mime='image/jpeg' i:Semantic='{semantic}' i:Label='private'{} i:Padding='{padding}'/></r:li>",
            length.map_or(String::new(), |value| format!(" i:Length='{value}'")),
        )
    }

    // Encode the TIFF index directly, independent of the production MPF serializer.
    fn indexed(mut frames: Vec<Vec<u8>>, reverse: bool, gap: &[u8]) -> Vec<u8> {
        let count = frames.len();
        let mut payload = b"MPF\0II\x2a\0\x08\0\0\0\x03\0".to_vec();
        for (tag, kind, total, value) in [
            (0xb000u16, 7u16, 4u32, *b"0100"),
            (0xb001, 4, 1, (count as u32).to_le_bytes()),
            (0xb002, 7, (count * 16) as u32, 50u32.to_le_bytes()),
        ] {
            payload.extend(tag.to_le_bytes());
            payload.extend(kind.to_le_bytes());
            payload.extend(total.to_le_bytes());
            payload.extend(value);
        }
        payload.extend(0u32.to_le_bytes());
        let mut sizes = frames.iter().map(Vec::len).collect::<Vec<_>>();
        sizes[0] += 4 + payload.len() + count * 16;
        let mut starts = vec![0];
        let mut end = sizes[0] + gap.len();
        for size in sizes.iter().skip(1) {
            starts.push(end);
            end += size;
        }
        let mut order = (1..count).collect::<Vec<_>>();
        if reverse {
            order.reverse();
        }
        order.insert(0, 0);
        for index in order {
            payload.extend((if index == 0 { 0x2003_0000u32 } else { 0u32 }).to_le_bytes());
            payload.extend((sizes[index] as u32).to_le_bytes());
            payload.extend(
                (if index == 0 {
                    0
                } else {
                    (starts[index] - 10) as u32
                })
                .to_le_bytes(),
            );
            payload.extend([0; 4]);
        }
        let mut segment = vec![0xff, 0xe2];
        segment.extend(((payload.len() + 2) as u16).to_be_bytes());
        segment.extend(payload);
        frames[0].splice(2..2, segment);
        let mut output = frames.remove(0);
        output.extend(gap);
        for frame in frames {
            output.extend(frame);
        }
        output
    }

    #[test]
    fn rebuilds_directory_lengths_and_preserves_physical_and_index_orders() {
        let secondary = image(
            &packet(
                "h:Version='1.0' h:GainMapMax='3.5' h:HDRCapacityMax='4.0'",
                "",
            ),
            true,
            0x32,
        );
        let auxiliary = image(b"ordinary private payload", true, 0x42);
        for reverse in [false, true] {
            for omitted_auxiliary in [false, true] {
                let gap = b"private gap";
                let primary_padding = gap.len()
                    + if omitted_auxiliary {
                        auxiliary.len()
                    } else {
                        0
                    };
                let mut items = item("Primary", None, primary_padding);
                if !omitted_auxiliary {
                    items.push_str(&item("Depth", Some(auxiliary.len()), 0));
                }
                items.push_str(&item("GainMap", Some(secondary.len()), 0));
                // A zero-length item shares the immediately preceding resource.
                items.push_str(&item("Confidence", Some(0), 0));
                let primary = image(
                    &packet(
                        "h:Version='1.0'",
                        &format!("<c:Directory><r:Seq>{items}</r:Seq></c:Directory>"),
                    ),
                    true,
                    0x21,
                );
                let source = indexed(
                    vec![primary, auxiliary.clone(), secondary.clone()],
                    reverse,
                    gap,
                );
                for orientation in [false, true] {
                    for profile in [false, true] {
                        let output = clean_jpeg_with_options(&source, orientation, profile)
                            .unwrap()
                            .0;
                        assert!(!output.windows(7).any(|bytes| bytes == b"private"));
                        verify_jpeg_cleaned(&output, orientation, profile).unwrap();
                        assert!(inspect_jpeg(&output).unwrap().is_empty());
                        let primary_end = jpeg_segments(&output).unwrap().last().unwrap().2.start;
                        let (metadata, _) =
                            image_metadata(&output[..primary_end]).unwrap().unwrap();
                        let mut physical = Vec::new();
                        let mut cursor = 0;
                        while cursor < output.len() {
                            let segments = jpeg_segments(&output[cursor..]).unwrap();
                            let size = segments
                                .last()
                                .filter(|segment| segment.0 == 0)
                                .map_or(output.len() - cursor, |segment| segment.2.start);
                            physical.push(cursor..cursor + size);
                            cursor += size;
                        }
                        assert_eq!(physical.len(), 3);
                        let associations = metadata.associations(&physical).unwrap();
                        let gainmap = metadata
                            .items
                            .iter()
                            .position(|item| item.semantic == "GainMap")
                            .unwrap();
                        assert_eq!(associations[gainmap], 2);
                        assert_eq!(metadata.items[gainmap].length, Some(physical[2].len()));
                        assert_eq!(
                            metadata.items[0].padding,
                            if omitted_auxiliary {
                                physical[1].len()
                            } else {
                                0
                            }
                        );
                        for (range, sample) in physical.iter().zip([0x21, 0x42, 0x32]) {
                            assert_eq!(&output[range.end - 3..range.end], &[sample, 0xff, 0xd9]);
                        }
                        assert_eq!(
                            clean_jpeg_with_options(&output, orientation, profile)
                                .unwrap()
                                .0,
                            output
                        );
                        // Independently read the second MP entry's offset to check index identity.
                        let segments = jpeg_segments(&output).unwrap();
                        let (_, payload, range) =
                            segments.iter().find(|entry| entry.0 == 0xe2).unwrap();
                        let second_offset = u32::from_le_bytes(
                            payload[4 + 50 + 16 + 8..4 + 50 + 16 + 12]
                                .try_into()
                                .unwrap(),
                        ) as usize;
                        let base = range.end - payload.len() + 4;
                        assert_eq!(
                            base + second_offset,
                            physical[if reverse { 2 } else { 1 }].start
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rejects_missing_or_conflicting_gainmap_resources() {
        let secondary = image(
            &packet("h:Version='1.0' h:GainMapMax='3' h:HDRCapacityMax='4'", ""),
            false,
            0x32,
        );
        for (semantic, length, padding) in [
            ("GainMap", secondary.len() + 1, 0),
            ("GainMap", secondary.len(), 1),
            ("GainMap", 0, 0),
            ("Primary", secondary.len(), 0),
            ("GainMap", usize::MAX, 0),
        ] {
            let items = format!(
                "{}{}",
                item("Primary", None, padding),
                item(semantic, Some(length), 0)
            );
            let primary = image(
                &packet(
                    "h:Version='1.0'",
                    &format!("<c:Directory><r:Seq>{items}</r:Seq></c:Directory>"),
                ),
                false,
                0x21,
            );
            let source = indexed(vec![primary, secondary.clone()], false, &[]);
            assert!(inspect_jpeg(&source).is_err());
            assert!(clean_jpeg_with_options(&source, true, true).is_err());
            assert!(verify_jpeg_cleaned(&source, true, true).is_err());
        }
        let items = format!(
            "{}{}",
            item("Primary", None, 0),
            item("GainMap", Some(secondary.len()), 0)
        );
        let payload = packet(
            "h:Version='1.0'",
            &format!("<c:Directory><r:Seq>{items}</r:Seq></c:Directory>"),
        );
        rejected(&payload);
        let no_rendering = image(&packet("h:Version='1.0'", ""), false, 0x32);
        let no_rendering_items = format!(
            "{}{}",
            item("Primary", None, 0),
            item("GainMap", Some(no_rendering.len()), 0)
        );
        let primary = image(
            &packet(
                "h:Version='1.0'",
                &format!("<c:Directory><r:Seq>{no_rendering_items}</r:Seq></c:Directory>"),
            ),
            false,
            0x21,
        );
        let source = indexed(vec![primary, no_rendering], false, &[]);
        assert!(clean_jpeg_with_options(&source, true, true).is_err());
    }
}
