import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
/** Reads current production source rather than using a saved generated or restored file snapshot. */
function source(relative) { return readFileSync(new URL(relative, root), "utf8").replace(/\r\n/g, "\n"); }
/** Reads actual public struct fields for lossless canonical-to-SDK schema comparison. */
function fields(text, name) {
  const declaration = new RegExp("pub struct " + name + "\\s*\\{([^}]+)\\}").exec(text);
  assert.notEqual(declaration, null, name);
  return [...declaration[1].matchAll(/pub ([A-Za-z_][A-Za-z0-9_]*):/g)].map(match => match[1]);
}
/** Evaluates exactly the current generated runtime helper with an explicit test native callback boundary. */
function directoryRuntime(native) {
  const generator = source("core/crates/plugin/codegen/src/runtime_bindings.rs");
  const start = generator.indexOf("/** Reads one declared SoftwareSettings directory through the existing typed host callback bridge. */");
  const marker = "globalThis.__operitSoftwareDirectorySequence = 0;", end = generator.indexOf(marker, start);
  assert.ok(start >= 0 && end > start);
  const context = vm.createContext({
    /** Runs only the simulated native transport boundary, never a substitute production manager. */
    __operitNativeReadSoftwareSettingsDirectoryAsync(id, method) { native(context, id, method); },
  });
  vm.runInContext(generator.slice(start, end + marker.length), context);
  return context;
}

/** Proves every declared settings capability uses a real narrow Rust host binding rather than a Tools or CLI dispatcher. */
test("software directories connect SDK declaration generator callback and actual canonical runtime managers", () => {
  const methods = ["listModelSummaries", "listTtsConfigs", "listThemeConfigs", "applyThemeConfig", "getCurrentTtsConfigId", "setCurrentTtsConfigId", "readToolSourceCatalog"];
  const bindings = source("core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs");
  const direct = bindings.slice(bindings.indexOf("pub const JS_DIRECT_HOST_BINDINGS"));
  assert.deepEqual([...direct.matchAll(/namespace: "SoftwareSettings", method: "([^"]+)"/g)].map(match => match[1]), methods);
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/software_settings.rs"), runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const engine = source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs"), native = source("core/crates/tool/services/src/tools/AIToolHandlerJsToolsHost.rs");
  for (const method of methods) {
    assert.match(sdk, new RegExp("fn " + method + "(?:<[^>]+>)?\\("));
    assert.match(runtime, new RegExp("fn " + method + "(?:<[^>]+>)?\\("));
    const argument = method === "applyThemeConfig" || method === "setCurrentTtsConfigId" ? "id" : "";
    assert.match(engine, new RegExp("executionHost\\s*\\.\\s*" + method + "\\(" + argument + "\\)"));
    assert.match(native, new RegExp("support\\s*\\.\\s*" + method + "\\(" + argument + "\\)"));
  }
  assert.match(runtime, /getAllModelSummaries\(\)/); assert.match(runtime, /getAllTtsConfigs\(\)/);
  for (const call of ["getAllToolNames", "getAvailablePackages", "getAvailableSkillsSnapshot", "getAvailableServerPackages"]) assert.match(runtime, new RegExp(call + "\\("));
  const directoryMethod = native.slice(native.indexOf("fn invoke_software_settings_directory"));
  assert.doesNotMatch(directoryMethod, /invoke_generated|execute_tool_call|SoftwareSettings\.exec|unwrap_or_default/);
  assert.match(engine, /"__operitNativeReadSoftwareSettingsDirectoryAsync"/);
});

/** Confirms the wire DTOs preserve every field from the actual model and speech records. */
test("directory SDK DTOs are complete canonical model and TTS records", () => {
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/software_settings.rs");
  const models = source("core/crates/foundation/model/src/ModelConfigData.rs"), tts = source("core/crates/foundation/model/src/TtsConfig.rs");
  assert.deepEqual(fields(sdk, "SoftwareModelSummary"), fields(models, "ProviderModelSummary"));
  assert.deepEqual(fields(sdk, "SoftwareModelCapabilities"), fields(models, "ModelCapabilities"));
  assert.deepEqual(fields(sdk, "SoftwareModelPricing"), fields(models, "ModelPricing"));
  assert.deepEqual(fields(sdk, "SoftwareTtsConfig"), fields(tts, "TtsConfig"));
  assert.deepEqual(fields(sdk, "SoftwareTtsHeader"), fields(tts, "TtsHttpHeader"));
  assert.deepEqual(fields(sdk, "SoftwareTtsResponseStep"), fields(tts, "TtsHttpResponsePipelineStep"));
});

/** Exercises the exact current generated Promise helper, independently of unavailable Rust compilation. */
test("software directory callback preserves full JSON and releases every callback identity", async () => {
  const values = new Map([["listModelSummaries", [{ providerId: "provider", modelId: "model", pricing: null }]], ["listTtsConfigs", [{ id: "speech", requestBody: "full", responsePipeline: [{ stepType: "json", path: "audio", headers: [{ name: "custom", value: "original" }] }] }]], ["readToolSourceCatalog", { builtinTools: [{ name: "read_file", displayName: "Read", description: "configured" }], packages: [{ name: "actual_package", displayName: "Package", description: "full" }], skills: [{ name: "actual_skill", displayName: "Skill", description: "full" }], mcpServers: [{ name: "actual_server", displayName: "MCP", description: "full" }] }]]);
  const calls = [], context = directoryRuntime(
    /** Supplies explicit unit-test native callback values without implementing a production host store. */
    (runtime, id, method) => { assert.ok(values.has(method)); calls.push(id); runtime[id](JSON.stringify(values.get(method)), false); },
  );
  for (const [method, value] of values) assert.deepEqual(JSON.parse(JSON.stringify(await context.__operitReadSoftwareSettingsDirectory(method))), value);
  assert.equal(new Set(calls).size, 3);
  for (const id of calls) assert.equal(Object.hasOwn(context, id), false);
});

/** Checks both asynchronous native failures and synchronous transport failures are retained without substitute data. */
test("software directory Promise rejects original host and transport errors", async () => {
  const context = directoryRuntime(
    /** Delivers one explicit host failure through the actual generated error envelope. */
    (runtime, id) => runtime[id](JSON.stringify({ message: " unchanged host failure " }), true),
  );
  await assert.rejects(context.__operitReadSoftwareSettingsDirectory("listTtsConfigs"), { message: " unchanged host failure " });
  const failure = new Error("native transport original"), broken = directoryRuntime(
    /** Rejects transport before any callback completion rather than inventing successful data. */
    () => { throw failure; },
  );
  await assert.rejects(broken.__operitReadSoftwareSettingsDirectory("readToolSourceCatalog"), error => error === failure);
  assert.equal(Object.hasOwn(broken, "__operit_directory_1"), false);
});

/** Statically checks the precise changed trait, implementation and executor boundaries without claiming Rust type checking. */
test("generic chat consumers have matching async contracts and no old character manager entry", () => {
  const trait = source("core/crates/tool/services/src/runtime_support.rs"), runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const tools = source("core/crates/tool/services/src/tools/defaultTool/standard/StandardChatManagerTool.rs"), registration = source("core/crates/tool/services/src/tools/ToolRegistration.rs");
  for (const file of [trait, runtime, tools]) assert.doesNotMatch(file, /RuntimeCharacterCardInfo|resolveCharacterCardName|(?:fn|\.)\s*(?:listCharacterCards|characterCardName)\(|CharacterCardManager|DEFAULT_CHARACTER_CARD_ID|role_card_id|character_card_id|ChatManagerToolOperation::ListCharacterCards/);
  for (const file of [trait, runtime]) {
    const compact = file.replace(/\s+/g, " ");
    assert.match(compact, /fn createChatRuntime<'a>\( &'a self, setAsCurrentChat: bool, sourceChatId: Option<String>, input: Option<serde_json::Value>, \) -> ToolRuntimeSupportFuture<'a, Result<String, String>>/);
    assert.match(compact, /fn switchMainChat<'a>\( &'a self, chatId: &'a str, \) -> ToolRuntimeSupportFuture<'a, Result<\(\), String>>/);
  }
  assert.match(runtime, /\.createNewChat\(setAsCurrentChat, sourceChatId, input\)\s*\.await\?/);
  assert.match(tools, /impl AsyncToolExecutor for ChatManagerToolExecutor/);
  assert.doesNotMatch(tools, /block_on|new_current_thread/);
  assert.match(registration, /RegisteredToolExecutor::Asynchronous\(Box::new\(\s*ChatManagerToolExecutor/);
  assert.doesNotMatch(registration, /registerChatTool\([\s\S]{0,150}BuiltinToolName::ListCharacterCards/);
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/chat.rs").split("#[cfg(test)]")[0];
  assert.doesNotMatch(sdk, /fn listCharacterCards|characterCardId|roleCardId/);
  assert.doesNotMatch(sdk, /group: Option<String>|selection: Option<String>/);
  assert.match(sdk, /fn createNew\(&self, options: Option<ChatCreateOptions>\)/);
  assert.match(sdk, /participantId: Option<String>/);
  const results = source("core/crates/plugin/sdk/src/js_sdk/results.rs");
  assert.deepEqual(fields(results, "ChatInfo"), ["id", "title", "messageCount", "createdAt", "updatedAt", "isCurrent", "inputTokens", "outputTokens"]);
  assert.deepEqual(fields(results, "ChatMessageInfo"), ["sender", "content", "timestamp", "variantIndex", "variantCount", "provider", "modelName"]);
  assert.doesNotMatch(tools, /roleName:|characterCardName:|pluginExtensions:/);
});

/** Proves the real ordinary and streaming package callers cannot select a stale script or a suffix-matched tool. */
test("actual package executor delegates one request to immutable exact SDK selection", () => {
  const executor = source("core/crates/tool/services/src/tools/PackageToolExecutor.rs"), manager = source("core/crates/plugin/javascript-bridge/src/javascript/JsToolManager.rs");
  assert.doesNotMatch(executor, /ends_with|\.tools|packageTool\.script/);
  assert.equal([...executor.matchAll(/execute_package_tool\(/g)].length, 1);
  assert.match(executor, /execute_package_tool\(&request\)\.await/);
  assert.match(executor, /vec!\[self\.invoke\(tool\)\.await\]/);
  const execute = manager.slice(manager.indexOf("pub async fn executeScript("), manager.indexOf("/// Releases JavaScript tool manager resources."));
  assert.equal([...execute.matchAll(/\.select_tool\(/g)].length, 1);
  assert.match(execute, /&selection\.definition\.script/);
  assert.doesNotMatch(execute, /\b_script\b|script: &str/);
  const host = source("core/crates/tool/services/src/tools/ToolJsRuntime.rs");
  assert.match(host, /packageRegistryReadiness\(\)\.require_ready\(\)\?/);
  assert.match(host, /isPackageEnabled\(package_name\)/);
});
