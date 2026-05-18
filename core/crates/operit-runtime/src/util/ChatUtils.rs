use crate::util::ChatMarkupRegex::ChatMarkupRegex;

pub struct ChatUtils;

impl ChatUtils {
    pub fn strip_gemini_thought_signature_meta(content: &str) -> String {
        ChatMarkupRegex::remove_gemini_thought_signature_meta(content)
    }

    pub fn strip_gemini_thought_signature_meta_messages(
        messages: &[(String, String)],
    ) -> Vec<(String, String)> {
        messages
            .iter()
            .map(|(role, content)| (role.clone(), Self::strip_gemini_thought_signature_meta(content)))
            .collect()
    }

    pub fn is_gemini_provider_model(provider_model: &str) -> bool {
        matches!(
            provider_model
                .split(':')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase()
                .as_str(),
            "GOOGLE" | "GEMINI_GENERIC"
        )
    }

    pub fn remove_thinking_content(content: &str) -> String {
        let mut ranges = ChatMarkupRegex::think_ranges(content);
        ranges.extend(ChatMarkupRegex::search_ranges(content));
        remove_ranges(content, ranges).trim().to_string()
    }

    pub fn extract_thinking_content(content: &str) -> (String, String) {
        let mut thinking = Vec::new();
        for (start, end) in ChatMarkupRegex::think_ranges(content) {
            let raw = &content[start..end];
            let body = raw
                .split_once('>')
                .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body))
                .unwrap_or("");
            let trimmed = body.trim();
            if !trimmed.is_empty() {
                thinking.push(trimmed.to_string());
            }
        }

        let mut ranges = ChatMarkupRegex::think_ranges(content);
        ranges.extend(ChatMarkupRegex::search_ranges(content));
        let content_without_think = remove_ranges(content, ranges).trim().to_string();
        (content_without_think, thinking.join("\n"))
    }

    pub fn estimate_token_count(text: &str) -> usize {
        let chinese_char_count = text
            .chars()
            .filter(|ch| ('\u{4E00}'..='\u{9FFF}').contains(ch))
            .count();
        let other_char_count = text.chars().count().saturating_sub(chinese_char_count);
        ((chinese_char_count as f64 * 1.5) + (other_char_count as f64 * 0.25)) as usize
    }

    pub fn extract_json(response: &str) -> String {
        let text = strip_markdown_fence(response.trim());
        let first = text.find('{');
        let last = text.rfind('}');
        match (first, last) {
            (Some(start), Some(end)) if start < end => text[start..=end].to_string(),
            _ => text.to_string(),
        }
    }

    pub fn extract_json_array(response: &str) -> String {
        let text = strip_markdown_fence(response.trim());
        let first = text.find('[');
        let last = text.rfind(']');
        match (first, last) {
            (Some(start), Some(end)) if start < end => text[start..=end].to_string(),
            _ => text.to_string(),
        }
    }
}

fn strip_markdown_fence(text: &str) -> &str {
    if !text.starts_with("```") {
        return text;
    }
    let mut lines = text.lines();
    lines.next();
    let mut collected: Vec<&str> = lines.collect();
    if collected.last().map(|line| line.trim() == "```").unwrap_or(false) {
        collected.pop();
    }
    let start = text.find('\n').map(|index| index + 1).unwrap_or(text.len());
    let end = if text.trim_end().ends_with("```") {
        text.rfind("```").unwrap_or(text.len())
    } else {
        text.len()
    };
    text[start..end].trim()
}

fn remove_ranges(content: &str, mut ranges: Vec<(usize, usize)>) -> String {
    ranges.sort_by_key(|range| range.0);
    let mut out = String::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if start >= cursor {
            out.push_str(&content[cursor..start]);
            cursor = end.min(content.len());
        }
    }
    out.push_str(&content[cursor..]);
    out
}
