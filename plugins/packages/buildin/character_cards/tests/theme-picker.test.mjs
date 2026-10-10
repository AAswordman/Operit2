import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

const drafts = loadModule("src/drafts.ts"), view = loadModule("web/features/characters/bindings/theme.ts");
const state = loadModule("web/features/characters/bindings/theme.ts"), actions = loadModule("web/features/characters/bindings/theme.ts");
const choices = [{ id: "independent-theme-one", label: "独立日间主题" }, { id: "independent-theme-two", label: "独立夜间主题" }];

/** Provides explicit UI-only editor state, not a Theme SDK result, production host or stored catalog. */
function dialog(kind, selected = null) {
  const parent = kind === "character" ? { type: "character", card: plain(drafts.createCharacterDraft()) } : { type: "group", group: plain(drafts.createGroupDraft()) };
  const result = { type: "theme", parent, choices: plain(choices), query: "", error: "", scroll: 0, busy: false };
  state.themeDraft(result).themeConfigId = selected; return result;
}

/** Both editor kinds use the same independent-ID picker without producing legacy role-prefixed targets. */
test("pure theme picker view renders exact independent IDs for character and own-group drafts", () => {
  for (const kind of ["character", "group"]) {
    const current = dialog(kind, "independent-theme-two"), spec = plain(view.themeSpec(current));
    assert.equal(spec.title, "选择主题配置"); assert.equal(spec.style, "chooser");
    assert.match(spec.body, /data-id="independent-theme-one"/); assert.match(spec.body, /data-id="independent-theme-two"/);
    assert.match(spec.body, /aria-label="不改变当前主题"/); assert.match(spec.footer, /取消/);
    assert.doesNotMatch(spec.body, /role:|card:|group:/); assert.equal(state.themeDraft(current).themeConfigId, "independent-theme-two");
  }
});

/** A deleted reference remains an explicit visible error, not an implicitly selected null or unrelated theme. */
test("pure theme picker reports a saved missing reference and searches labels without editing the owner", () => {
  const current = dialog("character", "deleted-independent-theme"); current.query = "夜间";
  const spec = view.themeSpec(current); assert.match(spec.body, /role="alert"/); assert.match(spec.body, /deleted-independent-theme/);
  assert.doesNotMatch(spec.body, /data-id="independent-theme-one"/); assert.match(spec.body, /data-id="independent-theme-two"/);
  assert.equal(current.parent.card.themeConfigId, "deleted-independent-theme");
});

/** Selecting in an editor stages a reference only, and all malformed or unknown choices fail before draft mutation. */
test("pure theme picker stages one exact reference or explicit null and rejects invalid catalog IDs", () => {
  for (const kind of ["character", "group"]) {
    const current = dialog(kind, "independent-theme-one");
    assert.throws(
      /** Requires a real entry in the explicitly supplied UI catalog before changing this editor draft. */
      () => state.chooseThemeReference(current, "missing-theme"), /主题配置不存在/,
    );
    assert.equal(state.themeDraft(current).themeConfigId, "independent-theme-one");
    state.chooseThemeReference(current, "independent-theme-two"); assert.equal(state.themeDraft(current).themeConfigId, "independent-theme-two");
    state.chooseThemeReference(current, null); assert.equal(state.themeDraft(current).themeConfigId, null);
  }
  for (const malformed of [[{ id: "", label: "bad" }], [{ id: "same", label: "first" }, { id: "same", label: "second" }], [{ id: "present", label: "" }]]) assert.throws(
    /** Keeps a bad projected directory explicit rather than inventing an empty or global choice. */
    () => state.assertThemeChoices(malformed),
  );
});

/** Real host capability wiring is intentionally outside this UI-only dependency test; a reader rejection remains the original failure. */
test("pure picker action awaits its explicit reader and never substitutes a successful catalog on rejection", async () => {
  const current = dialog("character", "independent-theme-one"), stack = [current.parent], original = new Error("Explicit UI catalog reader rejection");
  let calls = 0;
  const context = {
    /** Supplies only this test's real local parent editor, not a business or configuration service. */
    topDialog() { return stack[stack.length - 1]; },
    /** Records an actual typed feature request to open its local chooser. */
    pushDialog(value) { stack.push(value); },
  };
  const failed = actions.createThemeFeature(context,
    /** Represents an explicitly planned external reader error, not an available production Theme SDK. */
    async () => { calls += 1; throw original; },
  );
  await assert.rejects(failed.handleAction("select-theme", {}),
    /** Requires exact original error identity and no successful or empty substitute response. */
    failure => failure === original,
  );
  assert.equal(calls, 1); assert.equal(stack.length, 1); assert.equal(current.parent.card.themeConfigId, "independent-theme-one");
  const loaded = actions.createThemeFeature(context,
    /** Supplies labelled private presentation rows only for this isolated feature test. */
    async () => { calls += 1; return plain(choices); },
  );
  await loaded.handleAction("select-theme", {}); assert.equal(calls, 2); assert.equal(stack.length, 2); assert.equal(stack[1].type, "theme");
  assert.equal(current.parent.card.themeConfigId, "independent-theme-one");
});

/** Resolves bound names from the actual directory and keeps missing references visible without rewriting them. */
test("binding summary shows the configured name, missing-reference error and explicit unbind action", () => {
  assert.match(view.themeReferenceSection("independent-theme-two", choices), /独立夜间主题/);
  assert.match(view.themeReferenceSection("independent-theme-two", choices), /data-action="unbind-theme"/);
  assert.match(view.themeReferenceSection("deleted-theme", choices), /role="alert"[\s\S]*deleted-theme/);
  assert.doesNotMatch(view.themeReferenceSection(null, choices), /data-action="unbind-theme"/);
  const current = dialog("character"); current.choices = [];
  assert.match(view.themeSpec(current).body, /设置 → 外观/);
});

/** Clears only the owning editor's staged reference; it never changes another group, role or host appearance. */
test("explicit unbind edits only the current role or group draft", async () => {
  for (const kind of ["character", "group"]) {
    const current = dialog(kind, "independent-theme-one"); let renders = 0;
    const feature = actions.createThemeFeature({
      /** Returns this exact local owner rather than another selected participant. */
      topDialog() { return current.parent; },
      /** Records repainting after the staged reference has changed. */
      renderDialogs() { renders += 1; },
    },
      /** Rejects an unsolicited directory read during an explicit unbind. */
      async () => { throw new Error("Unbind must not read or apply any configuration"); },
    );
    await feature.handleAction("unbind-theme", {});
    assert.equal(state.themeDraft(current).themeConfigId, null); assert.equal(renders, 1);
  }
});
