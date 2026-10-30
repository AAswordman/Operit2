#!/usr/bin/env node
import { randomBytes, timingSafeEqual } from "node:crypto";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { VmClient, vmWebSocketUrl } from "./capture.mjs";
const extension = "ext.operit.design.runtime";
const fail = (status, message) => Object.assign(new Error(message), { status });
const object = (value) =>
  value && typeof value === "object" && !Array.isArray(value);

export function validateRequest(value) {
  if (!object(value)) throw fail(400, "请求必须是对象");
  const { operation, target } = value;
  if (!["snapshot", "event", "edit", "clear"].includes(operation))
    throw fail(400, "未知运行操作");
  if (
    target !== undefined &&
    (typeof target !== "string" || !target || target.length > 512)
  )
    throw fail(400, "非法控件标识");
  if (operation === "event") {
    const event = value.event;
    if (
      !object(event) ||
      !["tap", "scroll", "input"].includes(event.type) ||
      typeof event.target !== "string" ||
      !event.target ||
      event.target.length > 512
    )
      throw fail(400, "非法输入事件");
    const result = { type: event.type, target: event.target };
    for (const key of ["x", "y", "dx", "dy"])
      if (event[key] !== undefined) {
        if (typeof event[key] !== "number" || !Number.isFinite(event[key]))
          throw fail(400, "非法事件坐标");
        result[key] = event[key];
      }
    if (event.type === "input") {
      if (typeof event.text !== "string" || event.text.length > 65536)
        throw fail(400, "非法输入文字");
      result.text = event.text;
    }
    return { operation, event: result };
  }
  if (
    operation === "edit" &&
    (!target ||
      !object(value.properties) ||
      Object.keys(value.properties).length > 24)
  )
    throw fail(400, "编辑请求缺少控件或属性");
  return {
    operation,
    ...(target ? { target } : {}),
    ...(operation === "edit" ? { properties: value.properties } : {}),
  };
}

export async function connectRuntime(endpoint, scope) {
  const client = new VmClient(vmWebSocketUrl(endpoint));
  try {
    await client.connect();
    const vm = await client.request("getVM");
    let isolateId;
    for (const isolate of vm.isolates ?? []) {
      const details = await client.request("getIsolate", {
        isolateId: isolate.id,
      });
      if (details.extensionRPCs?.includes(extension)) {
        isolateId = isolate.id;
        break;
      }
    }
    if (!isolateId)
      throw new Error(
        "未发现设计运行接口，请启用 OPERIT_DESIGN_BRIDGE=true 后启动 debug 应用。",
      );
    return {
      request: (request) =>
        client.request(extension, {
          isolateId,
          ...(scope ? { scope } : {}),
          request: JSON.stringify(request),
        }),
      close: () => client.close(),
    };
  } catch (error) {
    client.close();
    throw error;
  }
}

/** Loopback HTTP transport. The capability token is never sent in a URL. */
export async function createLiveServer({
  runtime,
  port = 0,
  origins = [],
  token = randomBytes(32).toString("hex"),
  html,
}) {
  const allowed = new Set(
    origins.map((value) => {
      const url = new URL(value);
      if (!["http:", "https:"].includes(url.protocol) || value !== url.origin)
        throw new Error("Origin 必须是不带路径的完整 http(s) 来源");
      return value;
    }),
  );
  html ??= await readFile(
    new URL("./dist/live/index.html", import.meta.url),
    "utf8",
  );
  let origin;
  let queue = Promise.resolve();
  const requestRuntime = (request) => {
    const result = queue.then(async () => {
      const value = await runtime.request(request);
      if (
        value?.protocol !== "operit-design-runtime" ||
        value.version !== 1 ||
        value.document?.format !== "operit-design"
      )
        throw new Error("运行端返回了不兼容的数据");
      return value;
    });
    queue = result.catch(() => {});
    return result;
  };
  const server = createServer(async (req, res) => {
    res.setHeader("Cache-Control", "no-store");
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("Referrer-Policy", "no-referrer");
    const json = (status, value) => {
      res.writeHead(status, {
        "Content-Type": "application/json; charset=utf-8",
      });
      res.end(JSON.stringify(value));
    };
    try {
      if (req.headers.host !== new URL(origin).host)
        throw fail(403, "非法 Host");
      const from = req.headers.origin;
      if (from && from !== origin && !allowed.has(from))
        throw fail(403, "尚未允许此页面来源");
      if (from) {
        res.setHeader("Access-Control-Allow-Origin", from);
        res.setHeader("Vary", "Origin");
      }
      if (req.method === "OPTIONS" && req.url === "/runtime") {
        res.setHeader("Access-Control-Allow-Methods", "POST");
        res.setHeader(
          "Access-Control-Allow-Headers",
          "Authorization, Content-Type",
        );
        if (req.headers["access-control-request-private-network"] === "true")
          res.setHeader("Access-Control-Allow-Private-Network", "true");
        res.writeHead(204);
        res.end();
        return;
      }
      if (req.method === "GET" && req.url === "/") {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(html);
        return;
      }
      if (req.method !== "POST" || req.url !== "/runtime")
        throw fail(404, "入口不存在");
      const supplied = Buffer.from(
          req.headers.authorization?.replace(/^Bearer /, "") ?? "",
        ),
        expected = Buffer.from(token);
      if (
        supplied.length !== expected.length ||
        !timingSafeEqual(supplied, expected)
      )
        throw fail(401, "连接口令无效");
      if (!/^application\/json(?:;|$)/i.test(req.headers["content-type"] ?? ""))
        throw fail(415, "请使用 JSON 请求");
      const chunks = [];
      let size = 0;
      let overflow = false;
      for await (const chunk of req) {
        size += chunk.length;
        if (size > 256 * 1024) {
          overflow = true;
          continue;
        }
        chunks.push(chunk);
      }
      if (overflow) throw fail(413, "请求过大");
      let value;
      try {
        value = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      } catch {
        throw fail(400, "无效 JSON");
      }
      const result = await requestRuntime(validateRequest(value));
      json(200, result);
    } catch (error) {
      if (!res.headersSent) json(error.status ?? 502, { error: error.message });
      else res.end();
    }
  });
  server.requestTimeout = 60000;
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", resolve);
  });
  origin = `http://127.0.0.1:${server.address().port}`;
  return {
    server,
    origin,
    token,
    url: `${origin}/#token=${encodeURIComponent(token)}`,
    async close() {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
      runtime.close();
    },
  };
}

export async function main(args = process.argv.slice(2)) {
  if (!args.length || args.includes("--help")) {
    console.log(
      "用法：node tools/figma/live.mjs --vm-service-file <文件> [--scope <名称>] [--port <端口>] [--allow-origin <预览完整Origin>]\n也支持 --vm-url <地址>。先执行 npm --prefix tools/figma run build:live。",
    );
    return;
  }
  const values = {},
    origins = [];
  for (let i = 0; i < args.length; i++) {
    const key = args[i],
      value = args[++i];
    if (
      ![
        "--vm-service-file",
        "--vm-url",
        "--scope",
        "--port",
        "--allow-origin",
      ].includes(key) ||
      !value
    )
      throw new Error(`未知或缺少参数：${key}`);
    if (key === "--allow-origin") origins.push(value);
    else values[key] = value;
  }
  let endpoint = values["--vm-url"];
  if (values["--vm-service-file"]) {
    const text = (await readFile(values["--vm-service-file"], "utf8")).trim();
    endpoint = /^(?:https?|wss?):\/\//.test(text)
      ? text
      : (JSON.parse(text).uri ?? JSON.parse(text).vmServiceUri);
  }
  if (!endpoint) throw new Error("缺少 VM Service 地址");
  const port = Number(values["--port"] ?? 0);
  if (!Number.isInteger(port) || port < 0 || port > 65535)
    throw new Error("端口范围错误");
  const runtime = await connectRuntime(endpoint, values["--scope"]);
  try {
    const live = await createLiveServer({ runtime, port, origins });
    console.log(
      `Operit 实时设计连接：\n${live.url}\n仅监听本机。请勿把带口令的连接放进设计文件或版本库。`,
    );
    let closing = false;
    for (const signal of ["SIGINT", "SIGTERM"])
      process.on(signal, async () => {
        if (closing) return;
        closing = true;
        await live.close();
      });
    return live;
  } catch (error) {
    runtime.close();
    throw error;
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
