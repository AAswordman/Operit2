import { JavaScriptTransport, javascriptFrame } from "./operit_javascript_transport.js";

const nativeEval = eval;
const NativeFunction = Function;
const NativeString = String;
const nativeApply = Reflect.apply;
const defer = setTimeout.bind(globalThis);
const define = Object.defineProperty;
const hasOwn = Object.prototype.hasOwnProperty;
let activeTransport: JavaScriptTransport | null = null;
let registry: Record<string, Function> | null = null;

interface JavaScriptCommand {
  operation: string;
  name?: string;
  source?: string;
  args?: unknown[];
  parameters?: string[];
  factory?: string;
}

/** Keeps transport callback ownership inside the module rather than plugin globals. */
function checkActiveTransport(): void {
  if (activeTransport === null) throw new Error("JavaScript host call has no active execution");
}

/** Calls one Rust-owned host binding and returns its structured response synchronously. */
function callHost(name: string, args: unknown[]): unknown {
  checkActiveTransport();
  const transport = activeTransport!;
  transport.send(javascriptFrame.hostCall, [name, args]);
  const response = transport.receive();
  if (response.kind !== javascriptFrame.hostResult) throw new Error("Expected JavaScript host result");
  const result = response.value as { ok: boolean; value: unknown };
  if (!result.ok) throw new Error(NativeString(result.value));
  return result.value;
}

/** Executes the exact host operation in this worker's independent global environment. */
function execute(command: JavaScriptCommand): unknown {
  switch (command.operation) {
    case "initialize":
      nativeApply(nativeEval, undefined, [command.source]);
      registry = (globalThis as Record<string, unknown>).__operitHostPromiseRegistry as Record<string, Function>;
      return null;
    case "evaluateVoid":
      nativeApply(nativeEval, undefined, [command.source + "\n//# sourceURL=" + command.name]);
      return null;
    case "evaluateString":
      return NativeString(nativeApply(nativeEval, undefined, [command.source + "\n//# sourceURL=" + command.name]));
    case "constructor": {
      const parameters = command.parameters!;
      /** Compiles a plugin module in this realm using the host-fixed parameter names. */
      const constructor = (body: string): Function => new NativeFunction(...parameters, body);
      define(globalThis, command.name!, { value: constructor, writable: true, configurable: true });
      return null;
    }
    case "binding": {
      const name = command.name!;
      /** Transfers normalized arguments while keeping the guest callback in its owning realm. */
      const submit = (...args: unknown[]): unknown => callHost(name, args);
      const binding = nativeApply(registry![command.factory!], registry, [submit]);
      define(globalThis, name, { value: binding, writable: true, configurable: true });
      return null;
    }
    case "call": {
      if (!nativeApply(hasOwn, globalThis, [command.name])) {
        throw new Error("JavaScript lifecycle function is unavailable: " + command.name);
      }
      const result = nativeApply((globalThis as Record<string, unknown>)[command.name!] as Function, undefined, command.args!);
      return nativeApply(registry!.snapshotResult, registry, [result]);
    }
    case "settle":
      nativeApply(registry!.settle, registry, command.args!);
      return null;
    case "cancel":
      nativeApply(registry!.cancel, registry, command.args!);
      return null;
    case "jobs":
      return null;
    default:
      throw new Error("Unknown JavaScript runtime operation: " + command.operation);
  }
}

/** Processes commands serially and lets the browser drain microtasks before acknowledging completion. */
globalThis.addEventListener("message", (event: MessageEvent<{ command: JavaScriptCommand; buffer: SharedArrayBuffer; control: Int32Array }>): void => {
  const transport = new JavaScriptTransport(event.data.buffer, event.data.control);
  activeTransport = transport;
  let value: unknown;
  let failed = false;
  try {
    value = execute(event.data.command);
  } catch (error) {
    failed = true;
    value = error instanceof Error ? error.stack : NativeString(error);
  }
  /** Completes this command after all native Promise continuations in its task have run. */
  defer((): void => {
    transport.send(failed ? javascriptFrame.error : javascriptFrame.result, value);
    activeTransport = null;
  }, 0);
});

/** Announces that the browser has loaded all execution-worker module dependencies. */
(globalThis as unknown as { postMessage(message: unknown): void }).postMessage({ type: "operitJavaScriptReady" });
