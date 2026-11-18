import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { channel, chunkSize, formatOf, maxBase64, maxBytes, natural, text, type Command, type DocumentInfo, type FileInput, type Reply } from "./protocol";

const origin = "https://office.operit.local/";
const resources = [
  ["", "office_html", "index.html", "text/html"],
  ["app.js", "office_js", "app.js", "application/javascript"],
  ["app.css", "office_css", "app.css", "text/css"],
  ["pdfium.wasm", "office_pdfium", "pdfium.wasm", "application/wasm"],
] as const;

export default function screen(ctx: ComposeDslContext): ComposeNode {
  const [input] = ctx.useState<FileInput | null>("input", null);
  const [locale] = ctx.useState("locale", getLang());
  const [ready, setReady] = ctx.useState("office-ready", false);
  const [error, setError] = ctx.useState("office-error", "");
  const controller = ctx.createWebViewController("office");
  const assets = ctx.useRef("office-assets", new Map<string, { filePath: string; mimeType: string }>());
  const lifecycle = ctx.useRef("office-lifecycle", { stop: () => {} });

  ctx.useEffect("office-cleanup", () => () => lifecycle.current.stop(), []);

  async function initialize(): Promise<void> {
    if (!input || !input.path.startsWith("/") || input.path.includes("\0")) {
      setError("Open a workspace file to start Office.");
      return;
    }
    const path = input.path;
    const info: DocumentInfo = { ...input, format: formatOf(path), id: `office-${Date.now()}-${Math.random().toString(36).slice(2)}` };
    let active = true;
    let original: string | undefined;
    let pending: { size: number; length: number; parts: string[] } | undefined;
    let writing = false;
    let commandInFlight = false;
    let sequence = 0;
    const quotedRequests = new Set<string>();
    let unregister: (() => void) | undefined;
    async function readWorkingCopy(): Promise<string> {
      const file = await Tools.Files.readBinary(path);
      if (!active) throw new Error(locale.startsWith("zh") ? "文档已关闭，请重新打开。" : "The document was closed. Reopen it.");
      if (!file || typeof file.contentBase64 !== "string") {
        const failure = file as unknown as { error?: string; details?: string; message?: string } | string | null;
        const details = typeof failure === "string" ? failure : failure?.error || failure?.details || failure?.message;
        throw new Error(locale.startsWith("zh")
          ? `无法读取文档 ${input!.name}：${details || "未返回文件内容，请检查文件是否存在及读取权限。"}`
          : `Could not read ${input!.name}: ${details || "No file content returned. Check that the file exists and is readable."}`);
      }
      if (file.size > maxBytes || file.contentBase64.length > maxBase64) {
        const size = (file.size / 1024 / 1024).toFixed(1);
        throw new Error(locale.startsWith("zh")
          ? `文档大小为 ${size} MiB，超过 128 MiB 限制。`
          : `The document is ${size} MiB; the limit is 128 MiB.`);
      }
      if (!file.contentBase64.length) throw new Error(locale.startsWith("zh") ? "文档为空，请先写入文件内容再打开。" : "The document is empty. Write the file before opening it.");
      return file.contentBase64;
    }
    lifecycle.current.stop = () => {
      active = false;
      original = undefined;
      pending = undefined;
      unregister?.();
      void ToolPkg.ipc.call(channel.registry, { kind: "unregister", id: info.id }).catch(() => {});
      controller.removeJavascriptInterface("OfficeHost");
    };

    try {
      unregister = ToolPkg.ipc.on<Command, unknown>(channel.editor, async command => {
        if (!active || commandInFlight) throw new Error("Office editor is busy or closed");
        commandInFlight = true;
        const key = `${++sequence}`;
        try {
          const accepted = await controller.evaluateJavascript<boolean>(`Boolean(window.OfficeEditor?.submit(${JSON.stringify(key)},${JSON.stringify(command)}))`);
          if (accepted !== true) throw new Error("Office editor is not ready");
          const deadline = Date.now() + 25000;
          while (active && Date.now() < deadline) {
            const result = await controller.evaluateJavascript<Reply | string | null>(`window.OfficeEditor.result(${JSON.stringify(key)})`);
            if (result) {
              // The desktop bridge decodes JSON-looking strings; other hosts can return them as-is.
              const reply = typeof result === "string" ? JSON.parse(result) as Reply : result;
              if (!reply.ok) throw new Error(reply.error);
              return reply.value;
            }
            await Tools.System.sleep(150);
          }
          throw new Error("Office operation did not return in time; read the document again before retrying.");
        } finally {
          commandInFlight = false;
        }
      });
      controller.addJavascriptInterface("OfficeHost", {
        async bootstrap() {
          original = await readWorkingCopy();
          return { info, locale, length: original.length, chunkSize };
        },
        readPart(...args: unknown[]) {
          const [offset] = args[0] as unknown[];
          const start = natural(offset, "file offset");
          if (!active || original === undefined || start >= original.length) throw new Error("Invalid file chunk");
          return original.slice(start, start + chunkSize);
        },
        async editorReady() {
          if (!active || original === undefined) throw new Error("Office editor is closed");
          await ToolPkg.ipc.call(channel.registry, { kind: "register", info });
          return true;
        },
        beginSave(...args: unknown[]) {
          const [length] = args[0] as unknown[];
          const size = natural(length, "save length");
          if (!active || writing || size === 0 || size > maxBase64 || size % 4 !== 0) throw new Error("Invalid save length or save in progress");
          pending = { size, length: 0, parts: [] };
          return true;
        },
        appendSave(...args: unknown[]) {
          const [value] = args[0] as unknown[];
          const part = text(value, "file chunk", chunkSize);
          if (!active || !pending || !part || !/^[A-Za-z0-9+/]+={0,2}$/.test(part) || pending.length + part.length > pending.size) throw new Error("Invalid save chunk");
          pending.parts.push(part);
          pending.length += part.length;
          return true;
        },
        async commitSave() {
          if (!active || !pending || writing || pending.length !== pending.size || original === undefined) throw new Error("Incomplete save");
          const snapshot = pending.parts.join("");
          pending = undefined;
          writing = true;
          try {
            const current = await readWorkingCopy();
            if (current !== original) throw new Error("The workspace file changed outside this editor. Reopen it before saving.");
            const result = await Tools.Files.writeBinary(path, snapshot);
            if (!result.successful) throw new Error(result.details || "Could not save workspace file");
            original = snapshot;
            return true;
          } finally {
            writing = false;
          }
        },
        quote(...args: unknown[]) {
          const [value, requestId] = Array.isArray(args[0]) ? args[0] : [];
          if (!active || typeof value !== "string" || value.length > 12000 || !value.trim()) return null;
          // A bridge retry must not attach the same explicit quote twice.
          if (typeof requestId === "string") {
            if (quotedRequests.has(requestId)) return null;
            quotedRequests.add(requestId);
            if (quotedRequests.size > 128) quotedRequests.delete(quotedRequests.values().next().value!);
          }
          return { workspaceFileAction: "quote", text: value };
        },
        dirty(...args: unknown[]) {
          const [value] = args[0] as unknown[];
          return { workspaceFileAction: "dirty", dirty: value === true };
        },
      });
      for (const [suffix, key, name, mimeType] of resources) {
        assets.current.set(origin + suffix, { filePath: await ToolPkg.readResource(key, name), mimeType });
      }
      if (active) setReady(true);
    } catch (failure) {
      if (active) setError(String(failure));
    }
  }

  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, ready ? ctx.UI.WebView({
    key: "office", controller, url: origin, fillMaxSize: true, javaScriptEnabled: true,
    domStorageEnabled: true, supportZoom: false, useWideViewPort: true,
    onShouldOverrideUrlLoading: request => ({ action: request.url === origin ? "allow" : "cancel" }),
    onInterceptRequest: request => {
      const asset = assets.current.get(request.url);
      if (asset) return { action: "respond", response: { ...asset, statusCode: 200, reasonPhrase: "OK" } };
      return { action: /^(blob:|data:)/.test(request.url) ? "allow" : "block" };
    },
  }) : ctx.UI.Text({ text: error || (locale.startsWith("zh") ? "正在打开文档…" : "Opening document…") }));
}
