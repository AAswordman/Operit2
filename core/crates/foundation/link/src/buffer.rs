//! Immutable binary ownership preserved through structured proxy conversion.
use crate::{CoreBytes, CoreValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreBuffer(pub CoreBytes);

impl Deref for CoreBuffer {
    type Target = [u8];
    /// Borrows the immutable backing range without copying it.
    fn deref(&self) -> &[u8] { &self.0 }
}

impl From<CoreBytes> for CoreBuffer {
    /// Adopts an already owned immutable byte range.
    fn from(bytes: CoreBytes) -> Self { Self(bytes) }
}

impl Serialize for CoreBuffer {
    /// Carries a shared reference when the serializer owns a buffer capture scope.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::codec::serialize_shared_bytes(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for CoreBuffer {
    /// Requires binary data when decoding a typed envelope.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match CoreValue::deserialize(deserializer)? {
            CoreValue::Bytes(bytes) => Ok(Self(bytes)),
            _ => Err(serde::de::Error::custom("Expected a shared binary buffer")),
        }
    }
}
