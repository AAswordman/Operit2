import { MAX_FILE_BYTES, type DesignDocument, type Json } from "./bridge.ts";
import {
  MAX_ARCHIVE_BYTES,
  readSoftwarePack,
  writeSoftwarePack,
  type SoftwareEntry,
  type SoftwareIndex,
} from "./software_pack.ts";
let softwareQueue: Array<{ entry: SoftwareEntry; document: DesignDocument }> =
  [];
let softwareContext:
  | {
      packId: string;
      index: SoftwareIndex;
      total: number;
      done: number;
      failed: boolean;
    }
  | undefined;
const element = <T extends HTMLElement>(id: string) =>
  document.getElementById(id)! as T;
const status = element<HTMLOutputElement>("status");
const theme = element<HTMLTextAreaElement>("theme");
function send(
  type: string,
  text?: string,
  extra: Record<string, unknown> = {},
): void {
  document.querySelectorAll("button").forEach((b) => (b.disabled = true));
  status.className = "";
  status.textContent = "正在处理…";
  parent.postMessage({ pluginMessage: { type, text, ...extra } }, "*");
}
element("import").onclick = () => element<HTMLInputElement>("file").click();
function nextSoftware(): void {
  const capture = softwareQueue.shift();
  if (!capture || !softwareContext) return;
  const context = softwareContext;
  send("import", JSON.stringify(capture.document), {
    software: {
      packId: context.packId,
      index: context.index as unknown as Json,
      entry: capture.entry,
    },
  });
  status.textContent = `正在导入软件界面 ${context.done + 1}/${context.total}：${capture.entry.name}`;
}
function download(bytes: BlobPart, filename: string, type: string): void {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 3000);
}
element<HTMLInputElement>("file").onchange = async (event) => {
  const input = event.currentTarget as HTMLInputElement,
    file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    if (/\.zip$/i.test(file.name)) {
      if (file.size > MAX_ARCHIVE_BYTES)
        throw new Error("软件包超过 256 MiB，请按功能分包");
      const pack = readSoftwarePack(new Uint8Array(await file.arrayBuffer()));
      softwareQueue = pack.documents;
      softwareContext = {
        packId: `software-${Date.now()}-${Math.random().toString(36).slice(2)}`,
        index: pack.index,
        total: pack.documents.length,
        done: 0,
        failed: false,
      };
      const missing = Array.isArray(pack.index.coverage)
        ? pack.index.coverage.filter(
            (item: any) => item.status === "unrecorded",
          ).length
        : 0;
      element("coverage").textContent =
        `软件包：${pack.documents.length} 个实际界面；${missing} 个源码界面候选尚未记录。具体覆盖清单保留在导出包 index.json 中。`;
      nextSoftware();
    } else {
      if (file.size > MAX_FILE_BYTES)
        throw new Error("单场景文件超过 64 MiB，请分开采集");
      softwareQueue = [];
      softwareContext = undefined;
      send("import", await file.text());
    }
  } catch (error) {
    status.textContent = String(error);
    status.className = "error";
    document.querySelectorAll("button").forEach((b) => (b.disabled = false));
  }
};
element("inspect").onclick = () => send("inspect");
element("export").onclick = () => send("export");
element("export-software").onclick = () => send("export-software");
element("apply").onclick = () => send("apply-theme", theme.value);
window.onmessage = (event: MessageEvent) => {
  const message = event.data?.pluginMessage;
  if (!message) return;
  if (message.type === "idle") {
    if (softwareContext && !softwareContext.failed && softwareQueue.length) {
      nextSoftware();
      return;
    }
    if (
      softwareContext &&
      !softwareContext.failed &&
      softwareContext.done === softwareContext.total
    ) {
      status.textContent = `已导入 ${softwareContext.total} 个软件界面。可直接编辑各个画板；选中其中一个后可导出整包修改。`;
      softwareContext = undefined;
    }
    document.querySelectorAll("button").forEach((b) => (b.disabled = false));
    return;
  }
  if (message.type === "error") {
    if (softwareContext) {
      softwareContext.failed = true;
      softwareQueue = [];
    }
    status.textContent = message.message;
    status.className = "error";
    return;
  }
  if (message.type === "loaded") {
    if (softwareContext) softwareContext.done++;
    status.textContent = `已载入 ${message.name}。现在可编辑图层与 Variables。`;
    theme.value = message.theme ? JSON.stringify(message.theme, null, 2) : "";
  }
  if (message.type === "download") {
    download(message.text, message.filename, "application/json");
    status.textContent = `已导出设计，包含 ${message.changes} 处图层变更。主题稿可导回 Theme Studio；运行时稿保留来源供代码实施。`;
  }
  if (message.type === "download-pack") {
    try {
      const bytes = writeSoftwarePack(
        message.index as SoftwareIndex,
        message.documents,
      );
      download(
        bytes.slice().buffer,
        "operit-software-design-changes.zip",
        "application/zip",
      );
      status.textContent = `已导出 ${message.documents.length} 个软件界面的修改、图层与来源信息。`;
    } catch (error) {
      status.textContent = String(error);
      status.className = "error";
    }
  }
  if (message.warnings)
    element("warnings").textContent =
      message.warnings.join("\n\n") || "没有额外说明";
};
