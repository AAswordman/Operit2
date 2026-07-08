use super::Embedding::Embedding;

pub struct EmbeddingConverter;

impl EmbeddingConverter {
    pub fn convertToEntityProperty(databaseValue: Option<&[u8]>) -> Option<Embedding> {
        let databaseValue = databaseValue?;
        let mut vector = Vec::with_capacity(databaseValue.len() / 4);
        for chunk in databaseValue.chunks_exact(4) {
            vector.push(f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Some(Embedding { vector })
    }

    pub fn convertToDatabaseValue(entityProperty: Option<&Embedding>) -> Option<Vec<u8>> {
        let entityProperty = entityProperty?;
        let mut buffer = Vec::with_capacity(entityProperty.vector.len() * 4);
        for value in &entityProperty.vector {
            buffer.extend_from_slice(&value.to_be_bytes());
        }
        Some(buffer)
    }
}
