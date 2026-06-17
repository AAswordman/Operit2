use crate::util::ChatMarkupRegex::ChatMarkupRegex;
use regex::Regex;

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
            .map(|(role, content)| {
                (
                    role.clone(),
                    Self::strip_gemini_thought_signature_meta(content),
                )
            })
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
        let think_pattern =
            Regex::new(r"(?s)<think(?:ing)?>.*?(</think(?:ing)?>|\z)").expect("think regex");
        let search_pattern = Regex::new(r"(?s)<search>.*?(</search>|\z)").expect("search regex");
        search_pattern
            .replace_all(&think_pattern.replace_all(content, ""), "")
            .trim()
            .to_string()
    }

    pub fn extract_thinking_content(content: &str) -> (String, String) {
        let think_pattern =
            Regex::new(r"(?s)<think(?:ing)?>(.*?)</think(?:ing)?>").expect("think regex");
        let thinking = think_pattern
            .captures_iter(content)
            .map(|capture| capture.get(1).map(|value| value.as_str()).unwrap_or("").trim())
            .collect::<Vec<_>>()
            .join("\n");
        let search_pattern = Regex::new(r"(?s)<search>.*?(</search>|\z)").expect("search regex");
        let content_without_think = search_pattern
            .replace_all(&think_pattern.replace_all(content, ""), "")
            .trim()
            .to_string();
        (content_without_think, thinking)
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
    if collected
        .last()
        .map(|line| line.trim() == "```")
        .unwrap_or(false)
    {
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

#[cfg(test)]
mod tests {
    use super::ChatUtils;

    #[test]
    fn remove_thinking_content_mirrors_kt_pure_thinking_detection() {
        assert_eq!(ChatUtils::remove_thinking_content("<think>abc</think>"), "");
        assert_eq!(
            ChatUtils::remove_thinking_content("<thinking>abc</thinking>"),
            ""
        );
        assert_eq!(ChatUtils::remove_thinking_content("<think>abc"), "");
        assert_eq!(
            ChatUtils::remove_thinking_content("<think>abc</think>\n正文"),
            "正文"
        );
        assert_eq!(
            ChatUtils::remove_thinking_content("<search>source</search>\n正文"),
            "正文"
        );
    }

    #[test]
    fn extract_thinking_content_mirrors_kt_closed_think_extraction() {
        assert_eq!(
            ChatUtils::extract_thinking_content("<think>a</think>\n正文<search>x"),
            ("正文".to_string(), "a".to_string())
        );
        assert_eq!(
            ChatUtils::extract_thinking_content("<think>a</think><thinking>b</thinking>正文"),
            ("正文".to_string(), "a\nb".to_string())
        );
    }
}
