use serde::{Deserialize, Serialize};
use operit_store::ObjectBoxStore::ObjectBoxEntity;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Memory {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub contentType: String,
    pub source: String,
    pub credibility: f32,
    pub importance: f32,
    pub documentPath: Option<String>,
    pub isDocumentNode: bool,
    pub chunkIndexFilePath: Option<String>,
    pub folderPath: Option<String>,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub lastAccessedAt: i64,
    pub tags: Vec<MemoryTag>,
    pub properties: Vec<MemoryProperty>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MemoryTag {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryLink {
    pub id: i64,
    pub sourceMemoryId: i64,
    pub targetMemoryId: i64,
    pub type_: String,
    pub weight: f32,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MemoryProperty {
    pub id: i64,
    pub key: String,
    pub value: String,
}

impl ObjectBoxEntity for Memory {
    fn objectBoxId(&self) -> i64 {
        self.id
    }

    fn setObjectBoxId(&mut self, id: i64) {
        self.id = id;
    }
}

impl ObjectBoxEntity for MemoryLink {
    fn objectBoxId(&self) -> i64 {
        self.id
    }

    fn setObjectBoxId(&mut self, id: i64) {
        self.id = id;
    }
}
