import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile, unlink, writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const drafts = loadModule("src/drafts.ts");

/** Loads the actual file repository with IO-only adaptation and no domain host or data fixture. */
async function openRepository(disk) {
  const module = loadModule("src/storage/files.ts", disk.globals);
  return module.FileCharacterRepository.open();
}

/** Returns the exact paths declared by the current production file backend. */
function paths(disk) {
  const directory = path.join(disk.directory, "character-memory");
  return { directory, state: path.join(directory, "state.json"), pending: path.join(directory, "state.next.json") };
}

/** Supplies an explicit new-card input; the repository itself allocates identity and timestamps. */
function cardInput(name) {
  return { ...plain(drafts.createCharacterDraft()), name, characterSetting: "真实磁盘角色设定", openingStatement: "真实开场白" };
}

/** Supplies editable memory input without installing a successful memory service or repository substitute. */
function memoryInput(title) {
  return { title, content: "真实写入的记忆内容", contentType: "text/plain", source: "manual", credibility: 0.9, importance: 0.7, folderPath: "documents/notes", tags: ["完整标签"] };
}

/** Selects actual file publications by their exact resolved destination, not by a filename substring. */
function filePublications(disk, destination) {
  return disk.calls.filter(
    /** Requires the requested destination of the existing Files move operation. */
    call => call.method === "move" && path.resolve(call.args[1]) === destination,
  );
}

/** Counts only publication of the authoritative state.json snapshot; USER.md is asserted separately. */
function publications(disk) { return filePublications(disk, paths(disk).state); }

/** Lists every host file mutation so that read-only reopen cannot hide document writes or deletions. */
function mutations(disk) {
  return disk.calls.filter(
    /** Classifies the exact IO method names exposed by the real disk harness. */
    call => ["write", "move", "mkdir", "deleteFile"].some(
      /** Matches declared mutation names without guessing paths or API prefixes. */
      method => call.method === method,
    ),
  );
}

/** Verifies product first-start state on actual disk and distinguishes it from opening existing files. */
test("real file repository initializes a new plugin directory exactly once and reopens without writing", async t => {
  const disk = await createDiskHarness(t), files = paths(disk), repository = await openRepository(disk);
  const state = JSON.parse(await readFile(files.state, "utf8"));
  assert.equal(state.version, 3); assert.equal(typeof state.nextId, "string");
  assert.deepEqual(state.conversationGroups, []); assert.equal(Object.hasOwn(state, "chatBindings"), false);
  assert.equal(state.cards.length, 1); assert.equal(state.cards[0].isDefault, true);
  assert.deepEqual(state.active, { CharacterCard: { id: state.cards[0].id } });
  assert.deepEqual(state.owners.map(
    /** Compares the exact declared initial owner to the actual persisted default character. */
    owner => owner.ownerKey,
  ), ["character:" + state.cards[0].id]);
  assert.equal(publications(disk).length, 1);
  assert.equal((await disk.files.exists(files.pending)).exists, false);
  assert.equal(state.owners[0].userDocumentPath, "owners/character%3Adefault/USER.md");
  const user = path.join(files.directory, state.owners[0].userDocumentPath), ownerKey = state.owners[0].ownerKey;
  const userBytes = await readFile(user);
  assert.equal(userBytes.toString("utf8"), "", "The production first-install USER.md must actually exist with its declared empty content");
  assert.equal(filePublications(disk, user).length, 1);
  assert.deepEqual(filePublications(disk, user)[0].args.map(
    /** Compares native staging and destination paths without depending on host path separators. */
    file => path.resolve(file),
  ), [user + ".next", user]);
  assert.equal((await disk.files.exists(user + ".next")).exists, false);
  assert.deepEqual(plain(await repository.run(
    /** Reads the real first-install document through production methods as well as native fs. */
    async session => ({ profile: await session.readUser(ownerKey), path: await session.readUserPath(ownerKey) }),
  )), { profile: { ownerKey, content: "" }, path: user.replaceAll("\\", "/") });
  const cards = await repository.run(
    /** Reads the initial records through the production repository, not the JSON assertion alone. */
    session => session.listCharacters(),
  );
  assert.deepEqual(plain(cards), state.cards);
  const bytes = await readFile(files.state); disk.clearCalls();
  const reopened = await openRepository(disk);
  assert.deepEqual(plain(await reopened.run(
    /** Reads from a newly bundled runtime owner against the same real directory. */
    session => session.listCharacters(),
  )), state.cards);
  assert.equal(publications(disk).length, 0);
  assert.equal(filePublications(disk, user).length, 0);
  assert.equal(mutations(disk).length, 0, "An existing-file reopen must not write any state or owner document");
  assert.deepEqual(plain(await reopened.run(
    /** Reads USER.md from the separate runtime rather than from the first owner session. */
    session => session.readUser(ownerKey),
  )), { ownerKey, content: "" });
  assert.equal(mutations(disk).length, 0);
  assert.deepEqual(await readFile(files.state), bytes);
  assert.deepEqual(await readFile(user), userBytes);
});

/** Requires each read to observe externally edited USER.md bytes without republishing a cached profile. */
test("external USER.md edits are read from the real file by existing and reopened repositories without writes", async t => {
  const disk = await createDiskHarness(t), files = paths(disk), repository = await openRepository(disk);
  const ownerKey = "character:default", initial = "# Repository profile\n\n原始资料。\n", edited = "# External profile\n\n外部真实编辑：新资料 😀。\n";
  await repository.run(
    /** Establishes a different saved profile through the actual domain method before the external edit. */
    session => session.writeUser(ownerKey, initial),
  );
  const state = JSON.parse(await readFile(files.state, "utf8")), user = path.join(files.directory, state.owners[0].userDocumentPath);
  const stateBytes = await readFile(files.state);
  assert.equal(await readFile(user, "utf8"), initial);
  await writeFile(user, edited, "utf8"); disk.clearCalls();
  assert.deepEqual(plain(await repository.run(
    /** Reads the current document using the already-open production repository. */
    session => session.readUser(ownerKey),
  )), { ownerKey, content: edited });
  const reopened = await openRepository(disk);
  assert.deepEqual(plain(await reopened.run(
    /** Reads the same real external content through a newly loaded runtime owner. */
    session => session.readUser(ownerKey),
  )), { ownerKey, content: edited });
  assert.equal(mutations(disk).length, 0);
  assert.deepEqual(await readFile(files.state), stateBytes);
  assert.equal(await readFile(user, "utf8"), edited);
});

/** Rejects missing referenced USER.md during reads, writes, and reopen without recreating or repairing it. */
test("missing referenced USER.md produces explicit file errors and never recreates a profile", async t => {
  const disk = await createDiskHarness(t), files = paths(disk), repository = await openRepository(disk);
  const stateBytes = await readFile(files.state), state = JSON.parse(stateBytes.toString("utf8"));
  const ownerKey = state.owners[0].ownerKey, user = path.join(files.directory, state.owners[0].userDocumentPath);
  await unlink(user); disk.clearCalls();
  await assert.rejects(repository.run(
    /** Requires the real missing-file error instead of an empty successful profile. */
    session => session.readUser(ownerKey),
  ), { code: "ENOENT", path: user });
  await assert.rejects(repository.run(
    /** Ensures a profile save cannot silently recreate an externally missing referenced document. */
    session => session.writeUser(ownerKey, "Must not create USER.md"),
  ), { code: "ENOENT", path: user });
  await assert.rejects(openRepository(disk), { code: "ENOENT", path: user });
  assert.equal(mutations(disk).length, 0);
  assert.deepEqual(await readFile(files.state), stateBytes);
  await assert.rejects(readFile(user), { code: "ENOENT", path: user });
});

/** Runs complete card/tag mutations together and requires one actual snapshot publication. */
test("repository card and tag changes publish once and are read back by a new runtime", async t => {
  const disk = await createDiskHarness(t), repository = await openRepository(disk);
  const initial = await repository.run(
    /** Creates intentional inputs through actual repository methods before measuring the edit. */
    async session => {
      const removed = await session.createTag({ name: "删除标签", description: "old", promptContent: "旧提示", tagType: "CUSTOM" });
      const updated = await session.createTag({ name: "修改标签", description: "old", promptContent: "旧提示", tagType: "TONE" });
      const card = await session.createCharacter({ ...cardInput("文件角色"), attachedTagIds: [removed.id, updated.id] });
      return { removed, updated, card };
    },
  );
  disk.clearCalls();
  const changed = await repository.run(
    /** Stages all actual repository changes inside its private transaction snapshot. */
    async session => {
      const created = await session.createTag({ name: "新标签", description: "完整描述", promptContent: "新提示", tagType: "CUSTOM" });
      const updated = await session.updateTag({ ...initial.updated, description: "修改描述", promptContent: "修改提示" });
      await session.deleteTag(initial.removed.id);
      const card = await session.updateCharacter({ ...initial.card, otherContentVoice: "语音附加内容", marks: "持久化备注", attachedTagIds: [created.id, updated.id] });
      return { created, updated, card };
    },
  );
  assert.equal(publications(disk).length, 1);
  assert.equal(disk.calls.filter(
    /** Requires one staging write for the whole mutation, not one write per entity. */
    call => call.method === "write",
  ).length, 1);
  const reopened = await openRepository(disk);
  const result = await reopened.run(
    /** Reads the independently reopened domain records from the actual snapshot file. */
    async session => ({ card: await session.getCharacter(changed.card.id), tags: await session.listTags() }),
  );
  assert.deepEqual(plain(result.card), plain(changed.card));
  assert.deepEqual(plain(result.tags), plain([changed.updated, changed.created]));
  assert.equal(result.tags.some(
    /** Rejects the removed tag in the actual persisted tag collection. */
    tag => tag.id === initial.removed.id,
  ), false);
});

/** Keeps full owner-scoped records, relationships, settings, and USER.md on disk across runtime reload. */
test("real repository persists complete owner memories links settings and USER.md and rejects unknown owners", async t => {
  const disk = await createDiskHarness(t), files = paths(disk), repository = await openRepository(disk);
  const saved = await repository.run(
    /** Creates real persisted data solely through the actual repository's mutation methods. */
    async session => {
      const store = await session.createStore("独立共享库"), ownerKey = "shared:" + store.id;
      const card = await session.createCharacter({ ...cardInput("绑定角色"), memoryBindingMode: "SHARED", sharedMemoryId: store.id, sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: false }] });
      const first = await session.createMemory(ownerKey, memoryInput("文档记忆"));
      const document = await session.saveMemory(ownerKey, { ...first, documentPath: "plugin:/docs/document.txt", isDocumentNode: true, chunkIndexFilePath: "plugin:/docs/index.json", properties: [{ id: "1", key: "source-note", value: "完整属性" }] });
      const target = await session.createMemory(ownerKey, { ...memoryInput("目标记忆"), folderPath: "links" });
      const link = await session.createLink(ownerKey, { sourceMemoryId: document.id, targetMemoryId: target.id, type_: "影响", weight: 0.85, description: "磁盘完整关系描述" });
      const settings = { autoSaveIntervalMinutes: 21, nextAutoSaveRunAtMs: 987654, memoryExtractionCustomRules: "独立规则", profileAutoUpdateEnabled: false, profileAutoUpdateLocked: true, cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: "https://input.invalid/embedding", cloudEmbeddingApiKey: "explicit-test-input", cloudEmbeddingModel: "test-input-model" };
      const config = { scoreMode: "KEYWORD_FIRST", keywordWeight: 4, tagWeight: 2, vectorWeight: 3, edgeWeight: 1 };
      await session.writeMemorySettings(ownerKey, settings); await session.writeMemorySearchConfig(ownerKey, config);
      await session.writeUser(ownerKey, "# USER.md\n\n共享库资料：完整 Unicode。\n");
      await session.writeUser("character:" + card.id, "# 角色私有资料\n");
      return { store, ownerKey, card, document, target, link, settings, config };
    },
  );
  const bytes = await readFile(files.state), reopened = await openRepository(disk);
  const result = await reopened.run(
    /** Retrieves independent actual persisted fields through the new repository owner. */
    async session => ({ card: await session.getCharacter(saved.card.id), memories: await session.listMemories(saved.ownerKey), links: await session.listMemoryLinks(saved.ownerKey), graph: await session.readMemoryGraph(saved.ownerKey), settings: await session.readMemorySettings(saved.ownerKey), config: await session.readMemorySearchConfig(saved.ownerKey), profile: await session.readUser(saved.ownerKey), privateProfile: await session.readUser("character:" + saved.card.id), privateMemories: await session.listMemories("character:" + saved.card.id) }),
  );
  assert.deepEqual(plain(result.card), plain(saved.card));
  assert.deepEqual(plain(result.memories), plain([saved.document, saved.target]));
  assert.deepEqual(plain(result.links), plain([saved.link]));
  assert.deepEqual(plain(result.settings), saved.settings); assert.deepEqual(plain(result.config), saved.config);
  assert.equal(result.profile.content, "# USER.md\n\n共享库资料：完整 Unicode。\n");
  assert.equal(result.privateProfile.content, "# 角色私有资料\n"); assert.deepEqual(plain(result.privateMemories), []);
  assert.equal(result.graph.edges[0].id, saved.link.id); assert.equal(typeof result.graph.edges[0].id, "string");
  assert.equal(result.graph.edges[0].sourceId, saved.document.uuid); assert.equal(result.graph.edges[0].targetId, saved.target.uuid);
  assert.equal(result.graph.edges[0].metadata.description, saved.link.description);
  for (const ownerKey of ["shared:missing", "character:missing", "unknown:missing", "shared:" + saved.card.id]) {
    await assert.rejects(reopened.run(
      /** Invokes the production exact-owner lookup without creating a replacement space. */
      session => session.listMemories(ownerKey),
    ));
  }
  assert.deepEqual(await readFile(files.state), bytes);
});

/** Verifies operation errors discard actual staged domain edits without publishing any partial file. */
test("repository action failures publish neither staged cards nor tags", async t => {
  const disk = await createDiskHarness(t), repository = await openRepository(disk), files = paths(disk), before = await readFile(files.state);
  disk.clearCalls(); const failure = new Error("TEST_DOMAIN_MUTATION_REJECTED");
  await assert.rejects(repository.run(
    /** Stages real domain data and deliberately rejects the operation before publication. */
    async session => {
      const tag = await session.createTag({ name: "不能保存", description: "", promptContent: "", tagType: "CUSTOM" });
      await session.createCharacter({ ...cardInput("不能保存角色"), attachedTagIds: [tag.id] }); throw failure;
    },
  ),
    /** Requires unchanged error identity instead of a successful empty snapshot. */
    error => error === failure,
  );
  assert.equal(publications(disk).length, 0); assert.deepEqual(await readFile(files.state), before);
  const reopened = await openRepository(disk);
  assert.equal((await reopened.run(
    /** Reads the authoritative collection rather than the failed transaction's detached object. */
    session => session.listTags(),
  )).length, 0);
});

/** Exercises native rename failure through the real publisher and keeps incomplete publication explicitly broken. */
test("failed file publication keeps previous authoritative bytes and explicitly rejects unfinished reopen", async t => {
  const disk = await createDiskHarness(t), repository = await openRepository(disk), files = paths(disk), before = await readFile(files.state);
  const failure = new Error("TEST_PUBLICATION_REJECTED"); disk.failNext("move", failure);
  await assert.rejects(repository.run(
    /** Requests a real mutation whose publication is rejected only by the IO adapter. */
    session => session.createTag({ name: "失败写入", description: "", promptContent: "", tagType: "CUSTOM" }),
  ),
    /** Requires the original publication failure to escape the actual repository. */
    error => error === failure,
  );
  assert.deepEqual(await readFile(files.state), before);
  assert.equal((await disk.files.exists(files.pending)).exists, true);
  await assert.rejects(repository.run(
    /** Probes the failed owner without resuming or repairing its publication. */
    session => session.listTags(),
  ),
    /** Requires the retained failure rather than an automatic second write attempt. */
    error => error === failure,
  );
  await assert.rejects(openRepository(disk), /Unfinished plugin snapshot publication/);
  assert.deepEqual(await readFile(files.state), before);
});

/** Corrupts only actual plugin files and requires every malformed open to fail without repair writes. */
test("bad JSON wrong schema invalid IDs and dangling owner records explicitly reject without overwriting bytes", async t => {
  const disk = await createDiskHarness(t), files = paths(disk); await openRepository(disk);
  const state = JSON.parse(await readFile(files.state, "utf8"));
  const wrongVersion = { ...state, version: 2 }, missingOwners = { ...state, owners: [] }, numericId = { ...state, nextId: 1 };
  for (const content of ["{broken", "{}", JSON.stringify(wrongVersion), JSON.stringify(missingOwners), JSON.stringify(numericId)]) {
    await writeFile(files.state, content, "utf8"); disk.clearCalls();
    await assert.rejects(openRepository(disk));
    assert.equal(await readFile(files.state, "utf8"), content);
    assert.equal(disk.calls.filter(
      /** Rejects mutation while processing an existing corrupt plugin file. */
      call => call.method === "write" || call.method === "move" || call.method === "mkdir",
    ).length, 0);
  }
});

/** Distinguishes incomplete existing directories and read errors from a genuinely new installation. */
test("an existing directory without state and an inaccessible state file never trigger first initialization", async t => {
  const disk = await createDiskHarness(t), files = paths(disk);
  await disk.files.mkdir(files.directory, true); disk.clearCalls();
  await assert.rejects(openRepository(disk), { code: "ENOENT" });
  assert.equal(publications(disk).length, 0);
  const separate = await createDiskHarness(t), separateFiles = paths(separate); await openRepository(separate);
  const before = await readFile(separateFiles.state), failure = new Error("TEST_FILE_READ_REJECTED");
  separate.clearCalls(); separate.failNext("read", failure);
  await assert.rejects(openRepository(separate),
    /** Preserves the exact IO failure from the actual file-open path. */
    error => error === failure,
  );
  assert.deepEqual(await readFile(separateFiles.state), before); assert.equal(publications(separate).length, 0);
});

/** Exercises current-schema chunk reads and preservation, not document ingestion or old-user migration. */
test("current plugin-file document chunks survive actual reopen and unrelated memory edits", async t => {
  const disk = await createDiskHarness(t), files = paths(disk), repository = await openRepository(disk);
  const document = await repository.run(
    /** Creates the document node through the actual repository before supplying a current-format file input. */
    async session => {
      const record = await session.createMemory("character:default", memoryInput("带 chunk 的文档"));
      return session.saveMemory("character:default", { ...record, isDocumentNode: true, documentPath: "plugin:/document.txt", chunkIndexFilePath: "plugin:/chunks.json" });
    },
  );
  const state = JSON.parse(await readFile(files.state, "utf8")), owner = state.owners.find(
    /** Locates the exact new-format owner in the real on-disk schema. */
    record => record.ownerKey === "character:default",
  );
  const firstId = BigInt(state.nextId);
  owner.chunks = [{ id: firstId.toString(), memoryUuid: document.uuid, chunkIndex: 0, content: "第一段\n完整内容" }, { id: (firstId + 1n).toString(), memoryUuid: document.uuid, chunkIndex: 1, content: "第二段\n完整内容" }];
  state.nextId = (firstId + 2n).toString();
  const chunkInput = plain(owner.chunks); await writeFile(files.state, JSON.stringify(state), "utf8");
  const reopened = await openRepository(disk);
  await reopened.run(
    /** Mutates an actual reopened full document while preserving its existing chunk records. */
    session => session.updateMemory("character:default", document.id, { ...memoryInput(document.title), content: "更新后的正文" }),
  );
  const after = JSON.parse(await readFile(files.state, "utf8"));
  assert.deepEqual(after.owners[0].chunks, chunkInput);
  const memory = after.owners[0].memories[0];
  assert.equal(memory.documentPath, document.documentPath); assert.equal(memory.chunkIndexFilePath, document.chunkIndexFilePath);
  assert.equal(memory.uuid, document.uuid); assert.equal(memory.id, document.id); assert.equal(memory.isDocumentNode, true);
  await openRepository(disk);
});
