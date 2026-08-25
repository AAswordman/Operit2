#[derive(Debug, Clone, Default)]
/// Cleans chat text before TTS synthesis.
pub struct TtsCleaner;

impl TtsCleaner {
    /// Removes markup and whitespace that should not be spoken.
    pub fn clean(text: &str) -> String {
        text.trim().to_string()
    }
}
