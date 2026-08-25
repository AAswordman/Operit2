pub struct DimensionCount {
    pub dimension: i32,
    pub count: i32,
}

pub struct EmbeddingDimensionUsage {
    pub dimensions: Vec<DimensionCount>,
}

pub struct EmbeddingRebuildProgress {
    pub total: i32,
    pub completed: i32,
}
