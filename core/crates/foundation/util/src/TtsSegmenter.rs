#[derive(Debug, Clone, Default)]
/// Splits cleaned TTS text into synthesis-sized segments.
pub struct TtsSegmenter;

impl TtsSegmenter {
    /// Segments text without exceeding the requested maximum character count.
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
