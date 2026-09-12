use std::collections::HashMap;

use crate::ChatMarkupRegex::ChatMarkupRegex;

pub struct NativeXmlSplitter;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XmlOpeningTag {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
    pub end: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XmlNode {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
    pub body: String,
    pub children: Vec<XmlNode>,
}

impl NativeXmlSplitter {
    /// Splits complete XML-like children without requiring strict XML text escaping.
    pub fn split_xml_tag(content: &str) -> Vec<Vec<String>> {
        split_xml_tag(content)
    }

    /// Parses one opening tag from XML-like assistant markup.
    pub fn parse_opening_tag(content: &str) -> Option<XmlOpeningTag> {
        parse_opening_tag(content)
    }

    /// Parses one complete XML-like node whose boundary was found by StreamXmlPlugin.
    pub fn parse_complete_node(content: &str) -> Option<XmlNode> {
        parse_complete_node(content)
    }
}

/// Splits complete XML-like children without interpreting their text payload as XML.
pub fn split_xml_tag(content: &str) -> Vec<Vec<String>> {
    let mut results = Vec::new();
    let mut cursor = 0;
    while cursor < content.len() {
        let Some(relative_start) = content[cursor..].find('<') else {
            let text = &content[cursor..];
            if !text.trim().is_empty() {
                results.push(vec!["text".to_string(), text.to_string()]);
            }
            break;
        };
        let start = cursor + relative_start;
        if start > cursor {
            let text = &content[cursor..start];
            if !text.trim().is_empty() {
                results.push(vec!["text".to_string(), text.to_string()]);
            }
        }

        let Some(tag_name) = ChatMarkupRegex::extract_opening_tag_name(&content[start..]) else {
            cursor = start + 1;
            continue;
        };

        let Some(open_end_relative) = find_opening_end(&content[start..]) else {
            cursor = start + 1;
            continue;
        };
        let open_end = start + open_end_relative + 1;
        if content[start..open_end].trim_end().ends_with("/>") {
            results.push(vec![tag_name, content[start..open_end].to_string()]);
            cursor = open_end;
            continue;
        }

        let close = format!("</{tag_name}>");
        let lower_tail = content[open_end..].to_ascii_lowercase();
        let close_lower = close.to_ascii_lowercase();
        if let Some(relative_close) = lower_tail.find(&close_lower) {
            let end = open_end + relative_close + close.len();
            results.push(vec![tag_name, content[start..end].to_string()]);
            cursor = end;
        } else {
            results.push(vec![
                "text".to_string(),
                content[start..open_end].to_string(),
            ]);
            cursor = open_end;
        }
    }
    results
}

/// Parses one opening tag and its quoted attributes.
pub fn parse_opening_tag(content: &str) -> Option<XmlOpeningTag> {
    let leading_whitespace = content.len() - content.trim_start().len();
    let trimmed = &content[leading_whitespace..];
    let tag_name = ChatMarkupRegex::extract_opening_tag_name(trimmed)?;
    let relative_end = find_opening_end(trimmed)?;
    let opening = &trimmed[..=relative_end];
    let attributes = parse_attributes(opening, &tag_name);
    Some(XmlOpeningTag {
        tag_name,
        attributes,
        end: leading_whitespace + relative_end + 1,
    })
}

/// Parses one complete node after StreamXmlPlugin has emitted its end boundary.
pub fn parse_complete_node(content: &str) -> Option<XmlNode> {
    let opening = parse_opening_tag(content)?;
    let close = format!("</{}>", opening.tag_name);
    let close_start = content
        .to_ascii_lowercase()
        .rfind(&close.to_ascii_lowercase())?;
    if close_start < opening.end {
        return None;
    }
    let body = content[opening.end..close_start].to_string();
    let children = split_xml_tag(&body)
        .into_iter()
        .filter_map(|parts| {
            if parts.first().map(String::as_str) == Some("text") {
                return None;
            }
            parts.get(1).and_then(|raw| parse_complete_node(raw))
        })
        .collect();
    Some(XmlNode {
        tag_name: opening.tag_name,
        attributes: opening.attributes,
        body,
        children,
    })
}

/// Finds an opening-tag terminator while respecting quoted attribute values.
fn find_opening_end(content: &str) -> Option<usize> {
    let mut quote = None::<u8>;
    for (index, byte) in content.bytes().enumerate() {
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == b'>' {
            return Some(index);
        }
    }
    None
}

/// Parses all quoted attributes from one opening tag.
fn parse_attributes(opening: &str, tag_name: &str) -> HashMap<String, String> {
    let bytes = opening.as_bytes();
    let mut attributes = HashMap::new();
    let mut index = 1 + tag_name.len();
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] == b'>' || bytes[index] == b'/' {
            break;
        }
        let name_start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric()
                || bytes[index] == b'_'
                || bytes[index] == b'-'
                || bytes[index] == b':')
        {
            index += 1;
        }
        if name_start == index {
            return attributes;
        }
        let name = opening[name_start..index].to_string();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            return attributes;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let Some(&quote) = bytes.get(index) else {
            return attributes;
        };
        if quote != b'\'' && quote != b'"' {
            return attributes;
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        if index >= bytes.len() {
            return attributes;
        }
        attributes.insert(name, opening[value_start..index].to_string());
        index += 1;
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::parse_complete_node;

    /// Verifies raw URL ampersands remain valid assistant result text.
    #[test]
    fn parses_raw_ampersands_without_strict_xml_decoding() {
        let node = parse_complete_node(
            r#"<tool_result name="weather"><content>{"url":"https://x.test/?a=1&b=2"}</content></tool_result>"#,
        )
        .expect("tool result must parse");

        assert_eq!(node.tag_name, "tool_result");
        assert_eq!(
            node.attributes.get("name").map(String::as_str),
            Some("weather")
        );
        assert_eq!(node.children.len(), 1);
        assert_eq!(
            node.children[0].body,
            r#"{"url":"https://x.test/?a=1&b=2"}"#
        );
    }
}
