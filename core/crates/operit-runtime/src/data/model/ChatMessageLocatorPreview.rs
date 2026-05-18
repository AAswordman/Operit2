#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessageLocatorPreview {
    pub timestamp: i64,
    pub sender: String,
    pub previewContent: String,
    pub contentLength: i32,
    pub displayMode: String,
    pub isFavorite: bool,
}
