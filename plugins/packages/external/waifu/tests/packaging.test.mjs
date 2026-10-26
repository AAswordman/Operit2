import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import test from "node:test";

/** Requires the shipped consumer contract and every manifest resource to match the authored provider. */
test("QQBot declares the real waifu dependency and ships the exact standalone client with all sticker assets", async () => {
  const base = new URL("../", import.meta.url);
  const manifest = JSON.parse(await readFile(new URL("manifest.json", base), "utf8"));
  const consumer = JSON.parse(await readFile(new URL("../../qqbot/manifest.json", import.meta.url), "utf8"));
  assert.equal(manifest.api_version, "2.0.0"); assert.equal(manifest.public_api, "src/api.ts");
  assert.ok(consumer.requires.some(requirement => requirement.id === manifest.toolpkg_id && requirement.min_version === "1.0.0"));
  const authored = await readFile(new URL(manifest.public_api, base), "utf8");
  const copied = await readFile(new URL("../../qqbot/src/dependencies/waifu/api.ts", import.meta.url), "utf8");
  assert.equal(copied, authored); assert.doesNotMatch(authored, /^import\s/m);
  assert.equal(manifest.resources.length, 53);
  assert.equal(new Set(manifest.resources.map(resource => resource.key)).size, 53);
  for (const resource of manifest.resources) assert.ok((await stat(new URL(resource.path, base))).size > 0);
});
