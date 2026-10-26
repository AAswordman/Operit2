import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
export { clock, plain, settle } from "../../qqbot/tests/harness.mjs";
const require = createRequire(new URL("../../../buildin/workflow/package.json", import.meta.url));
const { buildSync } = require("esbuild");

/** Evaluates actual provider or public client source with explicit SDK boundaries. */
export function load(name, globals = {}) {
  const file = fileURLToPath(new URL(`../src/${name}.ts`, import.meta.url));
  const result = buildSync({ entryPoints: [file], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  const module = { exports: {} };
  vm.runInNewContext(result.outputFiles[0].text, { module, exports: module.exports, console, ...globals }, { filename: file });
  return module.exports;
}

/** Supplies the real main-runtime handlers and an authenticated dependency transport. */
export function provider(globals = {}) {
  const configs = new Map(); let flushes = 0;
  const resources = [];
  const PluginConfig = {
    async use(name, defaults) { if (!configs.has(name)) configs.set(name, defaults); return configs.get(name); },
    async flush() { flushes++; },
  };
  const service = load("service", { PluginConfig, ToolPkg: { async callDependency(_owner, method, input) { if (method === "chat.configuration.binding.read") return { chatId: input.chatId, selection: "card:alice" }; throw new Error(`Unexpected dependency method: ${method}`); }, async readResource(key) { resources.push(key); return `/host/resources/${key}.jpg`; } }, ...globals });
  const handlers = { "settings.get": service.settingsGet, "settings.set": service.settingsSet, "session.open": service.sessionOpen, "session.feed": service.sessionFeed, "session.close": service.sessionClose };
  /** Invokes a real declared public handler as a specified dependency owner. */
  async function call(method, payload, callerPackage = "consumer") {
    if (!(method in handlers)) throw new Error(`Unpublished method: ${method}`);
    return handlers[method]({ payload, callerPackage });
  }
  return { service, resources, call, flushes: () => flushes };
}
