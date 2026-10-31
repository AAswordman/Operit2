import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { composeNodes, keyedNode, mountRegisteredComposeRoute } from "./selector-render.mjs";
import { fixture } from "./fixtures.mjs";
import { plain } from "./runtime.mjs";

/** Leaves genuine first-install records without metadata, as the old failed writer did. */
async function interrupted(t, { ownerMissing = false, schema = 1 } = {}) {
  const runtime = await selectorEnvironment(t), { disk } = runtime;
  const before = disk.readState(), id = before.cards[0].id;
  before.cards[0].name = "Keep my existing character";
  before.cards[0].characterSetting = "Original authored content";
  before.owners[0].settings.autoSaveIntervalMinutes = 23;
  disk.replaceState(before);
  disk.storage.inspectPreferences("meta", values => {
    delete values.state;
    values.__operit_preferences_schema_version = schema;
  });
  if (ownerMissing) disk.storage.inspect(sql => sql.prepare("DELETE FROM records WHERE collection='owners'").run());
  const file = disk.directory + "/" + before.owners[0].userDocumentPath;
  await disk.files.write(file, "# Existing user profile\nMust not be replaced");
  return { ...runtime, before: plain(before), id, file };
}

/** Captures actual user-facing inspection without synthesizing a repair source. */
async function inspect(runtime) {
  const result = plain(await runtime.recovery({ action: "inspect" }));
  assert.equal(result.status, "repairable"); return result.source;
}

test("inspection remains read-only and reachable after lifecycle and domain initialization reject", async t => {
  const runtime = await interrupted(t), { disk } = runtime;
  await assert.rejects(runtime.main.onServiceInitialize({}), /unplanned configuration/);
  await assert.rejects(runtime.api("character.list", {}), /unplanned configuration/);
  disk.clearCalls();
  const source = await inspect(runtime);
  assert.equal(source.card.id, runtime.id);
  assert.equal(source.card.characterSetting, "Original authored content");
  assert.equal(disk.calls.some(call => ["storage.preferences.edit", "storage.commit", "write", "deleteFile", "move"].includes(call.method)), false);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.initialization), undefined);
});

test("approved plugin repair preserves existing card, settings and USER.md and revives the same runtime", async t => {
  const runtime = await interrupted(t), { disk } = runtime;
  await assert.rejects(runtime.main.onServiceInitialize({}), /unplanned configuration/);
  const source = await inspect(runtime);
  assert.deepEqual(plain(await runtime.recovery({ action: "repair", source })), { status: "ready" });
  const cards = await runtime.api("character.list", {});
  assert.equal(cards.length, 1);
  assert.deepEqual(plain(cards[0]), runtime.before.cards[0]);
  assert.equal(disk.readState().active.CharacterCard.id, runtime.id);
  assert.equal(disk.readState().owners[0].settings.autoSaveIntervalMinutes, 23);
  assert.equal((await disk.files.read(runtime.file)).content, "# Existing user profile\nMust not be replaced");
  assert.equal(disk.storage.inspectPreferences("meta", values => values.initialization), undefined);
  assert.equal((await runtime.web({ action: "snapshot" })).cards[0].id, runtime.id);
});

test("explicit repair fills an absent owner and USER.md without replacing the existing card", async t => {
  const runtime = await interrupted(t, { ownerMissing: true });
  await runtime.disk.files.deleteFile(runtime.file, false);
  const source = await inspect(runtime);
  assert.equal(source.owner, null);
  await runtime.recovery({ action: "repair", source });
  assert.equal(runtime.disk.readState().owners[0].ownerKey, "character:" + runtime.id);
  assert.equal((await runtime.disk.files.read(runtime.file)).content, "");
  assert.deepEqual(plain((await runtime.api("character.list", {}))[0]), runtime.before.cards[0]);
});

test("a stale user confirmation cannot overwrite a changed card", async t => {
  const runtime = await interrupted(t), source = await inspect(runtime);
  runtime.disk.storage.inspectPreferences("cards", values => { values["record_" + runtime.id].name = "Concurrent edit"; });
  await assert.rejects(runtime.recovery({ action: "repair", source }), /数据已在检查后发生变化/);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.initialization), undefined);
  assert.equal(runtime.disk.storage.inspectPreferences("cards", values => values["record_" + runtime.id].name), "Concurrent edit");
});

test("a stale user confirmation cannot overwrite changed owner settings", async t => {
  const runtime = await interrupted(t), source = await inspect(runtime);
  const owner = { ...source.owner, settings: { ...source.owner.settings, autoSaveIntervalMinutes: 27 } };
  runtime.disk.storage.inspect(sql => {
    sql.prepare("UPDATE records SET value=? WHERE collection='owners'").run(JSON.stringify(owner));
  });
  await assert.rejects(runtime.recovery({ action: "repair", source }), /数据已在检查后发生变化/);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.initialization), undefined);
});

test("concurrent repair requests cannot create two repository owners", async t => {
  const runtime = await interrupted(t), source = await inspect(runtime);
  await assert.rejects(runtime.main.onServiceInitialize({}), /unplanned configuration/);
  const outcomes = await Promise.allSettled([
    runtime.recovery({ action: "repair", source }),
    runtime.recovery({ action: "repair", source }),
  ]);
  assert.equal(outcomes.filter(result => result.status === "fulfilled").length, 1);
  const rejected = outcomes.find(result => result.status === "rejected");
  assert.match(String(rejected.reason), /已有初始化或修复请求正在执行/);
  assert.equal((await runtime.api("character.list", {}))[0].id, runtime.id);
});

test("repair refuses multiple cards rather than guessing which user data to take over", async t => {
  const runtime = await interrupted(t);
  runtime.disk.storage.inspectPreferences("cards", values => { values.record_other = { ...values["record_" + runtime.id], id: "other" }; });
  await assert.rejects(runtime.recovery({ action: "inspect" }), /恰好一张角色卡/);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(runtime.disk.storage.inspectPreferences("cards", values => Object.keys(values).length), 2);
});

test("a failed explicit repair retains a plan and the user can resume it from the plugin", async t => {
  const runtime = await interrupted(t, { ownerMissing: true });
  await runtime.disk.files.deleteFile(runtime.file, false);
  await assert.rejects(runtime.main.onServiceInitialize({}), /unplanned configuration/);
  const source = await inspect(runtime);
  runtime.disk.failNext("write", new Error("Interrupted explicit repair"));
  await assert.rejects(runtime.recovery({ action: "repair", source }), /Interrupted explicit repair/);
  const pending = await runtime.recovery({ action: "inspect" });
  assert.equal(pending.status, "planned");
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state), undefined);
  await runtime.recovery({ action: "resume" });
  assert.equal((await runtime.api("character.list", {}))[0].id, runtime.id);
});

test("repair does not poison a healthy retained service", async t => {
  const runtime = await interrupted(t), source = await inspect(runtime);
  await runtime.recovery({ action: "repair", source });
  await assert.rejects(runtime.recovery({ action: "repair", source }), /已经正常运行/);
  assert.equal((await runtime.api("character.list", {}))[0].id, runtime.id);
});

test("repair establishes the schema marker before publishing state and preserves old identifiers", async t => {
  const runtime = await interrupted(t, { schema: 0 }), source = await inspect(runtime);
  await runtime.recovery({ action: "repair", source });
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.__operit_preferences_schema_version), 1);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state.legacyImportPending), undefined);
  assert.equal((await runtime.api("character.list", {}))[0].id, runtime.id);
});

test("the real registered management screen offers inspection and requires consent before repair", async t => {
  const runtime = await interrupted(t);
  let bridge;
  const ui = mountRegisteredComposeRoute(runtime, null, "main", {}, request => {
    if (request.command === "addJavascriptInterface") bridge = request.payload.object;
  });
  await ui.dispatch({ __actionId: "__operit_theme_changed" }, plain(fixture().theme));
  await ui.load();
  await assert.rejects(ui.dispatch(bridge.loadScreen, []), /unplanned configuration/);
  const failed = ui.render().tree;
  assert.equal(failed.props.key, "initialization-recovery-panel");
  assert.equal(composeNodes(failed, node => node.type === "WebView").length, 0);
  await ui.dispatch(keyedNode(failed, "initialization-inspect").props.onClick);
  const inspected = ui.render().tree;
  assert.equal(keyedNode(inspected, "initialization-repair").props.enabled, false);
  assert.ok(composeNodes(inspected, node => node.props.text === runtime.before.cards[0].name).length > 0);
  assert.equal(runtime.disk.storage.inspectPreferences("meta", values => values.state), undefined);
  await ui.dispatch(keyedNode(inspected, "initialization-confirm").props.onCheckedChange, true);
  const confirmed = ui.render().tree;
  assert.equal(keyedNode(confirmed, "initialization-repair").props.enabled, true);
  await ui.dispatch(keyedNode(confirmed, "initialization-repair").props.onClick);
  const restored = ui.render().tree;
  assert.equal(restored.props.key, "character-memory-editor");
  await ui.dispatch(restored.props.onLoad);
  const bootstrap = await ui.dispatch(bridge.loadScreen, []);
  assert.equal(bootstrap.actionResult.snapshot.cards[0].id, runtime.id);
  assert.equal((await runtime.api("character.list", {}))[0].id, runtime.id);
});
