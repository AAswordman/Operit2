#[derive(Debug, Clone, Default)]
pub struct TtsCleaner;

impl TtsCleaner {
    pub fn clean(text: &str) -> String {
        text.trim().to_string()
    }
}
