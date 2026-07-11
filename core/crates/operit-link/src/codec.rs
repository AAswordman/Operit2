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
            Self::Encode(message) => write!(formatter, "CBOR encode error: {message}"),
            Self::Decode(message) => write!(formatter, "CBOR decode error: {message}"),
        }
    }
}

impl std::error::Error for CoreLinkCodecError {}

/// Encodes a serializable link value as CBOR bytes.
#[allow(non_snake_case)]
pub fn encodeCbor(value: impl Serialize) -> Result<Vec<u8>, CoreLinkCodecError> {
    let mut output = Vec::new();
    ciborium::ser::into_writer(&value, &mut output)
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()))?;
    Ok(output)
}

/// Decodes a CBOR byte slice into a link value.
#[allow(non_snake_case)]
pub fn decodeCbor<T>(bytes: &[u8]) -> Result<T, CoreLinkCodecError>
where
    T: DeserializeOwned,
{
    ciborium::de::from_reader(bytes).map_err(|error| CoreLinkCodecError::Decode(error.to_string()))
}

/// Encodes a serializable link value as MessagePack bytes.
#[allow(non_snake_case)]
pub fn encodeMessagePack(value: impl Serialize) -> Result<Vec<u8>, CoreLinkCodecError> {
    rmp_serde::to_vec_named(&value).map_err(|error| CoreLinkCodecError::Encode(error.to_string()))
}

/// Decodes a MessagePack byte slice into a link value.
#[allow(non_snake_case)]
pub fn decodeMessagePack<T>(bytes: &[u8]) -> Result<T, CoreLinkCodecError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes).map_err(|error| CoreLinkCodecError::Decode(error.to_string()))
}
