#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function vmWebSocketUrl(value) {
  const url = new URL(value);
  if (!["http:", "https:", "ws:", "wss:"].includes(url.protocol))
    throw new Error("VM Service URL 必须使用 http(s) 或 ws(s)");
  url.protocol =
    url.protocol === "http:"
      ? "ws:"
      : url.protocol === "https:"
        ? "wss:"
        : url.protocol;
  if (!url.pathname.endsWith("/ws"))
    url.pathname = url.pathname.replace(/\/?$/, "/") + "ws";
  return url.toString();
}
export class VmClient {
  constructor(url) {
    this.url = url;
    this.pending = new Map();
    this.next = 0;
  }
  async connect() {
    this.socket = new WebSocket(this.url);
    await new Promise((resolve, reject) => {
      this.socket.addEventListener("open", resolve, { once: true });
      this.socket.addEventListener(
        "error",
        () =>
          reject(
            new Error("无法连接 Flutter VM Service，请核对地址和端口转发"),
          ),
        { once: true },
      );
    });
    this.socket.addEventListener("message", (event) => {
      const m = JSON.parse(event.data);
      const pending = this.pending.get(m.id);
      if (!pending) return;
      this.pending.delete(m.id);
      clearTimeout(pending.timer);
      m.error
        ? pending.reject(
            new Error(
              m.error.message + ": " + JSON.stringify(m.error.data ?? ""),
            ),
          )
        : pending.resolve(m.result);
    });
    this.socket.addEventListener("close", () => {
      for (const item of this.pending.values()) {
        clearTimeout(item.timer);
        item.reject(new Error("VM Service 连接已关闭"));
      }
      this.pending.clear();
    });
  }
  request(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.next,
        timer = setTimeout(() => {
          this.pending.delete(id);
          reject(new Error(`${method} 超时`));
        }, 45000);
      this.pending.set(id, { resolve, reject, timer });
      this.socket.send(JSON.stringify({ jsonrpc: "2.0", id, method, params }));
    });
  }
  close() {
    this.socket?.close();
  }
}
export async function main(args = process.argv.slice(2)) {
  if (args.includes("--help") || !args.length) {
    console.log(
      "用法：node tools/figma/capture.mjs --vm-url <VM Service URL> [--scope <名称>] [--name <画板名称>] [--out <文件>] [--list]\n也可使用 --vm-service-file <Flutter --vmservice-out-file 生成的文件>\n应用需以 --dart-define=OPERIT_DESIGN_BRIDGE=true 启动 debug 构建。",
    );
    return;
  }
  const values = {};
  const flags = new Set();
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--list") {
      flags.add(args[i]);
      continue;
    }
    if (
      !["--vm-url", "--vm-service-file", "--scope", "--name", "--out"].includes(
        args[i],
      ) ||
      !args[i + 1]
    )
      throw new Error(`未知或缺少参数：${args[i]}`);
    values[args[i]] = args[++i];
  }
  let endpoint = values["--vm-url"];
  if (values["--vm-service-file"]) {
    const text = (await readFile(values["--vm-service-file"], "utf8")).trim();
    if (/^(?:https?|wss?):\/\//.test(text)) endpoint = text;
    else {
      const data = JSON.parse(text);
      endpoint = data.uri ?? data.vmServiceUri;
    }
  }
  if (!endpoint) throw new Error("请提供 --vm-url 或 --vm-service-file");
  const client = new VmClient(vmWebSocketUrl(endpoint));
  try {
    await client.connect();
    const vm = await client.request("getVM");
    let isolate;
    for (const item of vm.isolates ?? []) {
      const detail = await client.request("getIsolate", { isolateId: item.id });
      if (detail.extensionRPCs?.includes("ext.operit.design.capture")) {
        isolate = item.id;
        break;
      }
    }
    if (!isolate)
      throw new Error(
        "未找到设计采集扩展。请以 debug 和 OPERIT_DESIGN_BRIDGE=true 运行，并等待主窗口出现。",
      );
    const result = await client.request("ext.operit.design.capture", {
      isolateId: isolate,
      ...(flags.has("--list") ? { list: "true" } : {}),
      ...(values["--scope"] ? { scope: values["--scope"] } : {}),
      ...(values["--name"] ? { name: values["--name"] } : {}),
    });
    if (flags.has("--list")) {
      console.log(JSON.stringify(result, null, 2));
      return;
    }
    if (!result.document || result.document.format !== "operit-design")
      throw new Error("采集扩展未返回有效设计文件");
    const path = resolve(
      values["--out"] ?? "operit-capture.operit-design.json",
    );
    await writeFile(path, JSON.stringify(result.document, null, 2), "utf8");
    console.log(
      `已保存 ${path}\n${result.document.assets.length} 个素材；在 Figma 插件中导入此文件。`,
    );
    for (const warning of result.document.warnings ?? [])
      console.log(`说明：${warning}`);
  } finally {
    client.close();
  }
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
