use operit_store::ObjectBoxStore::ObjectBoxEntity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MemoryStoreOwnerKind {
    CHARACTER,
    SHARED,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryStoreOwner {
    pub kind: MemoryStoreOwnerKind,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SharedMemoryStore {
    pub id: String,
    pub name: String,
    pub createdAt: i64,
    pub updatedAt: i64,
}

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryGraph {
    pub nodes: Vec<MemoryGraphNode>,
    pub edges: Vec<MemoryGraphEdge>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MemoryGraphNode {
    pub id: String,
    pub label: String,
    pub color: i64,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryGraphEdge {
    pub id: i64,
    pub sourceId: String,
    pub targetId: String,
    pub label: Option<String>,
    pub weight: f32,
    pub metadata: std::collections::HashMap<String, String>,
    pub isCrossFolderLink: bool,
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
