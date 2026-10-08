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
    encodeLinkInto(value, &mut output)?;
    Ok(output)
}

/// Reuses an owned buffer for the identical Link representation. Failed output
/// allocation is a codec error, not a process abort. On error the buffer is
/// cleared; callers must not transmit or decode partially encoded data.
#[allow(non_snake_case)]
pub fn encodeLinkInto(value: impl Serialize, output: &mut Vec<u8>) -> Result<(), CoreLinkCodecError> {
    struct Output<'a>(&'a mut Vec<u8>);
    impl std::io::Write for Output<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.try_reserve_exact(bytes.len())
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    output.clear();
    // Serialize exactly once: stream serializers can capture source handles.
    // The writer grows fallibly by the actual additional byte count, without
    // an infallible amortized allocation of a second payload-sized block.
    let result = value.serialize(&mut rmp_serde::Serializer::new(Output(output)).with_struct_map())
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()));
    if result.is_err() { output.clear(); }
    result
}

/// Decodes a value from the single Link MessagePack representation.
#[allow(non_snake_case)]
pub fn decodeLink<T>(bytes: &[u8]) -> Result<T, CoreLinkCodecError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes).map_err(|error| CoreLinkCodecError::Decode(error.to_string()))
}
