/** Run after the Flutter capture tests: validates actual production-rendered
 * data through the same plugin importer/exporter used by Figma. */
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { parseDocument } from "../src/bridge.ts";
import { createFigmaMock } from "./figma_mock.ts";
const host = createFigmaMock();
(globalThis as any).figma = host.api;
(globalThis as any).__html__ = "";
await import("../src/plugin.ts");
for (const file of [
  "../../../apps/flutter/packages/operit_ui/.dart_tool/design-capture-test.operit-design.json",
  "../../../apps/flutter/app/.dart_tool/figma-deep-capture.operit-design.json",
]) {
  const text = await readFile(new URL(file, import.meta.url), "utf8");
  const source = parseDocument(text);
  await host.api.ui.onmessage({ type: "import", text });
  assert.ok(
    !host.messages.some((m) => m.type === "error"),
    JSON.stringify(host.messages),
  );
  const root = host.api.currentPage.selection[0];
  const paragraph = root.findAll(
    (n: any) =>
      n.type === "TEXT" &&
      (n.characters === "Nested plugin controls" ||
        n.characters === "Deep plugin UI"),
  )[0];
  assert.ok(paragraph, "Real nested paragraph was not imported as TEXT");
  paragraph.characters += " · 设计修改";
  paragraph.x += 12;
  host.messages.length = 0;
  await host.api.ui.onmessage({ type: "export" });
  assert.ok(
    !host.messages.some((m) => m.type === "error"),
    JSON.stringify(host.messages),
  );
  const returned = parseDocument(
    host.messages.find((m) => m.type === "download").text,
  );
  assert.equal(returned.assets.length, source.assets.length);
  assert.ok(
    returned.changes?.some(
      (c) => c.after?.text?.includes("设计修改") && c.source?.package,
    ),
  );
  console.log(
    `PASS ${source.name}: actual Flutter capture → editable Figma TEXT → source-linked handoff (${returned.changes!.length} change)`,
  );
  host.messages.length = 0;
}
