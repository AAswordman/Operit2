use serde::de::DeserializeOwned;
use serde_json::{Error, Value};

#[derive(Debug, Clone, Default)]
pub struct SerializationSetup;

/// Decode an owned JSON tree. Compact products reuse their existing slice
/// decoder instead of linking a second typed decoder for Value.
#[inline(never)]
pub fn fromJsonValue<T: DeserializeOwned>(value: Value) -> Result<T, Error> {
    #[cfg(feature = "compact-json")]
    { fromJsonValueRef(&value) }
    #[cfg(not(feature = "compact-json"))]
    { serde_json::from_value(value) }
}

/// Borrow the selected record; never serialize its containing preference table.
#[inline(never)]
pub fn fromJsonValueRef<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    #[cfg(feature = "compact-json")]
    { serde_json::from_slice(&serde_json::to_vec(value)?) }
    #[cfg(not(feature = "compact-json"))]
    { T::deserialize(value) }
}
