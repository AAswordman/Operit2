import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, composeNodes, keyedNode } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";
const themes = [{ id: "independent-theme-one", name: "独立日间主题" }, { id: "independent-theme-two", name: "独立夜间主题" }];
async function click(ui, key, prop = "onClick", payload) { return ui.dispatch(keyedNode(ui.render().tree, key).props[prop], payload); }
async function open(plugin, entity, id) {
  const ui = mountRegisteredComposeRoute(plugin, { requestId: "theme-native", input: { mode: "edit", entity, id } }, "main");
  await ui.load(); await new Promise(resolve => setImmediate(resolve)); return ui;
}

test("one native theme picker searches exact independent IDs and stages the owner reference only until save", async t => {
  const plugin = await selectorEnvironment(t, {Theme: { async listConfigs() { return { configs: themes }; }
  }});
  for (const entity of ["card", "group"]) {
    const api = entity === "card" ? "character" : "group", record = await plugin.api(api + ".create", { values: { name: "独立主题测试" } });
    let ui = await open(plugin, entity, record.id);
    await click(ui, entity === "card" ? "entity-theme-switch" : "entity-theme-picker", entity === "card" ? "onCheckedChange" : "onClick", true);
    assert.equal(composeNodes(ui.render().tree, node => node.type === "Dialog").length, 1);
    await click(ui, "entity-field-picker-query", "onValueChange", "夜间");
    assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-picker-independent-theme-one").length, 0);
    await click(ui, "entity-picker-independent-theme-two");
    assert.equal((await plugin.api(api + ".get", { id: record.id })).themeConfigId, null);
    await click(ui, "entity-cancel");
    assert.equal((await plugin.api(api + ".get", { id: record.id })).themeConfigId, null);
    ui = await open(plugin, entity, record.id);
    await click(ui, entity === "card" ? "entity-theme-switch" : "entity-theme-picker", entity === "card" ? "onCheckedChange" : "onClick", true);
    await click(ui, "entity-picker-independent-theme-one"); await click(ui, "entity-primary");
    const saved = await plugin.api(api + ".get", { id: record.id });
    assert.equal(saved.themeConfigId, "independent-theme-one");
    if (entity === "group") { assert.deepEqual(plain(saved.members), []); assert.equal(saved.ttsConfigId, undefined); }
  }
});

test("missing theme references stay visible; picker cancellation preserves the original record", async t => {
  const plugin = await selectorEnvironment(t, {Theme: { async listConfigs() { return { configs: themes }; }
  }});
  const card = await plugin.api("character.create", { values: { name: "失效绑定", themeConfigId: "deleted-theme" } });
  const ui = await open(plugin, "card", card.id);
  assert.ok(composeNodes(ui.render().tree, node => String(node.props.text ?? "").includes("deleted-theme")).length);
  await click(ui, "entity-theme-picker"); await click(ui, "entity-cancel"); await click(ui, "entity-cancel");
  assert.equal((await plugin.api("character.get", { id: card.id })).themeConfigId, "deleted-theme");
});

test("pending theme directory IO does not lock cancellation and late completion cannot reopen the native editor", async t => {
  for (const fail of [false, true]) {
    let resolve, reject; const pending = new Promise((yes, no) => { resolve = yes; reject = no; });
    const plugin = await selectorEnvironment(t, {Theme: { listConfigs() { return pending; }
  }});
    const card = await plugin.api("character.create", { values: { name: "异步取消" } });
    const ui = await open(plugin, "card", card.id);
    assert.equal(keyedNode(ui.render().tree, "entity-cancel").props.enabled, true);
    const result = await click(ui, "entity-cancel"); assert.equal(result.actionResult.type, "toolpkg.presentation.cancel");
    if (fail) reject(new Error("late theme error")); else resolve({ configs: themes });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-primary").length, 0);
    assert.equal((await plugin.api("character.get", { id: card.id })).themeConfigId, null);
  }
});

test("the single native picker receives its pending real catalogue while keeping cancellation available", async t => {
  let resolve; const pending = new Promise(yes => { resolve = yes; });
  const plugin = await selectorEnvironment(t, {Theme: { listConfigs() { return pending; }
  }});
  const card = await plugin.api("character.create", { values: { name: "主题异步加载" } });
  const ui = await open(plugin, "card", card.id);
  await click(ui, "entity-theme-switch", "onCheckedChange", true);
  assert.ok(composeNodes(ui.render().tree, node => node.props.text === "正在读取主题配置…").length);
  assert.equal(keyedNode(ui.render().tree, "entity-cancel").props.enabled, true);
  resolve({ configs: themes }); await new Promise(resolve => setImmediate(resolve));
  await click(ui, "entity-picker-independent-theme-two"); await click(ui, "entity-primary");
  assert.equal((await plugin.api("character.get", { id: card.id })).themeConfigId, "independent-theme-two");
});
