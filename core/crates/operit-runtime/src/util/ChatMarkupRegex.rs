use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TOOL_TAG_SUFFIX_REGEX_SOURCE: &str = "[A-Za-z0-9_]+";
pub const TOOL_TAG_NAME_REGEX_SOURCE: &str = "tool(?:_(?!result(?:_|$))[A-Za-z0-9_]+)?";
pub const TOOL_RESULT_TAG_NAME_REGEX_SOURCE: &str = "tool_result(?:_[A-Za-z0-9_]+)?";
pub const GEMINI_THOUGHT_SIGNATURE_PROVIDER: &str = "gemini:thought_signature";

static RANDOM_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ToolCallMatch {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
}

pub struct ChatMarkupRegex;

impl ChatMarkupRegex {
    pub fn is_tool_tag_name(tag_name: Option<&str>) -> bool {
        tag_name
            .map(|name| {
                let lower = name.to_ascii_lowercase();
                lower == "tool"
                    || (lower.starts_with("tool_")
                        && !lower.starts_with("tool_result")
                        && lower["tool_".len()..].chars().all(is_tool_suffix_char))
            })
            .unwrap_or(false)
    }

    pub fn is_tool_result_tag_name(tag_name: Option<&str>) -> bool {
        tag_name
            .map(|name| {
                let lower = name.to_ascii_lowercase();
                lower == "tool_result"
                    || (lower.starts_with("tool_result_")
                        && lower["tool_result_".len()..].chars().all(is_tool_suffix_char))
            })
            .unwrap_or(false)
    }

    pub fn normalize_tool_like_tag_name(tag_name: Option<&str>) -> Option<String> {
        match tag_name {
            Some(name) if Self::is_tool_tag_name(Some(name)) => Some("tool".to_string()),
            Some(name) if Self::is_tool_result_tag_name(Some(name)) => Some("tool_result".to_string()),
            Some(name) => Some(name.to_string()),
            None => None,
        }
    }

    pub fn contains_tool_tag(content: &str) -> bool {
        contains_start_tag(content, Self::is_tool_tag_name)
    }

    pub fn contains_tool_result_tag(content: &str) -> bool {
        contains_start_tag(content, Self::is_tool_result_tag_name)
    }

    pub fn contains_any_tool_like_tag(content: &str) -> bool {
        Self::contains_tool_tag(content) || Self::contains_tool_result_tag(content)
    }

    pub fn extract_opening_tag_name(xml: &str) -> Option<String> {
        let trimmed = xml.trim_start();
        if !trimmed.starts_with('<') {
            return None;
        }
        let mut name = String::new();
        for ch in trimmed[1..].chars() {
            if name.is_empty() {
                if ch.is_ascii_alphabetic() {
                    name.push(ch);
                } else {
                    return None;
                }
            } else if ch.is_ascii_alphanumeric() || ch == '_' {
                name.push(ch);
            } else {
                break;
            }
        }
        if name.is_empty() { None } else { Some(name) }
    }

    pub fn generate_random_tool_tag_name() -> String {
        format!("tool_{}", generate_random_tag_code(4))
    }

    pub fn generate_random_tool_result_tag_name() -> String {
        format!("tool_result_{}", generate_random_tag_code(4))
    }

    pub fn gemini_thought_signature_meta_tag(signature_base64: &str) -> String {
        format!(r#"<meta provider="{GEMINI_THOUGHT_SIGNATURE_PROVIDER}">{signature_base64}</meta>"#)
    }

    pub fn extract_gemini_thought_signature(content: &str) -> Option<String> {
        let mut last = None;
        for (start, end) in tag_ranges(content, "meta") {
            let tag = &content[start..end];
            if attr_value(tag, "provider")
                .map(|provider| provider.eq_ignore_ascii_case(GEMINI_THOUGHT_SIGNATURE_PROVIDER))
                .unwrap_or(false)
            {
                if let Some(body) = tag_body(tag, "meta") {
                    let trimmed = body.trim();
                    if !trimmed.is_empty() {
                        last = Some(trimmed.to_string());
                    }
                }
            }
        }
        last
    }

    pub fn remove_gemini_thought_signature_meta(content: &str) -> String {
        let mut out = String::new();
        let mut cursor = 0;
        let mut removed = false;
        for (start, end) in tag_ranges(content, "meta") {
            let tag = &content[start..end];
            let is_signature = attr_value(tag, "provider")
                .map(|provider| provider.eq_ignore_ascii_case(GEMINI_THOUGHT_SIGNATURE_PROVIDER))
                .unwrap_or(false);
            if is_signature {
                out.push_str(&content[cursor..start]);
                cursor = end;
                removed = true;
            }
        }
        out.push_str(&content[cursor..]);
        if removed {
            out.trim_end().to_string()
        } else {
            content.to_string()
        }
    }

    pub fn tool_call_matches(content: &str) -> Vec<ToolCallMatch> {
        find_named_blocks(content, Self::is_tool_tag_name)
            .into_iter()
            .filter_map(|block| {
                let name = attr_value(&block.raw, "name")?;
                Some(ToolCallMatch {
                    tag_name: block.tag_name,
                    name,
                    body: block.body,
                    start: block.start,
                    end: block.end,
                })
            })
            .collect()
    }

    pub fn tool_result_blocks(content: &str) -> Vec<XmlBlock> {
        find_named_blocks(content, Self::is_tool_result_tag_name)
    }

    pub fn think_ranges(content: &str) -> Vec<(usize, usize)> {
        let mut ranges = tag_ranges(content, "think");
        ranges.extend(tag_ranges(content, "thinking"));
        ranges.sort_by_key(|range| range.0);
        ranges
    }

    pub fn search_ranges(content: &str) -> Vec<(usize, usize)> {
        tag_ranges(content, "search")
    }

    pub fn attachment_ranges(content: &str) -> Vec<(usize, usize)> {
        let mut ranges = tag_ranges(content, "attachment");
        ranges.extend(self_closing_tag_ranges(content, "attachment"));
        ranges.sort_by_key(|range| range.0);
        ranges
    }

    pub fn workspace_attachment_ranges(content: &str) -> Vec<(usize, usize)> {
        tag_ranges(content, "workspace_attachment")
    }

    pub fn proxy_sender_name(content: &str) -> Option<String> {
        let start = content.find("<proxy_sender")?;
        let end = content[start..].find("/>")? + start + 2;
        attr_value(&content[start..end], "name")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XmlBlock {
    pub tag_name: String,
    pub raw: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
}

pub fn attr_value(text: &str, attr_name: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let lower_text = text.to_ascii_lowercase();
    let lower_attr_name = attr_name.to_ascii_lowercase();
    let mut index = 0;
    while index < bytes.len() {
        let found = lower_text[index..].find(&lower_attr_name)?;
        let name_start = index + found;
        let name_end = name_start + lower_attr_name.len();
        let before_ok = name_start == 0 || !is_attr_name_byte(bytes[name_start - 1]);
        let after_ok = name_end >= bytes.len() || !is_attr_name_byte(bytes[name_end]);
        if before_ok && after_ok {
            let mut cursor = name_end;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                    let quote = bytes[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    return Some(text[value_start..cursor].to_string());
                }
            }
        }
        index = name_end;
    }
    None
}

pub fn tag_body<'a>(tag: &'a str, tag_name: &str) -> Option<&'a str> {
    let open_end = tag.find('>')? + 1;
    let close = format!("</{tag_name}>");
    let close_start = tag.to_ascii_lowercase().rfind(&close.to_ascii_lowercase())?;
    Some(&tag[open_end..close_start])
}

pub fn tag_ranges(content: &str, tag_name: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    let lower = content.to_ascii_lowercase();
    let open_prefix = format!("<{}", tag_name.to_ascii_lowercase());
    let close = format!("</{}>", tag_name.to_ascii_lowercase());
    while let Some(relative_start) = lower[cursor..].find(&open_prefix) {
        let start = cursor + relative_start;
        let after_name = start + open_prefix.len();
        if lower.as_bytes().get(after_name).map(|byte| is_tag_boundary(*byte)).unwrap_or(false) {
            if let Some(relative_close) = lower[after_name..].find(&close) {
                let end = after_name + relative_close + close.len();
                ranges.push((start, end));
                cursor = end;
                continue;
            }
            ranges.push((start, content.len()));
            break;
        }
        cursor = after_name;
    }
    ranges
}

pub fn self_closing_tag_ranges(content: &str, tag_name: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    let lower = content.to_ascii_lowercase();
    let open_prefix = format!("<{}", tag_name.to_ascii_lowercase());
    while let Some(relative_start) = lower[cursor..].find(&open_prefix) {
        let start = cursor + relative_start;
        if let Some(relative_end) = lower[start..].find("/>") {
            let end = start + relative_end + 2;
            ranges.push((start, end));
            cursor = end;
        } else {
            break;
        }
    }
    ranges
}

fn find_named_blocks(content: &str, predicate: fn(Option<&str>) -> bool) -> Vec<XmlBlock> {
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = content[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(name) = ChatMarkupRegex::extract_opening_tag_name(&content[start..]) else {
            cursor = start + 1;
            continue;
        };
        if !predicate(Some(&name)) {
            cursor = start + 1;
            continue;
        }
        let close = format!("</{name}>");
        let lower_tail = content[start..].to_ascii_lowercase();
        let close_lower = close.to_ascii_lowercase();
        if let Some(relative_end) = lower_tail.find(&close_lower) {
            let end = start + relative_end + close.len();
            let raw = content[start..end].to_string();
            let body = tag_body(&raw, &name).unwrap_or("").to_string();
            blocks.push(XmlBlock {
                tag_name: name,
                raw,
                body,
                start,
                end,
            });
            cursor = end;
        } else {
            cursor = start + 1;
        }
    }
    blocks
}

fn contains_start_tag(content: &str, predicate: fn(Option<&str>) -> bool) -> bool {
    let mut cursor = 0;
    while let Some(relative_start) = content[cursor..].find('<') {
        let start = cursor + relative_start;
        if let Some(name) = ChatMarkupRegex::extract_opening_tag_name(&content[start..]) {
            if predicate(Some(&name)) {
                return true;
            }
        }
        cursor = start + 1;
    }
    false
}

fn generate_random_tag_code(length: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut value = now ^ RANDOM_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut out = String::with_capacity(length);
    for _ in 0..length {
        value = value.wrapping_mul(6364136223846793005).wrapping_add(1);
        out.push(CHARS[(value as usize) % CHARS.len()] as char);
    }
    out
}

fn is_tool_suffix_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_tag_boundary(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'>' || byte == b'/'
}

fn is_attr_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}
