// Generated from operit-plugin-sdk Rust declarations.

import type { HttpResponseData, VisitWebResultData } from "./results";

/**
 * Performs HTTP requests and controls the browser automation tools available at runtime.
 */
export namespace Net {
  /**
   * Accepts either a textual payload or a JSON value for a simple HTTP POST request.
   */
  export type HostHttpPostBody = string | unknown;

  /**
   * Configures readable webpage extraction from a URL or a prior visit result.
   */
  export interface HostVisitUrlOrParamsVariant2 {
    /**
     * Provides the page URL to visit directly.
     */
    url?: string;
    /**
     * References the stored context produced by an earlier visit.
     */
    visit_key?: string;
    /**
     * Selects a numbered link from the referenced visit context.
     */
    link_number?: number;
    /**
     * Includes discovered image URLs in the extracted page result.
     */
    include_image_links?: boolean;
    /**
     * Adds request headers used while loading the page.
     */
    headers?: Record<string, string>;
    /**
     * Selects a predefined browser user-agent profile.
     */
    user_agent_preset?: string;
    /**
     * Overrides the user-agent header with an explicit value.
     */
    user_agent?: string;
  }

  /**
   * Accepts a direct URL or structured webpage-visit options.
   */
  export type HostVisitUrlOrParams = string | HostVisitUrlOrParamsVariant2;

  /**
   * Accepts browser startup headers as a name-value map or serialized header text.
   */
  export type HostStartBrowserOptionsHeaders = Record<string, string> | string;

  /**
   * Configures the initial page and identity of a persistent browser session.
   */
  export interface HostStartBrowserOptions {
    /**
     * Sets the page opened when the session starts.
     */
    url?: string;
    /**
     * Adds headers to the initial browser navigation.
     */
    headers?: HostStartBrowserOptionsHeaders;
    /**
     * Overrides the browser session's user-agent string.
     */
    user_agent?: string;
    /**
     * Assigns a stable human-readable name to the session.
     */
    session_name?: string;
  }

  /**
   * Selects one browser session to close or requests closing every session.
   */
  export interface HostStopBrowserSessionIdOrOptionsVariant2 {
    /**
     * Identifies the browser session to stop.
     */
    session_id?: string;
    /**
     * Requests termination of all active browser sessions.
     */
    close_all?: boolean;
  }

  /**
   * Accepts a session identifier or structured browser shutdown options.
   */
  export type HostStopBrowserSessionIdOrOptions = string | HostStopBrowserSessionIdOrOptionsVariant2;

  /**
   * Accepts navigation headers as a name-value map or serialized header text.
   */
  export type HostBrowserNavigateUrlOrOptionsVariant2Headers = Record<string, string> | string;

  /**
   * Configures a browser navigation target and its request headers.
   */
  export interface HostBrowserNavigateUrlOrOptionsVariant2 {
    /**
     * Contains the destination URL.
     */
    url: string;
    /**
     * Adds headers to the navigation request.
     */
    headers?: HostBrowserNavigateUrlOrOptionsVariant2Headers;
  }

  /**
   * Accepts a direct destination URL or structured browser navigation options.
   */
  export type HostBrowserNavigateUrlOrOptions = string | HostBrowserNavigateUrlOrOptionsVariant2;

  /**
   * Selects the mouse button used for a browser element click.
   */
  export type HostBrowserClickOptionsButton = "left" | "right" | "middle";

  /**
   * Identifies a keyboard modifier held during a browser click.
   */
  export type HostBrowserClickOptionsModifiersItem = "Alt" | "Control" | "ControlOrMeta" | "Meta" | "Shift";

  /**
   * Identifies a browser element and configures the pointer action performed on it.
   */
  export interface HostBrowserClickOptions {
    /**
     * Targets a specific persistent browser session.
     */
    session_id?: string;
    /**
     * Selects an element by the stable reference emitted in a page snapshot.
     */
    ref?: string;
    /**
     * Selects an element using a browser selector expression.
     */
    selector?: string;
    /**
     * Provides a human-readable element description for diagnostics.
     */
    element?: string;
    /**
     * Selects the mouse button used for the click.
     */
    button?: HostBrowserClickOptionsButton;
    /**
     * Holds keyboard modifiers while dispatching the click.
     */
    modifiers?: HostBrowserClickOptionsModifiersItem[];
    /**
     * Dispatches a double-click instead of a single click.
     */
    doubleClick?: boolean;
  }

  /**
   * Filters browser console messages by severity and optional output file.
   */
  export interface HostBrowserConsoleMessagesOptions {
    /**
     * Restricts messages to the requested console severity.
     */
    level?: string;
    /**
     * Writes the collected messages to this file when provided.
     */
    filename?: string;
  }

  /**
   * Identifies the source and destination elements of a browser drag operation.
   */
  export interface HostBrowserDragOptions {
    /**
     * Describes the element where the drag begins.
     */
    startElement: string;
    /**
     * Selects the source element by its page-snapshot reference.
     */
    startRef: string;
    /**
     * Describes the element where the drag ends.
     */
    endElement: string;
    /**
     * Selects the destination element by its page-snapshot reference.
     */
    endRef: string;
  }

  /**
   * Configures JavaScript evaluation against the page or a selected element.
   */
  export interface HostBrowserEvaluateOptions {
    /**
     * Contains the JavaScript function or expression to execute.
     */
    function: string;
    /**
     * Selects the evaluation target by page-snapshot reference.
     */
    ref?: string;
    /**
     * Describes the selected element for diagnostics.
     */
    element?: string;
  }

  /**
   * Supplies local files to an active browser file chooser.
   */
  export interface HostBrowserFileUploadOptions {
    /**
     * Lists the local file paths selected for upload; omission cancels the chooser.
     */
    paths?: string[];
  }

  /**
   * Carries a textual, numeric, boolean, or structured value assigned to a form field.
   */
  export type HostBrowserFillFormOptionsFieldsItemValue = string | number | boolean | unknown;

  /**
   * Describes one browser form control and the value assigned to it.
   */
  export interface HostBrowserFillFormOptionsFieldsItem {
    /**
     * Names the form field for diagnostics and result reporting.
     */
    name: string;
    /**
     * Identifies the form control kind and determines how the value is applied.
     */
    type: string;
    /**
     * Contains the value written to the control.
     */
    value: HostBrowserFillFormOptionsFieldsItemValue;
    /**
     * Selects the control by page-snapshot reference.
     */
    ref?: string;
    /**
     * Selects the control using a browser selector expression.
     */
    selector?: string;
  }

  /**
   * Groups the browser form controls populated in one operation.
   */
  export interface HostBrowserFillFormOptions {
    /**
     * Lists each control and value to apply.
     */
    fields: HostBrowserFillFormOptionsFieldsItem[];
  }

  /**
   * Chooses how an active browser alert, confirmation, or prompt is resolved.
   */
  export interface HostBrowserHandleDialogOptions {
    /**
     * Accepts the dialog when true and dismisses it when false.
     */
    accept: boolean;
    /**
     * Supplies text entered into a prompt dialog before it is accepted.
     */
    promptText?: string;
  }

  /**
   * Identifies the browser element that receives a hover action.
   */
  export interface HostBrowserHoverOptions {
    /**
     * Selects the element by page-snapshot reference.
     */
    ref: string;
    /**
     * Describes the selected element for diagnostics.
     */
    element?: string;
  }

  /**
   * Configures collection of network requests observed by the browser session.
   */
  export interface HostBrowserNetworkRequestsOptions {
    /**
     * Includes requests for static assets such as images, scripts, and stylesheets.
     */
    includeStatic?: boolean;
    /**
     * Writes the collected request log to this file when provided.
     */
    filename?: string;
  }

  /**
   * Wraps the keyboard key dispatched to the active browser page.
   */
  export interface HostBrowserPressKeyKeyOrOptionsVariant2 {
    /**
     * Contains a Playwright-compatible key name or shortcut chord.
     */
    key: string;
  }

  /**
   * Accepts a direct key name or object-style keyboard options.
   */
  export type HostBrowserPressKeyKeyOrOptions = string | HostBrowserPressKeyKeyOrOptionsVariant2;

  /**
   * Sets the dimensions of the browser viewport in CSS pixels.
   */
  export interface HostBrowserResizeOptions {
    /**
     * Sets the viewport width in CSS pixels.
     */
    width: number;
    /**
     * Sets the viewport height in CSS pixels.
     */
    height: number;
  }

  /**
   * Supplies Playwright-style automation code for execution in the browser session.
   */
  export interface HostBrowserRunCodeOptions {
    /**
     * Contains the automation program executed against the current page.
     */
    code: string;
  }

  /**
   * Identifies a select control and the option values chosen within it.
   */
  export interface HostBrowserSelectOptionOptions {
    /**
     * Selects the control by page-snapshot reference.
     */
    ref: string;
    /**
     * Lists the option values to select.
     */
    values: string[];
    /**
     * Describes the selected control for diagnostics.
     */
    element?: string;
  }

  /**
   * Configures the scope and optional file output of a textual page snapshot.
   */
  export interface HostBrowserSnapshotOptions {
    /**
     * Writes the snapshot to this file when provided.
     */
    filename?: string;
    /**
     * Restricts the snapshot to the subtree matching this selector.
     */
    selector?: string;
    /**
     * Limits how deeply descendant elements are included.
     */
    depth?: number;
  }

  /**
   * Selects the image encoding used for a browser screenshot.
   */
  export type HostBrowserTakeScreenshotOptionsType = "png" | "jpeg";

  /**
   * Configures screenshot encoding, target, page coverage, and file output.
   */
  export interface HostBrowserTakeScreenshotOptions {
    /**
     * Selects PNG or JPEG image encoding.
     */
    type?: HostBrowserTakeScreenshotOptionsType;
    /**
     * Writes the screenshot to this file when provided.
     */
    filename?: string;
    /**
     * Describes the target element for diagnostics.
     */
    element?: string;
    /**
     * Restricts the screenshot to an element selected by snapshot reference.
     */
    ref?: string;
    /**
     * Captures the complete scrollable page instead of the visible viewport.
     */
    fullPage?: boolean;
  }

  /**
   * Selects a browser tab-management action and optional tab index.
   */
  export interface HostBrowserTabsOptions {
    /**
     * Names the tab operation, such as listing, opening, closing, or selecting a tab.
     */
    action: string;
    /**
     * Identifies the tab affected by an indexed action.
     */
    index?: number;
  }

  /**
   * Configures text entry into a browser element selected from a page snapshot.
   */
  export interface HostBrowserTypeOptions {
    /**
     * Selects the input element by page-snapshot reference.
     */
    ref: string;
    /**
     * Contains the text entered into the element.
     */
    text: string;
    /**
     * Describes the selected element for diagnostics.
     */
    element?: string;
    /**
     * Submits the containing form after text entry.
     */
    submit?: boolean;
    /**
     * Types character by character to trigger keyboard-driven page behavior.
     */
    slowly?: boolean;
  }

  /**
   * Selects a duration or text condition to await in the browser session.
   */
  export interface HostBrowserWaitForOptions {
    /**
     * Waits for this duration in seconds.
     */
    time?: number;
    /**
     * Waits until this text appears on the page.
     */
    text?: string;
    /**
     * Waits until this text is no longer present on the page.
     */
    textGone?: string;
  }

  /**
   * Configures which installed browser userscripts are listed.
   */
  export interface HostBrowserUserscriptListOptions {
    /**
     * Includes installed scripts that are currently disabled.
     */
    include_disabled?: boolean;
  }

  /**
   * Supplies a remote, local, or inline source for browser userscript installation.
   */
  export interface HostBrowserUserscriptInstallOptions {
    /**
     * Downloads the userscript from this remote URL.
     */
    url?: string;
    /**
     * Reads the userscript from this local file path.
     */
    path?: string;
    /**
     * Contains inline userscript source code.
     */
    source?: string;
    /**
     * Records the canonical source URL associated with inline or local code.
     */
    source_url?: string;
    /**
     * Provides a human-readable source label shown in management UI.
     */
    source_display?: string;
  }

  /**
   * Accepts the string or numeric identifier assigned to an installed userscript.
   */
  export type HostBrowserUserscriptStartOptionsScriptId = string | number;

  /**
   * Identifies an installed browser userscript to enable.
   */
  export interface HostBrowserUserscriptStartOptions {
    /**
     * Selects the script by its installation identifier.
     */
    script_id?: HostBrowserUserscriptStartOptionsScriptId;
    /**
     * Selects the script by metadata name.
     */
    name?: string;
    /**
     * Disambiguates scripts with the same metadata name.
     */
    namespace?: string;
    /**
     * Selects the script by its canonical source URL.
     */
    source_url?: string;
  }

  /**
   * Accepts the string or numeric identifier assigned to an installed userscript.
   */
  export type HostBrowserUserscriptStopOptionsScriptId = string | number;

  /**
   * Identifies an installed browser userscript to disable.
   */
  export interface HostBrowserUserscriptStopOptions {
    /**
     * Selects the script by its installation identifier.
     */
    script_id?: HostBrowserUserscriptStopOptionsScriptId;
    /**
     * Selects the script by metadata name.
     */
    name?: string;
    /**
     * Disambiguates scripts with the same metadata name.
     */
    namespace?: string;
    /**
     * Selects the script by its canonical source URL.
     */
    source_url?: string;
  }

  /**
   * Accepts the string or numeric identifier assigned to an installed userscript.
   */
  export type HostBrowserUserscriptUninstallOptionsScriptId = string | number;

  /**
   * Identifies an installed browser userscript to remove.
   */
  export interface HostBrowserUserscriptUninstallOptions {
    /**
     * Selects the script by its installation identifier.
     */
    script_id?: HostBrowserUserscriptUninstallOptionsScriptId;
    /**
     * Selects the script by metadata name.
     */
    name?: string;
    /**
     * Disambiguates scripts with the same metadata name.
     */
    namespace?: string;
    /**
     * Selects the script by its canonical source URL.
     */
    source_url?: string;
  }

  /**
   * Selects the HTTP request method used by the configurable network API.
   */
  export type HostHttpOptionsMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS";

  /**
   * Accepts a textual or JSON-compatible HTTP request body.
   */
  export type HostHttpOptionsBody = string | unknown;

  /**
   * Selects how the HTTP response body is decoded for the plugin.
   */
  export type HostHttpOptionsResponseType = "text" | "json" | "arraybuffer" | "blob";

  /**
   * Configures an HTTP request, transport policy, and response decoding.
   */
  export interface HostHttpOptions {
    /**
     * Contains the absolute request URL.
     */
    url: string;
    /**
     * Selects the HTTP method sent to the server.
     */
    method?: HostHttpOptionsMethod;
    /**
     * Adds request header names and values.
     */
    headers?: Record<string, string>;
    /**
     * Contains the optional request body.
     */
    body?: HostHttpOptionsBody;
    /**
     * Sets the maximum time allowed to establish a connection.
     */
    connect_timeout?: number;
    /**
     * Sets the maximum wait for response data after connection.
     */
    read_timeout?: number;
    /**
     * Controls whether HTTP redirects are followed automatically.
     */
    follow_redirects?: boolean;
    /**
     * Disables TLS certificate validation for this request.
     */
    ignore_ssl?: boolean;
    /**
     * Selects how the response body is represented in the result.
     */
    responseType?: HostHttpOptionsResponseType;
    /**
     * Requests status-code validation as part of request completion.
     */
    validateStatus?: boolean;
  }

  /**
   * Selects the HTTP method used for a multipart file upload.
   */
  export type HostUploadFileOptionsMethod = "POST" | "PUT";

  /**
   * Describes one local file part included in a multipart request.
   */
  export interface HostUploadFileOptionsFilesItem {
    /**
     * Sets the multipart form field name for the file part.
     */
    field_name: string;
    /**
     * Identifies the local file uploaded in this part.
     */
    file_path: string;
    /**
     * Overrides the MIME type reported for the file part.
     */
    content_type?: string;
    /**
     * Overrides the file name reported in multipart metadata.
     */
    file_name?: string;
  }

  /**
   * Configures a multipart request containing files and textual form fields.
   */
  export interface HostUploadFileOptions {
    /**
     * Contains the upload endpoint URL.
     */
    url: string;
    /**
     * Selects POST or PUT for the upload request.
     */
    method?: HostUploadFileOptionsMethod;
    /**
     * Adds request headers to the multipart upload.
     */
    headers?: Record<string, string>;
    /**
     * Adds textual fields alongside the uploaded files.
     */
    form_data?: Record<string, string>;
    /**
     * Disables TLS certificate validation for this upload.
     */
    ignore_ssl?: boolean;
    /**
     * Lists the local file parts included in the request.
     */
    files: HostUploadFileOptionsFilesItem[];
  }

  /**
   * Accepts a serialized cookie header or a name-value map when storing cookies.
   */
  export type CookieManagerSetCookies = string | Record<string, string>;

  /**
   * Click an element by snapshot ref or selector.
   * Only accepts one options object.
   */
  function browserClick(options: HostBrowserClickOptions): Promise<string>;
  /**
   * Close the current browser tab.
   */
  function browserClose(options?: Record<string, never>): Promise<string>;
  /**
   * Close all browser tabs.
   */
  function browserCloseAll(options?: Record<string, never>): Promise<string>;
  /**
   * Read console messages from the browser session.
   */
  function browserConsoleMessages(options?: HostBrowserConsoleMessagesOptions): Promise<string>;
  /**
   * Drag between two elements by snapshot refs.
   */
  function browserDrag(options: HostBrowserDragOptions): Promise<string>;
  /**
   * Evaluate JavaScript in the browser session.
   */
  function browserEvaluate(options: HostBrowserEvaluateOptions): Promise<string>;
  /**
   * Resolve an active file chooser in the browser session.
   * If `paths` is omitted, the file chooser is cancelled.
   */
  function browserFileUpload(options?: HostBrowserFileUploadOptions): Promise<string>;
  /**
   * Fill multiple form fields in the browser session.
   */
  function browserFillForm(options: HostBrowserFillFormOptions): Promise<string>;
  /**
   * Handle an active dialog.
   */
  function browserHandleDialog(options: HostBrowserHandleDialogOptions): Promise<string>;
  /**
   * Hover over an element by snapshot ref.
   */
  function browserHover(options: HostBrowserHoverOptions): Promise<string>;
  /**
   * Navigate a browser session to a target URL.
   */
  function browserNavigate(urlOrOptions: HostBrowserNavigateUrlOrOptions): Promise<string>;
  /**
   * Go back in browser history.
   */
  function browserNavigateBack(options?: Record<string, never>): Promise<string>;
  /**
   * Read network requests from the browser session.
   */
  function browserNetworkRequests(options?: HostBrowserNetworkRequestsOptions): Promise<string>;
  /**
   * Press a keyboard key in the browser session.
   */
  function browserPressKey(keyOrOptions: HostBrowserPressKeyKeyOrOptions): Promise<string>;
  /**
   * Resize the browser viewport.
   */
  function browserResize(options: HostBrowserResizeOptions): Promise<string>;
  /**
   * Run Playwright-style code in the browser session.
   */
  function browserRunCode(options: HostBrowserRunCodeOptions): Promise<string>;
  /**
   * Select options in a dropdown by snapshot ref.
   */
  function browserSelectOption(options: HostBrowserSelectOptionOptions): Promise<string>;
  /**
   * Capture a text snapshot of current page.
   */
  function browserSnapshot(options?: HostBrowserSnapshotOptions): Promise<string>;
  /**
   * Manage browser tabs.
   */
  function browserTabs(options: HostBrowserTabsOptions): Promise<string>;
  /**
   * Take a screenshot of the current page or a target element.
   */
  function browserTakeScreenshot(options: HostBrowserTakeScreenshotOptions): Promise<string>;
  /**
   * Type text into an element by snapshot ref.
   */
  function browserType(options: HostBrowserTypeOptions): Promise<string>;
  /**
   * Installs a browser userscript from exactly one supported source.
   */
  function browserUserscriptInstall(options: HostBrowserUserscriptInstallOptions): Promise<string>;
  /**
   * Lists installed browser session userscripts.
   */
  function browserUserscriptList(options?: HostBrowserUserscriptListOptions): Promise<string>;
  /**
   * Enables one installed browser session userscript.
   */
  function browserUserscriptStart(options: HostBrowserUserscriptStartOptions): Promise<string>;
  /**
   * Disables one installed browser session userscript.
   */
  function browserUserscriptStop(options: HostBrowserUserscriptStopOptions): Promise<string>;
  /**
   * Uninstalls one browser session userscript.
   */
  function browserUserscriptUninstall(options: HostBrowserUserscriptUninstallOptions): Promise<string>;
  /**
   * Wait for text or time in the browser session.
   */
  function browserWaitFor(options: HostBrowserWaitForOptions): Promise<string>;
  /**
   * Enhanced HTTP request with flexible options
   * @param options - HTTP request options
   */
  function http(options: HostHttpOptions): Promise<HttpResponseData>;
  /**
   * Perform HTTP GET request
   * @param url - URL to request
   */
  function httpGet(url: string, ignore_ssl?: boolean): Promise<HttpResponseData>;
  /**
   * Perform HTTP POST request
   * @param url - URL to request
   * @param data - Data to post
   */
  function httpPost(url: string, body: HostHttpPostBody, ignore_ssl?: boolean): Promise<HttpResponseData>;
  /**
   * Starts a persistent browser session hosted in a floating WebView.
   */
  function startBrowser(options?: HostStartBrowserOptions): Promise<string>;
  /**
   * Stops one persistent browser session or every active session.
   */
  function stopBrowser(sessionIdOrOptions?: HostStopBrowserSessionIdOrOptions): Promise<string>;
  /**
   * Upload file using multipart request
   * @param options - Upload options
   */
  function uploadFile(options: HostUploadFileOptions): Promise<HttpResponseData>;
  /**
   * Visit a webpage and extract readable webpage content.
   * Not a replacement for raw HTTP GET/POST: when you actually need API
   * responses or precise response bodies, use httpGet/httpPost/http instead,
   * otherwise this may return empty or incomplete content.
   * @param urlOrParams - URL to visit, or an object with visit parameters.
   */
  function visit(urlOrParams: HostVisitUrlOrParams): Promise<VisitWebResultData>;
  /**
   * Reads, writes, and clears cookies associated with network domains.
   */
  export interface CookieManager {
    /**
     * Get cookies for a domain
     * @param domain - Domain to get cookies for
     */
    get(domain: string): Promise<HttpResponseData>;
    /**
     * Set cookies for a domain
     * @param domain - Domain to set cookies for
     * @param cookies - Cookies to set (can be string or object)
     */
    set(domain: string, cookies: CookieManagerSetCookies): Promise<HttpResponseData>;
    /**
     * Clear cookies for a domain
     * @param domain - Domain to clear cookies for
     */
    clear(domain?: string): Promise<HttpResponseData>;
  }

  /**
   * Exposes cookie operations through the network service.
   */
  const cookies: CookieManager;

}
