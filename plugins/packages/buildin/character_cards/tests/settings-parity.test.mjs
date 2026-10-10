import assert from "node:assert/strict";
import test from "node:test";
import { fixture } from "./fixtures.mjs";
import { loadModule } from "./runtime.mjs";

const page = loadModule("web/features/characters/page.ts");
const character = loadModule("web/features/characters/editor.ts");
const groups = loadModule("web/features/groups/view.ts");
const groupActions = loadModule("web/features/groups/actions.ts");

/** Guards the settings controls against additions absent from the native page before 824dcb7c. */
test("character management retains only original section and row actions", () => {
  const { snapshot } = fixture(), app = { innerHTML: "" };
  page.createPageRenderer({ state: { snapshot, managementView: "characters", avatarSources: new Map(), error: "" }, app, snapshot: () => snapshot }).renderMain();
  const actions = [...app.innerHTML.matchAll(/data-action="([^"]+)"/g)].map(match => match[1]);
  assert.deepEqual([...new Set(actions)].sort(), ["activate-card", "activate-group", "card-graph", "card-user", "create-card", "create-group", "edit-card", "edit-group", "import-card", "import-group"].sort());
  assert.doesNotMatch(app.innerHTML, /刷新|预览群组|class="appbar"|<h1|data-action="memory-settings"/);
  assert.match(app.innerHTML, /<h2>角色卡<\/h2>/);
  assert.match(app.innerHTML, /<h2>群组<\/h2>/);
});

/** Retains the requested theme picker alongside the original binding sections. */
test("role editor binding tab retains model, theme, TTS, memory and tool sections", () => {
  const { snapshot } = fixture(), card = snapshot.cards[1];
  card.themeConfigId = "retained-theme-reference";
  const body = character.characterBody({ card, tab: 2, tags: snapshot.tags, ttsBindingEnabled: false }, snapshot, new Map(), fixture().themeChoices);
  assert.equal([...body.matchAll(/<section class="binding"/g)].length, 5);
  assert.deepEqual([...body.matchAll(/role="switch"[^>]*aria-label="([^"]+)"/g)].map(match => match[1]), ["聊天模型", "TTS 配置", "记忆绑定", "工具访问"]);
  assert.match(body, /主题配置/);
  assert.match(body, /data-action="select-theme"/);
  assert.match(body, /retained-theme-reference/);
  assert.equal(card.themeConfigId, "retained-theme-reference");
});

/** Matches old group rows: members as subtitle, a count badge, and activate as the sole side action. */
test("group tile drops extra preview, edit, description and member-name badge", () => {
  const { snapshot } = fixture(), group = snapshot.groups[0];
  const body = groups.groupTile(group, snapshot);
  assert.match(body, /class="entity-subtitle">Operit、旅行助理<\/div>/);
  assert.equal([...body.matchAll(/class="badge"/g)].length, 1);
  assert.doesNotMatch(body, /preview-group|预览群组|aria-label="编辑群组"|多个角色一起交流/);
  assert.deepEqual([...body.matchAll(/data-action="([^"]+)"/g)].map(match => match[1]), ["edit-group", "activate-group"]);
});

/** Groups retain the theme picker, name, description and member checkboxes without notices or reordering. */
test("group editor retains theme controls but omits notices and ordering actions", () => {
  const { snapshot } = fixture(), group = snapshot.groups[0];
  group.themeConfigId = "retained-group-theme";
  const body = groups.groupBody({ group }, snapshot, fixture().themeChoices);
  assert.equal([...body.matchAll(/class="field[ "]/g)].length, 2);
  assert.equal([...body.matchAll(/type="checkbox"/g)].length, snapshot.cards.length);
  assert.match(body, /data-action="select-theme"/);
  assert.match(body, /retained-group-theme/);
  assert.doesNotMatch(body, /notice|member-up|member-down/);
  const feature = groupActions.createGroupsActionsFeature({});
  assert.deepEqual(Array.from(feature.names), ["activate-group", "create-group", "edit-group", "save-group", "delete-group"]);
  assert.equal(group.themeConfigId, "retained-group-theme");
});
