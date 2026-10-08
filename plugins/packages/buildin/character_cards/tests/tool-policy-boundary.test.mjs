import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const root = new URL("../../../../../", import.meta.url);
const paths = {
  manager: "core/crates/tool/services/src/ToolExecutionManager.rs",
  hook: "core/crates/tool/services/src/tools/AIToolHook.rs",
  handler: "core/crates/tool/services/src/tools/AIToolHandler.rs",
  terminal: "core/crates/tool/services/src/tools/AIToolHandlerJsToolsHost.rs",
  lifecycle: "core/crates/runtime/application/src/plugins/toolpkg/ToolPkgToolLifecycleBridge.rs",
  prompt: "core/crates/runtime/application/src/plugins/toolpkg/ToolPkgPromptHookBridge.rs",
  registry: "core/crates/provider/services/src/chat/hooks/PromptHookRegistry.rs",
  enhanced: "core/crates/provider/services/src/chat/EnhancedAIService.rs",
  system: "core/crates/provider/services/src/chat/config/SystemPromptConfig.rs",
  toolsPrompt: "core/crates/provider/services/src/chat/config/SystemToolPrompts.rs",
  sdk: "core/crates/plugin/sdk/src/js_sdk/toolpkg.rs",
  jsRuntime: "core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs",
  jsTools: "core/crates/plugin/javascript-bridge/src/javascript/JsToolManager.rs",
  registration: "core/crates/tool/services/src/tools/ToolRegistration.rs",
  prelude: "core/crates/plugin/sdk/src/JsExecutionScriptBuilder.rs",
};

/** Reads only the assigned production boundaries; this static suite does not execute Rust or prove host integration. */
function sources() {
  const result = {};
  for (const [name, path] of Object.entries(paths)) result[name] = readFileSync(new URL(path, root), "utf8");
  return result;
}

/** Selects an explicitly named source region without guessing plugin business from arbitrary strings. */
function region(source, start, end) {
  const first = source.indexOf(start), last = source.indexOf(end, first + start.length);
  assert.ok(first !== -1 && last > first, "Missing explicit source region: " + start);
  assert.equal(source.indexOf(start, first + start.length), -1, "Duplicate named source boundary: " + start);
  return source.slice(first, last);
}

/** Reports missing owner isolation and invoked-error propagation in the exact current Rust call paths. */
function violations(input) {
  const failures = [];
  const owner = region(input.lifecycle, "fn requireExecutionOwner(", "/// Validates the native configuration snapshot");
  if (!/getToolPkgContainerRuntime\(owner\)/.test(owner) || !/runtime\.packageName\s*!=\s*owner/.test(owner)
    || !/getEnabledToolPkgContainerRuntimes\(\)/.test(owner) || !/0\s*=>\s*Err\(/.test(owner)) failures.push("enabled exact registered owner");
  const projection = region(input.lifecycle, "pub fn executionContextForOwner(", "/// Registers tool lifecycle hooks");
  if (!/requireExecutionOwner\(manager,\s*registeredOwner\)\?/.test(projection)
    || !/validateExecutionSnapshot\(manager,\s*context\)\?/.test(projection)
    || !/context\.extensionOwner\s*==\s*registeredOwner/.test(projection)
    || !/Value::Object\(context\.messageExtension\.clone\(\)\)/.test(projection)
    || !/Value::Null/.test(projection)) failures.push("owner-only message object or explicit null");
  const intercept = region(input.lifecycle, "fn onToolCallInterceptAsync<'a>(", "fn onToolPermissionChecked");
  const prompt = region(input.prompt, "impl ToolPromptComposeHook for ToolPromptComposeBridge", "/// Strictly decodes an invoked tool policy");
  for (const [name, text] of [["intercept", intercept], ["prompt", prompt]]) {
    if (!/currentToolRuntimeContext\(\)/.test(text) || !/readySnapshot\([\s\S]*?\)\.await\?/.test(text)
      || !/validateExecutionSnapshot\([\s\S]*?\)\?/.test(text)
      || !/executionContextForOwner\([\s\S]*?\)\?/.test(text)
      || !/runToolPkgMainHookWithTimeoutMillis\([\s\S]*?\)\.await\?/.test(text)
      || !/remainingTimeoutMillis\(\)\.ok_or_else\(/.test(text) || !/hasExpired\(\)[\s\S]*?return Err\(/.test(text)) failures.push(name + " awaited isolated handler failure/timeout");
  }
  if (!/Future<Output\s*=\s*Result<AIToolHookDecision,\s*String>>/.test(input.hook)
    || !/checkToolInterception\(&invocation\.tool\)\.await\?/.test(input.manager)
    || !/hook\.onToolCallInterceptAsync\(tool\)\.await\?/.test(input.handler)) failures.push("batch aborts original policy error");
  const dispatch = region(input.registry, "pub async fn dispatchToolPromptComposeHooks(", "/// Dispatches prompt-finalize hooks");
  if (!/Result<PromptHookContext,\s*String>/.test(dispatch) || !/hook\.on_event_async\(&current\)\.await\?/.test(dispatch)) failures.push("tool prompt registry error Result");
  const prepare = region(input.enhanced, "pub async fn prepareConversationHistory(", "pub async fn generateSummary(");
  const filter = region(input.enhanced, "async fn applyToolPromptComposeHooksToAvailableTools(", "/// Constructs the native-authenticated tool snapshot");
  for (const [name, text] of [["prepare", prepare], ["filter", filter]]) {
    if (!/toolRuntimeContextForConfiguration\(/.test(text) || !/scopeToolRuntimeContext\(toolContext,/.test(text)) failures.push(name + " independent configuration scope");
  }
  if (!/scopeToolRuntimeContext\(\s*toolRuntimeContext,/.test(input.enhanced)
    || !/messageExtension:\s*configuration\.messageExtension\.clone\(\)/.test(input.enhanced)
    || !/extensionOwner:\s*configuration\.extensionOwner\.clone\(\)/.test(input.enhanced)) failures.push("scheduled turn captures resolved configuration");
  const catalog = region(input.lifecycle, "pub async fn filterToolCatalog(", "/// Accepts only a unique subset");
  if (!/decode_tool_prompt_policy_result\(raw\)\?/.test(catalog)
    || !/validate_catalog_descriptors\(&descriptors,\s*&available\)\?/.test(catalog)
    || !/if descriptor\s*!=\s*original/.test(input.lifecycle)) failures.push("hidden catalog strict immutable descriptor policy");
  return failures;
}

/** Checks actual source wiring only, leaving Rust runtime acceptance explicitly unclaimed. */
test("static: three real tool policy entries retain isolated awaited snapshots and Result failures", () => {
  assert.deepEqual(violations(sources()), []);
});

/** Proves the checker rejects an allow-shaped recovery from the actual intercepted failure. */
test("static negative: removing batch failure propagation is a red item", () => {
  const input = sources();
  input.manager = input.manager.replace("checkToolInterception(&invocation.tool).await?", "checkToolInterception(&invocation.tool).await.unwrap_or(AIToolHookDecision::Allow)");
  assert.ok(violations(input).indexOf("batch aborts original policy error") !== -1);
});

/** Proves a policy registry cannot return unfiltered context by concealing the invoked Result. */
test("static negative: swallowing prompt policy errors is a red item", () => {
  const input = sources();
  input.registry = input.registry.replace("hook.on_event_async(&current).await?", "hook.on_event_async(&current).await.ok().flatten()");
  assert.ok(violations(input).indexOf("tool prompt registry error Result") !== -1);
});

/** Proves loss of the estimate/filter task-local scope cannot be mistaken for a valid non-chat null context. */
test("static negative: unscoped model tool filtering is a red item", () => {
  const input = sources();
  input.enhanced = input.enhanced.replace("let hookContext = ToolExecutionManager::scopeToolRuntimeContext(toolContext,", "let hookContext = dispatchWithoutSnapshot(toolContext,");
  assert.ok(violations(input).indexOf("filter independent configuration scope") !== -1);
});

/** Proves returning the source object's namespace to every registered recipient breaks the isolation boundary. */
test("static negative: removing recipient namespace isolation is a red item", () => {
  const input = sources();
  input.lifecycle = input.lifecycle.replace("context.extensionOwner == registeredOwner", "true");
  assert.ok(violations(input).indexOf("owner-only message object or explicit null") !== -1);
});

/** Verifies all assigned native tool transport paths use the same generic participant name without aliases. */
test("static: participant metadata has one generic reserved transport key and no card aliases", () => {
  const input = sources();
  for (const name of ["manager", "enhanced", "jsRuntime", "jsTools", "registration", "prelude"]) {
    assert.doesNotMatch(input[name], /\b(?:callerCardId|roleCardId|getCallerCardId)\b|__operit_package_caller_card_id/);
  }
  for (const name of ["manager", "jsRuntime", "jsTools", "registration"]) assert.match(input[name], /__operit_package_caller_participant_id/);
  assert.match(input.prelude, /getCallerParticipantId/);
});

/** Keeps non-tool prompt metadata separate from required owner-isolated tool-policy payloads in the authoritative SDK. */
test("static: SDK tool snapshot is typed separately from ordinary prompt metadata", () => {
  const sdk = sources().sdk;
  const ordinary = region(sdk, "pub struct ToolPkgExecutionContext {", "/// Exposes only the receiving registered package");
  assert.doesNotMatch(ordinary, /messageExtension|extensionOwner/);
  const tool = region(sdk, "pub struct ToolPkgToolExecutionContext {", "/// Carries a required isolated snapshot");
  assert.match(tool, /extensionOwner:\s*String/);
  assert.match(tool, /messageExtension:\s*super::JsNullable<ToolPkgJsonObject>/);
  assert.match(sdk, /runtimeContext:\s*super::JsNullable<ToolPkgToolExecutionContext>/);
  assert.match(sdk, /ToolPkgHookEventBase<ToolPkgToolPromptComposeEventName,\s*ToolPkgToolPromptHookEventPayload>/);
});
