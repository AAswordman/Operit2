import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { loadModule, plain } from "./runtime.mjs";

const { createCharacterDraft, createGroupDraft } = loadModule("src/drafts.ts");
const { groupPortrait, groupAvatarLayout } = loadModule("src/group-avatar.ts");

/** Builds actual domain-shaped records with explicit portrait sources and persisted member ordering. */
function records(count) {
  const cards = Array.from({ length: count }, (_, index) => ({ ...createCharacterDraft(), id: "member-" + index, name: "Member " + index, avatarUri: "/avatars/" + index + ".png", updatedAt: index }));
  const group = { ...createGroupDraft(), id: "group", name: "Group", members: cards.map((card, index) => ({ characterCardId: card.id, orderIndex: count - index })) };
  return { cards, group };
}

/** Controls only the generic native bitmap ABI so tests can audit composition, caching and ownership. */
function imageHost() {
  const calls = [], alive = new Set();
  let next = 0, fail = false;
  /** Represents one explicitly owned native bitmap handle. */
  function bitmap() {
    const id = ++next; alive.add(id);
    return { id,
      /** Records the exact square cover request without implementing another image resizer. */
      async cover(width, height) { calls.push(["cover", width, height]); if (fail) throw new Error("Controlled cover failure"); return bitmap(); },
      /** Records production layout coordinates and verifies both handles are still alive. */
      async composite(source, x, y) { assert.ok(alive.has(id) && alive.has(source.id)); calls.push(["composite", x, y]); },
      /** Returns a labelled host encoding result; pixel correctness belongs to the native cover test. */
      async getBase64(mime) { assert.equal(mime, "image/png"); return "encoded-portrait-" + id; },
      /** Detects double release as well as leaked production handles. */
      async release() { assert.equal(alive.delete(id), true); calls.push(["release", id]); },
    };
  }
  const globals = {
    Jimp: { MIME_PNG: "image/png", async create(width, height) { calls.push(["create", width, height]); return bitmap(); }, async read(bytes) { calls.push(["read", bytes]); return bitmap(); } },
    ToolPkg: { async readResource(key, name) { calls.push(["resource", key, name]); return "/resources/default.png"; } },
    Tools: { Files: { async readBinary(uri) { calls.push(["file", uri]); return { contentBase64: "file:" + uri }; } }, Net: { async http(options) { calls.push(["http", plain(options)]); return { contentBase64: "http-bytes" }; } } },
  };
  return { globals, calls, alive, fail(value) { fail = value; } };
}

/** Preserves released groups and caps automatic portraits independently from the group's actual participant count. */
test("group avatars preserve custom sources and order at most nine actual member portraits", () => {
  const { cards, group } = records(12); delete group.avatarUri;
  const portrait = groupPortrait(group, cards);
  assert.equal(portrait.uri, null);
  assert.deepEqual(plain(portrait.members.map(member => member.id)), [...cards].reverse().slice(0, 9).map(card => card.id));
  group.avatarUri = "/custom/group.png";
  assert.equal(groupPortrait(group, cards).uri, group.avatarUri);
  assert.throws(() => groupPortrait(group, []), /角色不存在/);
});

/** Checks every supported grid size, centered partial rows, consistent crop sizes and non-overlapping cells. */
test("group avatar geometry centers one to nine members inside the same portrait bounds", () => {
  for (let count = 1; count <= 9; count++) {
    const layout = groupAvatarLayout(count, 300);
    assert.equal(layout.rows.reduce((sum, row) => sum + row.count, 0), count);
    for (const row of layout.rows) {
      assert.ok(Math.abs(row.x * 2 + row.count * layout.tile + (row.count - 1) * layout.gap - 300) < 0.001);
      assert.ok(row.x >= 0 && row.y >= 0 && row.y + layout.tile <= 300);
    }
    if (layout.rows.length > 1) assert.ok(layout.rows[1].y >= layout.rows[0].y + layout.tile);
  }
  assert.throws(() => groupAvatarLayout(10, 300), /one to nine/);
});

/** Runs production composition through the controlled bitmap ABI and verifies bounded handle ownership. */
test("group image composition shares pending work, follows layout and invalidates changed portraits", async () => {
  const host = imageHost(), { groupAvatarUri } = loadModule("src/group-avatar-image.ts", host.globals), { cards, group } = records(5);
  const results = await Promise.all([groupAvatarUri(group, cards), groupAvatarUri(group, cards)]);
  assert.equal(results[0], results[1]); assert.match(results[0], /^data:image\/png;base64,/);
  assert.equal(host.calls.filter(call => call[0] === "create").length, 1);
  assert.equal(host.calls.filter(call => call[0] === "cover").length, 5);
  const layout = groupAvatarLayout(5, 300), expected = layout.rows.flatMap(row => Array.from({ length: row.count }, (_, column) => ["composite", Math.round(row.x + column * (layout.tile + layout.gap)), Math.round(row.y)]));
  assert.deepEqual(host.calls.filter(call => call[0] === "composite"), plain(expected));
  assert.equal(host.alive.size, 0);
  cards[0].updatedAt++;
  assert.notEqual(await groupAvatarUri(group, cards), results[0]);
  assert.equal(host.alive.size, 0);
  group.avatarUri = "/custom.png"; const before = host.calls.length;
  assert.equal(await groupAvatarUri(group, cards), group.avatarUri); assert.equal(host.calls.length, before);
});

/** Preserves exact failures and permits another explicit request after every bitmap allocation has been released. */
test("group image failures release source and canvas without substituting another avatar", async () => {
  const host = imageHost(), { groupAvatarUri } = loadModule("src/group-avatar-image.ts", host.globals), { cards, group } = records(2);
  host.fail(true); await assert.rejects(groupAvatarUri(group, cards), /Controlled cover failure/);
  assert.equal(host.alive.size, 0);
  host.fail(false); await groupAvatarUri(group, cards);
  assert.equal(host.alive.size, 0); assert.equal(host.calls.filter(call => call[0] === "create").length, 2);
});

/** Reads each declared image scheme through its corresponding host capability and uses the packaged default for unset members. */
test("group images support file, HTTP and inline sources and packaged member portraits", async () => {
  const host = imageHost(), { groupAvatarUri } = loadModule("src/group-avatar-image.ts", host.globals), { cards, group } = records(4);
  cards[0].avatarUri = null; cards[1].avatarUri = "https://images.example/member.png"; cards[2].avatarUri = "data:image/png;base64,aW1hZ2U=";
  await groupAvatarUri(group, cards);
  assert.equal(host.calls.filter(call => call[0] === "resource").length, 1);
  assert.ok(host.calls.some(call => call[0] === "file" && call[1] === "/resources/default.png"));
  assert.ok(host.calls.some(call => call[0] === "http" && call[1].url === cards[1].avatarUri));
  assert.ok(host.calls.some(call => call[0] === "read" && call[1] === "aW1hZ2U="));
  assert.equal(host.alive.size, 0);
});

/** Exercises the actual bundled Jimp facade against the generic Promise ABI, without a native runtime or compilation. */
test("Jimp cover returns a separately owned image through the existing structured host Promise", async () => {
  const calls = []; let Jimp;
  const source = readFileSync(new URL("../../../../../core/crates/plugin/javascript-bridge/src/javascript/Jimp.script.js", import.meta.url), "utf8");
  vm.runInNewContext(source, {
    __operitExpose(name, api) { assert.equal(name, "Jimp"); Jimp = api; },
    async __operitInvokeHostAsync(command, args) { return command(...args); },
    async __operitNativeImageProcessing(operation, args) { calls.push([operation, plain(args)]); return "handle-" + calls.length; },
  });
  const original = await Jimp.read("image-bytes"), resized = await original.cover(138, 138);
  assert.notEqual(resized.id, original.id);
  assert.deepEqual(calls[1], ["cover", [original.id, 138, 138]]);
  await resized.release(); assert.equal(original.id, "handle-1"); await original.release();
});
