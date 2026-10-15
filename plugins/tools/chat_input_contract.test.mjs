import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

/** Reads authoritative workspace files without loading generated plugin bundles. */
function source(relative) {
  return readFileSync(new URL(relative, import.meta.url), "utf8");
}

/** Keeps the SDK declaration and runtime on the original exact action spelling. */
test("chat input SDK and runtime preserve PascalCase actions", () => {
  const actions = ["Allow", "Block", "Replace", "Consume"];
  const rust = source("../../core/crates/plugin/sdk/src/js_sdk/toolpkg.rs");
  const enumBody = rust.match(/pub enum ToolPkgChatInputHookObjectResultAction \{([^}]+)\}/)[1];
  assert.doesNotMatch(enumBody, /serde/);
  assert.doesNotMatch(rust, /#\[serde\([^\n]+\)\]\s*pub enum ToolPkgChatInputHookObjectResultAction/);
  assert.deepEqual(enumBody.trim().split(/,\s*/).filter(Boolean), actions);
  const declarations = source("../types/toolpkg.d.ts");
  const declaredActions = declarations.match(/export type ChatInputHookObjectResultAction = ([^;]+);/)[1];
  assert.deepEqual([...declaredActions.matchAll(/"([^"\n]+)"/g)].map(
    /** Extracts only the exact declared protocol literals. */
    match => match[1],
  ), actions);
  const bridge = source("../../core/crates/runtime/application/src/plugins/toolpkg/ToolPkgChatInputHookBridge.rs");
  for (const action of actions) {
    assert.match(bridge, new RegExp(`pub const CHAT_INPUT_SUBMIT_ACTION_${action.toUpperCase()}: &str = "${action}";`));
  }
  assert.match(bridge, /Unknown chat input action/);
});
