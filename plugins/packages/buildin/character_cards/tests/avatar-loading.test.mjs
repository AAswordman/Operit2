import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

const origin = "https://characters.operit.local/";
const request = (url, extra = {}) => ({ url, method: "GET", isMainFrame: false, headers: {}, ...extra });

test("twenty default avatars share a path reference and never read binary bytes in TS", async () => {
  let exports = 0;
  const { createAvatarResources } = loadModule("src/avatar-resources.ts", {
    ToolPkg: { async readResource(name, file) {
      exports++;
      assert.equal(name, "character_default_avatar"); assert.equal(file, "operit-avatar.png");
      return "/app/data/resources/operit-avatar.png";
    } },
  });
  const avatars = createAvatarResources(origin);
  const sources = await Promise.all(Array.from({ length: 20 }, () => avatars.source(null)));
  assert.equal(exports, 1); assert.equal(new Set(sources).size, 1);
  assert.equal(sources[0], "avatars/1");
  assert.deepEqual(plain(avatars.intercept(request(origin + sources[0]))), {
    action: "respond", response: { statusCode: 200, reasonPhrase: "OK", filePath: "/app/data/resources/operit-avatar.png" },
  });
  assert.equal(await avatars.source(null), sources[0]); assert.equal(exports, 1);
});

test("resource export failures and empty results can retry", async () => {
  let calls = 0;
  const { createAvatarResources } = loadModule("src/avatar-resources.ts", { ToolPkg: {
    async readResource() { calls++; if (calls === 1) throw new Error("unavailable"); return calls === 2 ? " " : "/app/default.png"; },
  } });
  const avatars = createAvatarResources(origin);
  await assert.rejects(avatars.source(null), /unavailable/);
  await assert.rejects(avatars.source(null), /不能为空/);
  assert.equal(await avatars.source(null), "avatars/1"); assert.equal(calls, 3);
});

test("custom images keep paths, legacy inline images and registered remote references", async () => {
  const { createAvatarResources } = loadModule("src/avatar-resources.ts");
  const avatars = createAvatarResources(origin);
  const local = await avatars.source("/app/config/avatars/custom.image");
  assert.equal(await avatars.source("/app/config/avatars/custom.image"), local);
  assert.equal(avatars.intercept(request(origin + local)).response.filePath, "/app/config/avatars/custom.image");
  const remote = await avatars.source("https://example.com/avatar.png");
  assert.deepEqual(plain(avatars.intercept(request(origin + remote))), { action: "rewrite", url: "https://example.com/avatar.png" });
  assert.deepEqual(plain(avatars.intercept(request("https://example.com/avatar.png"))), { action: "allow" });
  const inline = "data:image/png;base64,iVBORw0KGgo=";
  assert.equal(await avatars.source(inline), inline);
  assert.equal(avatars.intercept(request(origin + "unregistered")), null);
  for (const url of [origin + local, origin + remote]) {
    assert.equal(avatars.intercept(request(url, { isMainFrame: true })).action, "block");
    assert.equal(avatars.intercept(request(url, { method: "POST" })).action, "block");
  }
});

test("overlapping browser loads deduplicate both default and custom avatar requests", async () => {
  const { createAvatarSources } = loadModule("web/bridge/avatar-sources.ts");
  const calls = [];
  let complete;
  const gate = new Promise(resolve => { complete = resolve; });
  const sources = new Map();
  const avatars = createAvatarSources(sources, async path => { calls.push(path); await gate; return path === null ? "avatars/1" : "avatars/2"; });
  const first = avatars.load(Array(20).fill(null));
  const second = avatars.load([null, "/app/custom.image", "/app/custom.image"]);
  const single = avatars.source(null);
  await Promise.resolve(); assert.deepEqual(calls, [null, "/app/custom.image"]);
  complete(); await Promise.all([first, second, single]);
  assert.equal(sources.get(""), "avatars/1");
  await avatars.load([null, "/app/custom.image"]); assert.equal(calls.length, 2);
});

test("all concurrent failures clear browser pending state and allow a later retry", async () => {
  const { createAvatarSources } = loadModule("web/bridge/avatar-sources.ts");
  let calls = 0;
  const sources = new Map();
  const avatars = createAvatarSources(sources, async () => { if (++calls === 1) throw new Error("temporary"); return "avatars/1"; });
  const results = await Promise.allSettled([avatars.source(null), avatars.load([null, null]), avatars.source(null)]);
  assert.ok(results.every(result => result.status === "rejected")); assert.equal(calls, 1); assert.equal(sources.size, 0);
  await avatars.load([null]); assert.equal(calls, 2); assert.equal(sources.get(""), "avatars/1");
});
