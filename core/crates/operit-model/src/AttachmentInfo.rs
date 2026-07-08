use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub filePath: String,
    pub fileName: String,
    pub mimeType: String,
    pub fileSize: i64,
    pub content: String,
}

impl AttachmentInfo {
    pub fn new(filePath: String, fileName: String, mimeType: String, fileSize: i64) -> Self {
        Self {
            filePath,
            fileName,
            mimeType,
            fileSize,
            content: String::new(),
        }
    }
}
