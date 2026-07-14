//! Core tool invocation, runtime utilities, and native bridge contracts for plugins.
use super::{JsAny, JsDate, JsFuture, JsObject, JsUndefined};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;
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
/// Selects the native algorithm used to decompress plugin data.
pub enum NativeInterfaceHostDecompressAlgorithm {
    #[serde(rename = "deflate")]
    Deflate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the native cryptographic operation family requested by a plugin.
pub enum NativeInterfaceHostCryptoAlgorithm {
    #[serde(rename = "md5")]
    Md5,
    #[serde(rename = "aes")]
    Aes,
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
/// Provides synchronous Android runtime services used by plugin scripts.
pub trait NativeInterfaceHost: Send + Sync {
    ///
    ///Call a tool synchronously (legacy method)
    ///@param toolType - Tool type
    ///@param toolName - Tool name
    ///@param paramsJson - Parameters as JSON string
    ///@returns A JSON string representing a ToolResult object
    ///
    fn callTool(&self, toolType: String, toolName: String, paramsJson: String) -> String;
    ///
    ///Call a tool asynchronously
    ///@param callbackId - Unique callback ID
    ///@param toolType - Tool type
    ///@param toolName - Tool name
    ///@param paramsJson - Parameters as JSON string
    ///The callback will receive a ToolResult object
    ///
    fn callToolAsync(
        &self,
        callbackId: String,
        toolType: String,
        toolName: String,
        paramsJson: String,
    ) -> ();
    /// Starts an asynchronous tool call and routes both intermediate and final results to callbacks.
    fn callToolAsyncStreaming(
        &self,
        callbackId: String,
        intermediateCallbackId: String,
        toolType: String,
        toolName: String,
        paramsJson: String,
    ) -> ();
    ///
    ///Set the result of script execution
    ///@param result - Result string
    ///
    fn setResult(&self, result: String) -> ();
    ///
    ///Set an error for script execution
    ///@param error - Error message
    ///
    fn setError(&self, error: String) -> ();
    ///
    ///Log informational message
    ///@param message - Message to log
    ///
    fn logInfo(&self, message: String) -> ();
    ///
    ///Log error message
    ///@param message - Error message to log
    ///
    fn logError(&self, message: String) -> ();
    ///
    ///Log debug message with data
    ///@param message - Debug message
    ///@param data - Debug data
    ///
    fn logDebug(&self, message: String, data: String) -> ();
    ///
    ///Register a toolbox UI module for current toolpkg main registration session.
    ///@param specJson - JSON object string describing a toolbox UI module
    ///
    fn registerToolPkgToolboxUiModule(&self, specJson: String) -> ();
    ///
    ///Register an app lifecycle hook for current toolpkg main registration session.
    ///@param specJson - JSON object string describing an app lifecycle hook
    ///
    fn registerToolPkgAppLifecycleHook(&self, specJson: String) -> ();
    ///
    ///Register a message processing plugin for current toolpkg main registration session.
    ///@param specJson - JSON object string describing a message processing plugin
    ///
    fn registerToolPkgMessageProcessingPlugin(&self, specJson: String) -> ();
    ///
    ///Register an XML render plugin for current toolpkg main registration session.
    ///@param specJson - JSON object string describing an XML render plugin
    ///
    fn registerToolPkgXmlRenderPlugin(&self, specJson: String) -> ();
    ///
    ///Resolve the persistent config directory for a package or toolpkg.
    ///Returns an absolute path under `/sdcard/Download/Operit/plugins/<id>`.
    ///
    fn getPluginConfigDir(&self, pluginId: String) -> String;
    ///
    ///Decompress native deflate data from a base64 string or binary handle.
    ///
    fn decompress(
        &self,
        data: String,
        algorithm: NativeInterfaceHostDecompressAlgorithm,
    ) -> String;
    ///
    ///Execute native crypto operations used by the CryptoJS bridge.
    ///
    fn crypto(
        &self,
        algorithm: NativeInterfaceHostCryptoAlgorithm,
        operation: String,
        argsJson: String,
    ) -> String;
    ///
    ///Execute native image operations used by the Jimp bridge.
    ///
    fn image_processing(
        &self,
        callbackId: String,
        operation: String,
        argsJson: String,
    ) -> ();
    ///
    ///Register an input menu toggle plugin for current toolpkg main registration session.
    ///@param specJson - JSON object string describing an input menu toggle plugin
    ///
    fn registerToolPkgInputMenuTogglePlugin(&self, specJson: String) -> ();
    ///
    ///Register a chat input hook for current toolpkg main registration session.
    ///@param specJson - JSON object string describing a chat input hook
    ///
    fn registerToolPkgChatInputHook(&self, specJson: String) -> ();
    ///
    ///Register an image from base64-encoded data into the global image pool
    ///and return a `<link type="image" id="...">` tag string that can be
    ///embedded into tool results or messages.
    ///
    fn registerImageFromBase64(&self, base64: String, mimeType: String) -> String;
    ///
    ///Register an image from a file path on the device into the global image
    ///pool and return a `<link type="image" id="...">` tag string that can
    ///be embedded into tool results or messages.
    ///
    fn registerImageFromPath(&self, path: String) -> String;
    ///
    ///Report a script error with its source line and stack details.
    ///@param errorType - Error type
    ///@param errorMessage - Error message
    ///@param errorLine - Line number where error occurred
    ///@param errorStack - Error stack trace
    ///
    fn reportError(
        &self,
        errorType: String,
        errorMessage: String,
        errorLine: f64,
        errorStack: String,
    ) -> ();
}
