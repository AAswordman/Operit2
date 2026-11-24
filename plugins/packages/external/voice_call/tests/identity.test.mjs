import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { loadModule, plain } from "../../../buildin/character_cards/tests/runtime.mjs";

/** Supplies stateful Compose primitives while retaining the real call screen and dependency client. */
function callScreen(resolveIdentity, { floatingWindows, chat, stt, tts, timers, files, groupTurn } = {}) {
  const states = new Map(), refs = new Map(), effects = new Map(), calls = [];
  const routes = [], entries = [];
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
    setInterval, clearInterval,
    ...timers,
    ToolPkg: {
      getSpaceDataDir: () => "/shared/voice_call",
      registerUiRoute: route => routes.push(route),
      registerNavigationEntry: entry => entries.push(entry),
      ui: { floatingWindows: {
        async capabilities() { return {}; },
        async list() { return [{ id: "voice-call" }]; },
        async record() { calls.push("record"); return null; },
        ...floatingWindows,
      } },
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        if (method === "avatar.default") return "data:image/png;base64,character-default-avatar";
        if (method === "chat.groupTurn") return groupTurn ? groupTurn(plain(payload)) : { status: "not_group", replies: [], error: null };
        assert.equal(method, "chat.identity");
        calls.push(plain(payload));
        return resolveIdentity(payload);
      },
    },
    Tools: {
      Files: {
        async mkdir() { return { successful: true }; },
        async copy() { return { successful: true }; },
        async deleteFile() { return { successful: true }; },
        ...files,
      },
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
  return { states, effects, calls, routes, entries, register: plugin.registerToolPkg, renderMenu: () => {
    if (!states.has("presentation")) states.set("presentation", { requestId: "call-presentation", input: null });
    return plugin.screen(ctx);
  }, renderControl: () => plugin.control(ctx), render: () => plugin.call(ctx) };
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
  assert.equal(rendered.find(node => node.type === "MediaStage").props.imageUri, "vfs://shared/avatars/bound.png");
  assert.equal(rendered.find(node => node.type === "MediaStage").props.imageTitle, "Bound caller");
  assert.equal(screen.render().type, "MediaStage");
});

test("explicit MediaStage uses the bound portrait and keeps plugin-owned call controls", async () => {
  const screen = callScreen(({ chatId }) => chatId === null
    ? { title: "Active role", avatarUri: "vfs://shared/avatars/active.png" }
    : { title: "Bound role", avatarUri: "vfs://shared/avatars/bound.png" });
  screen.render();
  screen.effects.get("call-avatar")();
  await new Promise(resolve => setImmediate(resolve));
  const ready = screen.render();
  assert.equal(ready.type, "MediaStage");
  assert.equal(ready.props.camera, true);
  assert.equal(ready.props.imageTitle, "Active role");
  await nodes(ready).find(node => node.type === "FilledTonalButton").props.onClick();
  const call = screen.render();
  assert.equal(call.props.imageUri, "vfs://shared/avatars/bound.png");
  assert.equal(call.props.imageTitle, "Bound role");
  assert.ok(nodes(call).some(node => node.type === "Button"));
  assert.ok(!nodes(call).some(node => node.type === "Image"));
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
  assert.equal(screen.render().props.imageUri, "data:image/png;base64,character-default-avatar");
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
  let microphoneLevel = 0;
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
      async recordingLevel() { return microphoneLevel; },
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
    tick(level) { microphoneLevel = level; for (const callback of timers) callback(); return settle(); },
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

test("loud microphone input cannot interrupt playback and playback keeps the microphone idle", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  assert.equal(screen.requests.length, 1);
  await screen.tick(1);
  await screen.tick(1);
  await screen.tick(1);
  assert.deepEqual(screen.cancelled, []);
  assert.equal(screen.states.get("phase"), "speaking");
  assert.equal(screen.requests.length, 1);
  screen.finishPlayback();
  await screen.settle();
  assert.equal(screen.requests.length, 2);
  assert.equal(screen.states.get("phase"), "listening");
  await screen.hangup();
  await call;
});

test("manual interruption cancels speech and starts a fresh recording", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  await screen.button("Interrupt and speak").props.onClick();
  await screen.settle();
  assert.deepEqual(screen.cancelled, ["call-actual-call-1"]);
  assert.equal(screen.states.get("phase"), "listening");
  assert.equal(screen.requests.length, 2);
  assert.equal(screen.requests[1].echoCancellation, undefined);
  await screen.hangup();
  await call;
});

test("playback failure pauses without opening another microphone capture", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  screen.failPlayback();
  await call;
  assert.equal(screen.requests.length, 1);
  assert.equal(screen.states.get("phase"), "error");
  assert.match(screen.states.get("failure"), /TTS device unavailable/);
});

test("hanging up during playback cancels speech without opening a capture", async () => {
  const screen = playbackScreen();
  const { call } = await screen.startPlayback();
  await screen.hangup();
  await call;
  assert.equal(screen.states.get("phase"), "closing");
  assert.equal(screen.requests.length, 1);
  assert.deepEqual(screen.cancelled, ["call-actual-call-1"]);
});
test("the call sends a camera attachment with its transcribed turn", async () => {
  const frame = { path: "/app/camera.jpg", name: "camera.jpg", mimeType: "image/jpeg", size: 42 };
  const sent = [], copied = [], removed = [];
  const screen = callScreen(() => null, {
    files: {
      async copy(source, destination) { copied.push({source, destination}); return {successful: true}; },
      async deleteFile(path) { removed.push(path); return {successful: true}; },
    },
    floatingWindows: {
      async record() { return sent.length === 0 ? { text: "What am I holding?" } : null; },
      async captureCameraFrame(...args) {
        assert.deepEqual(args, ["voice-call"]);
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
  assert.equal(copied.length, 1);
  assert.equal(copied[0].source, frame.path);
  assert.match(copied[0].destination, /^\/shared\/voice_call\/camera_frames\/frame-.*\.jpg$/);
  assert.deepEqual(removed, [frame.path]);
  assert.deepEqual(sent, [{ text: "What am I holding?", attachments: [{ vfsPath: copied[0].destination, filePath: copied[0].destination, fileName: frame.name, mimeType: frame.mimeType, fileSize: frame.size, content: "" }], replyToMessageTimestamp: null }]);
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


test("one route serves the package launcher and plus menu without presentation state", async () => {
  const opened = [];
  const screen = callScreen(() => null, { floatingWindows: {
    async capabilities() { return { supported: true, permissionGranted: true }; },
    async list() { return []; },
    async open(options) { opened.push(plain(options)); },
  } });
  screen.register();
  assert.equal(screen.routes.length, 1);
  assert.ok(screen.entries.every(entry => entry.route === screen.routes[0].route));
  screen.states.set("presentation", null);
  screen.states.set("chatId", "source-group-chat");
  await nodes(screen.renderMenu()).find(node => node.type === "Button").props.onClick();
  assert.deepEqual(opened, [{ id: "voice-call", route: "toolpkg:com.operit.voice_call:ui:control",
    applicationOnly: false, state: { callSurface: true, cameraControls: true, sourceChatId: "source-group-chat" },
    options: { width: 340, height: 460, draggable: true } }]);
  screen.states.set("callSurface", true);
  assert.equal(screen.renderMenu().type, "MediaStage");
});

test("plus menu reuses the working window instead of creating another session", async () => {
  const actions = [];
  const screen = callScreen(() => null, { floatingWindows: {
    async capabilities() { return { supported: true, permissionGranted: true }; },
    async list() { return [{ id: "voice-call", route: "toolpkg:com.operit.voice_call:ui:control" }]; },
    async show(id) { actions.push(id); },
    async open() { assert.fail("existing call must be reused"); },
  } });
  await nodes(screen.renderMenu()).find(node => node.type === "Button").props.onClick();
  assert.deepEqual(actions, ["voice-call"]);
});

test("launcher reports permission denial and can retry after authorization", async () => {
  let granted = false;
  const opened = [];
  const screen = callScreen(() => null, { floatingWindows: {
    async capabilities() { return { supported: true, permissionGranted: granted }; },
    async list() { return []; },
    async open(options) { opened.push(plain(options)); },
  } });
  await nodes(screen.renderMenu()).find(node => node.type === "Button").props.onClick();
  assert.match(screen.states.get("status"), /Grant permission/);
  assert.equal(opened.length, 0);
  granted = true;
  await nodes(screen.renderMenu()).find(node => node.type === "Button").props.onClick();
  assert.equal(opened.length, 1);
});

test("automatic call inherits the source chat without selecting or changing it", async () => {
  const created = [];
  const screen = callScreen(() => ({ title: "Source role", avatarUri: "role.png" }), { chat: {
    async createNew(options) { created.push(plain(options)); return { chatId: "actual-call" }; },
  } });
  screen.states.set("autoStart", true);
  screen.states.set("sourceChatId", "source-chat");
  screen.render();
  screen.effects.get("call-avatar")();
  screen.effects.get("call-auto-start")();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(created, [{ setAsCurrentChat: false, sourceChatId: "source-chat" }]);
  assert.deepEqual(screen.calls, [{ chatId: "source-chat" }, { chatId: "actual-call" }, "record"]);
});


test("automatic call waits for window ownership and cannot start after disposal", async () => {
  let ready;
  const screen = callScreen(() => null, { floatingWindows: {
    list() { return new Promise(resolve => { ready = resolve; }); },
  } });
  screen.states.set("autoStart", true);
  screen.render();
  const dispose = screen.effects.get("call-auto-start")();
  assert.deepEqual(screen.calls, []);
  dispose();
  ready([{ id: "voice-call" }]);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(screen.calls, []);
});


test("phone menu explicitly requests application scope when the camera requires it", async () => {
  const queries = [], opened = [];
  const screen = callScreen(() => null, { floatingWindows: {
    async capabilities(options) {
      queries.push(plain(options ?? {}));
      return { supported: true, permissionGranted: options?.applicationOnly === true, cameraRequiresApplicationWindow: true };
    },
    async list() { return []; },
    async open(options) { opened.push(plain(options)); },
    async requestPermission() { assert.fail("application cameras do not require system overlay permission"); },
  } });
  await nodes(screen.renderMenu()).find(node => node.type === "Button").props.onClick();
  assert.deepEqual(queries, [{}, { applicationOnly: true }]);
  assert.equal(opened.length, 1);
  assert.equal(opened[0].applicationOnly, true);
});

test("voice overlay declares a media surface without camera controls", () => {
  const screen = callScreen(() => null);
  screen.states.set("cameraControls", false);
  const tree = screen.render();
  assert.equal(tree.type, "MediaStage");
  assert.equal(tree.props.camera, false);
});


test("camera retention failure releases the temporary file and sends the voice turn", async () => {
  const sent = [], removed = [];
  const screen = callScreen(() => null, {
    floatingWindows: {
      async record() { return sent.length === 0 ? { text: "Hello" } : null; },
      async captureCameraFrame() { return { path: "/temp/camera.jpg", name: "camera.jpg", mimeType: "image/jpeg", size: 42 }; },
    },
    files: {
      async copy() { return { successful: false, details: "Storage unavailable" }; },
      async deleteFile(path) { removed.push(path); return { successful: true }; },
    },
    stt: { async transcribe(audio) { return audio; } },
    chat: { async sendMessage(request) { sent.push(plain(request.input)); return { outcome: { type: "completed" }, aiResponse: "Hello", sentAt: 1 }; } },
    tts: { async speak() {} },
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.deepEqual(sent, [{ text: "Hello", attachments: [], replyToMessageTimestamp: null }]);
  assert.equal(removed[0], "/temp/camera.jpg");
  assert.match(removed[1], /^\/shared\/voice_call\/camera_frames\//);
  assert.equal(screen.states.get("cameraNotice"), "Couldn't send the camera frame for this turn.");
});

const groupReplies = [
  { participantId: "bob", name: "Bob", avatarUri: "bob.png", ttsConfigId: "voice-bob", text: "Bob speaks" },
  { participantId: "alice", name: "Alice", avatarUri: "alice.png", ttsConfigId: "voice-alice", text: "Alice speaks" },
];

test("group call speaks ordered member replies with their portraits and configured voices", async () => {
  const spoken = [], submitted = [];
  let recordings = 0;
  const screen = callScreen(() => ({ title: "Friends", avatarUri: "group.png" }), {
    floatingWindows: { async record() { return recordings++ === 0 ? { text: "Hello everyone" } : null; } },
    stt: { async transcribe(audio) { return audio; } },
    groupTurn(request) {
      submitted.push(request);
      assert.equal(request.action, "start");
      return { status: "completed", replies: groupReplies, error: null };
    },
    chat: { async sendMessage() { assert.fail("group input must not also execute as a single response"); } },
    tts: { async speak(request) { spoken.push({ request: plain(request), identity: plain(screen.states.get("identity")) }); } },
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(submitted.length, 1);
  assert.equal(submitted[0].text, "Hello everyone");
  assert.deepEqual(spoken.map(item => [item.request.text, item.request.configId, item.identity.title]), [
    ["Bob speaks", "voice-bob", "Bob"], ["Alice speaks", "voice-alice", "Alice"],
  ]);
  assert.deepEqual(spoken.map(item => item.identity.avatarUri), ["bob.png", "alice.png"]);
  assert.equal(screen.states.get("identity").title, "Friends");
});

test("manual group interruption cancels pending members and returns to listening", async () => {
  const spoken = [], controls = [];
  let recordings = 0, finishSpeech, releaseRecording;
  const screen = callScreen(() => null, {
    floatingWindows: {
      record() { return recordings++ === 0 ? Promise.resolve({ text: "Hi" }) : new Promise(resolve => { releaseRecording = resolve; }); },
      async cancelRecording() { releaseRecording(null); }, async close() {},
    },
    stt: { async transcribe(audio) { return audio; } },
    groupTurn(request) {
      controls.push(request.action);
      return { status: request.action === "cancel" ? "cancelled" : "running", replies: groupReplies, error: null };
    },
    tts: {
      speak(request) { spoken.push(request.text); return new Promise(resolve => { finishSpeech = resolve; }); },
      async cancel() { finishSpeech(); return true; },
    },
  });
  const call = nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(screen.states.get("phase"), "speaking");
  assert.equal(recordings, 1);
  await nodes(screen.render()).find(node => node.props.text === "Interrupt and speak").props.onClick();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(spoken, ["Bob speaks"]);
  assert.deepEqual(controls, ["start", "cancel"]);
  assert.equal(screen.states.get("phase"), "listening");
  await nodes(screen.render()).find(node => node.type === "Button").props.onClick();
  await call;
});

test("group generation failure pauses the call with the provider error", async () => {
  const screen = callScreen(() => null, {
    floatingWindows: { async record() { return { text: "Hi" }; } },
    stt: { async transcribe(audio) { return audio; } },
    groupTurn() { return { status: "failed", replies: [], error: "Group service unavailable" }; },
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(screen.states.get("phase"), "error");
  assert.match(screen.states.get("failure"), /Group service unavailable/);
});
