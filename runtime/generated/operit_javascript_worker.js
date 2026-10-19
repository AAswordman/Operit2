import { JavaScriptTransport, javascriptFrame } from "./operit_javascript_transport.js";
const nativeEval = eval;
const NativeFunction = Function;
const NativeString = String;
const nativeApply = Reflect.apply;
const defer = setTimeout.bind(globalThis);
const define = Object.defineProperty;
const hasOwn = Object.prototype.hasOwnProperty;
let activeTransport = null;
let registry = null;
function checkActiveTransport() {
    if (activeTransport === null)
        throw new Error("JavaScript host call has no active execution");
}
function callHost(name, args) {
    checkActiveTransport();
    const transport = activeTransport;
    transport.send(javascriptFrame.hostCall, [name, args], checkActiveTransport);
    const response = transport.receive(checkActiveTransport);
    if (response.kind !== javascriptFrame.hostResult)
        throw new Error("Expected JavaScript host result");
    const result = response.value;
    if (!result.ok)
        throw new Error(NativeString(result.value));
    return result.value;
}
function execute(command) {
    switch (command.operation) {
        case "initialize":
            nativeApply(nativeEval, undefined, [command.source]);
            registry = globalThis.__operitHostPromiseRegistry;
            return null;
        case "evaluateVoid":
            nativeApply(nativeEval, undefined, [command.source + "\n//# sourceURL=" + command.name]);
            return null;
        case "evaluateString":
            return NativeString(nativeApply(nativeEval, undefined, [command.source + "\n//# sourceURL=" + command.name]));
        case "constructor": {
            const parameters = command.parameters;
            const constructor = (body) => new NativeFunction(...parameters, body);
            define(globalThis, command.name, { value: constructor, writable: true, configurable: true });
            return null;
        }
        case "binding": {
            const name = command.name;
            const submit = (...args) => callHost(name, args);
            const binding = nativeApply(registry[command.factory], registry, [submit]);
            define(globalThis, name, { value: binding, writable: true, configurable: true });
            return null;
        }
        case "call": {
            if (!nativeApply(hasOwn, globalThis, [command.name])) {
                throw new Error("JavaScript lifecycle function is unavailable: " + command.name);
            }
            const result = nativeApply(globalThis[command.name], undefined, command.args);
            return nativeApply(registry.snapshotResult, registry, [result]);
        }
        case "settle":
            nativeApply(registry.settle, registry, command.args);
            return null;
        case "cancel":
            nativeApply(registry.cancel, registry, command.args);
            return null;
        case "jobs":
            return null;
        default:
            throw new Error("Unknown JavaScript runtime operation: " + command.operation);
    }
}
globalThis.addEventListener("message", (event) => {
    const transport = new JavaScriptTransport(event.data.buffer);
    activeTransport = transport;
    let value;
    let failed = false;
    try {
        value = execute(event.data.command);
    }
    catch (error) {
        failed = true;
        value = error instanceof Error ? error.stack : NativeString(error);
    }
    defer(() => {
        transport.send(failed ? javascriptFrame.error : javascriptFrame.result, value, checkActiveTransport);
        activeTransport = null;
    }, 0);
});
globalThis.postMessage({ type: "operitJavaScriptReady" });
