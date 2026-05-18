pub enum MemoryScoreMode {
    VECTOR,
    KEYWORD,
    HYBRID,
}

pub struct MemorySearchConfig {
    pub scoreMode: MemoryScoreMode,
}
