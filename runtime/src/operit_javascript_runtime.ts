import { JavaScriptTransport, javascriptFrame } from "./operit_javascript_transport.js";

/** Describes the synchronous structured callback supplied by the Rust host. */
type JavaScriptHostCallback = (name: string, args: unknown[]) => unknown;

/** Identifies script exceptions without invalidating the browser execution realm. */
class JavaScriptGuestError extends Error {}

interface BrowserJavaScriptHost {
  create(): number;
  postCommand(id: number, command: unknown, buffer: SharedArrayBuffer, control: Int32Array): void;
  dispose(id: number): void;
}

/** Starts execution workers on their browser owner before Core begins synchronous waiting. */
export function createBrowserJavaScriptHost(): {
  create(): Promise<number>;
  postCommand(id: number, command: unknown, buffer: SharedArrayBuffer, control: Int32Array): void;
  dispose(id: number): void;
} {
  const workers = new Map<number, Worker>();
  let nextId = 0;
  return {
    /** Waits for module initialization while the browser owner event loop remains available. */
    async create(): Promise<number> {
      const worker = new Worker(new URL("./operit_javascript_worker.js", import.meta.url), {
        type: "module", name: "operit-plugin-javascript",
      });
      try {
        await new Promise<void>((resolve, reject) => {
          /** Acknowledges only the execution worker's explicit module-ready notification. */
          const ready = (event: MessageEvent): void => {
            if (event.data?.type !== "operitJavaScriptReady") return;
            worker.removeEventListener("message", ready);
            worker.removeEventListener("error", failed);
            resolve();
          };
          /** Reports startup failures to Core instead of starting a synchronous wait. */
          const failed = (event: ErrorEvent): void => reject(new Error(event.message));
          worker.addEventListener("message", ready);
          worker.addEventListener("error", failed, { once: true });
        });
      } catch (error) {
        worker.terminate();
        throw error;
      }
      const id = ++nextId;
      workers.set(id, worker);
      return id;
    },
    /** Sends a command only to an initialized, host-owned execution worker. */
    postCommand(id: number, command: unknown, buffer: SharedArrayBuffer, control: Int32Array): void {
      const worker = workers.get(id);
      if (worker === undefined) throw new Error("JavaScript runtime has been destroyed");
      worker.postMessage({ command, buffer, control });
    },
    /** Releases an execution worker and its host ownership entry. */
    dispose(id: number): void {
      const worker = workers.get(id);
      if (worker !== undefined) {
        worker.terminate();
        workers.delete(id);
      }
    },
  };
}

/** Owns one browser engine realm and services its synchronous host calls while waiting. */
class BrowserJavaScriptRuntime {
  private readonly host: BrowserJavaScriptHost;
  private readonly id: number;
  private disposed = false;

  /** Starts a dedicated engine worker without sharing plugin globals with Core. */
  constructor() {
    this.host = (globalThis as unknown as { __operitHost: { javaScriptRuntime: BrowserJavaScriptHost } }).__operitHost.javaScriptRuntime;
    this.id = this.host.create();
  }

  /** Runs one finite operation and preserves synchronous host callbacks across workers. */
  execute(command: unknown, callback: JavaScriptHostCallback, control: Int32Array): unknown {
    if (this.disposed) throw new Error("JavaScript runtime has been destroyed");
    const transport = JavaScriptTransport.create(control);
    this.host.postCommand(this.id, command, transport.buffer, control);
    try {
      for (;;) {
        const response = transport.receive();
        switch (response.kind) {
          case javascriptFrame.hostCall: {
            const [name, args] = response.value as [string, unknown[]];
            let result: { ok: boolean; value: unknown };
            try {
              result = { ok: true, value: callback(name, args) };
            } catch (error) {
              result = { ok: false, value: String(error) };
            }
            transport.send(javascriptFrame.hostResult, result);
            break;
          }
          case javascriptFrame.result:
            return response.value;
          case javascriptFrame.error:
            throw new JavaScriptGuestError(String(response.value));
          default:
            throw new Error("Unexpected JavaScript worker response");
        }
      }
    } catch (error) {
      if (error instanceof JavaScriptGuestError) throw error;
      // Protocol failures leave the worker ownership indeterminate; release it immediately.
      this.dispose();
      throw error;
    }
  }

  /** Terminates the realm and releases all pending plugin tasks and memory. */
  dispose(): void {
    this.disposed = true;
    this.host.dispose(this.id);
  }
}

/** Creates the browser implementation consumed exclusively by the Web Rust host. */
(globalThis as typeof globalThis & { __operitCreateJavaScriptRuntime: () => BrowserJavaScriptRuntime })
  .__operitCreateJavaScriptRuntime = (): BrowserJavaScriptRuntime => new BrowserJavaScriptRuntime();
