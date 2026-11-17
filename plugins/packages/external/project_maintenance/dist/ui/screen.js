"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = screen;
const client_js_1 = require("../client.js");
const ORIGIN = "https://maintenance.operit.local/";
const ACTIONS = new Set(["dashboard", "prepare_channel_chat", "available_chats", "bind_chat", "create_from_binding", "get", "inspect", "start", "add_note"]);
/** The React bundle is offline; only this exact document receives host capabilities. */
function screen(ctx) {
    const controller = ctx.createWebViewController("maintenance-control");
    const [file, setFile] = ctx.useState("maintenance-web-file", "");
    const [error, setError] = ctx.useState("maintenance-web-error", "");
    ctx.useEffect("maintenance-web-cleanup", () => () => controller.removeJavascriptInterface("MaintenanceHost"), []);
    async function initialize() {
        try {
            controller.addJavascriptInterface("MaintenanceHost", {
                request: async (...args) => {
                    // Action failures must return through the WebView, not escape into Compose.
                    try {
                        const params = args[0];
                        if (!Array.isArray(params) || params.length !== 1)
                            throw Error("Invalid bridge arguments");
                        const request = params[0];
                        if (!request || typeof request !== "object" || !ACTIONS.has(request.action))
                            throw Error("Unsupported dashboard action");
                        return { ok: true, data: await (0, client_js_1.call)(request) };
                    }
                    catch (error) {
                        return { ok: false, error: error instanceof Error ? error.message : String(error) };
                    }
                },
            });
            setFile(await ToolPkg.readResource("maintenance_web", "maintenance-control.html"));
        }
        catch (e) {
            setError(String(e));
        }
    }
    return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, file ? ctx.UI.WebView({
        key: "maintenance-control", controller, url: ORIGIN, fillMaxSize: true, javaScriptEnabled: true,
        domStorageEnabled: false, supportZoom: false, useWideViewPort: true,
        onShouldOverrideUrlLoading: r => r.url === ORIGIN ? { action: "allow" } : { action: "cancel" },
        onInterceptRequest: r => r.url === ORIGIN ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: file } } : { action: "block" },
    }) : ctx.UI.Text({ text: error || "正在加载维护中控…", padding: 24 }));
}
