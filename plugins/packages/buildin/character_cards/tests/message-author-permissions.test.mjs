import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Resolves real persisted cards and grants, with only Core's message identity supplied by the fixture. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const repository = await loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open();
  const service = loadModule("src/service-runtime.ts", disk.globals).createCharacterCardsService(repository);
  let author = "default";
  const chat = loadModule("src/memory-jobs/chat.ts", { ...disk.globals, Tools: {
    ...disk.globals.Tools,
    Chat: { async readExtension(target) {
      assert.equal(target.kind, "message");
      return { extension: author === null ? null : { participantId: author } };
    } },
  } });
  const message = { sender: "ai", content: "Authored reply", timestamp: 10, variantIndex: 0, variantCount: 1,
    roleName: "Display text is not an ID", provider: "provider", modelName: "model" };
  return { disk, repository, service, chat, message, setAuthor(value) { author = value; } };
}

test("historical message memory access follows the author's current binding and revoked mounts", async t => {
  const h = await environment(t);
  const shared = await h.service.dispatchDomain("memory.shared.create", { name: "Writable mount" });
  await h.service.dispatchDomain("character.update", { id: "default", changes: { sharedMemoryMounts: [
    { sharedMemoryId: shared.id, readable: true, writable: true },
  ] } });
  assert.equal((await h.chat.currentMessageMemoryAccess("chat", h.message, h.repository)).primaryOwnerKey, "character:default");
  await h.repository.run(repository => h.chat.requireMessageOwner("chat", h.message, "shared:" + shared.id, repository));
  await h.service.dispatchDomain("character.update", { id: "default", changes: { sharedMemoryMounts: [
    { sharedMemoryId: shared.id, readable: true, writable: false },
  ] } });
  await assert.rejects(() => h.repository.run(repository => h.chat.requireMessageOwner("chat", h.message, "shared:" + shared.id, repository)), /not currently writable/);
  await h.service.dispatchDomain("character.update", { id: "default", changes: { memoryBindingMode: "SHARED", sharedMemoryId: shared.id, sharedMemoryMounts: [] } });
  h.disk.clearCalls();
  const current = plain(await h.chat.currentMessageMemoryAccess("chat", h.message, h.repository));
  assert.equal(current.primaryOwnerKey, "shared:" + shared.id);
  assert.deepEqual(current.resources, [{ key: "shared:" + shared.id, readable: true, writable: true }]);
  assert.equal(h.disk.calls.filter(call => ["storage.execute", "storage.commit", "storage.commit"].includes(call.method)).length, 0);
});

test("author ID stays exact when other roles share its display name or the current chat changes", async t => {
  const h = await environment(t);
  const a = await h.service.dispatchDomain("character.create", { values: { name: "Original author" } });
  const b = await h.service.dispatchDomain("character.create", { values: { name: "Other author" } });
  h.setAuthor(a.id);
  assert.equal((await h.chat.currentMessageMemoryAccess("chat", h.message, h.repository)).primaryOwnerKey, "character:" + a.id);
  await h.service.dispatchDomain("activePrompt.setCard", { id: b.id });
  assert.equal((await h.chat.currentMessageMemoryAccess("chat", h.message, h.repository)).primaryOwnerKey, "character:" + a.id);
  await h.service.dispatchDomain("character.delete", { id: a.id });
  await h.service.dispatchDomain("character.update", { id: b.id, changes: { name: a.name } });
  await assert.rejects(() => h.chat.currentMessageMemoryAccess("chat", h.message, h.repository), /not found|does not identify exactly one record/);
});

test("model-only and missing-author messages never inherit another role's memory grants", async t => {
  const h = await environment(t);
  h.setAuthor("model:chat");
  assert.deepEqual(plain(await h.chat.currentMessageMemoryAccess("chat", h.message, h.repository)), { primaryOwnerKey: "", resources: [] });
  h.setAuthor(null);
  await assert.rejects(() => h.chat.currentMessageMemoryAccess("chat", h.message, h.repository), /no character author identity/);
});
