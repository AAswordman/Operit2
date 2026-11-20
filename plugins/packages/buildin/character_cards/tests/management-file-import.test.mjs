import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, composeNodes, keyedNode } from "./selector-render.mjs";
import { buildUiScreenScripts } from "../scripts/build.mjs";
import { plain } from "./runtime.mjs";
const modules = await buildUiScreenScripts();

/** Invokes the actual registered management callback, including the add menu. */
async function action(ui, key, payload) {
  if (key === "import-card") return ui.dispatch(keyedNode(ui.render().tree, "add-card").props.onSelect, 1);
  if (key === "import-group") return ui.dispatch(keyedNode(ui.render().tree, "add-group").props.onSelect, 1);
  const node = keyedNode(ui.render().tree, key);
  return ui.dispatch(node.props.onClick, payload);
}

/** Opens management against real plugin storage with only the host picker controlled. */
async function environment(t) {
  let selected = null, pickerOptions;
  const plugin = await selectorEnvironment(t, { Theme: { async listConfigs() { return { configs: [] }; } } }, {
    async __operitNativeComposeFilePickerCommand(payload) {
      pickerOptions = plain(payload.options);
      return selected === null ? { cancelled: true, files: [] } : { cancelled: false, files: [{ path: selected, uri: selected, size: 100 }] };
    },
  }, modules);
  const ui = mountRegisteredComposeRoute(plugin, null, "main"); await ui.load();
  return { plugin, ui, select(path) { selected = path; }, pickerOptions() { return pickerOptions; } };
}

for (const format of ["operit", "tavern"]) test(`native ${format} character import selects a JSON file and cancellation leaves storage unchanged`, async t => {
  const env = await environment(t), { plugin, ui } = env;
  const card = await plugin.api("character.create", { values: { name: "文件导入角色", description: "保留角色设定" } });
  const content = await plugin.web({ action: "exportCharacter", id: card.id, format });
  await plugin.api("character.delete", { id: card.id }); await ui.load();
  await action(ui, "import-card");
  if (format === "tavern") await action(ui, "import-format", 1);
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "import-content").length, 0);
  const before = plain(await plugin.web({ action: "snapshot" }));
  await action(ui, "import-submit");
  assert.deepEqual(plain(await plugin.web({ action: "snapshot" })), before);
  keyedNode(ui.render().tree, "management-dialog");
  assert.deepEqual(env.pickerOptions(), { picker: "document", allowMultiple: false, mimeTypes: ["application/json"] });
  const path = plugin.disk.directory + "/character.json";
  await plugin.disk.files.write(path, "\uFEFF" + content, false); env.select(path);
  await action(ui, "import-submit");
  const after = await plugin.web({ action: "snapshot" });
  const imported = after.cards.find(item => !before.cards.some(previous => previous.id === item.id));
  assert.equal(after.cards.length, before.cards.length + 1, JSON.stringify(composeNodes(ui.render().tree, node => node.type === "Text").map(node => node.props.text)));
  assert.equal(imported.name, card.name); assert.equal(imported.description, card.description);
  keyedNode(ui.render().tree, "card:" + imported.id + "-open");
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "management-dialog").length, 0);
});

test("native group file import preserves the dialog on invalid or empty JSON and allows retry", async t => {
  const env = await environment(t), { plugin, ui } = env;
  const card = await plugin.api("character.create", { values: { name: "群组成员" } });
  const group = await plugin.api("group.create", { values: { name: "文件导入群组", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const content = await plugin.web({ action: "exportGroup", id: group.id });
  await plugin.api("group.delete", { id: group.id }); await ui.load();
  const before = plain(await plugin.web({ action: "snapshot" }));
  await action(ui, "import-group");
  const path = plugin.disk.directory + "/group.json"; env.select(path);
  for (const invalid of ["\uFEFF \n", "{invalid", "{}"]) {
    await plugin.disk.files.write(path, invalid, false);
    await action(ui, "import-submit");
    assert.deepEqual(plain(await plugin.web({ action: "snapshot" })), before);
    keyedNode(ui.render().tree, "import-submit");
    assert.ok(composeNodes(keyedNode(ui.render().tree, "management-dialog"), node => node.type === "Text" && String(node.props.text).includes("Error")).length > 0);
  }
  await plugin.disk.files.write(path, content, false); await action(ui, "import-submit");
  const after = await plugin.web({ action: "snapshot" });
  const imported = after.groups.find(item => !before.groups.some(previous => previous.id === item.id));
  assert.equal(after.groups.length, before.groups.length + 1, JSON.stringify(composeNodes(ui.render().tree, node => node.type === "Text").map(node => node.props.text)));
  assert.equal(imported.name, group.name); assert.deepEqual(plain(imported.members), plain(group.members));
  keyedNode(ui.render().tree, "group:" + imported.id + "-open");
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "management-dialog").length, 0);
});
