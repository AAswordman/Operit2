import { installScopedHostRuntime, installGeneratedToolsRuntime } from "../../../../../tools/tests/fixtures/scoped_host_runtime.mjs";
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
/** Evaluates the current scoped host-directory helper using an explicit native callback boundary. */
function directoryRuntime(native) {
  const context = vm.createContext({}), registry = installScopedHostRuntime(context);
  context.__operitNativeCallToolStructured = registry.binding((id, type, name, parameters) => {
    assert.equal(type, 'default'); assert.deepEqual(structuredClone(parameters), {});
    return native({settle(requestId,value,reject) { registry.settle(requestId,reject ? value : {success:true,data:value},reject); }}, id, name);
  });
  installGeneratedToolsRuntime(context);
  return context;
}

/** Confirms the domain DTOs preserve every field from the actual model and speech records. */
test("directory SDK DTOs are complete canonical model and TTS records", () => {
  const sdkModels = source("core/crates/plugin/sdk/src/js_sdk/models.rs"), sdkTts = source("core/crates/plugin/sdk/src/js_sdk/tts.rs");
  const models = source("core/crates/foundation/model/src/ModelConfigData.rs"), tts = source("core/crates/foundation/model/src/TtsConfig.rs");
  assert.deepEqual(fields(sdkModels, "ModelsSummary"), fields(models, "ProviderModelSummary"));
  assert.deepEqual(fields(sdkModels, "ModelsCapabilities"), fields(models, "ModelCapabilities"));
  assert.deepEqual(fields(sdkModels, "ModelsPricing"), fields(models, "ModelPricing"));
  assert.deepEqual(fields(sdkTts, "TtsConfig"), fields(tts, "TtsConfig"));
  assert.deepEqual(fields(sdkTts, "TtsHeader"), fields(tts, "TtsHttpHeader"));
  assert.deepEqual(fields(sdkTts, "TtsResponseStep"), fields(tts, "TtsHttpResponsePipelineStep"));
});

/** Preserves complete structured directory records and releases each scoped request. */
test("domain directory callback preserves full records and releases callback identities", async () => {
  const values = new Map([["models_list_summaries", [{ providerId: "provider", modelId: "model", pricing: null }]], ["tts_list_configs", [{ id: "speech", requestBody: "full", responsePipeline: [{ stepType: "json", path: "audio", headers: [{ name: "custom", value: "original" }] }] }]], ["catalog_read_tool_sources", { builtinTools: [{ name: "read_file", displayName: "Read", description: "configured" }], packages: [{ name: "actual_package", displayName: "Package", description: "full" }], skills: [{ name: "actual_skill", displayName: "Skill", description: "full" }], mcpServers: [{ name: "actual_server", displayName: "MCP", description: "full" }] }]]);
  const calls = [], context = directoryRuntime(
    /** Supplies test-controlled host values through the production structured callback protocol. */
    (registry, id, method) => { assert.ok(values.has(method)); calls.push(id); registry.settle(id, values.get(method), false); },
  );
  const methods = [() => context.Tools.Models.listSummaries(), () => context.Tools.Tts.listConfigs(), () => context.Tools.Catalog.readToolSources()];
  let index = 0;
  for (const value of values.values()) assert.deepEqual(structuredClone(await methods[index++]()), value);
  assert.equal(new Set(calls).size, 3); assert.equal(context.refs, 0);
});

/** Checks original asynchronous and synchronous transport failure messages are retained. */
test("domain directory Promise rejects original host and transport errors", async () => {
  const context = directoryRuntime(
    /** Delivers an explicit native failure through the real scoped host callback. */
    (registry, id) => registry.settle(id, " unchanged host failure ", true),
  );
  await assert.rejects(context.Tools.Tts.listConfigs(), { message: " unchanged host failure " });
  const broken = directoryRuntime(
    /** Rejects scheduling before callback completion without returning substitute data. */
    () => { throw new (vm.runInContext("Error", broken))("native transport original"); },
  );
  await assert.rejects(broken.Tools.Catalog.readToolSources(), { message: "native transport original" });
  assert.equal(broken.refs, 0);
});

/** Statically checks the precise changed trait, implementation and executor boundaries without claiming Rust type checking. */
test("generic chat consumers have matching async contracts and no old character manager entry", () => {
  const trait = source("core/crates/tool/services/src/runtime_support.rs"), runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const tools = source("core/crates/tool/services/src/tools/defaultTool/standard/StandardChatManagerTool.rs"), registration = source("core/crates/tool/services/src/tools/ToolRegistration.rs");
  for (const file of [trait, runtime, tools]) assert.doesNotMatch(file, /RuntimeCharacterCardInfo|resolveCharacterCardName|(?:fn|\.)\s*(?:listCharacterCards|characterCardName)\(|CharacterCardManager|DEFAULT_CHARACTER_CARD_ID|role_card_id|character_card_id|ChatManagerToolOperation::ListCharacterCards/);
  for (const file of [trait, runtime]) {
    const compact = file.replace(/\s+/g, " ");
    assert.match(compact, /fn createChatRuntime<'a>\( &'a self, setAsCurrentChat: bool, sourceChatId: Option<String>, parentChatId: Option<String>, input: Option<serde_json::Value>, \) -> ToolRuntimeSupportFuture<'a, Result<String, String>>/);
    assert.match(compact, /fn switchMainChat<'a>\( &'a self, chatId: &'a str, \) -> ToolRuntimeSupportFuture<'a, Result<\(\), String>>/);
  }
  assert.match(runtime, /\.createNewChat\(setAsCurrentChat, sourceChatId, parentChatId, input\)\s*\.await\?/);
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
  assert.match(executor, /let host = context\.host\(\)\.clone\(\)/);
  assert.doesNotMatch(executor, /packageExecutor:|self\.invoke\(/);
  const execute = manager.slice(manager.indexOf("pub async fn executeScript("), manager.indexOf("/// Releases JavaScript tool manager resources."));
  assert.equal([...execute.matchAll(/\.select_tool\(/g)].length, 1);
  assert.match(execute, /execute_script_function_owned\(\s*selection\.definition\.script, functionName, runtimeParams, BTreeMap::new\(\)/);
  assert.doesNotMatch(execute, /\b_script\b|script: &str/);
  const host = source("core/crates/tool/services/src/tools/ToolJsRuntime.rs");
  assert.match(host, /packageRegistryReadiness\(\)\.require_ready\(\)\?/);
  assert.match(host, /isPackageEnabled\(package_name\)/);
});
