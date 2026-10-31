import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import {
  createCipheriv,
  createDecipheriv,
  createHash,
  randomBytes,
} from "node:crypto";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { provider } from "../../waifu/tests/harness.mjs";
import { clock, plain, settle } from "../../qqbot/tests/harness.mjs";

export { clock, plain, settle };
const require = createRequire(
  new URL("../../../buildin/workflow/package.json", import.meta.url),
);
const { buildSync } = require("esbuild");

/** Loads authored plugin code with explicit host ports instead of substituting service logic. */
export function load(name, globals = {}) {
  const file = fileURLToPath(new URL(`../src/${name}.ts`, import.meta.url));
  const output = buildSync({
    entryPoints: [file],
    bundle: true,
    format: "cjs",
    platform: "neutral",
    target: "es2020",
    write: false,
  });
  const module = { exports: {} };
  const errors = [];
  vm.runInNewContext(
    output.outputFiles[0].text,
    {
      module,
      exports: module.exports,
      console: {
        log() {},
        error(...args) {
          errors.push(args);
        },
      },
      ...globals,
    },
    { filename: file },
  );
  return { ...module.exports, errors };
}

/** Exercises the production CryptoJS facade with real Node crypto as the native-operation port. */
export function binaryCrypto() {
  const context = {
    __operitExpose(name, value) {
      context[name] = value;
    },
    __operitNativeCrypto(algorithm, operation, args) {
      if (algorithm !== "binary")
        throw new Error("Unexpected crypto algorithm");
      if (operation === "randomHex")
        return randomBytes(Number(args[0])).toString("hex");
      if (operation === "md5")
        return createHash("md5")
          .update(Buffer.from(args[0], "base64"))
          .digest("hex");
      const transform =
        operation === "encryptAesEcb"
          ? createCipheriv
          : operation === "decryptAesEcb"
            ? createDecipheriv
            : null;
      if (transform === null) throw new Error("Unknown binary operation");
      const cipher = transform(
        "aes-128-ecb",
        Buffer.from(args[1], "hex"),
        null,
      );
      return Buffer.concat([
        cipher.update(Buffer.from(args[0], "base64")),
        cipher.final(),
      ]).toString("base64");
    },
  };
  const script = readFileSync(
    new URL(
      "../../../../../core/crates/plugin/javascript-bridge/src/javascript/CryptoJS.script.js",
      import.meta.url,
    ),
    "utf8",
  );
  vm.runInNewContext(script, context);
  return context.CryptoJS;
}

/** Builds the exact official inbound message shape, including the reply context. */
export function message(id = 1, user = "alice@im.wechat", content = "你好") {
  return {
    message_id: id,
    from_user_id: user,
    to_user_id: "bot",
    message_type: 1,
    message_state: 2,
    create_time_ms: 100000,
    context_token: `ctx-${user}`,
    item_list: [{ type: 1, text_item: { text: content } }],
  };
}

/** Creates an authentic HTTP response envelope for the host port. */
export function response(data = { ret: 0 }, extra = {}) {
  return {
    statusCode: 200,
    headers: {},
    content: JSON.stringify(data),
    contentBase64: "",
    size: 0,
    ...extra,
  };
}

/** Runs the actual service, dependency client and Waifu provider against controlled host boundaries. */
export async function fixture(options = {}) {
  const sourceClock = clock();
  const time = {
    ...sourceClock,
    /** Advances real asynchronous chains between timer ticks instead of jumping past their continuations. */
    async advance(milliseconds) {
      for (let elapsed = 0; elapsed < milliseconds; elapsed += 50) {
        await sourceClock.advance(Math.min(50, milliseconds - elapsed));
        await settle();
      }
    },
  };
  const config = options.config ?? load("model").defaultConfig();
  if (!options.config) {
    config.account = options.loggedOut
      ? null
      : {
          botId: "bot",
          userId: "owner@im.wechat",
          token: "secret-token",
          baseUrl: "https://ilinkai.weixin.qq.com",
        };
    config.autoReply.enabled = options.auto === true;
  }
  const requests = [],
    snapshots = [],
    created = [],
    submitted = [],
    cancelled = [],
    uploads = [],
    files = new Map();
  const crypto = binaryCrypto();
  const waifu = provider({ ...time });
  const settings = await waifu.call("settings.get", { profile: "default" });
  await waifu.call("settings.set", {
    profile: "default",
    settings: {
      ...settings,
      charDelayMs: 0,
      enableEmoticons: options.emoticons === true,
    },
  });
  let pendingPoll = null,
    pendingQR = null,
    pendingReply = null,
    sent = 0;
  const batches = [],
    qrStates = [];
  const Tools = {
    Net: {
      async http(request) {
        requests.push(plain(request));
        if (request.url.endsWith("/getupdates")) {
          if (batches.length) return batches.shift();
          return new Promise((resolve) => {
            pendingPoll = resolve;
          });
        }
        if (/get_bot_qrcode\?/.test(request.url)) {
          if (options.qrFailure) throw new Error("QR network failed");
          return response({
            qrcode: "login-secret",
            qrcode_img_content: "https://liteapp.weixin.qq.com/q/test",
          });
        }
        if (/get_qrcode_status\?/.test(request.url)) {
          if (qrStates.length) return response(qrStates.shift());
          return new Promise((resolve) => {
            pendingQR = resolve;
          });
        }
        if (request.url.endsWith("/sendmessage")) {
          sent++;
          if (options.failSendAt === sent)
            return response({ ret: 400, errmsg: "delivery denied" });
          return response();
        }
        if (request.url.endsWith("/getconfig"))
          return response({ ret: 0, typing_ticket: "typing-ticket" });
        if (request.url.endsWith("/getuploadurl"))
          return response({ upload_param: "upload-query" });
        if (/\/download\?/.test(request.url))
          return response(
            {},
            { contentBase64: options.downloadData ?? "", size: 8 },
          );
        if (request.url.endsWith("/notifystart") && options.startFailure)
          return response({ ret: -14, errmsg: "expired" });
        return response();
      },
    },
    Files: {
      async info(path) {
        return {
          size: (files.get(path) ?? options.defaultFile ?? Buffer.from("image"))
            .length,
        };
      },
      async readBinary(path) {
        const bytes =
          files.get(path) ?? options.defaultFile ?? Buffer.from("image");
        return { contentBase64: bytes.toString("base64"), size: bytes.length };
      },
      async mkdir() {
        return { successful: true };
      },
      async writeBinary(path, data) {
        files.set(path, Buffer.from(data, "base64"));
        return { successful: true };
      },
    },
    Chat: {
      async createNew(input) {
        created.push(plain(input));
        return { chatId: `chat-${created.length}` };
      },
      async updateTitle() {},
      async updateGroup() {},
      async cancel(chatId) {
        cancelled.push(chatId);
        if (pendingReply !== null) {
          pendingReply({
            chatId,
            outcome: { type: "committed", status: "cancelled" },
          });
          pendingReply = null;
        }
        return { chatId, cancelRequested: true };
      },
      async *sendMessageStreaming(input) {
        submitted.push(plain(input));
        if (options.deferReply) {
          if (options.firstPart)
            yield {
              type: "part",
              messageTimestamp: 1,
              parts: [{ kind: "markdown", content: options.firstPart }],
            };
          const result = await new Promise((resolve) => {
            pendingReply = resolve;
          });
          yield { type: "completed", result };
          return;
        }
        if (options.replyFailure) throw new Error("model failed");
        const content = options.replyText ?? "实际模型回复。";
        yield {
          type: "part",
          messageTimestamp: 1,
          parts: [{ kind: "markdown", content }],
        };
        yield {
          type: "completed",
          result: {
            chatId: input.chatId,
            outcome: { type: "committed", status: "completed" },
          },
        };
      },
    },
  };
  const builder = {
    connectTimeout() {
      return this;
    },
    readTimeout() {
      return this;
    },
    writeTimeout() {
      return this;
    },
    followRedirects() {
      return this;
    },
    retryOnConnectionFailure() {
      return this;
    },
    build() {
      return this;
    },
    newRequest() {
      const request = {};
      return {
        url(value) {
          request.url = value;
          return this;
        },
        method(value) {
          request.method = value;
          return this;
        },
        header(key, value) {
          request[key] = value;
          return this;
        },
        body(value, type) {
          request.body = value;
          request.bodyType = type;
          return this;
        },
        build() {
          return this;
        },
        async execute() {
          uploads.push(request);
          return {
            raw: response(
              {},
              { headers: { "x-encrypted-param": "download-query" } },
            ),
          };
        },
      };
    },
  };
  const globals = {
    ...time,
    Tools,
    CryptoJS: crypto,
    getPluginConfigDir: () => "/plugins/wechat",
    OkHttp: { newBuilder: () => builder },
    PluginConfig: {
      async use() {
        return config;
      },
      async flush() {
        snapshots.push(plain(config));
      },
    },
    ToolPkg: {
      ipc: { on() {} },
      async callDependency(owner, method, input) {
        if (owner === "com.operit.character_cards")
          return [{ id: "alice", name: "Alice" }];
        if (owner !== "com.operit.waifu")
          throw new Error("Unexpected dependency");
        return waifu.call(method, input, "com.operit.wechat_bot");
      },
    },
  };
  const service = load("service", globals);
  /** Delivers one exact getUpdates response to the active host request. */
  async function batch(messages, cursor = "cursor-1", extra = {}) {
    const result = response({
      ret: 0,
      msgs: messages,
      get_updates_buf: cursor,
      ...extra,
    });
    if (pendingPoll === null) batches.push(result);
    else {
      const resolve = pendingPoll;
      pendingPoll = null;
      resolve(result);
    }
    await settle();
  }
  /** Delivers one QR status without inventing account confirmation. */
  async function qr(result) {
    if (pendingQR === null) qrStates.push(result);
    else {
      const resolve = pendingQR;
      pendingQR = null;
      resolve(response(result));
    }
    await settle();
  }
  /** Finishes pending host calls so stop and shutdown can complete deterministically. */
  async function finish(operation) {
    const task = operation();
    await settle();
    if (pendingPoll !== null) await batch([], "stop-cursor");
    if (pendingQR !== null) await qr({ status: "wait" });
    await time.advance(100);
    return task;
  }
  return {
    time,
    config,
    requests,
    snapshots,
    created,
    submitted,
    cancelled,
    uploads,
    files,
    crypto,
    globals,
    service,
    batch,
    qr,
    finish,
    call: (request) => service.receive(request),
    status: async () => (await service.receive({ action: "status" })).status,
    sends: () =>
      requests.filter((request) => request.url.endsWith("/sendmessage")),
    stop: () => finish(() => service.receive({ action: "stop" })),
  };
}
