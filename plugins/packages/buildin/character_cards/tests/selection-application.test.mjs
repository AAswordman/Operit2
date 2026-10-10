import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const application = loadModule("src/selection-application.ts"), drafts = loadModule("src/drafts.ts");

/** Supplies controlled independent-ID boundaries only for local application tests, not a production SDK or DTO. */
function independentConfigurations() {
  const calls = [], themes = new Set(["theme-one", "theme-two", "group-theme"]), speech = new Set(["tts-one", "tts-two"]);
  const current = { theme: "theme-one", tts: "tts-one" }, faults = new Map();
  /** Models one actual boundary acknowledgement, including an explicitly injected post-write failure. */
  async function apply(type, id) {
    calls.push({ type: "apply-" + type, id }); current[type] = id;
    if (faults.has(type)) { const failure = faults.get(type); faults.delete(type); throw failure; }
  }
  const access = {
    /** Supplies explicit test-only presentation rows, never a production SDK Theme schema or stored values. */
    async readThemeChoices() { return [...themes].map(
      /** Labels each declared independent test ID without creating a replacement role-owned theme. */
      id => ({ id, label: id }),
    ); },
    /** Rejects an absent explicitly declared test theme; no selection mutation is performed by this boundary. */
    async validateTheme(id) { calls.push({ type: "validate-theme", id }); if (!themes.has(id)) throw new Error("Unknown independent theme: " + id); },
    /** Rejects an absent explicitly declared test speech config instead of choosing the current independent one. */
    async validateTts(id) { calls.push({ type: "validate-tts", id }); if (!speech.has(id)) throw new Error("Unknown independent TTS: " + id); },
    /** Applies this exact controlled test ID; the production SDK adapter is not supplied by this fixture. */
    applyTheme: id => apply("theme", id),
    /** Applies this exact controlled test ID without copying any role values to its independent store. */
    applyTts: id => apply("tts", id),
  };
  return { access, calls, current, themes, speech,
    /** Injects a write-then-reject scenario so a test cannot wrongly assert cross-store atomicity. */
    failAfterWrite(type, failure) { assert.equal(faults.has(type), false); faults.set(type, failure); },
  };
}

/** Initializes actual plugin files and a complete record whose independent references are explicitly declared. */
async function environment(t, themeConfigId = "theme-two", ttsConfigId = "tts-two") {
  const disk = await createDiskHarness(t), owner = await loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open();
  const card = await owner.run(
    /** Persists a real plugin role record while keeping the product initial active selection unchanged. */
    repository => repository.createCharacter({ ...plain(drafts.createCharacterDraft()), name: "Actual file application role", themeConfigId, ttsConfigId }),
  );
  const config = independentConfigurations(), coordinator = application.createSelectionApplication(config.access);
  disk.clearCalls();
  return { disk, owner, card, config, coordinator, statePath: path.join(disk.directory, "character-memory/state.json") };
}

/** Runs validation and selection mutation within the actual file owner's serialized record snapshot. */
function commitActive(env, selection = "card:" + env.card.id) {
  return env.coordinator.commit(
    /** Returns only after actual file publication succeeds, with the exact immutable prevalidation receipt. */
    validate => env.owner.run(
      /** Validates every explicit reference before staging the genuine active selection mutation. */
      async repository => {
        const references = await validate(await application.readSelectionReferences(repository, selection));
        const value = selection.startsWith("card:") ? { CharacterCard: { id: selection.slice(5) } } : { CharacterGroup: { id: selection.slice(6) } };
        await repository.writeActive(value); return { references, value };
      },
    ),
  );
}

/** Reads actual published state bytes outside the plugin IO adapter, not a cached or fixture active value. */
async function state(env) { return JSON.parse((await env.disk.stateBytes()).toString("utf8")); }

/** Both independent references must validate before the real selection file is published. */
test("selection application prevalidates all explicit references before file commit and awaits each exact apply", async t => {
  const env = await environment(t); const result = await commitActive(env);
  assert.deepEqual(plain(result), { CharacterCard: { id: env.card.id } });
  assert.deepEqual((await state(env)).active, plain(result));
  assert.deepEqual(env.config.calls, [
    { type: "validate-theme", id: "theme-two" }, { type: "validate-tts", id: "tts-two" },
    { type: "apply-theme", id: "theme-two" }, { type: "apply-tts", id: "tts-two" },
  ]);
  assert.deepEqual(env.config.current, { theme: "theme-two", tts: "tts-two" });
});

/** Unknown IDs cannot silently become null, use global configs or publish a new selection. */
test("unknown theme or speech prevalidation has zero selection writes and preserves exact file bytes", async t => {
  for (const [theme, tts, message] of [["deleted-theme", "tts-two", /Unknown independent theme/], ["theme-two", "deleted-tts", /Unknown independent TTS/]]) {
    const env = await environment(t, theme, tts), before = await env.disk.stateBytes();
    await assert.rejects(commitActive(env), message);
    assert.equal(Buffer.compare(await env.disk.stateBytes(), before), 0);
    assert.equal(env.disk.calls.filter(
      /** Counts genuine publication IO rather than merely inspecting a private staged object. */
      call => call.method === "write" || call.method === "move",
    ).length, 0);
    assert.equal(env.config.calls.some(
      /** Rejects all configuration application when any explicit reference failed its existence check. */
      call => call.type === "apply-theme" || call.type === "apply-tts",
    ), false);
    assert.deepEqual(env.config.current, { theme: "theme-one", tts: "tts-one" });
  }
});

/** A theme owner may persist and then reject; selection remains committed and the failed write stays unverified. */
test("theme apply rejection reports committed selection and the original error without rollback or retry", async t => {
  const env = await environment(t), original = new Error("Independent theme storage acknowledgement failed");
  env.config.failAfterWrite("theme", original);
  await assert.rejects(commitActive(env),
    /** Requires accurate partial metadata and original cause instead of a false atomic transaction claim. */
    failure => {
      assert.equal(failure.name, "SelectionApplicationFailure"); assert.equal(failure.cause, original);
      assert.deepEqual(plain(failure.completed), [{ type: "selection", selection: "card:" + env.card.id }]);
      assert.deepEqual(plain(failure.failedStep), { type: "theme", id: "theme-two" });
      assert.match(failure.message, /选择已提交/); assert.match(failure.message, /实际存储状态待核对/); assert.match(failure.message, /acknowledgement failed/);
      return true;
    },
  );
  assert.deepEqual((await state(env)).active, { CharacterCard: { id: env.card.id } });
  assert.deepEqual(env.config.current, { theme: "theme-two", tts: "tts-one" });
  assert.deepEqual(env.config.calls, [{ type: "validate-theme", id: "theme-two" }, { type: "validate-tts", id: "tts-two" }, { type: "apply-theme", id: "theme-two" }]);
});

/** A later independent speech failure must preserve the already confirmed theme and committed selection. */
test("speech apply rejection reports actual earlier theme success and does not compensate either independent store", async t => {
  const env = await environment(t), original = new Error("Independent TTS storage failed after writing");
  env.config.failAfterWrite("tts", original);
  await assert.rejects(commitActive(env),
    /** Retains the original rejected write while distinguishing confirmed and merely observed test-store state. */
    failure => {
      assert.equal(failure.cause, original);
      assert.deepEqual(plain(failure.completed), [{ type: "selection", selection: "card:" + env.card.id }, { type: "theme", id: "theme-two" }]);
      assert.deepEqual(plain(failure.failedStep), { type: "tts", id: "tts-two" });
      assert.match(failure.message, /主题已确认应用/); assert.match(failure.message, /TTS应用失败/);
      return true;
    },
  );
  assert.deepEqual((await state(env)).active, { CharacterCard: { id: env.card.id } });
  assert.deepEqual(env.config.current, { theme: "theme-two", tts: "tts-two" });
  assert.equal(env.config.calls.length, 4);
});

/** Explicit null means no request to change an independent config, not a replacement for an invalid ID. */
test("explicit null references change only the actual selection and make no config calls", async t => {
  const env = await environment(t, null, null); await commitActive(env);
  assert.deepEqual(env.config.calls, []); assert.deepEqual(env.config.current, { theme: "theme-one", tts: "tts-one" });
  assert.deepEqual((await state(env)).active, { CharacterCard: { id: env.card.id } });
});

/** A role group owns its theme and never reads or applies its members' independent speech or theme refs. */
test("group application uses only its own independent theme and never infers first-member TTS", async t => {
  const env = await environment(t), group = await env.owner.run(
    /** Saves an actual role group whose member references deliberately differ from its own theme. */
    repository => repository.createGroup({ ...plain(drafts.createGroupDraft()), name: "Actual independent group", themeConfigId: "group-theme", members: [{ characterCardId: env.card.id, orderIndex: 0 }] }),
  );
  env.disk.clearCalls(); await commitActive(env, "group:" + group.id);
  assert.deepEqual(env.config.calls, [{ type: "validate-theme", id: "group-theme" }, { type: "apply-theme", id: "group-theme" }]);
  assert.deepEqual(env.config.current, { theme: "group-theme", tts: "tts-one" });
  assert.deepEqual((await state(env)).active, { CharacterGroup: { id: group.id } });
});

/** A selection owner error is propagated unchanged; no independent application is allowed without confirmation. */
test("failed real selection publication propagates the original IO error and performs zero config apply", async t => {
  const env = await environment(t), original = new Error("Actual selection file write rejected"); env.disk.failNext("storage.commit", original);
  await assert.rejects(commitActive(env),
    /** Preserves the genuine file-owner rejection instead of inventing a successful selection receipt. */
    failure => failure === original,
  );
  assert.deepEqual((await state(env)).active, { CharacterCard: { id: "default" } });
  assert.deepEqual(env.config.calls, [{ type: "validate-theme", id: "theme-two" }, { type: "validate-tts", id: "tts-two" }]);
});

/** A complete record with missing required refs is rejected without creating a null field or mutating its owner. */
test("malformed reference plans and mismatched record identities are rejected before any configuration call", async () => {
  const config = independentConfigurations(), coordinator = application.createSelectionApplication(config.access); let commits = 0;
  for (const plan of [{ selection: "card:actual", themeConfigId: undefined, ttsConfigId: null }, { selection: "card:actual", themeConfigId: null, ttsConfigId: undefined }, { selection: "card:actual", themeConfigId: " ", ttsConfigId: null }]) await assert.rejects(coordinator.commit(
    /** Counts only the unreachable mutating stage after the actual validation helper. */
    async validate => { const references = await validate(plan); commits += 1; return { references, value: null }; },
  ));
  assert.equal(commits, 0); assert.deepEqual(config.calls, []);
  await assert.rejects(application.readSelectionReferences({
    /** Supplies an intentionally wrong controlled read identity without defining a production repository. */
    async getCharacter() { return { ...plain(drafts.createCharacterDraft()), id: "other-record" }; },
  }, "card:actual"), /读取结果与选择标识不一致/);
});
