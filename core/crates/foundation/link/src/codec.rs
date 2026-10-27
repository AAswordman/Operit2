use crate::CoreBytes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::sync::Arc;

pub const SHARED_BUFFER_EXT: i8 = 0x42;

thread_local! {
    static ENCODE_BUFFERS: RefCell<Option<Vec<CoreBytes>>> = const { RefCell::new(None) };
    static DECODE_SOURCE: RefCell<Option<CoreBytes>> = const { RefCell::new(None) };
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
    let buffers =
        ENCODE_BUFFERS.with(|slot| slot.borrow_mut().take().expect("active buffer capture"));
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
pub(crate) fn serialize_shared_bytes<S: serde::Serializer>(
    bytes: &CoreBytes,
    serializer: S,
) -> Result<S::Ok, S::Error> {
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
            serializer.serialize_newtype_struct(
                rmp_serde::MSGPACK_EXT_STRUCT_NAME,
                &(SHARED_BUFFER_EXT, serde_bytes::Bytes::new(&reference)),
            )
        }
        None => serializer.serialize_bytes(bytes),
    }
}

/// Validates a reference and retains the indicated slice without reading its contents.
pub(crate) fn resolve_shared_bytes(tag: i8, payload: &[u8]) -> Result<CoreBytes, &'static str> {
    if tag != SHARED_BUFFER_EXT || payload.len() != 24 {
        return Err("invalid shared buffer extension");
    }
    let number = |start| {
        usize::try_from(u64::from_be_bytes(
            payload[start..start + 8].try_into().unwrap(),
        ))
        .map_err(|_| "shared buffer range overflow")
    };
    let index = number(0)?;
    let offset = number(8)?;
    let length = number(16)?;
    let end = offset
        .checked_add(length)
        .ok_or("shared buffer range overflow")?;
    DECODE_BUFFERS.with(|slot| {
        let slot = slot.borrow();
        let buffers = slot.as_ref().ok_or("shared buffer context is absent")?;
        let bytes = buffers.get(index).ok_or("shared buffer index is invalid")?;
        if end > bytes.len() {
            return Err("shared buffer range is invalid");
        }
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
// Keep one serializer instance per value type across routing and transport.
#[inline(never)]
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
        /// Grows fallibly in bounded blocks across small serializer writes.
        /// Counts encoded bytes without retaining payload data.
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let memory = || std::io::Error::from(std::io::ErrorKind::OutOfMemory);
            if !cfg!(target_os = "espidf") {
                self.0.try_reserve(bytes.len()).map_err(|_| memory())?;
                self.0.extend_from_slice(bytes);
                return Ok(bytes.len());
            }
            let required = self.0.len().checked_add(bytes.len()).ok_or_else(memory)?;
            if required > self.0.capacity() {
                // Keep growth bounded while the complete source tree is still
                // alive. Doubling a multi-KiB buffer needs two large blocks at
                // once on small fragmented heaps. Serialize only once so stream
                // source capture and attachment registration remain unchanged.
                let growth = self.0.capacity().clamp(256, 512);
                let capacity = required.max(self.0.capacity().saturating_add(growth));
                self.0
                    .try_reserve_exact(capacity - self.0.len())
                    .map_err(|_| memory())?;
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        /// Completes an in-memory write without flushing an external destination.
        /// Completes a counting writer without allocating storage.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    // Serialize exactly once: stream serializers can capture source handles.
    // Fallible bounded growth avoids reallocating for every scalar or map key.
    let result = value
        .serialize(&mut rmp_serde::Serializer::new(Output(output)).with_struct_map())
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()));
    if result.is_err() {
        output.clear();
    }
    result
}

/// Measures plain serializable data without allocating an encoded buffer. The
/// serializer still runs once; do not use this with consuming replies or streams.
#[allow(non_snake_case)]
pub fn encodedLinkSize(value: impl Serialize) -> Result<usize, CoreLinkCodecError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        /// Counts encoded bytes without retaining payload data.
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
            Ok(bytes.len())
        }
        /// Completes a counting writer without allocating storage.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Counter(0);
    value
        .serialize(&mut rmp_serde::Serializer::new(&mut count).with_struct_map())
        .map_err(|error| CoreLinkCodecError::Encode(error.to_string()))?;
    Ok(count.0)
}

/// Decodes a value from the single Link MessagePack representation.
// Reuse each typed MessagePack decoder across callers instead of inlining
// another copy into every routing branch under firmware Fat LTO.
#[inline(never)]
#[allow(non_snake_case)]
pub fn decodeLink<T>(bytes: &[u8]) -> Result<T, CoreLinkCodecError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes).map_err(|error| CoreLinkCodecError::Decode(error.to_string()))
}

/// Decodes owned transport bytes while retaining binary fields as shared slices.
/// Borrowed/scratch slices outside this explicit scope still copy normally.
#[allow(non_snake_case)]
pub fn decodeLinkOwned<T: DeserializeOwned>(bytes: CoreBytes) -> Result<T, CoreLinkCodecError> {
    decodeOwnedWith(bytes, decodeLink)
}

/// Retains one owned frame while a typed decoder selects its borrowed fields.
pub(crate) fn decodeOwnedWith<T>(bytes: CoreBytes, decode: impl FnOnce(&[u8]) -> Result<T, CoreLinkCodecError>) -> Result<T, CoreLinkCodecError> {
    struct Scope(Option<CoreBytes>);
    impl Drop for Scope {
        /// Restores the previous owned decode source after this scope.
        fn drop(&mut self) {
            DECODE_SOURCE.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _scope = Scope(DECODE_SOURCE.with(|slot| slot.replace(Some(bytes.clone()))));
    decode(&bytes)
}
pub(crate) fn retain_decoded_slice(value: &[u8]) -> Option<CoreBytes> {
    DECODE_SOURCE.with(|slot| {
        let source = slot.borrow();
        let source = source.as_ref()?;
        let offset = (value.as_ptr() as usize).checked_sub(source.as_ptr() as usize)?;
        let end = offset.checked_add(value.len())?;
        (end <= source.len()).then(|| source.slice(offset..end))
    })
}

/// Retains the exact attachment currently exposed to a structured serializer.
pub(crate) fn retain_encoded_buffer(
    index: usize,
    offset: usize,
    length: usize,
) -> Result<CoreBytes, &'static str> {
    let end = offset
        .checked_add(length)
        .ok_or("shared buffer range overflow")?;
    ENCODE_BUFFERS.with(|slot| {
        let slot = slot.borrow();
        let buffers = slot.as_ref().ok_or("shared encoding context is absent")?;
        let bytes = buffers.get(index).ok_or("shared buffer index is invalid")?;
        if end > bytes.len() {
            return Err("shared buffer range is invalid");
        }
        Ok(bytes.slice(offset..end))
    })
}

#[cfg(test)]
mod owned_decode_tests {
    use super::*;
    use crate::CoreValue;
    #[test]
    fn owned_binary_retains_source_and_borrowed_binary_still_copies() {
        let encoded = encodeLink(CoreValue::Bytes(vec![7; 8192].into())).unwrap();
        let source: CoreBytes = encoded.into();
        let base = source.as_ptr() as usize;
        let end = base + source.len();
        let CoreValue::Bytes(value) = decodeLinkOwned::<CoreValue>(source.clone()).unwrap() else {
            panic!("binary expected")
        };
        assert!((base..end).contains(&(value.as_ptr() as usize)));
        let CoreValue::Bytes(copied) = decodeLink::<CoreValue>(&source).unwrap() else {
            panic!("binary expected")
        };
        assert!(!(base..end).contains(&(copied.as_ptr() as usize)));
        assert_eq!(value, copied);
        drop(source);
        assert!(
            value.try_into_mut().is_ok(),
            "completed decode scope must release its source"
        );
    }
    #[test]
    fn nested_decode_restores_the_outer_owned_source() {
        struct Nested(CoreValue);
        impl<'de> serde::Deserialize<'de> for Nested {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                assert!(decodeLinkOwned::<CoreValue>(vec![0xc1].into()).is_err());
                CoreValue::deserialize(deserializer).map(Nested)
            }
        }
        let source: CoreBytes = encodeLink(CoreValue::Bytes(vec![3; 1024].into()))
            .unwrap()
            .into();
        let base = source.as_ptr() as usize;
        let end = base + source.len();
        let Nested(CoreValue::Bytes(value)) = decodeLinkOwned::<Nested>(source).unwrap() else {
            panic!("binary expected")
        };
        assert!((base..end).contains(&(value.as_ptr() as usize)));
        assert!(value.try_into_mut().is_ok());
    }
    #[test]
    fn failed_owned_decode_does_not_retain_a_source_scope() {
        let value: CoreBytes = vec![0xc1].into();
        assert!(decodeLinkOwned::<CoreValue>(value.clone()).is_err());
        assert!(value.try_into_mut().is_ok());
    }
}

/// Finds a map field without building a value tree or recursing through nested payloads.
/// The eventual typed decoder remains responsible for validating the complete schema.
#[allow(non_snake_case)]
pub fn linkMapField<'a>(
    bytes: &'a [u8],
    field: &str,
) -> Result<Option<&'a [u8]>, CoreLinkCodecError> {
    let mut cursor = FieldCursor { bytes, at: 0 };
    let count = match cursor.marker()? {
        rmp::Marker::FixMap(count) => usize::from(count),
        rmp::Marker::Map16 => cursor.number(2)?,
        rmp::Marker::Map32 => cursor.number(4)?,
        _ => return Err(FieldCursor::invalid()),
    };
    let mut selected = None;
    for _ in 0..count {
        let length = match cursor.marker()? {
            rmp::Marker::FixStr(length) => usize::from(length),
            rmp::Marker::Str8 => cursor.number(1)?,
            rmp::Marker::Str16 => cursor.number(2)?,
            rmp::Marker::Str32 => cursor.number(4)?,
            _ => return Err(FieldCursor::invalid()),
        };
        let key = std::str::from_utf8(cursor.take(length)?).map_err(|_| FieldCursor::invalid())?;
        let start = cursor.at;
        cursor.skip()?;
        if key == field {
            if selected.is_some() { return Err(FieldCursor::invalid()); }
            selected = Some(&bytes[start..cursor.at]);
        }
    }
    if cursor.at != bytes.len() { return Err(FieldCursor::invalid()); }
    Ok(selected)
}

/// Selects a declared struct field from named-map or positional-array encoding.
#[allow(non_snake_case)]
pub fn linkStructField<'a>(bytes: &'a [u8], field: &str, index: usize)
    -> Result<Option<&'a [u8]>, CoreLinkCodecError> {
    let mut cursor = FieldCursor { bytes, at: 0 };
    let count = match cursor.marker()? {
        rmp::Marker::FixMap(_) | rmp::Marker::Map16 | rmp::Marker::Map32 => return linkMapField(bytes, field),
        rmp::Marker::FixArray(count) => usize::from(count),
        rmp::Marker::Array16 => cursor.number(2)?,
        rmp::Marker::Array32 => cursor.number(4)?,
        _ => return Err(FieldCursor::invalid()),
    };
    let mut selected = None;
    for current in 0..count {
        let start = cursor.at;
        cursor.skip()?;
        if current == index { selected = Some(&bytes[start..cursor.at]); }
    }
    if cursor.at != bytes.len() { return Err(FieldCursor::invalid()); }
    Ok(selected)
}

/// Advances over encoded values with a counter instead of recursive decoder frames.
struct FieldCursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> FieldCursor<'a> {
    /// Reports invalid or truncated field envelopes consistently.
    fn invalid() -> CoreLinkCodecError {
        CoreLinkCodecError::Decode("Invalid MessagePack map field".into())
    }

    /// Returns a checked byte range and advances the cursor.
    fn take(&mut self, length: usize) -> Result<&'a [u8], CoreLinkCodecError> {
        let end = self.at.checked_add(length).ok_or_else(Self::invalid)?;
        let value = self.bytes.get(self.at..end).ok_or_else(Self::invalid)?;
        self.at = end;
        Ok(value)
    }

    /// Reads one format marker using the shared MessagePack marker definitions.
    fn marker(&mut self) -> Result<rmp::Marker, CoreLinkCodecError> {
        Ok(rmp::Marker::from_u8(self.take(1)?[0]))
    }

    /// Reads an unsigned MessagePack length with checked native-size arithmetic.
    fn number(&mut self, width: usize) -> Result<usize, CoreLinkCodecError> {
        self.take(width)?.iter().try_fold(0usize, |value, byte| {
            value
                .checked_mul(256)
                .and_then(|value| value.checked_add(usize::from(*byte)))
                .ok_or_else(Self::invalid)
        })
    }

    /// Skips exactly one encoded value with constant stack and no payload allocation.
    fn skip(&mut self) -> Result<(), CoreLinkCodecError> {
        use rmp::Marker::*;
        let mut remaining = 1usize;
        while remaining != 0 {
            remaining -= 1;
            let mut children = 0;
            let length = match self.marker()? {
                FixPos(_) | FixNeg(_) | Null | False | True => 0,
                U8 | I8 => 1,
                U16 | I16 => 2,
                U32 | I32 | F32 => 4,
                U64 | I64 | F64 => 8,
                FixStr(length) => usize::from(length),
                Str8 | Bin8 => self.number(1)?,
                Str16 | Bin16 => self.number(2)?,
                Str32 | Bin32 => self.number(4)?,
                FixExt1 => 2,
                FixExt2 => 3,
                FixExt4 => 5,
                FixExt8 => 9,
                FixExt16 => 17,
                Ext8 => self.number(1)?.checked_add(1).ok_or_else(Self::invalid)?,
                Ext16 => self.number(2)?.checked_add(1).ok_or_else(Self::invalid)?,
                Ext32 => self.number(4)?.checked_add(1).ok_or_else(Self::invalid)?,
                FixArray(count) => {
                    children = usize::from(count);
                    0
                }
                Array16 => {
                    children = self.number(2)?;
                    0
                }
                Array32 => {
                    children = self.number(4)?;
                    0
                }
                FixMap(count) => {
                    children = usize::from(count) * 2;
                    0
                }
                Map16 => {
                    children = self.number(2)?.checked_mul(2).ok_or_else(Self::invalid)?;
                    0
                }
                Map32 => {
                    children = self.number(4)?.checked_mul(2).ok_or_else(Self::invalid)?;
                    0
                }
                Reserved => return Err(Self::invalid()),
            };
            self.take(length)?;
            remaining = remaining.checked_add(children).ok_or_else(Self::invalid)?;
            if remaining > self.bytes.len().saturating_sub(self.at) {
                return Err(Self::invalid());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod field_scan_tests {
    use super::*;

    /// Named and positional structs select the same field without decoding preceding payloads.
    #[test]
    fn struct_fields_select_declared_wire_layouts() {
        for value in [serde_json::json!({"ignore":{"deep":[1,2]},"selected":42}),
            serde_json::json!([{"deep":[1,2]},42])] {
            let bytes = encodeLink(value).unwrap();
            let selected = linkStructField(&bytes, "selected", 1).unwrap().unwrap();
            assert_eq!(decodeLink::<u32>(selected).unwrap(), 42);
            assert!(linkStructField(&bytes, "missing", 2).unwrap().is_none());
            assert!(linkStructField(&bytes[..bytes.len()-1], "selected", 1).is_err());
        }
        assert!(linkStructField(&[0xc0], "selected", 0).is_err());
        let duplicate = [0x82, 0xa1, b'x', 0x01, 0xa1, b'x', 0x02];
        assert!(linkStructField(&duplicate, "x", 0).is_err());
        assert!(linkStructField(&[0x91, 0x01, 0x02], "x", 0).is_err());
    }

    /// Scans nested payloads without requiring recursive deserialization or allocating their contents.
    #[test]
    fn nested_payloads_and_truncation_are_checked() {
        let mut bytes = vec![0x82, 0xa7];
        bytes.extend_from_slice(b"payload");
        bytes.extend(std::iter::repeat_n(0x91, 2048));
        bytes.push(0xc0);
        bytes.extend_from_slice(&[0xa6, b'r', b'e', b's', b'u', b'l', b't', 0xc3]);
        assert_eq!(linkMapField(&bytes, "result").unwrap(), Some(&[0xc3][..]));
        for end in 0..bytes.len() {
            assert!(
                linkMapField(&bytes[..end], "result").is_err(),
                "accepted prefix {end}"
            );
        }
        assert!(linkMapField(&[0xdf, 0xff, 0xff, 0xff, 0xff], "result").is_err());
        assert!(linkMapField(&[0x81, 0xa1, b'x', 0xc1], "result").is_err());
    }
}
