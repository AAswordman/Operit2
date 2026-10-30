/** Verify the distributable, not only the TypeScript entry point. */
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import assert from "node:assert/strict";
import { unzipSync, strFromU8 } from "fflate";
import { parseDocument } from "../src/bridge.ts";
import { createFigmaMock } from "./figma_mock.ts";

const archive = unzipSync(
  await readFile(new URL("../dist/operit-figma-designer.zip", import.meta.url)),
);
const file = (path: string): string => {
  const bytes = archive[`operit-figma/${path}`];
  assert.ok(bytes, `Missing packaged file: ${path}`);
  return strFromU8(bytes);
};
const manifest = JSON.parse(file("manifest.json"));
const html = file(manifest.ui);
assert.ok(!html.includes("/* OPERIT_UI_SCRIPT */"));
assert.ok(html.includes("pluginMessage"));
const host = createFigmaMock();
runInNewContext(file(manifest.main), { figma: host.api, __html__: html });
await host.api.ui.onmessage({
  type: "import",
  text: file("examples/theme-starter.operit-design.json"),
});
assert.ok(
  !host.messages.some((m) => m.type === "error"),
  JSON.stringify(host.messages),
);
const root = host.api.currentPage.selection[0];
assert.ok(root.findAll((n: any) => n.type === "COMPONENT").length >= 8);
const text = root.findAll((n: any) => n.type === "TEXT")[0];
const oldText = text.characters;
text.characters = "设计师修改了文字";
const color = root.findAll((n: any) => n.name === "用户气泡")[0];
color.fills = [{ type: "SOLID", color: { r: 0, g: 1, b: 0 }, opacity: 1 }];
host.messages.length = 0;
await host.api.ui.onmessage({ type: "export" });
assert.ok(
  !host.messages.some((m) => m.type === "error"),
  JSON.stringify(host.messages),
);
const output = parseDocument(
  host.messages.find((m) => m.type === "download").text,
);
assert.equal((output.theme!.chat as any).user.color, "#00FF00FF");
assert.ok(
  output.changes?.some(
    (change) =>
      change.before?.text === oldText &&
      change.after?.text === "设计师修改了文字",
  ),
);
console.log(
  "PASS distributable ZIP: packaged manifest + compiled UI/plugin → native editable components → theme color and text handoff",
);
