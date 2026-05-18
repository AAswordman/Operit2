pub struct Memory {
    pub id: String,
    pub content: String,
}

pub struct MemoryTag {
    pub id: String,
    pub name: String,
}

pub struct MemoryLink {
    pub id: String,
    pub sourceMemoryId: String,
    pub targetMemoryId: String,
}

pub struct MemoryProperty {
    pub key: String,
    pub value: String,
}
