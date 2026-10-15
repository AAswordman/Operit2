import assert from "node:assert/strict";
import test from "node:test";
import { fixture } from "./fixtures.mjs";
import { loadModule } from "./runtime.mjs";

const page = loadModule("web/features/characters/page.ts");
const groups = loadModule("web/features/groups/tiles.ts");

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

/** Matches old group rows: members as subtitle, a count badge, and activate as the sole side action. */
test("group tile drops extra preview, edit, description and member-name badge", () => {
  const { snapshot } = fixture(), group = snapshot.groups[0];
  const body = groups.groupTile(group, snapshot);
  assert.match(body, /class="entity-subtitle">Operit、旅行助理<\/div>/);
  assert.equal([...body.matchAll(/class="badge"/g)].length, 1);
  assert.doesNotMatch(body, /preview-group|预览群组|aria-label="编辑群组"|多个角色一起交流/);
  assert.deepEqual([...body.matchAll(/data-action="([^"]+)"/g)].map(match => match[1]), ["edit-group", "activate-group"]);
});
