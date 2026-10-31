"use strict";
// Copy this self-contained public client into a dependent package.
Object.defineProperty(exports, "__esModule", { value: true });
exports.getSettings = getSettings;
exports.setSettings = setSettings;
exports.stream = stream;
/** Calls only the provider's explicitly registered public methods. */
function call(method, payload) {
    return ToolPkg.callDependency("com.operit.waifu", method, payload);
}
/** Reads one explicitly named, persisted waifu profile. */
function getSettings(profile = "default") { return call("settings.get", { profile }); }
/** Replaces one explicitly named profile after provider-side validation. */
function setSettings(profile, settings) { return call("settings.set", { profile, settings }); }
/** Adapts a native Chat stream into ordered waifu segments without transferring live objects through IPC. */
async function* stream(request, profile = "default") {
    const session = await call("session.open", { chatId: request.chatId, profile });
    let source;
    try {
        source = Tools.Chat.sendMessageStreaming(request)[Symbol.asyncIterator]();
    }
    catch (error) {
        await call("session.close", { sessionId: session.sessionId });
        throw error;
    }
    const queue = [];
    let wake = null;
    let terminal = false;
    let stopped = false, done = false, failure = null, index = 0;
    let snapshot = "", timestamp = null;
    const delayState = { timer: null, finish: null };
    /** Wakes the sole waiting consumer after a producer state change. */
    function signal() { if (wake !== null) {
        const callback = wake;
        wake = null;
        callback();
    } }
    /** Appends bounded, ordered delivery work while model observation continues independently. */
    async function feed(final) {
        if (timestamp === null)
            return;
        const segments = await call("session.feed", { sessionId: session.sessionId, messageTimestamp: timestamp, snapshot, final });
        if (stopped)
            return;
        if (queue.length + segments.length > 256)
            throw new Error("Waifu delivery queue exceeds 256 segments");
        for (const segment of segments)
            queue.push({ type: "segment", index: index++, segment });
        signal();
    }
    /** Observes semantic snapshots eagerly so typing and network delivery never block model collection. */
    async function produce() {
        let completed = false;
        try {
            while (!stopped) {
                const next = await source.next();
                if (next.done)
                    break;
                if (stopped)
                    break;
                const event = next.value;
                if (event.type === "part") {
                    if (timestamp !== null && timestamp !== event.messageTimestamp)
                        await feed(true);
                    timestamp = event.messageTimestamp;
                    snapshot = event.parts.filter(part => part.kind === "markdown").map(part => part.content).join("\n");
                    await feed(false);
                }
                else {
                    terminal = true;
                    const result = event.result;
                    if (result.outcome.type !== "committed" || result.outcome.status !== "completed")
                        throw new Error(`Waifu Chat did not complete: ${JSON.stringify(result.outcome)}`);
                    await feed(true);
                    queue.push({ type: "completed", result });
                    completed = true;
                    break;
                }
            }
            if (!stopped && !completed)
                throw new Error("Chat stream ended without a completed receipt");
        }
        catch (error) {
            failure = error;
        }
        finally {
            done = true;
            signal();
        }
    }
    const producer = produce();
    try {
        while (!stopped) {
            if (failure !== null)
                throw failure;
            const event = queue.shift();
            if (event === undefined) {
                if (done)
                    break;
                await new Promise(resolve => { wake = resolve; });
                continue;
            }
            if (event.type === "segment" && event.index > 0 && event.segment.kind === "text") {
                const delay = Math.min(3000, Array.from(event.segment.text).length * session.settings.charDelayMs);
                if (delay > 0)
                    await new Promise(resolve => { delayState.finish = resolve; delayState.timer = setTimeout(resolve, delay); });
                delayState.timer = null;
                delayState.finish = null;
                if (failure !== null)
                    throw failure;
            }
            yield event;
        }
    }
    finally {
        stopped = true;
        signal();
        if (delayState.timer !== null)
            clearTimeout(delayState.timer);
        if (delayState.finish !== null)
            delayState.finish();
        try {
            try {
                if (!terminal)
                    await Tools.Chat.cancel(request.chatId);
            }
            finally {
                await source.return?.();
                await producer;
            }
        }
        finally {
            await call("session.close", { sessionId: session.sessionId });
        }
    }
}
