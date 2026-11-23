import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { loadModule, plain } from "../../../buildin/character_cards/tests/runtime.mjs";

/** Supplies stateful Compose primitives while retaining the real call screen and dependency client. */
function callScreen(resolveIdentity, { floatingWindows, chat, stt, tts, timers } = {}) {
  const states = new Map(), refs = new Map(), effects = new Map(), calls = [];
  const color = { copy() { return color; } };
  const ctx = {
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), value => states.set(key, value)];
    },
    useRef(key, initial) {
      if (!refs.has(key)) refs.set(key, { current: initial });
      return refs.get(key);
    },
    useEffect(key, callback) { effects.set(key, callback); },
    UI: new Proxy({}, { get: (_, type) => (props, children = []) => ({ type, props, children }) }),
    Modifier: { clip: shape => shape },
    MaterialTheme: { colorScheme: new Proxy({}, { get: () => color }) },
  };
  const plugin = loadModule("../../external/voice_call/src/main.ts", {
    getLang: () => "en",
    ...timers,
    ToolPkg: {
      ui: { floatingWindows: {
        async capabilities() { return {}; },
        async record() { calls.push("record"); return null; },
        ...floatingWindows,
      } },
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        assert.equal(method, "chat.identity");
        calls.push(plain(payload));
        return resolveIdentity(payload);
      },
    },
    Tools: {
      Chat: {
        async createNew(options) {
          assert.deepEqual(plain(options), { setAsCurrentChat: false });
          calls.push("create");
          return { chatId: "actual-call" };
        },
        ...chat,
      },
      Stt: stt,
      Tts: tts,
    },
  });
  return { states, effects, calls, render: () => plugin.call(ctx) };
}

/** Traverses actual Compose output to inspect rendered identity and invoke its start control. */
function nodes(node) { return [node, ...node.children.flatMap(nodes)]; }

/** Exercises the real screen's preview race and confirms recording uses the created chat identity. */
test("call renders the bound custom role and rejects a late active-role preview", async () => {
  let finishPreview;
  const screen = callScreen(({ chatId }) => chatId === null
    ? new Promise(resolve => { finishPreview = resolve; })
    : { title: "Bound caller", avatarUri: "vfs://shared/avatars/bound.png" });
  const initial = screen.render();
  screen.effects.get("call-avatar")();
  await nodes(initial).find(node => node.type === "FilledTonalButton").props.onClick();
  finishPreview({ title: "Different active role", avatarUri: "vfs://shared/avatars/other.png" });
  await Promise.resolve();
  assert.deepEqual(screen.calls, [{ chatId: null }, "create", { chatId: "actual-call" }, "record"]);
  const rendered = nodes(screen.render());
  assert.equal(rendered.find(node => node.type === "Image").props.path, "vfs://shared/avatars/bound.png");
  assert.equal(rendered.find(node => node.type === "Image").props.contentDescription, "Bound caller");
  assert.ok(rendered.some(node => node.type === "Text" && node.props.text === "Bound caller"));
});

/** A failed identity read must pause before recording and preserve its actual error. */
test("call pauses on identity failure and rereads the same chat when resumed", async () => {
  let rejectRead = true;
  const screen = callScreen(() => {
    if (rejectRead) throw new Error("Role storage unavailable");
    return { title: "Restored caller", avatarUri: null };
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(screen.states.get("phase"), "error");
  assert.match(screen.states.get("failure"), /Role storage unavailable/);
  assert.ok(!screen.calls.includes("record"));
  rejectRead = false;
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(screen.calls.filter(call => call === "create").length, 1);
  assert.deepEqual(screen.calls, ["create", { chatId: "actual-call" }, { chatId: "actual-call" }, "record"]);
  assert.ok(!nodes(screen.render()).some(node => node.type === "Image"));
});

/** Locks the copied public contract and minimum provider version to the feature actually consumed. */
test("voice call ships the current dependency client and matching version requirement", () => {
  const client = readFileSync(new URL("../src/dependencies/character_cards/api.ts", import.meta.url), "utf8");
  assert.equal(client, readFileSync(new URL("../../../buildin/character_cards/src/api.ts", import.meta.url), "utf8"));
  const provider = JSON.parse(readFileSync(new URL("../../../buildin/character_cards/manifest.json", import.meta.url), "utf8"));
  const consumer = JSON.parse(readFileSync(new URL("../manifest.json", import.meta.url), "utf8"));
  assert.equal(consumer.requires.find(item => item.id === provider.toolpkg_id).min_version, provider.version);
});


function playbackScreen({ echoCancellation = true } = {}) {
  const timers = new Set();
  const requests = [], captures = [], transcribed = [], chatInputs = [], cancelled = [];
  let hasSpeech = false;
  let settledPlayback;
  const screen = callScreen(() => null, {
    timers: {
      setInterval(callback) { timers.add(callback); return callback; },
      clearInterval(callback) { timers.delete(callback); },
    },
    floatingWindows: {
      async capabilities() { return { recordingEchoCancellation: echoCancellation }; },
      record(_id, options) {
        requests.push(plain(options));
        return new Promise(resolve => captures.push(resolve));
      },
      async recordingHasSpeech() { return hasSpeech; },
      async cancelRecording() { captures.at(-1)(null); },
      async close() {},
    },
    stt: { async transcribe(audio) { transcribed.push(audio); return { text: audio.text }; } },
    chat: { async sendMessage(request) {
      chatInputs.push(request.input.text);
      if (chatInputs.length > 1) return new Promise(() => {});
      return { outcome: { type: "completed" }, aiResponse: "A long reply", sentAt: 1 };
    }, async cancel() {} },
    tts: {
      speak() { return new Promise((resolve, reject) => { settledPlayback = { resolve, reject }; }); },
      async cancel(id) {
        cancelled.push(id);
        settledPlayback.reject(new Error("TTS playback was cancelled"));
        return true;
      },
    },
  });
  const button = label => nodes(screen.render()).find(node => node.type === "FilledTonalButton" && node.props.text === label);
  const hangup = () => nodes(screen.render()).find(node => node.type === "Button").props.onClick();
  const settle = () => new Promise(resolve => setImmediate(resolve));
  return {
    ...screen, requests, captures, transcribed, chatInputs, cancelled, button, hangup, settle,
    tick(speech) { hasSpeech = speech; for (const callback of timers) callback(); return settle(); },
    finishPlayback() { settledPlayback.resolve(); },
    failPlayback() { settledPlayback.reject(new Error("TTS device unavailable")); },
    async startPlayback() {
      const call = button("Start call").props.onClick();
      await settle();
      captures[0]({ text: "Question" });
      await settle();
      assert.equal(screen.states.get("phase"), "speaking");
      return { call };
    },
  };
}

test("speech during playback cancels the utterance and transcribes the same capture", async () => {
  const screen = playbackScreen();
  await screen.startPlayback();
  assert.equal(screen.requests[1].echoCancellation, true);
  await screen.tick(false);
  assert.deepEqual(screen.cancelled, []);
  screen.captures[1](null); // The bounded capture expires during a long reply.
  await screen.settle();
  assert.equal(screen.requests[2].echoCancellation, true);
  await screen.tick(true);
  assert.deepEqual(screen.cancelled, ["call-actual-call-1"]);
  assert.equal(screen.states.get("phase"), "listening");
  const audio = { text: "Stop, I meant next week" };
  screen.captures[2](audio);
  await screen.settle();
  assert.equal(screen.transcribed[1], audio);
  assert.deepEqual(screen.chatInputs, ["Question", "Stop, I meant next week"]);
});

test("natural playback completion keeps the open microphone without cancelling playback", async () => {
  const screen = playbackScreen();
  await screen.startPlayback();
  screen.finishPlayback();
  await screen.settle();
  assert.equal(screen.states.get("phase"), "listening");
  screen.captures[1]({ text: "Follow-up" });
  await screen.settle();
  assert.deepEqual(screen.cancelled, []);
  assert.deepEqual(screen.chatInputs, ["Question", "Follow-up"]);
  assert.equal(screen.requests.length, 2);
});

test("manual interruption resumes recording on hosts without echo cancellation", async () => {
  const screen = playbackScreen({ echoCancellation: false });
  const { call } = await screen.startPlayback();
  assert.equal(screen.requests.length, 1);
  await screen.button("Interrupt and speak").props.onClick();
  await screen.settle();
  assert.equal(screen.states.get("phase"), "listening");
  assert.deepEqual(screen.cancelled, ["call-actual-call-1"]);
  assert.equal(screen.requests[1].echoCancellation, undefined);
  screen.captures[1](null);
  await call;
  assert.equal(screen.states.get("phase"), "idle");
});

test("playback failure releases the open microphone and pauses the call", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  const errorLog = console.error;
  try {
    console.error = () => {};
    screen.failPlayback();
    await call;
  } finally {
    console.error = errorLog;
  }
  assert.equal(screen.requests.length, 2);
  assert.equal(screen.states.get("phase"), "error");
  assert.match(screen.states.get("failure"), /TTS device unavailable/);
});

test("hanging up during playback cancels both the utterance and its capture", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  await screen.hangup();
  await call;
  assert.equal(screen.states.get("phase"), "closing");
  assert.equal(screen.requests.length, 2);
  assert.deepEqual(screen.cancelled, ["call-actual-call-1"]);
  assert.deepEqual(screen.chatInputs, ["Question"]);
});
test("the call sends a camera attachment with its transcribed turn", async () => {
  const frame = { filePath: "/app/camera.jpg", fileName: "camera.jpg", mimeType: "image/jpeg", fileSize: 42, content: "" };
  const sent = [];
  const screen = callScreen(() => null, {
    floatingWindows: {
      async record() { return sent.length === 0 ? { text: "What am I holding?" } : null; },
      async captureCameraFrame(id, chatId) {
        assert.equal(id, "voice-call");
        assert.equal(chatId, "actual-call");
        return frame;
      },
    },
    stt: { async transcribe(audio) { return audio; } },
    chat: { async sendMessage(request) {
      sent.push(plain(request.input));
      return { outcome: { type: "completed" }, aiResponse: "A mug", sentAt: 1 };
    } },
    tts: { async speak() {} },
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.deepEqual(sent, [{ text: "What am I holding?", attachments: [frame], replyToMessageTimestamp: null }]);
});

test("when taking a camera frame fails, the voice turn still sends", async () => {
  const sent = [];
  const screen = callScreen(() => null, {
    floatingWindows: {
      async record() { return sent.length === 0 ? { text: "Hello" } : null; },
      async captureCameraFrame() { throw new Error("Camera unavailable"); },
    },
    stt: { async transcribe(audio) { return audio; } },
    chat: { async sendMessage(request) {
      sent.push(plain(request.input));
      return { outcome: { type: "completed" }, aiResponse: "Hello", sentAt: 1 };
    } },
    tts: { async speak() {} },
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.deepEqual(sent, [{ text: "Hello", attachments: [], replyToMessageTimestamp: null }]);
  assert.equal(screen.states.get("cameraNotice"), "Couldn't send the camera frame for this turn.");
});
