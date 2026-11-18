// Copy this self-contained public client into a dependent package.

export type SendRequest = Parameters<typeof Tools.Chat.sendMessageStreaming>[0];
export type Receipt = Awaited<ReturnType<typeof Tools.Chat.sendMessage>>;
export interface Settings {
  charDelayMs: number;
  removePunctuation: boolean;
  enableEmoticons: boolean;
  enableSelfie: boolean;
  customPrompt: string;
  selfiePrompt: string;
  emojis: Record<string, string[]>;
}
export type Segment = { kind: "text"; text: string } | { kind: "image"; source: string; emotion: string | null };
export type StreamEvent = { type: "segment"; index: number; segment: Segment } | { type: "completed"; result: Receipt };
type NativeEvent = ReturnType<typeof Tools.Chat.sendMessageStreaming> extends AsyncIterable<infer Event> ? Event : never;
export interface OpenRequest { chatId: string; profile: string }
export interface OpenResult { sessionId: string; settings: Settings }
export interface FeedRequest { sessionId: string; messageTimestamp: number; snapshot: string; final: boolean }

/** Calls only the provider's explicitly registered public methods. */
function call<T, R>(method: string, payload: T): Promise<R> {
  return ToolPkg.callDependency<T, R>("com.operit.waifu", method, payload);
}

/** Reports whether Waifu is active for a conversation. */
export function modeStatus(chatId: string): Promise<{ enabled: boolean }> { return call("mode.status", { chatId }); }

/** Reads one explicitly named, persisted waifu profile. */
export function getSettings(profile = "default"): Promise<Settings> { return call("settings.get", { profile }); }

/** Replaces one explicitly named profile after provider-side validation. */
export function setSettings(profile: string, settings: Settings): Promise<Settings> { return call("settings.set", { profile, settings }); }

export interface StreamControls {
  /** Defers creating the model stream until Waifu is ready, or skips a stale turn. */
  openSource?: () => AsyncIterator<NativeEvent> | null;
  /** Wakes the delivery consumer and skips any unsent segments when resolved. */
  stopDelivery?: Promise<void>;
  /** Allows callers that also cancel by chat ID to share their in-flight cancellation. */
  cancelSource?: () => Promise<unknown> | undefined;
}

/** Adapts a native Chat stream into ordered waifu segments without transferring live objects through IPC. */
export async function* stream(request: SendRequest, profile = "default", controls: StreamControls = {}): AsyncGenerator<StreamEvent> {
  const session = await call<OpenRequest, OpenResult>("session.open", { chatId: request.chatId, profile });
  let source: AsyncIterator<NativeEvent> | null;
  try {
    source = controls.openSource === undefined
      ? Tools.Chat.sendMessageStreaming(request)[Symbol.asyncIterator]()
      : controls.openSource();
  }
  catch (error) { await call("session.close", { sessionId: session.sessionId }); throw error; }
  if (source === null) { await call("session.close", { sessionId: session.sessionId }); return; }
  const native = source;
  const queue: StreamEvent[] = [];
  let wake: (() => void) | null = null;
  let terminal = false;
  let stopped = false, done = false, failure: unknown = null, index = 0;
  let snapshot = "", timestamp: number | null = null;
  const delayState: { timer: number | null; finish: (() => void) | null } = { timer: null, finish: null };

  /** Wakes the sole waiting consumer after a producer state change. */
  function signal(): void { if (wake !== null) { const callback = wake; wake = null; callback(); } }

  /** Appends bounded, ordered delivery work while model observation continues independently. */
  async function feed(final: boolean): Promise<void> {
    if (timestamp === null) return;
    const segments = await call<FeedRequest, Segment[]>("session.feed", { sessionId: session.sessionId, messageTimestamp: timestamp, snapshot, final });
    if (stopped) return;
    if (queue.length + segments.length > 256) throw new Error("Waifu delivery queue exceeds 256 segments");
    for (const segment of segments) queue.push({ type: "segment", index: index++, segment });
    signal();
  }

  /** Observes semantic snapshots eagerly so typing and network delivery never block model collection. */
  async function produce(): Promise<void> {
    let completed = false;
    try {
      while (!stopped) {
        const next = await native.next();
        if (next.done) break;
        if (stopped) break;
        const event = next.value;
        if (event.type === "part") {
          if (timestamp !== null && timestamp !== event.messageTimestamp) await feed(true);
          timestamp = event.messageTimestamp;
          snapshot = event.parts.filter(part => part.kind === "markdown").map(part => part.content).join("\n");
          await feed(false);
        } else {
          terminal = true;
          const result = event.result;
          if (result.outcome.type !== "committed" || result.outcome.status !== "completed") throw new Error(`Waifu Chat did not complete: ${JSON.stringify(result.outcome)}`);
          await feed(true);
          queue.push({ type: "completed", result }); completed = true;
          break;
        }
      }
      if (!stopped && !completed) throw new Error("Chat stream ended without a completed receipt");
    } catch (error) { failure = error; }
    finally { done = true; signal(); }
  }

  // Release any typing delay and wake the consumer when the caller supersedes the turn.
  const stop = () => {
    if (stopped) return;
    stopped = true;
    signal();
    if (delayState.timer !== null) clearTimeout(delayState.timer);
    delayState.finish?.();
  };
  void controls.stopDelivery?.then(stop);
  const producer = produce();
  try {
    while (!stopped) {
      if (failure !== null) throw failure;
      const event = queue.shift();
      if (event === undefined) {
        if (done) break;
        await new Promise<void>(resolve => { wake = resolve; });
        continue;
      }
      if (event.type === "segment" && event.index > 0 && event.segment.kind === "text") {
        const delay = Math.min(3000, Array.from(event.segment.text).length * session.settings.charDelayMs);
        if (delay > 0) await new Promise<void>(resolve => { delayState.finish = resolve; delayState.timer = setTimeout(resolve, delay); });
        delayState.timer = null; delayState.finish = null;
        if (stopped) break;
        if (failure !== null) throw failure;
      }
      yield event;
    }
  } finally {
    stop();
    try {
      try { if (!terminal) await (controls.cancelSource?.() ?? Tools.Chat.cancel(request.chatId)); }
      finally { await native.return?.(); await producer; }
    }
    finally { await call("session.close", { sessionId: session.sessionId }); }
  }
}
