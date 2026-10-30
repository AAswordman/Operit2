import { test } from "node:test";
import assert from "node:assert/strict";
import fixture from "../../../apps/flutter/packages/operit_ui/assets/fixtures/theme-v1.json" with { type: "json" };
import {
  clone,
  parseDocument,
  validateTheme,
  validateDocument,
  setPath,
  changedNodes,
  rgba,
  hex,
  type DesignDocument,
  type Theme,
} from "../src/bridge.ts";
import { themeDocument } from "../src/theme_scene.ts";
const theme = () => clone(fixture) as Theme;
test("kit contains real native nodes, dark/light desktop/phone and full theme", () => {
  const doc = themeDocument(theme(), []);
  assert.equal(doc.scenes.length, 4);
  assert.deepEqual(doc.theme, fixture);
  const json = JSON.stringify(doc);
  assert.match(json, /ThemeComposerChrome/);
  assert.match(json, /BubbleSurface/);
  assert.deepEqual(parseDocument(json), doc);
});
test("schema rejects unknown fields, wrong types and bounds without changing original", () => {
  const t = theme();
  t.chat = { ...(t.chat as Theme), unknown: true };
  assert.throws(() => validateTheme(t), /未知字段/);
  const t2 = theme();
  setPath(t2, "input.sendAssetSize", 1000);
  assert.throws(() => validateTheme(t2), /超出范围/);
  assert.throws(() => setPath(t2, "__proto__.polluted", true));
  assert.equal(({} as any).polluted, undefined);
});
test("import rejects path traversal, duplicate ids, missing resources and non-finite dimensions", () => {
  const doc = themeDocument(theme(), []);
  doc.assets = [
    {
      path: "assets/images/../evil.png",
      mediaType: "image/png",
      base64: "AA==",
    },
  ];
  assert.throws(() => validateDocument(doc), /路径/);
  const duplicate = themeDocument(theme(), []);
  duplicate.scenes.push(clone(duplicate.scenes[0]));
  assert.throws(() => validateDocument(duplicate), /ID/);
  const missing = theme();
  setPath(missing, "background.asset", "assets/images/missing.png");
  assert.throws(() => themeDocument(missing, []), /缺少素材/);
  const invalid = themeDocument(theme(), []);
  invalid.scenes[0].width = Infinity;
  assert.throws(() => validateDocument(invalid), /尺寸/);
});
test("RGBA round trip preserves alpha and channel order", () => {
  const c = rgba("#33669980");
  assert.equal(hex(c, c.a), "#33669980");
});
test("handoff records text/layout/add/delete with owner identity", () => {
  const before: DesignDocument["scenes"] = [
    {
      id: "a",
      name: "tool result",
      type: "text",
      x: 0,
      y: 0,
      width: 100,
      height: 20,
      text: "old",
      source: { kind: "tool-result", tool: "read_file" },
    },
  ];
  const after = clone(before);
  after[0].text = "new";
  after[0].width = 120;
  const changes = changedNodes(before, after);
  assert.equal(changes.length, 1);
  assert.equal(changes[0].source?.tool, "read_file");
  assert.equal(changes[0].after?.text, "new");
  assert.equal(changedNodes(before, [])[0].after, null);
  assert.equal(changedNodes([], after)[0].before, null);
});
test("handoff detects layer reparenting and stacking order even when coordinates stay equal", () => {
  const doc = themeDocument(theme(), []);
  const before = clone(doc.scenes);
  const children = doc.scenes[0].children!;
  [children[0], children[1]] = [children[1], children[0]];
  const changes = changedNodes(before, doc.scenes);
  assert.ok(changes.some((c) => c.indexBefore !== c.indexAfter));
});
