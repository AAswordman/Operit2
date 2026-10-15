use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::CoreBytes;
use std::cell::RefCell;
use std::sync::Arc;

pub const SHARED_BUFFER_EXT: i8 = 0x42;

thread_local! {
    static ENCODE_BUFFERS: RefCell<Option<Vec<CoreBytes>>> = const { RefCell::new(None) };
    static DECODE_BUFFERS: RefCell<Option<Arc<[CoreBytes]>>> = const { RefCell::new(None) };
}

struct EncodeBufferScope(Option<Vec<CoreBytes>>);
impl Drop for EncodeBufferScope {
    /// Restores the enclosing serializer's ownership context even during unwinding.
    fn drop(&mut self) {
        ENCODE_BUFFERS.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

struct DecodeBufferScope(Option<Arc<[CoreBytes]>>);
impl Drop for DecodeBufferScope {
    /// Restores the enclosing decoder without retaining a completed request table.
    fn drop(&mut self) {
        DECODE_BUFFERS.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

/// Captures shared binary ownership while serializing only reference metadata.
#[allow(non_snake_case)]
pub fn captureLinkBuffers<T>(encode: impl FnOnce() -> T) -> (T, Vec<CoreBytes>) {
    let scope = EncodeBufferScope(ENCODE_BUFFERS.with(|slot| slot.replace(Some(Vec::new()))));
    let value = encode();
    let buffers = ENCODE_BUFFERS.with(|slot| slot.borrow_mut().take().expect("active buffer capture"));
    drop(scope);
    (value, buffers)
}

/// Resolves shared references during synchronous decoding, before async dispatch.
#[allow(non_snake_case)]
pub fn withLinkBuffers<T>(buffers: Arc<[CoreBytes]>, decode: impl FnOnce() -> T) -> T {
    let scope = DecodeBufferScope(DECODE_BUFFERS.with(|slot| slot.replace(Some(buffers))));
    let value = decode();
    drop(scope);
    value
}

/// Serializes shared references in a capture scope and ordinary bytes on byte carriers.
pub(crate) fn serialize_shared_bytes<S: serde::Serializer>(bytes: &CoreBytes, serializer: S) -> Result<S::Ok, S::Error> {
    let index = ENCODE_BUFFERS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.as_mut().map(|buffers| {
            let index = buffers.len();
            buffers.push(bytes.clone());
            index
        })
    });
    match index {
        Some(index) => {
            let mut reference = [0u8; 24];
            reference[..8].copy_from_slice(&(index as u64).to_be_bytes());
            reference[16..].copy_from_slice(&(bytes.len() as u64).to_be_bytes());
            serializer.serialize_newtype_struct(rmp_serde::MSGPACK_EXT_STRUCT_NAME,
                &(SHARED_BUFFER_EXT, serde_bytes::Bytes::new(&reference)))
        }
        None => serializer.serialize_bytes(bytes),
    }
}

/// Validates a reference and retains the indicated slice without reading its contents.
pub(crate) fn resolve_shared_bytes(tag: i8, payload: &[u8]) -> Result<CoreBytes, &'static str> {
    if tag != SHARED_BUFFER_EXT || payload.len() != 24 { return Err("invalid shared buffer extension"); }
    let number = |start| usize::try_from(u64::from_be_bytes(payload[start..start + 8].try_into().unwrap()))
        .map_err(|_| "shared buffer range overflow");
    let index = number(0)?;
    let offset = number(8)?;
    let length = number(16)?;
    let end = offset.checked_add(length).ok_or("shared buffer range overflow")?;
    DECODE_BUFFERS.with(|slot| {
        let slot = slot.borrow();
        let buffers = slot.as_ref().ok_or("shared buffer context is absent")?;
        let bytes = buffers.get(index).ok_or("shared buffer index is invalid")?;
        if end > bytes.len() { return Err("shared buffer range is invalid"); }
        Ok(bytes.slice(offset..end))
    })
}

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
pub fn encodeLinkInto(
    value: impl Serialize,
    output: &mut Vec<u8>,
) -> Result<(), CoreLinkCodecError> {
    output.clear();
    appendLink(value, output)
}

/// Appends one value after an existing frame header without copying the payload.
/// Serialization runs once; failure clears the entire incomplete frame.
#[allow(non_snake_case)]
pub fn appendLink(value: impl Serialize, output: &mut Vec<u8>) -> Result<(), CoreLinkCodecError> {
    struct Output<'a>(&'a mut Vec<u8>);
    impl std::io::Write for Output<'_> {
        /// Grows fallibly with amortized capacity across small serializer writes.
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .try_reserve(bytes.len())
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        /// Completes an in-memory write without flushing an external destination.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    // Serialize exactly once: stream serializers can capture source handles.
    // Fallible amortized growth avoids reallocating for every scalar or map key.
    let result = value
        .serialize(&mut rmp_serde::Serializer::new(Output(output)).with_struct_map())
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()));
    if result.is_err() {
        output.clear();
    }
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
