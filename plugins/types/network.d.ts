/**
 * Network operation type definitions for Assistance Package Tools
 */

import { HttpResponseData, VisitWebResultData, StringResultData } from './results';

/**
 * Network operations namespace
 */
export namespace Net {
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
    function httpPost(url: string, body: string | object, ignore_ssl?: boolean): Promise<HttpResponseData>;

    /**
     * Visit a webpage and extract readable webpage content.
     * Not a replacement for raw HTTP GET/POST: when you actually need API
     * responses or precise response bodies, use httpGet/httpPost/http instead,
     * otherwise this may return empty or incomplete content.
     * @param urlOrParams - URL to visit, or an object with visit parameters.
     */
    function visit(urlOrParams: string | {
        url?: string;
        visit_key?: string;
        link_number?: number;
        include_image_links?: boolean;
        headers?: Record<string, string>;
        user_agent_preset?: string;
        user_agent?: string;
    }): Promise<VisitWebResultData>;

    /**
     * Start a persistent browser session (floating window WebView).
     * Returns StringResultData whose `value` is a JSON string payload.
     */
    function startBrowser(options?: {
        url?: string;
        headers?: Record<string, string> | string;
        user_agent?: string;
        session_name?: string;
    }): Promise<StringResultData>;

    /**
     * Stop one browser session or all browser sessions.
     * Returns StringResultData whose `value` is a JSON string payload.
     */
    function stopBrowser(sessionIdOrOptions?: string | {
        session_id?: string;
        close_all?: boolean;
    }): Promise<StringResultData>;

    /**
     * Navigate a browser session to a target URL.
     */
    function browserNavigate(
        urlOrOptions: string | {
            url: string;
            headers?: Record<string, string> | string;
        }
    ): Promise<StringResultData>;

    /**
     * Go back in browser history.
     */
    function browserNavigateBack(options?: Record<string, unknown>): Promise<StringResultData>;

    /**
     * Click an element by snapshot ref or selector.
     * Only accepts one options object.
     */
    function browserClick(options: {
        session_id?: string;
        ref?: string;
        selector?: string;
        element?: string;
        button?: 'left' | 'right' | 'middle';
        modifiers?: Array<'Alt' | 'Control' | 'ControlOrMeta' | 'Meta' | 'Shift'>;
        doubleClick?: boolean;
    }): Promise<StringResultData>;

    /**
     * Close the current browser tab.
     */
    function browserClose(options?: Record<string, unknown>): Promise<StringResultData>;

    /**
     * Close all browser tabs.
     */
    function browserCloseAll(options?: Record<string, unknown>): Promise<StringResultData>;

    /**
     * Read console messages from the browser session.
     */
    function browserConsoleMessages(options?: {
        level?: string;
        filename?: string;
    }): Promise<StringResultData>;

    /**
     * Drag between two elements by snapshot refs.
     */
    function browserDrag(options: {
        startElement: string;
        startRef: string;
        endElement: string;
        endRef: string;
    }): Promise<StringResultData>;

    /**
     * Evaluate JavaScript in the browser session.
     */
    function browserEvaluate(options: {
        function: string;
        ref?: string;
        element?: string;
    }): Promise<StringResultData>;

    /**
     * Resolve an active file chooser in the browser session.
     * If `paths` is omitted, the file chooser is cancelled.
     */
    function browserFileUpload(options?: {
        paths?: string[];
    }): Promise<StringResultData>;

    /**
     * Fill multiple form fields in the browser session.
     */
    function browserFillForm(options: {
        fields: Array<{
            name: string;
            type: string;
            value: string | number | boolean | object;
            ref?: string;
            selector?: string;
        }>;
    }): Promise<StringResultData>;

    /**
     * Handle an active dialog.
     */
    function browserHandleDialog(options: {
        accept: boolean;
        promptText?: string;
    }): Promise<StringResultData>;

    /**
     * Hover over an element by snapshot ref.
     */
    function browserHover(options: {
        ref: string;
        element?: string;
    }): Promise<StringResultData>;

    /**
     * Read network requests from the browser session.
     */
    function browserNetworkRequests(options?: {
        includeStatic?: boolean;
        filename?: string;
    }): Promise<StringResultData>;

    /**
     * Press a keyboard key in the browser session.
     */
    function browserPressKey(keyOrOptions: string | {
        key: string;
    }): Promise<StringResultData>;

    /**
     * Resize the browser viewport.
     */
    function browserResize(options: {
        width: number;
        height: number;
    }): Promise<StringResultData>;

    /**
     * Run Playwright-style code in the browser session.
     */
    function browserRunCode(options: {
        code: string;
    }): Promise<StringResultData>;

    /**
     * Select options in a dropdown by snapshot ref.
     */
    function browserSelectOption(options: {
        ref: string;
        values: string[];
        element?: string;
    }): Promise<StringResultData>;

    /**
     * Capture a text snapshot of current page.
     */
    function browserSnapshot(options?: {
        filename?: string;
        selector?: string;
        depth?: number;
    }): Promise<StringResultData>;

    /**
     * Manage browser tabs.
     */
    function browserTabs(options: {
        action: string;
        index?: number;
    }): Promise<StringResultData>;

    /**
     * Type text into an element by snapshot ref.
     */
    function browserType(options: {
        ref: string;
        text: string;
        element?: string;
        submit?: boolean;
        slowly?: boolean;
    }): Promise<StringResultData>;

    /**
     * Wait for text or time in the browser session.
     */
    function browserWaitFor(options: {
        time?: number;
        text?: string;
        textGone?: string;
    }): Promise<StringResultData>;

    /**
     * List installed browser session userscripts.
     */
    function browserUserscriptList(options?: {
        include_disabled?: boolean;
    }): Promise<StringResultData>;

    /**
     * Install a browser session userscript from a remote URL, local file path, or inline source text.
     * Exactly one of `url`, `path`, or `source` is required.
     */
    function browserUserscriptInstall(options: {
        url?: string;
        path?: string;
        source?: string;
        source_url?: string;
        source_display?: string;
    }): Promise<StringResultData>;

    /**
     * Enable an installed browser session userscript.
     */
    function browserUserscriptStart(options: {
        script_id?: string | number;
        name?: string;
        namespace?: string;
        source_url?: string;
    }): Promise<StringResultData>;

    /**
     * Disable an installed browser session userscript.
     */
    function browserUserscriptStop(options: {
        script_id?: string | number;
        name?: string;
        namespace?: string;
        source_url?: string;
    }): Promise<StringResultData>;

    /**
     * Uninstall an installed browser session userscript.
     */
    function browserUserscriptUninstall(options: {
        script_id?: string | number;
        name?: string;
        namespace?: string;
        source_url?: string;
    }): Promise<StringResultData>;

    /**
     * Enhanced HTTP request with flexible options
     * @param options - HTTP request options
     */
    function http(options: {
        url: string;
        method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';
        headers?: Record<string, string>;
        body?: string | object;
        connect_timeout?: number;
        read_timeout?: number;
        follow_redirects?: boolean;
        ignore_ssl?: boolean;
        responseType?: 'text' | 'json' | 'arraybuffer' | 'blob';
        validateStatus?: boolean;
    }): Promise<HttpResponseData>;

    /**
     * Upload file using multipart request
     * @param options - Upload options
     */
    function uploadFile(options: {
        url: string;
        method?: 'POST' | 'PUT';
        headers?: Record<string, string>;
        form_data?: Record<string, string>;
        ignore_ssl?: boolean;
        files: {
            field_name: string;
            file_path: string;
            content_type?: string;
            file_name?: string;
        }[];
    }): Promise<HttpResponseData>;

    /**
     * Cookie management interface
     */
    interface CookieManager {
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
        set(domain: string, cookies: string | Record<string, string>): Promise<HttpResponseData>;

        /**
         * Clear cookies for a domain
         * @param domain - Domain to clear cookies for
         */
        clear(domain?: string): Promise<HttpResponseData>;
    }

    /**
     * Cookie management
     */
    const cookies: CookieManager;
}
