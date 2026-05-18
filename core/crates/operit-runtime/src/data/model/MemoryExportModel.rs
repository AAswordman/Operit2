use serde::{Deserialize, Serialize};

pub struct DateSerializer;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SerializableMemory {
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub contentType: String,
    pub source: String,
    pub credibility: f32,
    pub importance: f32,
    pub folderPath: Option<String>,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub tagNames: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SerializableLink {
    pub sourceUuid: String,
    pub targetUuid: String,
    pub type_: String,
    pub weight: f32,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryExportData {
    pub memories: Vec<SerializableMemory>,
    pub links: Vec<SerializableLink>,
    pub exportDate: i64,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ImportStrategy {
    SKIP,
    UPDATE,
    CREATE_NEW,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MemoryImportResult {
    pub newMemories: i32,
    pub updatedMemories: i32,
    pub skippedMemories: i32,
    pub newLinks: i32,
}
