use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug)]
pub enum CoreLinkCodecError {
    Encode(String),
    Decode(String),
}

impl std::fmt::Display for CoreLinkCodecError {
    /// Formats a codec error for diagnostics.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(message) => write!(formatter, "MessagePack encode error: {message}"),
            Self::Decode(message) => write!(formatter, "MessagePack decode error: {message}"),
        }
    }
}

impl std::error::Error for CoreLinkCodecError {}

/// Encodes a serializable value using the single Link MessagePack representation.
#[allow(non_snake_case)]
pub fn encodeLink(value: impl Serialize) -> Result<Vec<u8>, CoreLinkCodecError> {
    let mut output = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut output).with_struct_map();
    value
        .serialize(&mut serializer)
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()))?;
    Ok(output)
}

/// Decodes a value from the single Link MessagePack representation.
#[allow(non_snake_case)]
pub fn decodeLink<T>(bytes: &[u8]) -> Result<T, CoreLinkCodecError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes).map_err(|error| CoreLinkCodecError::Decode(error.to_string()))
}
