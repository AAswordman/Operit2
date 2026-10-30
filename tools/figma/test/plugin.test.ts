import { test } from "node:test";
import assert from "node:assert/strict";
import fixture from "../../../apps/flutter/packages/operit_ui/assets/fixtures/theme-v1.json" with { type: "json" };
import { clone, type DesignDocument, type Theme } from "../src/bridge.ts";
import { themeDocument } from "../src/theme_scene.ts";
import { createFigmaMock } from "./figma_mock.ts";
const host = createFigmaMock();
(globalThis as any).figma = host.api;
(globalThis as any).__html__ = "";
await import("../src/plugin.ts");
async function send(message: any) {
  host.messages.length = 0;
  await host.api.ui.onmessage(message);
  const error = host.messages.find((m) => m.type === "error");
  if (error) throw new Error(error.message);
  return host.messages;
}
async function load(doc = themeDocument(clone(fixture) as Theme, [])) {
  await send({ type: "import", text: JSON.stringify(doc) });
  return host.api.currentPage.selection[0];
}
async function output(): Promise<DesignDocument> {
  const messages = await send({ type: "export" });
  return JSON.parse(messages.find((m) => m.type === "download").text);
}
test("plugin imports editable components, persists source, exports no false edits and restores theme", async () => {
  const root = await load();
  assert.ok(root.findAll((n: any) => n.type === "COMPONENT").length >= 8);
  const doc = await output();
  assert.deepEqual(doc.theme, fixture);
  assert.equal(doc.changes?.length, 0);
});
test("variables and direct component colors round trip to schema paths", async () => {
  const root = await load();
  const pairs = JSON.parse(root.getPluginData("operit.variables"));
  const primary = host.variables.get(
    pairs.find(([p]: string[]) => p === "palette.dark.primary")[1],
  );
  primary.setValueForMode("default", { r: 1, g: 0, b: 0, a: 0.5 });
  const bubble = root.findAll((n: any) => n.name === "用户气泡")[0];
  bubble.fills = [{ type: "SOLID", color: { r: 0, g: 1, b: 0 }, opacity: 1 }];
  const doc = await output();
  assert.equal((doc.theme!.palette as any).dark.primary, "#FF000080");
  assert.equal((doc.theme!.chat as any).user.color, "#00FF00FF");
});
test("conflicting global theme edits fail instead of arbitrarily choosing a viewport", async () => {
  const root = await load();
  const bubbles = root.findAll((n: any) => n.name === "用户气泡");
  bubbles[0].fills = [{ type: "SOLID", color: { r: 0, g: 1, b: 0 } }];
  bubbles[1].fills = [{ type: "SOLID", color: { r: 1, g: 0, b: 0 } }];
  await assert.rejects(output, /不同值/);
});
test("deep tool/plugin text edits, additions and removals retain owners in exported handoff", async () => {
  const doc: DesignDocument = {
    format: "operit-design",
    version: 1,
    name: "Deep UI",
    origin: "flutter-runtime",
    assets: [],
    warnings: [],
    scenes: [
      {
        id: "plugin",
        name: "Plugin",
        type: "frame",
        x: 0,
        y: 0,
        width: 400,
        height: 300,
        children: [
          {
            id: "tool",
            name: "Tool result",
            type: "text",
            x: 10,
            y: 20,
            width: 300,
            height: 30,
            text: "before",
            fontFamily: "Inter",
            source: {
              kind: "plugin-compose",
              package: "demo",
              composeNode: "Text",
            },
          },
        ],
      },
    ],
  };
  const root = await load(doc);
  const text = root.findAll((n: any) => n.name === "Tool result")[0];
  text.characters = "after";
  text.x = 24;
  const out = await output();
  const change = out.changes!.find((c) => c.nodeId === "tool")!;
  assert.equal(change.source?.package, "demo");
  assert.equal(change.after?.text, "after");
  assert.equal(change.after?.x, 24);
  text.remove();
  const added = host.api.createText();
  added.characters = "new";
  root.children[0].appendChild(added);
  const out2 = await output();
  assert.equal(out2.changes!.find((c) => c.nodeId === "tool")!.after, null);
  assert.ok(
    out2.changes!.some((c) => c.before === null && c.after?.text === "new"),
  );
});
test("theme parameter editor preserves explicit nulls and nonvisual fields", async () => {
  const root = await load();
  const theme = clone(fixture) as any;
  theme.chat.showAvatar = false;
  theme.typography.user = "cascadia-mono";
  await send({ type: "apply-theme", text: JSON.stringify(theme) });
  const out = await output();
  assert.equal((out.theme!.chat as any).showAvatar, false);
  assert.equal((out.theme!.typography as any).user, "cascadia-mono");
});
test("replaced JPEG assets retain their actual media type and return a theme reference", async () => {
  const t = clone(fixture) as any;
  t.background.asset = "assets/images/original.png";
  const doc = themeDocument(t, [
    {
      path: "assets/images/original.png",
      mediaType: "image/png",
      base64: Buffer.from([0x89, 0x50, 0x4e, 0x47, 1]).toString("base64"),
    },
  ]);
  const root = await load(doc),
    image = root.findAll((n: any) => n.name === "背景图片")[0];
  const replacement = host.api.createImage(
    new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 1]),
  );
  image.fills = [
    { type: "IMAGE", imageHash: replacement.hash, scaleMode: "FILL" },
  ];
  const out = await output(),
    path = (out.theme!.background as any).asset;
  assert.match(path, /\.jpg$/);
  assert.equal(
    out.assets.find((a) => a.path === path)?.mediaType,
    "image/jpeg",
  );
});
test("complete parameter edits can reset an explicit component color to null", async () => {
  const t = clone(fixture) as any;
  t.input.color = "#123456FF";
  await load(themeDocument(t, []));
  t.input.color = null;
  await send({ type: "apply-theme", text: JSON.stringify(t) });
  const out = await output();
  assert.equal((out.theme!.input as any).color, null);
});
test("duplicating a native Figma layer creates a new handoff identity without corrupting source ids", async () => {
  const root = await load(),
    original = root.findAll((n: any) => n.type === "TEXT")[0];
  const copied = host.api.createText();
  copied.name = original.name + " copy";
  copied.characters = "Duplicated design";
  copied.setPluginData("operit.id", original.getPluginData("operit.id"));
  root.children[0].appendChild(copied);
  const out = await output();
  assert.ok(
    out.changes?.some(
      (c) =>
        c.before === null &&
        c.after?.text === "Duplicated design" &&
        c.source?.copiedFrom,
    ),
  );
});
test("editing an inherited palette variable keeps nullable component colors inherited", async () => {
  const root = await load(),
    pairs = JSON.parse(root.getPluginData("operit.variables"));
  const variable = host.variables.get(
    pairs.find(([p]: string[]) => p === "palette.light.primaryContainer")[1],
  );
  variable.setValueForMode("default", { r: 0.5, g: 0.5, b: 0.5, a: 1 });
  const bubble = root.findAll((n: any) => n.name === "用户气泡")[0];
  bubble.fills[0].color = { r: 0.5, g: 0.5, b: 0.5 };
  const out = await output();
  assert.equal((out.theme!.chat as any).user.color, null);
  assert.equal((out.theme!.palette as any).light.primaryContainer, "#808080FF");
});

test("runtime text keeps its captured box when the host auto-sizes newly created text", async () => {
  const doc: DesignDocument = {
    format: "operit-design",
    version: 1,
    name: "Fixed text box",
    origin: "flutter-runtime",
    assets: [],
    warnings: [],
    scenes: [
      {
        id: "screen",
        name: "Screen",
        type: "frame",
        x: 0,
        y: 0,
        width: 500,
        height: 300,
        children: [
          {
            id: "paragraph",
            name: "Wrapped plugin text",
            type: "text",
            x: 24,
            y: 32,
            width: 180,
            height: 44,
            text: "First line\nSecond line",
            fontSize: 14,
            source: {
              kind: "plugin-compose",
              package: "demo",
              composeId: "body:2",
            },
          },
        ],
      },
    ],
  };
  const root = await load(doc);
  const paragraph = root.findAll((n: any) => n.type === "TEXT")[0];
  assert.equal(paragraph.textAutoResize, "NONE");
  assert.equal(paragraph.width, 180);
  assert.equal(paragraph.height, 44);
  assert.equal((await output()).changes?.length, 0);
});
