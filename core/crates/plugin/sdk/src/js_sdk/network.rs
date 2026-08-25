//! HTTP, browser automation, userscript, and cookie APIs exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts either a textual payload or a JSON value for a simple HTTP POST request.
pub enum NetHostHttpPostBody {
    Variant1(String),
    Variant2(serde_json::Value),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures readable webpage extraction from a URL or a prior visit result.
pub struct NetHostVisitUrlOrParamsVariant2 {
    /// Provides the page URL to visit directly.
    pub url: Option<String>,
    /// References the stored context produced by an earlier visit.
    pub visit_key: Option<String>,
    /// Selects a numbered link from the referenced visit context.
    pub link_number: Option<f64>,
    /// Includes discovered image URLs in the extracted page result.
    pub include_image_links: Option<bool>,
    /// Adds request headers used while loading the page.
    pub headers: Option<BTreeMap<String, String>>,
    /// Selects a predefined browser user-agent profile.
    pub user_agent_preset: Option<String>,
    /// Overrides the user-agent header with an explicit value.
    pub user_agent: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a direct URL or structured webpage-visit options.
pub enum NetHostVisitUrlOrParams {
    Variant1(String),
    Variant2(NetHostVisitUrlOrParamsVariant2),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts browser startup headers as a name-value map or serialized header text.
pub enum NetHostStartBrowserOptionsHeaders {
    Variant1(BTreeMap<String, String>),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures the initial page and identity of a persistent browser session.
pub struct NetHostStartBrowserOptions {
    /// Sets the page opened when the session starts.
    pub url: Option<String>,
    /// Adds headers to the initial browser navigation.
    pub headers: Option<NetHostStartBrowserOptionsHeaders>,
    /// Overrides the browser session's user-agent string.
    pub user_agent: Option<String>,
    /// Assigns a stable human-readable name to the session.
    pub session_name: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects one browser session to close or requests closing every session.
pub struct NetHostStopBrowserSessionIdOrOptionsVariant2 {
    /// Identifies the browser session to stop.
    pub session_id: Option<String>,
    /// Requests termination of all active browser sessions.
    pub close_all: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a session identifier or structured browser shutdown options.
pub enum NetHostStopBrowserSessionIdOrOptions {
    Variant1(String),
    Variant2(NetHostStopBrowserSessionIdOrOptionsVariant2),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts navigation headers as a name-value map or serialized header text.
pub enum NetHostBrowserNavigateUrlOrOptionsVariant2Headers {
    Variant1(BTreeMap<String, String>),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a browser navigation target and its request headers.
pub struct NetHostBrowserNavigateUrlOrOptionsVariant2 {
    /// Contains the destination URL.
    pub url: String,
    /// Adds headers to the navigation request.
    pub headers: Option<NetHostBrowserNavigateUrlOrOptionsVariant2Headers>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a direct destination URL or structured browser navigation options.
pub enum NetHostBrowserNavigateUrlOrOptions {
    Variant1(String),
    Variant2(NetHostBrowserNavigateUrlOrOptionsVariant2),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the mouse button used for a browser element click.
pub enum NetHostBrowserClickOptionsButton {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "middle")]
    Middle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a keyboard modifier held during a browser click.
pub enum NetHostBrowserClickOptionsModifiersItem {
    #[serde(rename = "Alt")]
    Alt,
    #[serde(rename = "Control")]
    Control,
    #[serde(rename = "ControlOrMeta")]
    ControlOrMeta,
    #[serde(rename = "Meta")]
    Meta,
    #[serde(rename = "Shift")]
    Shift,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a browser element and configures the pointer action performed on it.
pub struct NetHostBrowserClickOptions {
    /// Targets a specific persistent browser session.
    pub session_id: Option<String>,
    /// Selects an element by the stable reference emitted in a page snapshot.
    pub r#ref: Option<String>,
    /// Selects an element using a browser selector expression.
    pub selector: Option<String>,
    /// Provides a human-readable element description for diagnostics.
    pub element: Option<String>,
    /// Selects the mouse button used for the click.
    pub button: Option<NetHostBrowserClickOptionsButton>,
    /// Holds keyboard modifiers while dispatching the click.
    pub modifiers: Option<Vec<NetHostBrowserClickOptionsModifiersItem>>,
    /// Dispatches a double-click instead of a single click.
    #[serde(rename = "doubleClick")]
    pub double_click: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Filters browser console messages by severity and optional output file.
pub struct NetHostBrowserConsoleMessagesOptions {
    /// Restricts messages to the requested console severity.
    pub level: Option<String>,
    /// Writes the collected messages to this file when provided.
    pub filename: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies the source and destination elements of a browser drag operation.
pub struct NetHostBrowserDragOptions {
    /// Describes the element where the drag begins.
    #[serde(rename = "startElement")]
    pub start_element: String,
    /// Selects the source element by its page-snapshot reference.
    #[serde(rename = "startRef")]
    pub start_ref: String,
    /// Describes the element where the drag ends.
    #[serde(rename = "endElement")]
    pub end_element: String,
    /// Selects the destination element by its page-snapshot reference.
    #[serde(rename = "endRef")]
    pub end_ref: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures JavaScript evaluation against the page or a selected element.
pub struct NetHostBrowserEvaluateOptions {
    /// Contains the JavaScript function or expression to execute.
    pub function: String,
    /// Selects the evaluation target by page-snapshot reference.
    pub r#ref: Option<String>,
    /// Describes the selected element for diagnostics.
    pub element: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies local files to an active browser file chooser.
pub struct NetHostBrowserFileUploadOptions {
    /// Lists the local file paths selected for upload; omission cancels the chooser.
    pub paths: Option<Vec<String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Carries a textual, numeric, boolean, or structured value assigned to a form field.
pub enum NetHostBrowserFillFormOptionsFieldsItemValue {
    Variant1(String),
    Variant2(f64),
    Variant3(bool),
    Variant4(serde_json::Value),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes one browser form control and the value assigned to it.
pub struct NetHostBrowserFillFormOptionsFieldsItem {
    /// Names the form field for diagnostics and result reporting.
    pub name: String,
    /// Identifies the form control kind and determines how the value is applied.
    pub r#type: String,
    /// Contains the value written to the control.
    pub value: NetHostBrowserFillFormOptionsFieldsItemValue,
    /// Selects the control by page-snapshot reference.
    pub r#ref: Option<String>,
    /// Selects the control using a browser selector expression.
    pub selector: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Groups the browser form controls populated in one operation.
pub struct NetHostBrowserFillFormOptions {
    /// Lists each control and value to apply.
    pub fields: Vec<NetHostBrowserFillFormOptionsFieldsItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Chooses how an active browser alert, confirmation, or prompt is resolved.
pub struct NetHostBrowserHandleDialogOptions {
    /// Accepts the dialog when true and dismisses it when false.
    pub accept: bool,
    /// Supplies text entered into a prompt dialog before it is accepted.
    #[serde(rename = "promptText")]
    pub prompt_text: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies the browser element that receives a hover action.
pub struct NetHostBrowserHoverOptions {
    /// Selects the element by page-snapshot reference.
    pub r#ref: String,
    /// Describes the selected element for diagnostics.
    pub element: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures collection of network requests observed by the browser session.
pub struct NetHostBrowserNetworkRequestsOptions {
    /// Includes requests for static assets such as images, scripts, and stylesheets.
    #[serde(rename = "includeStatic")]
    pub include_static: Option<bool>,
    /// Writes the collected request log to this file when provided.
    pub filename: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Wraps the keyboard key dispatched to the active browser page.
pub struct NetHostBrowserPressKeyKeyOrOptionsVariant2 {
    /// Contains a Playwright-compatible key name or shortcut chord.
    pub key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a direct key name or object-style keyboard options.
pub enum NetHostBrowserPressKeyKeyOrOptions {
    Variant1(String),
    Variant2(NetHostBrowserPressKeyKeyOrOptionsVariant2),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Sets the dimensions of the browser viewport in CSS pixels.
pub struct NetHostBrowserResizeOptions {
    /// Sets the viewport width in CSS pixels.
    pub width: f64,
    /// Sets the viewport height in CSS pixels.
    pub height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies Playwright-style automation code for execution in the browser session.
pub struct NetHostBrowserRunCodeOptions {
    /// Contains the automation program executed against the current page.
    pub code: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a select control and the option values chosen within it.
pub struct NetHostBrowserSelectOptionOptions {
    /// Selects the control by page-snapshot reference.
    pub r#ref: String,
    /// Lists the option values to select.
    pub values: Vec<String>,
    /// Describes the selected control for diagnostics.
    pub element: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures the scope and optional file output of a textual page snapshot.
pub struct NetHostBrowserSnapshotOptions {
    /// Writes the snapshot to this file when provided.
    pub filename: Option<String>,
    /// Restricts the snapshot to the subtree matching this selector.
    pub selector: Option<String>,
    /// Limits how deeply descendant elements are included.
    pub depth: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the image encoding used for a browser screenshot.
pub enum NetHostBrowserTakeScreenshotOptionsType {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpeg")]
    Jpeg,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures screenshot encoding, target, page coverage, and file output.
pub struct NetHostBrowserTakeScreenshotOptions {
    /// Selects PNG or JPEG image encoding.
    pub r#type: Option<NetHostBrowserTakeScreenshotOptionsType>,
    /// Writes the screenshot to this file when provided.
    pub filename: Option<String>,
    /// Describes the target element for diagnostics.
    pub element: Option<String>,
    /// Restricts the screenshot to an element selected by snapshot reference.
    pub r#ref: Option<String>,
    /// Captures the complete scrollable page instead of the visible viewport.
    #[serde(rename = "fullPage")]
    pub full_page: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects a browser tab-management action and optional tab index.
pub struct NetHostBrowserTabsOptions {
    /// Names the tab operation, such as listing, opening, closing, or selecting a tab.
    pub action: String,
    /// Identifies the tab affected by an indexed action.
    pub index: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures text entry into a browser element selected from a page snapshot.
pub struct NetHostBrowserTypeOptions {
    /// Selects the input element by page-snapshot reference.
    pub r#ref: String,
    /// Contains the text entered into the element.
    pub text: String,
    /// Describes the selected element for diagnostics.
    pub element: Option<String>,
    /// Submits the containing form after text entry.
    pub submit: Option<bool>,
    /// Types character by character to trigger keyboard-driven page behavior.
    pub slowly: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects a duration or text condition to await in the browser session.
pub struct NetHostBrowserWaitForOptions {
    /// Waits for this duration in seconds.
    pub time: Option<f64>,
    /// Waits until this text appears on the page.
    pub text: Option<String>,
    /// Waits until this text is no longer present on the page.
    #[serde(rename = "textGone")]
    pub text_gone: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures which installed browser userscripts are listed.
pub struct NetHostBrowserUserscriptListOptions {
    /// Includes installed scripts that are currently disabled.
    pub include_disabled: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies a remote, local, or inline source for browser userscript installation.
pub struct NetHostBrowserUserscriptInstallOptions {
    /// Downloads the userscript from this remote URL.
    pub url: Option<String>,
    /// Reads the userscript from this local file path.
    pub path: Option<String>,
    /// Contains inline userscript source code.
    pub source: Option<String>,
    /// Records the canonical source URL associated with inline or local code.
    pub source_url: Option<String>,
    /// Provides a human-readable source label shown in management UI.
    pub source_display: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts the string or numeric identifier assigned to an installed userscript.
pub enum NetHostBrowserUserscriptStartOptionsScriptId {
    Variant1(String),
    Variant2(f64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies an installed browser userscript to enable.
pub struct NetHostBrowserUserscriptStartOptions {
    /// Selects the script by its installation identifier.
    pub script_id: Option<NetHostBrowserUserscriptStartOptionsScriptId>,
    /// Selects the script by metadata name.
    pub name: Option<String>,
    /// Disambiguates scripts with the same metadata name.
    pub namespace: Option<String>,
    /// Selects the script by its canonical source URL.
    pub source_url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts the string or numeric identifier assigned to an installed userscript.
pub enum NetHostBrowserUserscriptStopOptionsScriptId {
    Variant1(String),
    Variant2(f64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies an installed browser userscript to disable.
pub struct NetHostBrowserUserscriptStopOptions {
    /// Selects the script by its installation identifier.
    pub script_id: Option<NetHostBrowserUserscriptStopOptionsScriptId>,
    /// Selects the script by metadata name.
    pub name: Option<String>,
    /// Disambiguates scripts with the same metadata name.
    pub namespace: Option<String>,
    /// Selects the script by its canonical source URL.
    pub source_url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts the string or numeric identifier assigned to an installed userscript.
pub enum NetHostBrowserUserscriptUninstallOptionsScriptId {
    Variant1(String),
    Variant2(f64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies an installed browser userscript to remove.
pub struct NetHostBrowserUserscriptUninstallOptions {
    /// Selects the script by its installation identifier.
    pub script_id: Option<NetHostBrowserUserscriptUninstallOptionsScriptId>,
    /// Selects the script by metadata name.
    pub name: Option<String>,
    /// Disambiguates scripts with the same metadata name.
    pub namespace: Option<String>,
    /// Selects the script by its canonical source URL.
    pub source_url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the HTTP request method used by the configurable network API.
pub enum NetHostHttpOptionsMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a textual or JSON-compatible HTTP request body.
pub enum NetHostHttpOptionsBody {
    Variant1(String),
    Variant2(serde_json::Value),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects how the HTTP response body is decoded for the plugin.
pub enum NetHostHttpOptionsResponseType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "arraybuffer")]
    Arraybuffer,
    #[serde(rename = "blob")]
    Blob,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures an HTTP request, transport policy, and response decoding.
pub struct NetHostHttpOptions {
    /// Contains the absolute request URL.
    pub url: String,
    /// Selects the HTTP method sent to the server.
    pub method: Option<NetHostHttpOptionsMethod>,
    /// Adds request header names and values.
    pub headers: Option<BTreeMap<String, String>>,
    /// Contains the optional request body.
    pub body: Option<NetHostHttpOptionsBody>,
    /// Sets the maximum time allowed to establish a connection.
    pub connect_timeout: Option<f64>,
    /// Sets the maximum wait for response data after connection.
    pub read_timeout: Option<f64>,
    /// Controls whether HTTP redirects are followed automatically.
    pub follow_redirects: Option<bool>,
    /// Disables TLS certificate validation for this request.
    pub ignore_ssl: Option<bool>,
    /// Selects how the response body is represented in the result.
    #[serde(rename = "responseType")]
    pub response_type: Option<NetHostHttpOptionsResponseType>,
    /// Requests status-code validation as part of request completion.
    #[serde(rename = "validateStatus")]
    pub validate_status: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the HTTP method used for a multipart file upload.
pub enum NetHostUploadFileOptionsMethod {
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PUT")]
    PUT,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes one local file part included in a multipart request.
pub struct NetHostUploadFileOptionsFilesItem {
    /// Sets the multipart form field name for the file part.
    pub field_name: String,
    /// Identifies the local file uploaded in this part.
    pub file_path: String,
    /// Overrides the MIME type reported for the file part.
    pub content_type: Option<String>,
    /// Overrides the file name reported in multipart metadata.
    pub file_name: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a multipart request containing files and textual form fields.
pub struct NetHostUploadFileOptions {
    /// Contains the upload endpoint URL.
    pub url: String,
    /// Selects POST or PUT for the upload request.
    pub method: Option<NetHostUploadFileOptionsMethod>,
    /// Adds request headers to the multipart upload.
    pub headers: Option<BTreeMap<String, String>>,
    /// Adds textual fields alongside the uploaded files.
    pub form_data: Option<BTreeMap<String, String>>,
    /// Disables TLS certificate validation for this upload.
    pub ignore_ssl: Option<bool>,
    /// Lists the local file parts included in the request.
    pub files: Vec<NetHostUploadFileOptionsFilesItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a serialized cookie header or a name-value map when storing cookies.
pub enum NetCookieManagerSetCookies {
    Variant1(String),
    Variant2(BTreeMap<String, String>),
}
/// Performs HTTP requests and controls the browser automation tools available at runtime.
pub trait NetHost: Send + Sync {
    ///
    ///Perform HTTP GET request
    ///@param url - URL to request
    ///
    fn httpGet(&self, url: String, ignore_ssl: Option<bool>) -> JsFuture<HttpResponseData>;
    ///
    ///Perform HTTP POST request
    ///@param url - URL to request
    ///@param data - Data to post
    ///
    fn httpPost(
        &self,
        url: String,
        body: NetHostHttpPostBody,
        ignore_ssl: Option<bool>,
    ) -> JsFuture<HttpResponseData>;
    ///
    ///Visit a webpage and extract readable webpage content.
    ///Not a replacement for raw HTTP GET/POST: when you actually need API
    ///responses or precise response bodies, use httpGet/httpPost/http instead,
    ///otherwise this may return empty or incomplete content.
    ///@param urlOrParams - URL to visit, or an object with visit parameters.
    ///
    fn visit(&self, urlOrParams: NetHostVisitUrlOrParams) -> JsFuture<VisitWebResultData>;
    ///
    ///Navigate a browser session to a target URL.
    ///
    fn browserNavigate(&self, urlOrOptions: NetHostBrowserNavigateUrlOrOptions)
        -> JsFuture<String>;
    ///
    ///Go back in browser history.
    ///
    fn browserNavigateBack(
        &self,
        options: Option<BTreeMap<String, super::JsNever>>,
    ) -> JsFuture<String>;
    ///
    ///Click an element by snapshot ref or selector.
    ///Only accepts one options object.
    ///
    fn browserClick(&self, options: NetHostBrowserClickOptions) -> JsFuture<String>;
    ///
    ///Close the current browser tab.
    ///
    fn browserClose(&self, options: Option<BTreeMap<String, super::JsNever>>) -> JsFuture<String>;
    ///
    ///Close all browser tabs.
    ///
    fn browserCloseAll(
        &self,
        options: Option<BTreeMap<String, super::JsNever>>,
    ) -> JsFuture<String>;
    ///
    ///Read console messages from the browser session.
    ///
    fn browserConsoleMessages(
        &self,
        options: Option<NetHostBrowserConsoleMessagesOptions>,
    ) -> JsFuture<String>;
    ///
    ///Drag between two elements by snapshot refs.
    ///
    fn browserDrag(&self, options: NetHostBrowserDragOptions) -> JsFuture<String>;
    ///
    ///Evaluate JavaScript in the browser session.
    ///
    fn browserEvaluate(&self, options: NetHostBrowserEvaluateOptions) -> JsFuture<String>;
    ///
    ///Resolve an active file chooser in the browser session.
    ///If `paths` is omitted, the file chooser is cancelled.
    ///
    fn browserFileUpload(
        &self,
        options: Option<NetHostBrowserFileUploadOptions>,
    ) -> JsFuture<String>;
    ///
    ///Fill multiple form fields in the browser session.
    ///
    fn browserFillForm(&self, options: NetHostBrowserFillFormOptions) -> JsFuture<String>;
    ///
    ///Handle an active dialog.
    ///
    fn browserHandleDialog(&self, options: NetHostBrowserHandleDialogOptions) -> JsFuture<String>;
    ///
    ///Hover over an element by snapshot ref.
    ///
    fn browserHover(&self, options: NetHostBrowserHoverOptions) -> JsFuture<String>;
    ///
    ///Read network requests from the browser session.
    ///
    fn browserNetworkRequests(
        &self,
        options: Option<NetHostBrowserNetworkRequestsOptions>,
    ) -> JsFuture<String>;
    ///
    ///Press a keyboard key in the browser session.
    ///
    fn browserPressKey(&self, keyOrOptions: NetHostBrowserPressKeyKeyOrOptions)
        -> JsFuture<String>;
    ///
    ///Resize the browser viewport.
    ///
    fn browserResize(&self, options: NetHostBrowserResizeOptions) -> JsFuture<String>;
    ///
    ///Run Playwright-style code in the browser session.
    ///
    fn browserRunCode(&self, options: NetHostBrowserRunCodeOptions) -> JsFuture<String>;
    ///
    ///Select options in a dropdown by snapshot ref.
    ///
    fn browserSelectOption(&self, options: NetHostBrowserSelectOptionOptions) -> JsFuture<String>;
    ///
    ///Capture a text snapshot of current page.
    ///
    fn browserSnapshot(&self, options: Option<NetHostBrowserSnapshotOptions>) -> JsFuture<String>;
    ///
    ///Take a screenshot of the current page or a target element.
    ///
    fn browserTakeScreenshot(
        &self,
        options: NetHostBrowserTakeScreenshotOptions,
    ) -> JsFuture<String>;
    ///
    ///Manage browser tabs.
    ///
    fn browserTabs(&self, options: NetHostBrowserTabsOptions) -> JsFuture<String>;
    ///
    ///Type text into an element by snapshot ref.
    ///
    fn browserType(&self, options: NetHostBrowserTypeOptions) -> JsFuture<String>;
    ///
    ///Wait for text or time in the browser session.
    ///
    fn browserWaitFor(&self, options: NetHostBrowserWaitForOptions) -> JsFuture<String>;
    ///
    ///Enhanced HTTP request with flexible options
    ///@param options - HTTP request options
    ///
    fn http(&self, options: NetHostHttpOptions) -> JsFuture<HttpResponseData>;
    ///
    ///Upload file using multipart request
    ///@param options - Upload options
    ///
    fn uploadFile(&self, options: NetHostUploadFileOptions) -> JsFuture<HttpResponseData>;
}
/// Declares browser session and userscript APIs reserved for a future runtime capability.
pub trait NetFutureHost: Send + Sync {
    /// Starts a persistent browser session hosted in a floating WebView.
    fn startBrowser(&self, options: Option<NetHostStartBrowserOptions>) -> JsFuture<String>;
    /// Stops one persistent browser session or every active session.
    fn stopBrowser(
        &self,
        sessionIdOrOptions: Option<NetHostStopBrowserSessionIdOrOptions>,
    ) -> JsFuture<String>;
    /// Lists installed browser session userscripts.
    fn browserUserscriptList(
        &self,
        options: Option<NetHostBrowserUserscriptListOptions>,
    ) -> JsFuture<String>;
    /// Installs a browser userscript from exactly one supported source.
    fn browserUserscriptInstall(
        &self,
        options: NetHostBrowserUserscriptInstallOptions,
    ) -> JsFuture<String>;
    /// Enables one installed browser session userscript.
    fn browserUserscriptStart(
        &self,
        options: NetHostBrowserUserscriptStartOptions,
    ) -> JsFuture<String>;
    /// Disables one installed browser session userscript.
    fn browserUserscriptStop(
        &self,
        options: NetHostBrowserUserscriptStopOptions,
    ) -> JsFuture<String>;
    /// Uninstalls one browser session userscript.
    fn browserUserscriptUninstall(
        &self,
        options: NetHostBrowserUserscriptUninstallOptions,
    ) -> JsFuture<String>;
}
/// Reads, writes, and clears cookies associated with network domains.
pub trait NetCookieManager: Send + Sync {
    ///
    ///Get cookies for a domain
    ///@param domain - Domain to get cookies for
    ///
    fn get(&self, domain: String) -> JsFuture<HttpResponseData>;
    ///
    ///Set cookies for a domain
    ///@param domain - Domain to set cookies for
    ///@param cookies - Cookies to set (can be string or object)
    ///
    fn set(
        &self,
        domain: String,
        cookies: NetCookieManagerSetCookies,
    ) -> JsFuture<HttpResponseData>;
    ///
    ///Clear cookies for a domain
    ///@param domain - Domain to clear cookies for
    ///
    fn clear(&self, domain: Option<String>) -> JsFuture<HttpResponseData>;
}
/// Exposes cookie operations through the network service.
pub struct NetCookiesBinding(pub Arc<dyn NetCookieManager>);
