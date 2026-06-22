#[derive(Debug, Clone, Default)]
pub struct TtsSegmenter;

impl TtsSegmenter {
    pub fn segment(text: &str, maxChars: usize) -> Vec<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        if maxChars == 0 {
            return vec![trimmed.to_string()];
        }
        let mut result = Vec::new();
        let mut current = String::new();
        let mut currentLen = 0usize;
        for ch in trimmed.chars() {
            if currentLen >= maxChars {
                result.push(current.trim().to_string());
                current.clear();
                currentLen = 0;
            }
            current.push(ch);
            currentLen += 1;
        }
        let tail = current.trim();
        if !tail.is_empty() {
            result.push(tail.to_string());
        }
        result
    }
}
