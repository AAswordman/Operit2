use crate::util::ChatMarkupRegex::ChatMarkupRegex;

pub struct NativeXmlSplitter;

impl NativeXmlSplitter {
    pub fn split_xml_tag(content: &str) -> Vec<Vec<String>> {
        split_xml_tag(content)
    }
}

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

        let Some(open_end_relative) = content[start..].find('>') else {
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
            results.push(vec!["text".to_string(), content[start..open_end].to_string()]);
            cursor = open_end;
        }
    }
    results
}
