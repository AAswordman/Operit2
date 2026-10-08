//! Core tool invocation, runtime utilities, and native bridge contracts for plugins.
use super::{JsAny, JsDate, JsFuture, JsObject, JsUndefined};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

/// Contains a JSON object with complete values preserved by the native JSON parser.
pub type JsonObject = BTreeMap<String, serde_json::Value>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Stores a scalar or JSON value assigned to an arbitrary tool parameter.
pub enum ToolParamsAdditionalValue {
    Variant1(String),
    Variant2(f64),
    Variant3(bool),
    Variant4(serde_json::Value),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Stores the named arguments passed to a tool invocation.
pub struct ToolParams {
    /// Stores arbitrary named arguments serialized into the tool request object.
    #[serde(flatten)]
    pub additional_properties: BTreeMap<String, ToolParamsAdditionalValue>,
}
/// Configures an object-style tool invocation and its streaming callback.
pub struct ToolConfig {
    /// Selects the tool category when the runtime requires one.
    pub r#type: Option<String>,
    /// Identifies the tool to invoke.
    pub name: String,
    /// Contains the tool arguments.
    pub params: Option<ToolParams>,
    /// Receives intermediate values produced by a streaming tool.
    pub onIntermediateResult: Option<Arc<dyn Fn(JsAny) + Send + Sync>>,
}
/// Configures callbacks for a global tool call.
pub struct ToolCallOptions<TIntermediate = JsAny> {
    /// Receives an intermediate tool result.
    pub onIntermediateResult: Option<Arc<dyn Fn(TIntermediate) + Send + Sync>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Reports whether an operation succeeded and carries its error when present.
pub struct BaseResult {
    /// Reports whether the tool operation succeeded.
    #[serde(rename = "success")]
    pub success: bool,
    /// Contains the operation error message.
    #[serde(rename = "error")]
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Returns a string together with the operation status.
pub struct StringResult {
    /// Carries the common success and error metadata for the string result.
    #[serde(flatten)]
    pub base_result: BaseResult,
    /// Contains the returned string.
    #[serde(rename = "data")]
    pub data: String,
}
impl StringResult {
    /// Returns the string stored in this result.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.data.clone()
    }
}
/// Contains a boolean tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BooleanResult {
    /// Carries the common success and error metadata for the boolean result.
    #[serde(flatten)]
    pub base_result: BaseResult,
    /// Contains the returned boolean.
    #[serde(rename = "data")]
    pub data: bool,
}
impl BooleanResult {
    /// Formats the boolean stored in this result.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.data.to_string()
    }
}
/// Contains a numeric tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumberResult {
    /// Carries the common success and error metadata for the numeric result.
    #[serde(flatten)]
    pub base_result: BaseResult,
    /// Contains the returned number.
    #[serde(rename = "data")]
    pub data: f64,
}
impl NumberResult {
    /// Formats the number stored in this result.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.data.to_string()
    }
}
/// Contains a dynamically typed structured tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicToolResult {
    /// Carries the common success, message, and error metadata for the dynamic result.
    #[serde(flatten)]
    pub base_result: BaseResult,
    /// Contains the tool-specific result value.
    pub data: JsAny,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Holds any scalar or JSON-compatible result returned by a tool invocation.
pub enum ToolResult {
    /// Contains a string result.
    String(StringResult),
    /// Contains a boolean result.
    Boolean(BooleanResult),
    /// Contains a numeric result.
    Number(NumberResult),
    /// Contains another JSON-compatible result.
    Dynamic(DynamicToolResult),
}
/// Describes one real executable tool parameter returned by the global catalog.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCatalogParameter {
    pub name: String,
    pub r#type: String,
    pub description: String,
    pub required: bool,
    pub default: JsAny,
}
/// Describes one executable tool in the existing global catalog, not a source-directory entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCatalogEntry {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolCatalogParameter>,
    pub category: String,
    pub source: String,
}
/// Describes the real global executable catalog envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCatalog {
    pub tools: Vec<ToolCatalogEntry>,
}
/// Resolves a statically known tool name to its declared result type.
pub struct ToolReturnType<T>(PhantomData<T>);
/// Configures an object-style call whose tool name remains statically typed.
pub struct NamedToolConfig<T: AsRef<str>> {
    /// Selects the tool category when the runtime requires one.
    pub r#type: Option<String>,
    /// Contains the statically known tool name.
    pub name: T,
    /// Contains the tool arguments.
    pub params: Option<ToolParams>,
    /// Receives intermediate values produced by a streaming tool.
    pub onIntermediateResult: Option<Arc<dyn Fn(JsAny) + Send + Sync>>,
}
/// Invokes registered tools and completes the current plugin execution.
pub trait CoreHost: Send + Sync {
    ///
    ///Global function to call a tool and get a result
    ///Note: Promise-based waiting does not guarantee the underlying tool work is truly parallel.
    ///@returns A Promise with the tool result data of the appropriate type
    ///
    fn toolCall_overload_1<T: AsRef<str>>(
        &self,
        toolType: String,
        toolName: T,
        toolParams: Option<ToolParams>,
    ) -> JsFuture<ToolReturnType<T>>;
    /// Calls a tool by its globally registered name.
    fn toolCall_overload_2<T: AsRef<str>>(
        &self,
        toolName: T,
        toolParams: Option<ToolParams>,
    ) -> JsFuture<ToolReturnType<T>>;
    /// Calls a tool with object-style configuration.
    fn toolCall_overload_3<T: AsRef<str>>(
        &self,
        config: NamedToolConfig<T>,
    ) -> JsFuture<ToolReturnType<T>>;
    /// Calls a categorized tool and receives intermediate results.
    fn toolCall_overload_4<T: AsRef<str>, TIntermediate>(
        &self,
        toolType: String,
        toolName: T,
        toolParams: JsUndefined<ToolParams>,
        options: ToolCallOptions<TIntermediate>,
    ) -> JsFuture<ToolReturnType<T>>;
    /// Calls a globally named tool and receives intermediate results.
    fn toolCall_overload_5<T: AsRef<str>, TIntermediate>(
        &self,
        toolName: T,
        toolParams: JsUndefined<ToolParams>,
        options: ToolCallOptions<TIntermediate>,
    ) -> JsFuture<ToolReturnType<T>>;
    /// Calls a dynamically named tool.
    fn toolCall_overload_6(&self, toolName: String) -> JsFuture<JsAny>;
    ///
    ///Global function to complete tool execution with a result
    ///Result values must be JSON-serializable.
    ///@param result - The result to return
    ///
    fn complete<T>(&self, result: T);
}
/// Supplies either an array or an object to a collection utility.
pub enum LodashCollection<T> {
    /// Contains an array collection.
    Array(Vec<T>),
    /// Contains a non-array object collection.
    Object(JsObject),
}
/// Provides collection iteration and dynamic value predicates to plugin scripts.
pub trait LodashApi: Send + Sync {
    /// Reports whether a dynamic value is empty.
    fn isEmpty(&self, value: JsAny) -> bool;
    /// Reports whether a dynamic value is a string.
    fn isString(&self, value: JsAny) -> bool;
    /// Reports whether a dynamic value is a number.
    fn isNumber(&self, value: JsAny) -> bool;
    /// Reports whether a dynamic value is a boolean.
    fn isBoolean(&self, value: JsAny) -> bool;
    /// Reports whether a dynamic value is an object.
    fn isObject(&self, value: JsAny) -> bool;
    /// Reports whether a dynamic value is an array.
    fn isArray(&self, value: JsAny) -> bool;
    /// Invokes a callback for every collection entry.
    fn forEach<T>(
        &self,
        collection: LodashCollection<T>,
        iteratee: Arc<dyn Fn(JsAny, JsAny, JsAny) + Send + Sync>,
    ) -> JsAny;
    /// Maps every collection entry to a new result value.
    fn map<T, R>(
        &self,
        collection: LodashCollection<T>,
        iteratee: Arc<dyn Fn(JsAny, JsAny, JsAny) -> R + Send + Sync>,
    ) -> Vec<R>;
}
/// Exposes the lodash-like utility service as a plugin global.
pub struct LodashGlobal<TApi: LodashApi>(pub TApi);
/// Accepts a date value or date string for formatting.
pub struct DataUtilsDateInput(pub String);
/// Parses, serializes, and formats values used by plugin scripts.
pub trait DataUtilsApi: Send + Sync {
    /// Parses a JSON string into a dynamic JavaScript value.
    fn parseJson(&self, jsonString: String) -> JsAny;
    /// Serializes a dynamic JavaScript value as JSON.
    fn stringifyJson(&self, obj: JsAny) -> String;
    /// Formats an optional date or string value.
    fn formatDate(&self, date: Option<DataUtilsDateInput>) -> String;
}
/// Exposes data conversion utilities as a plugin global.
pub struct DataUtilsGlobal(pub Arc<dyn DataUtilsApi>);
/// Stores the named values assigned to CommonJS module exports.
pub struct CommonJsExports(pub BTreeMap<String, JsAny>);
