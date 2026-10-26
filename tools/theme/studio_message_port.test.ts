// Node's TypeScript stripping runs these protocol tests without a browser.
// No rendering/parity claims are made by this fake transport.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";
import { ThemeStudioMessagePort, exportWithContract } from "./studio_message_port.ts";
import type { ThemeStudioMessage, ClientShellReceipt, ThemeContractBridge, PreviewViewport } from "./studio_message_port.ts";

class FakePort {
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onmessageerror: (() => void) | null = null;
  sent: unknown[] = [];
  closed = false;
  start(): void {}
  close(): void { this.closed = true; }
  postMessage(data: unknown): void { this.sent.push(data); }
}
class FakeChannel {
  static latest: FakeChannel;
  port1 = new FakePort();
  port2 = new FakePort();
  constructor() { FakeChannel.latest = this; }
}

const header = { protocolVersion: 1, nonce: "0123456789abcdef", rendererReleaseId: "test-release" };
const fixtureVersion = "client-shell-v7";
const themeJson = JSON.stringify({ typography: { global: "noto-sans-sc", user: null, ai: null } });
const receipt = (revision: number, brightness: "light" | "dark" = "light", viewport: PreviewViewport = { width: 1280, height: 800 }, showConversation = false): ClientShellReceipt => ({
  scope: "client-shell", parityVerified: false, revision, fixtureVersion, showConversation, fixtureSourceSha256: "b".repeat(64),
  themeSha256: "a".repeat(64), brightness, viewport, assetSha256: {},
  fonts: ["noto-sans-sc", "cascadia-mono", "noto-sans-sc", "noto-sans-sc"].map(reference => ({
    reference, sha256: "c".repeat(64), family: "OperitThemeFont_" + "c".repeat(64), bytes: 100,
  })),
});

function harness(run: (context: {
  channel: ThemeStudioMessagePort; messages: ThemeStudioMessage[];
  port: FakePort; advertise: (overrides?: Record<string, unknown>) => void;
  reply: (data: Record<string, unknown>) => void; transfers: unknown[];
}) => void | Promise<void>): void | Promise<void> {
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const originalChannel = Object.getOwnPropertyDescriptor(globalThis, "MessageChannel");
  const listeners = new Set<(event: Record<string, unknown>) => void>();
  const fakeWindow = {
    location: { origin: "https://studio.test" },
    addEventListener: (_: string, handler: (event: Record<string, unknown>) => void) => listeners.add(handler),
    removeEventListener: (_: string, handler: (event: Record<string, unknown>) => void) => listeners.delete(handler),
  };
  Object.defineProperty(globalThis, "window", { configurable: true, value: fakeWindow });
  Object.defineProperty(globalThis, "MessageChannel", { configurable: true, value: FakeChannel });
  const transfers: unknown[] = [];
  const contentWindow = { postMessage: (...args: unknown[]) => transfers.push(args) };
  const iframe = { src: "", contentWindow };
  const channel = new ThemeStudioMessagePort({
    iframe: iframe as unknown as HTMLIFrameElement, previewUrl: "https://preview.test/renderer/",
    nonce: header.nonce, rendererReleaseId: header.rendererReleaseId, fixtureVersion,
    fixtureSourceSha256: "b".repeat(64),
  });
  const messages: ThemeStudioMessage[] = [];
  const cleanup = () => {
    channel.close();
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
    if (originalChannel) Object.defineProperty(globalThis, "MessageChannel", originalChannel);
    else Reflect.deleteProperty(globalThis, "MessageChannel");
  };
  try {
    channel.connect(message => messages.push(message));
    assert.equal(new URL(iframe.src).searchParams.get("studioOrigin"), fakeWindow.location.origin);
    const port = FakeChannel.latest.port1;
    const advertise = (overrides: Record<string, unknown> = {}) => {
      const event = { origin: "https://preview.test", source: contentWindow,
        data: { type: "operit.theme.preview.available.v1", ...header }, ...overrides };
      for (const listener of [...listeners]) listener(event);
    };
    const result = run({ channel, port, messages, advertise, transfers,
      reply: data => port.onmessage?.({ data: { ...header, ...data } }),
    });
    if (result instanceof Promise) return result.finally(cleanup);
    cleanup();
  } catch (error) { cleanup(); throw error; }
}

test("origin/source/nonce must match before transferring exactly one port", () => harness(context => {
  context.advertise({ origin: "https://attacker.test" });
  context.advertise({ source: {} });
  context.advertise({ data: { type: "operit.theme.preview.available.v1", ...header, nonce: "wrong" } });
  assert.equal(context.transfers.length, 0);
  assert.throws(() => context.channel.requestReceipt());
  context.advertise(); context.advertise();
  assert.equal(context.transfers.length, 1);
  context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  assert.equal(context.messages.length, 1);
  assert.throws(() => context.channel.connect(() => {}));
}));

test("loaded is not rendered; only a matching current frame can create evidence", () => harness(context => {
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], showConversation: false });
  context.reply({ type: "operit.theme.preview.loaded.v1", revision: 2, themeSha256: "a".repeat(64) });
  assert.equal(context.channel.receipt, null);
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2) });
  assert.equal(context.channel.receipt?.revision, 2);
  assert.equal(context.channel.receipt?.parityVerified, false);
  const copy = context.channel.receipt!; copy.themeSha256 = "changed";
  assert.equal(context.channel.receipt?.themeSha256, "a".repeat(64));
  context.channel.loadTheme({ revision: 3, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], showConversation: false });
  assert.equal(context.channel.receipt, null);
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2) });
  assert.equal(context.channel.receipt, null);
  context.reply({ type: "operit.theme.preview.error.v1", revision: 3, code: "theme-load-invalid", message: "failed media" });
  assert.equal(context.channel.receipt, null);
}));

test("scene mode invalidates old evidence and binds the next receipt and PNG", async () => harness(async context => {
  context.advertise();
  context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({
    revision: 2,
    canonicalThemeJson: themeJson,
    themeSha256: "a".repeat(64),
    assets: [],
    showConversation: false,
  });
  context.reply({
    type: "operit.theme.preview.rendered.v1",
    revision: 2,
    fixtureVersion,
    receipt: receipt(2),
  });

  context.channel.setConversationMode({ revision: 2, showConversation: true });
  assert.equal(context.channel.receipt, null);
  assert.equal((context.port.sent.at(-1) as { type: string }).type, "operit.theme.preview.sceneMode.v1");
  context.reply({
    type: "operit.theme.preview.rendered.v1",
    revision: 2,
    fixtureVersion,
    receipt: receipt(2, "light", { width: 1280, height: 800 }, true),
  });
  assert.equal(context.channel.receipt?.showConversation, true);

  const pending = context.channel.capturePng();
  const request = context.port.sent.at(-1) as { showConversation?: boolean };
  assert.equal(request.showConversation, true);
  const png = new Uint8Array(33);
  png.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(png.buffer);
  view.setUint32(16, 1280);
  view.setUint32(20, 800);
  context.reply({
    type: "operit.theme.preview.captured.v1",
    revision: 2,
    fixtureVersion,
    showConversation: true,
    viewport: { width: 1280, height: 800 },
    themeSha256: "a".repeat(64),
    pngSha256: createHash("sha256").update(png).digest("hex"),
    pngBytes: png,
  });
  assert.deepEqual(await pending, png);
}));

test("a later frame error revokes success for the same revision", () => harness(context => {
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], showConversation: false });
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2) });
  assert.ok(context.channel.receipt);
  context.reply({ type: "operit.theme.preview.error.v1", revision: 2, code: "preview-frame-failed", message: "shader failed" });
  assert.equal(context.channel.receipt, null);
}));

test("a captured PNG must match the current rendered revision and digest", async () => harness(async context => {
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], showConversation: false });
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2) });
  const pending = context.channel.capturePng();
  const png = new Uint8Array(33);
  png.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(png.buffer); view.setUint32(16, 1280); view.setUint32(20, 800);
  context.reply({ type: "operit.theme.preview.captured.v1", revision: 2, fixtureVersion, showConversation: false, viewport: { width: 1280, height: 800 },
    themeSha256: "a".repeat(64), pngSha256: createHash("sha256").update(png).digest("hex"), pngBytes: png });
  assert.deepEqual(await pending, png);
}));

test("the selected viewport is bound to the rendered receipt and PNG", async () => harness(async context => {
  const viewport = { width: 390, height: 844 };
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], viewport, showConversation: false });
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2, "light", viewport) });
  const pending = context.channel.capturePng();
  const png = new Uint8Array(33);
  png.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(png.buffer); view.setUint32(16, viewport.width); view.setUint32(20, viewport.height);
  context.reply({ type: "operit.theme.preview.captured.v1", revision: 2, fixtureVersion, showConversation: false,
    viewport, themeSha256: "a".repeat(64), pngSha256: createHash("sha256").update(png).digest("hex"), pngBytes: png });
  assert.deepEqual(await pending, png);
}));

test("missing or mismatched viewport evidence is rejected", () => harness(context => {
  const viewport = { width: 390, height: 844 };
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  assert.throws(() => context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson,
    themeSha256: "a".repeat(64), assets: [], viewport: { width: 0, height: 844 }, showConversation: false }));
  context.channel.loadTheme({ revision: 3, canonicalThemeJson: themeJson,
    themeSha256: "a".repeat(64), assets: [], viewport, showConversation: false });
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 3, fixtureVersion,
    receipt: receipt(3, "light", { width: 1280, height: 800 }) });
  assert.equal(context.channel.receipt, null);
  assert.ok(context.port.closed);
}));

test("dark-mode frames are capturable for the visual matrix", async () => harness(async context => {
  context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], brightness: "dark", showConversation: false });
  context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2, "dark") });
  const pending = context.channel.capturePng();
  const png = new Uint8Array(33);
  png.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(png.buffer); view.setUint32(16, 1280); view.setUint32(20, 800);
  context.reply({ type: "operit.theme.preview.captured.v1", revision: 2, fixtureVersion, showConversation: false, viewport: { width: 1280, height: 800 },
    themeSha256: "a".repeat(64), pngSha256: createHash("sha256").update(png).digest("hex"), pngBytes: png });
  assert.deepEqual(await pending, png);
}));

test("Inspector selection is revision-bound and cannot be captured into a preview PNG", () => harness(context => {
  context.advertise();
  context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
  context.channel.loadTheme({
    revision: 2,
    canonicalThemeJson: themeJson,
    themeSha256: "a".repeat(64),
    assets: [],
    showConversation: false,
  });
  context.reply({
    type: "operit.theme.preview.rendered.v1",
    revision: 2,
    fixtureVersion,
    receipt: receipt(2),
  });
  context.channel.setInspectMode({ revision: 2, enabled: true });
  assert.throws(() => context.channel.capturePng(), /selection mode/);
  assert.throws(() => context.channel.setInspectMode({
    revision: 2,
    enabled: true,
    targetId: "window.eval",
  }), /target ID/);
  assert.throws(() => context.channel.setInspectMode({
    revision: 1,
    enabled: false,
  }), /current rendered revision/);
  context.reply({
    type: "operit.theme.preview.selection.v1",
    revision: 2,
    enabled: true,
    targetId: "chat.userMessage",
  });
  assert.equal(context.messages.at(-1)?.type, "operit.theme.preview.selection.v1");
  assert.equal((context.messages.at(-1) as { targetId?: string }).targetId, "chat.userMessage");
}));

test("wrong release/fixture/theme/asset/font evidence fails closed", () => {
  for (const change of [
    { rendererReleaseId: "another-release" }, { protocolVersion: 2 }, { fixtureVersion: "another-fixture" },
    { receipt: { ...receipt(2), themeSha256: "d".repeat(64) } },
    { receipt: { ...receipt(2), brightness: "dark" } },
    { receipt: { ...receipt(2), assetSha256: { "assets/images/extra.png": "d".repeat(64) } } },
    { receipt: { ...receipt(2), fonts: [] } },
    { receipt: { ...receipt(2), parityVerified: undefined } },
    { receipt: { ...receipt(2), showConversation: true } },
    { receipt: { ...receipt(2), fixtureSourceSha256: "d".repeat(64) } },
  ]) harness(context => {
    context.advertise(); context.reply({ type: "operit.theme.preview.ready.v1", nextRevision: 2 });
    context.channel.loadTheme({ revision: 2, canonicalThemeJson: themeJson, themeSha256: "a".repeat(64), assets: [], showConversation: false });
    context.reply({ type: "operit.theme.preview.rendered.v1", revision: 2, fixtureVersion, receipt: receipt(2), ...change });
    assert.equal(context.channel.receipt, null);
    assert.ok(context.port.closed);
    assert.equal(context.messages.at(-1)?.type, "operit.theme.preview.error.v1");
  });
});

test("raw export delegates binary bytes to the contract, never to browser ZIP code", async () => {
  let exports = 0;
  const bridge: ThemeContractBridge = {
    normalizeDraft: async () => JSON.stringify({ ok: true }),
    validatePreview: async () => { throw new Error("not a render operation"); },
    exportPackage: async input => { exports++; assert.deepEqual(input.previewPng, new Uint8Array([1, 2])); return new Uint8Array([80, 75]); },
  };
  const input = { metadataJson: new Uint8Array(), themeJson: new Uint8Array(), previewPng: new Uint8Array([1, 2]), assets: [] };
  assert.deepEqual(await exportWithContract(bridge, input), new Uint8Array([80, 75]));
  bridge.normalizeDraft = async () => JSON.stringify({ ok: false, errors: [{ code: "theme-schema-invalid" }] });
  await assert.rejects(exportWithContract(bridge, input));
  assert.equal(exports, 1);
});
