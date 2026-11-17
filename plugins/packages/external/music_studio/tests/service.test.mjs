import test from "node:test";
import assert from "node:assert/strict";
import { load } from "./helpers.mjs";
const { StudioService } = await load("src/service.ts");
function fixture() { let db = null; let fail = false; return { service: new StudioService({ async read() { return structuredClone(db); }, async write(value) { if (fail) throw new Error("Disk full"); db = structuredClone(value); } }), persisted: () => db, fail: v => fail = v }; }
const ref = s => ({ projectId: s.project.id, revision: s.project.revision });
const get = service => service.request({ action: "get" });
test("first load persists example; export roundtrips and restart reads it", async () => {
  const f = fixture(); const s = await get(f.service); assert.equal(f.persisted().projects[0].id, s.project.id);
  const exportResult = await f.service.request({ action: "export" }); assert.deepEqual(JSON.parse(exportResult.json), s.project);
  const restarted = new StudioService({ async read() { return f.persisted(); }, async write() {} }); assert.deepEqual((await get(restarted)).project, s.project);
});
test("optimistic revisions serialize simultaneous writers", async () => {
  const { service } = fixture(); const s = await get(service);
  const write = name => service.request({ action: "batch", ...ref(s), operations: [{ type: "project.set", patch: { name } }] });
  const results = await Promise.allSettled([write("A"), write("B")]); assert.equal(results[0].status, "fulfilled"); assert.equal(results[1].status, "rejected"); assert.match(String(results[1].reason), /REVISION_CONFLICT/); assert.equal((await get(service)).project.name, "A");
});
test("failed durable write doesn't publish a new revision or undo history", async () => {
  const f = fixture(); const s = await get(f.service); f.fail(true);
  await assert.rejects(f.service.request({ action: "batch", ...ref(s), operations: [{ type: "project.set", patch: { bpm: 100 } }] }), /Disk full/);
  assert.deepEqual((await get(f.service)).project, s.project); f.fail(false);
  await assert.rejects(f.service.request({ action: "undo", ...ref(s) }), /Nothing to undo/);
});
test("undo redo revisions monotonic; switching catalog uses persisted active ID", async () => {
  const { service } = fixture(); let s = await get(service); const original = s.project.name;
  s = await service.request({ action: "batch", ...ref(s), operations: [{ type: "project.set", patch: { name: "Changed" } }] }); assert.equal(s.project.revision, 1);
  s = await service.request({ action: "undo", ...ref(s) }); assert.equal(s.project.name, original); assert.equal(s.project.revision, 2);
  s = await service.request({ action: "redo", ...ref(s) }); assert.equal(s.project.name, "Changed"); assert.equal(s.project.revision, 3);
  const id = s.project.id; s = await service.request({ action: "duplicate", ...ref(s) }); assert.notEqual(s.project.id, id); assert.equal(s.projects.length, 2);
  s = await service.request({ action: "open", ...ref(s), id }); assert.equal(s.project.name, "Changed");
});
test("no invisible playback or false export success; heartbeat, queue, ack", async () => {
  const { service } = fixture(); const s = await get(service);
  await assert.rejects(service.request({ action: "command", ...ref(s), type: "play" }), /未打开/);
  const status = { unlocked: false, playing: false, beat: 0, peak: 0, voices: 0, dropped: 0, projectId: s.project.id };
  await service.request({ action: "sync", status, ...ref(s) });
  await assert.rejects(service.request({ action: "command", ...ref(s), type: "play" }), /点击一次/);
  await service.request({ action: "sync", status: { ...status, unlocked: true }, ...ref(s) });
  const receipt = await service.request({ action: "command", ...ref(s), type: "play" }); assert.equal(receipt.state, "queued"); assert.equal((await get(service)).commands.length, 1);
  await service.request({ action: "ack", id: receipt.id, state: "done", message: "Applied play" }); assert.equal((await get(service)).commands.length, 0); assert.equal((await get(service)).receipts[0].state, "done");
  const lean = await service.request({ action: "sync", ...ref(s), status }); assert.equal(lean.project, undefined);
});
test("project edits cancel pending commands; queued export is not completed", async () => {
  const { service } = fixture(); let s = await get(service);
  await service.request({ action: "sync", ...ref(s), status: { unlocked: true, playing: false, beat: 0, peak: 0, voices: 0, dropped: 0, projectId: s.project.id } });
  const c = await service.request({ action: "command", ...ref(s), type: "render" });
  s = await service.request({ action: "batch", ...ref(s), operations: [{ type: "project.set", patch: { name: "Edited" } }] });
  assert.equal(s.commands.length, 0); assert.equal(s.receipts.find(r => r.id === c.id).state, "failed");
});
test("import generates new identity; corrupt data is not overwritten", async () => {
  const { service } = fixture(); let s = await get(service); const old = s.project.id;
  s = await service.request({ action: "import", ...ref(s), json: JSON.stringify(s.project) }); assert.notEqual(s.project.id, old);
  let writes = 0; const broken = new StudioService({ async read() { return { version: 999 }; }, async write() { writes++; } });
  await assert.rejects(get(broken), /Unsupported/); assert.equal(writes, 0);
});
