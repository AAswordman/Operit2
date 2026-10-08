import { plain } from "./runtime.mjs";

/** Adds only explicitly supplied SDK directory inputs to a real-disk unit test; this is not native-host integration. */
export function createSoftwareSettingsFixture(disk, input) {
  const values = plain(input), calls = [], faults = new Map();

  /** Audits exact no-argument directory calls and propagates a deliberately injected original error. */
  function attempted(method, args) {
    if (args.length !== 0) throw new Error("SoftwareSettings fixture requires the SDK's zero-argument directory call: " + method);
    calls.push({ method, args: [...args] });
    if (faults.has(method)) {
      const failure = faults.get(method); faults.delete(method); throw failure;
    }
  }

  const methods = {
    /** Returns the explicitly supplied complete SDK model records without changing their fields. */
    async listModelSummaries(...args) { attempted("listModelSummaries", args); return plain(values.models); },
    /** Returns the explicitly supplied complete SDK speech records without changing their fields. */
    async listTtsConfigs(...args) { attempted("listTtsConfigs", args); return plain(values.ttsConfigs); },
    /** Returns the explicitly supplied builtin, package, skill and MCP directory records. */
    async readToolSourceCatalog(...args) { attempted("readToolSourceCatalog", args); return plain(values.toolCatalog); },
  };
  const settings = new Proxy(methods, {
    /** Rejects every undeclared settings method, including CLI execution and environment mutation. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Undeclared SoftwareSettings fixture capability: " + String(property));
      return target[property];
    },
  });
  const tools = new Proxy({ Files: disk.files, SoftwareSettings: settings }, {
    /** Exposes only real disk IO and the three explicit test directory methods, without touching the IO-only harness. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Undeclared Tools fixture capability: " + String(property));
      return target[property];
    },
  });
  return {
    harness: { ...disk, globals: { ...disk.globals, Tools: tools } }, calls,
    /** Schedules an exact directory failure without inventing an empty result or a production success. */
    failNext(method, failure) {
      if (!Object.hasOwn(methods, method)) throw new Error("Cannot fail an undeclared SoftwareSettings fixture method: " + method);
      if (faults.has(method)) throw new Error("A SoftwareSettings failure is already scheduled for " + method);
      faults.set(method, failure);
    },
    /** Clears the directory-call audit only, preserving all explicitly supplied inputs and scheduled errors. */
    clearCalls() { calls.length = 0; },
  };
}
