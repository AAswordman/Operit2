//! Canonical Rust contracts for plugin APIs and generated TypeScript declarations.

use std::future::Future;
use std::pin::Pin;

use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Represents a TypeScript `any` value at an explicitly dynamic SDK position.
pub type JsAny = serde_json::Value;

/// Represents a TypeScript bare `object` value at the SDK boundary.
pub type JsObject = serde_json::Value;

pub mod chat;
pub mod edge;
pub mod compose_dsl;
pub mod compose_dsl_material3_generated;
pub mod core;
pub mod files;
pub mod material_icons;
pub mod network;
pub mod results;
pub mod runtime_bindings;
pub mod software_settings;
pub mod storage;
pub mod system;
pub mod tool_types;
pub mod toolpkg;
pub mod ui;

/// Describes a rejected JavaScript host operation without erasing its message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsHostError {
    /// Contains the stable rejection message exposed to the JavaScript caller.
    pub message: String,
}

impl JsHostError {
    /// Creates a host rejection from its stable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for JsHostError {
    /// Formats the host rejection message.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for JsHostError {}

/// Represents the resolved or rejected output of one JavaScript host operation.
pub type JsHostResult<T> = Result<T, JsHostError>;

/// Represents a JavaScript promise returned by an SDK operation.
pub type JsFuture<T> = Pin<Box<dyn Future<Output = JsHostResult<T>> + Send>>;

/// Pulls one event at a time and separates observer disposal from business cancellation.
pub trait JsAsyncIterator<T>: Send + Sync {
    /// Waits for the next event or the authoritative end of observation.
    fn next(&self) -> JsFuture<Option<T>>;
    /// Detaches observation without cancelling the accepted operation.
    fn close(&self) -> JsFuture<()>;
}

/// Represents the single-consumer JavaScript async iterable returned by a streaming operation.
pub type JsAsyncIterable<T> = std::sync::Arc<dyn JsAsyncIterator<T>>;

/// Retains a rejected stream opening as the original iterator rejection, not a fabricated event.
struct RejectedJsAsyncIterator<T> {
    error: JsHostError,
    item: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> JsAsyncIterator<T> for RejectedJsAsyncIterator<T> {
    /// Rejects observation with the exact opening error.
    fn next(&self) -> JsFuture<Option<T>> {
        let error = self.error.clone();
        Box::pin(async move { Err(error) })
    }
    /// Releases a failed opening that never acquired a live observation.
    fn close(&self) -> JsFuture<()> { Box::pin(async { Ok(()) }) }
}

/// Creates an explicitly rejected iterator when a synchronous host opening fails.
pub fn rejected_js_async_iterable<T: Send + Sync + 'static>(error: JsHostError) -> JsAsyncIterable<T> {
    std::sync::Arc::new(RejectedJsAsyncIterator { error, item: std::marker::PhantomData })
}

/// Represents a JavaScript Date value on the SDK boundary.
pub type JsDate = String;

/// Selects a value type from a Rust-declared type map by its key type.
pub struct JsTypeIndex<TMap, TKey>(std::marker::PhantomData<(TMap, TKey)>);

/// Represents a TypeScript `never` position that cannot contain a value.
pub enum JsNever {}

/// Preserves a required TypeScript value whose value may be `undefined`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum JsUndefined<T> {
    /// Represents an explicit `undefined` value.
    #[default]
    Undefined,
    /// Contains a concrete value.
    Value(T),
}

impl<T> JsUndefined<T> {
    /// Converts an optional source into a required value-or-undefined boundary value.
    pub fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Value(value),
            None => Self::Undefined,
        }
    }

    /// Borrows the concrete value when this boundary value is defined.
    pub fn as_value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Undefined => None,
        }
    }
}

impl<T> Serialize for JsUndefined<T>
where
    T: Serialize,
{
    /// Serializes a concrete value and rejects non-JSON `undefined` values.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Undefined => Err(S::Error::custom(
                "TypeScript undefined cannot be represented as a JSON value",
            )),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for JsUndefined<T>
where
    T: Deserialize<'de>,
{
    /// Deserializes a concrete JSON value into the defined variant.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Value)
    }
}

/// Preserves the distinct TypeScript states `undefined`, `null`, and a concrete value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum JsOptional<T> {
    /// Represents an omitted property or an explicit `undefined` value.
    #[default]
    Undefined,
    /// Represents an explicit `null` value.
    Null,
    /// Contains a concrete value.
    Value(T),
}

impl<T> JsOptional<T> {
    /// Converts an optional nullable source where absence represents explicit null.
    pub fn from_nullable_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        }
    }

    /// Converts an optional source where absence represents undefined.
    pub fn from_optional_value(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Value(value),
            None => Self::Undefined,
        }
    }

    /// Reports whether this value represents TypeScript `undefined`.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Borrows the concrete value without conflating null with undefined.
    pub fn as_value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Null | Self::Undefined => None,
        }
    }
}

impl<T> Serialize for JsOptional<T>
where
    T: Serialize,
{
    /// Serializes null and concrete values while rejecting non-JSON `undefined` values.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Undefined => Err(S::Error::custom(
                "TypeScript undefined must be omitted by the containing field",
            )),
            Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for JsOptional<T>
where
    T: Deserialize<'de>,
{
    /// Deserializes JSON null and concrete values into their distinct TypeScript states.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

/// Preserves a required TypeScript value that may explicitly be `null`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsNullable<T> {
    /// Represents an explicit `null` value.
    Null,
    /// Contains a concrete value.
    Value(T),
}

impl<T> JsNullable<T> {
    /// Converts an optional source into a required nullable value.
    pub fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        }
    }

    /// Borrows the concrete value while preserving an explicit null state.
    pub fn as_value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Null => None,
        }
    }
}

impl<T> Serialize for JsNullable<T>
where
    T: Serialize,
{
    /// Serializes the required nullable value as JSON null or its concrete value.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for JsNullable<T>
where
    T: Deserialize<'de>,
{
    /// Deserializes JSON null and concrete values without introducing an undefined state.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

/// Requires an embedding host to implement every namespace exposed by the global Tools object.
pub trait JsToolsHost:
    files::FilesHost
    + network::NetHost
    + network::NetCookieManager
    + system::SystemHost
    + system::SystemBluetoothHost
    + system::SystemBluetoothBleHost
    + system::SystemTerminalHost
    + system::SystemMusicHost
    + software_settings::SoftwareSettingsHost
    + chat::ChatHost
    + edge::EdgeHost
    + edge::IoHost
    + storage::StorageHost
    + Send
    + Sync
{
}

impl<T> JsToolsHost for T where
    T: files::FilesHost
        + network::NetHost
        + network::NetCookieManager
        + system::SystemHost
        + system::SystemBluetoothHost
        + system::SystemBluetoothBleHost
        + system::SystemTerminalHost
        + system::SystemMusicHost
        + software_settings::SoftwareSettingsHost
        + chat::ChatHost
        + edge::EdgeHost
        + edge::IoHost
        + storage::StorageHost
        + Send
        + Sync
{
}

/// Collects declarations reserved for host capabilities that are not active runtime Tools yet.
pub trait JsFutureToolsHost: network::NetFutureHost + ui::UIHost + Send + Sync {}

impl<T> JsFutureToolsHost for T where T: network::NetFutureHost + ui::UIHost + Send + Sync {}

/// Requires an embedding host to implement the complete ToolPkg runtime API.
pub trait ToolPkgHost:
    toolpkg::ToolPkgRegistryMethods + toolpkg::ToolPkgIpcApiMethods + toolpkg::GlobalHost + Send + Sync
{
}

impl<T> ToolPkgHost for T where
    T: toolpkg::ToolPkgRegistryMethods
        + toolpkg::ToolPkgIpcApiMethods
        + toolpkg::GlobalHost
        + Send
        + Sync
{
}
