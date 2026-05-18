pub struct PromptTag {
    pub id: String,
    pub name: String,
    pub tagType: TagType,
}

pub enum TagType {
    SYSTEM,
    USER,
}
