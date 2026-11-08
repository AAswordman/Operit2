import { JavaScriptTransport, javascriptFrame } from "./operit_javascript_transport.js";
class JavaScriptGuestError extends Error {
}
export function createBrowserJavaScriptHost() {
    const workers = new Map();
    let nextId = 0;
    return {
        async create() {
            const worker = new Worker(new URL("./operit_javascript_worker.js", import.meta.url), {
                type: "module", name: "operit-plugin-javascript",
            });
            try {
                await new Promise((resolve, reject) => {
                    const ready = (event) => {
                        if (event.data?.type !== "operitJavaScriptReady")
                            return;
                        worker.removeEventListener("message", ready);
                        worker.removeEventListener("error", failed);
                        resolve();
                    };
                    const failed = (event) => reject(new Error(event.message));
                    worker.addEventListener("message", ready);
                    worker.addEventListener("error", failed, { once: true });
                });
            }
            catch (error) {
                worker.terminate();
                throw error;
            }
            const id = ++nextId;
            workers.set(id, worker);
            return id;
        },
        postCommand(id, command, buffer, control) {
            const worker = workers.get(id);
            if (worker === undefined)
                throw new Error("JavaScript runtime has been destroyed");
            worker.postMessage({ command, buffer, control });
        },
        dispose(id) {
            const worker = workers.get(id);
            if (worker !== undefined) {
                worker.terminate();
                workers.delete(id);
            }
        },
    };
}
class BrowserJavaScriptRuntime {
    host;
    id;
    disposed = false;
    constructor() {
        this.host = globalThis.__operitHost.javaScriptRuntime;
        this.id = this.host.create();
    }
    execute(command, callback, control) {
        if (this.disposed)
            throw new Error("JavaScript runtime has been destroyed");
        const transport = JavaScriptTransport.create(control);
        this.host.postCommand(this.id, command, transport.buffer, control);
        try {
            for (;;) {
                const response = transport.receive();
                switch (response.kind) {
                    case javascriptFrame.hostCall: {
                        const [name, args] = response.value;
                        let result;
                        try {
                            result = { ok: true, value: callback(name, args) };
                        }
                        catch (error) {
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
        }
        catch (error) {
            if (error instanceof JavaScriptGuestError)
                throw error;
            this.dispose();
            throw error;
        }
    }
    dispose() {
        this.disposed = true;
        this.host.dispose(this.id);
    }
}
globalThis
    .__operitCreateJavaScriptRuntime = () => new BrowserJavaScriptRuntime();
