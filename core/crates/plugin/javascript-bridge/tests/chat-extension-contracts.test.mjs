import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
/** Reads current authoritative production sources without generating declarations or building Rust. */
function source(relative) {
  return readFileSync(new URL(relative, root), "utf8").replace(/\r\n/g, "\n");
}

/** Evaluates the exact generated JavaScript helper with an explicitly simulated native transport. */
function extensionRuntime(native) {
  const generator = source("core/crates/plugin/codegen/src/runtime_bindings.rs");
  const marker = "/** Validates complete JSON objects without coercing undefined values or non-finite numbers. */";
  const start = generator.indexOf(marker);
  const endMarker = "globalThis.__operitChatExtensionSequence = 0;";
  const end = generator.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start);
  const context = vm.createContext({
    /** Simulates only the transport boundary and never substitutes for a production record store. */
    __operitNativeChatExtensionAsync(...args) { return native(context, ...args); },
  });
  vm.runInContext(generator.slice(start, end + endMarker.length), context);
  return context;
}

/** Normalizes values returned from the isolated VM without changing their JSON payload. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Checks actual schemas and native registration do not route metadata through a user-controlled tool parameter map. */
test("typed extensions require exact targets and a native engine-bound actual owner", () => {
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/chat.rs");
  const engine = source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs");
  const handler = source("core/crates/tool/services/src/tools/AIToolHandler.rs");
  const toolsHost = source("core/crates/tool/services/src/tools/AIToolHandlerJsToolsHost.rs");
  const bindings = source("core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs");
  const generator = source("core/crates/plugin/codegen/src/runtime_bindings.rs");
  assert.match(sdk, /#\[serde\(tag = "kind", deny_unknown_fields\)\]/);
  assert.match(sdk, /Message\s*\{\s*chatId: String,\s*messageTimestamp: i64,\s*variantIndex: i32,/);
  for (const method of ["readExtension", "writeExtension", "deleteExtension"]) {
    assert.match(bindings, new RegExp('namespace: "Chat",\\s*method: "' + method + '"'));
  }
  assert.match(generator, /Tools\[\\"Chat\\"\] = __operitToolPkgApi\.namespace\(\\"Tools\.Chat\\"/);
  assert.match(generator, /arguments\.length !== \{arity\}/);
  assert.match(engine, /host\.for_toolpkg_execution_context\(context\)\?/);
  assert.match(engine, /let extensionContext = self\.toolPkgContext\.clone\(\)/);
  assert.match(engine, /let \[callbackId, method, targetJson, valueJson\] = exactHostJavaScriptArguments\(\s*"__operitNativeChatExtensionAsync",\s*arguments,?\s*\)\?/);
  assert.match(engine, /chatExtensionExecutionOwner\(extensionContext\.as_ref\(\)\)/);
  const binding = handler.slice(handler.indexOf("fn for_toolpkg_execution_context("), handler.indexOf("fn get_tool_catalog("));
  assert.match(binding, /getToolPkgContainerRuntime\(&context\.container_package_name\)/);
  assert.match(binding, /isPackageEnabled\(&context\.container_package_name\)/);
  assert.match(binding, /let mut host = self\.clone\(\)/);
  assert.match(binding, /host\.authenticatedExtensionOwner = Some\(context\.container_package_name\.clone\(\)\)/);
  assert.doesNotMatch(binding, /request\.parameters|parameters\.get\(|namespace.*get\(/);
  const direct = toolsHost.slice(toolsHost.indexOf("fn authenticated_chat_extension_owner"), toolsHost.indexOf("include!(concat!"));
  assert.doesNotMatch(direct, /invoke_generated|execute_tool_call|ToolParameter|unwrap_or_default|SoftwareSettings\.exec/);
  assert.match(direct, /getToolPkgContainerRuntime\(owner\)/);
  assert.match(direct, /isPackageEnabled\(owner\)/);
});

/** Checks the actual package tool executor releases its registry mutex before native per-engine context authentication. */
test("package engine creation authenticates outside the original registry lock", () => {
  const runtime = source("core/crates/tool/services/src/tools/ToolJsRuntime.rs");
  const caller = runtime.slice(runtime.indexOf("fn toolpkg_execution_engine("), runtime.indexOf("/// Materializes one ToolPkg resource"));
  assert.match(caller, /let manager = self\s*\.package_manager\s*\.lock\(\)[\s\S]*?\.clone\(\);\s*manager\.getToolPkgExecutionEngine\(context_key, container_package_name\)/);
  assert.doesNotMatch(caller, /\.lock\(\)[\s\S]*?\.expect\([^;]+\)\s*\.getToolPkgExecutionEngine/);
});

/** Verifies authoritative target codegen emits its discriminator rather than inventing an untagged TypeScript object. */
test("Rust declaration generator covers tagged objects and keeps extension fields required", () => {
  const generator = source("core/crates/plugin/codegen/src/declarations.rs");
  assert.match(generator, /if let Some\(tag\) = serde_tag\(&item\.attrs\)/);
  assert.match(generator, /fn emits_internally_tagged_chat_extension_targets\(/);
  assert.match(generator, /kind: \\\"message\\\"; chatId: string; messageTimestamp: number; variantIndex: number/);
});

/** Exercises write-object preservation, existing-record absence, booleans, and callback disposal in the real generated helper. */
test("extension Promise preserves complete JSON null and false without substitute responses", async () => {
  const calls = [];
  const runtime = extensionRuntime(
    /** Returns controlled fixture envelopes solely to validate the generated native Promise contract. */
    (context, id, method, target, value) => {
      calls.push({ id, method, target: JSON.parse(target), value: JSON.parse(value) });
      const result = method === "writeExtension" ? value : method === "readExtension" ? "null" : "false";
      context[id](result, false);
    },
  );
  const value = await vm.runInContext('__operitChatExtension("writeExtension", {kind:"message",chatId:"chat",messageTimestamp:17,variantIndex:2}, {actor:{id:"opaque"},voice:null,items:[true,2,"text"]})', runtime);
  assert.deepEqual(plain(value), { actor: { id: "opaque" }, voice: null, items: [true, 2, "text"] });
  assert.equal(await vm.runInContext('__operitChatExtension("readExtension", {kind:"chat",chatId:"chat"}, null)', runtime), null);
  assert.equal(await vm.runInContext('__operitChatExtension("deleteExtension", {kind:"chat",chatId:"chat"}, null)', runtime), false);
  assert.equal(new Set(calls.map(call => call.id)).size, 3);
  assert.deepEqual(calls[0].target, { kind: "message", chatId: "chat", messageTimestamp: 17, variantIndex: 2 });
  for (const { id } of calls) assert.equal(Object.hasOwn(runtime, id), false);
});

/** Rejects lossy or invalid JavaScript object inputs before any native call can be scheduled. */
test("extension write rejects non-JSON values and array holes without reaching native", async () => {
  let calls = 0;
  const runtime = extensionRuntime(
    /** Counts forbidden transport attempts instead of supplying successful data. */
    () => { calls += 1; throw new Error("invalid write reached native"); },
  );
  for (const value of ["null", "[]", '"text"', "{value:undefined}", "{value:Infinity}", "{value:NaN}", "{value:()=>1}", "{value:1n}", "{value:Symbol('x')}", "{value:new Date(0)}", "{items:[,1]}", "{[Symbol('x')]:1}", "Object.defineProperty({},'secret',{value:1})", "({get value(){return 1}})"]) {
    await assert.rejects(vm.runInContext(`__operitChatExtension("writeExtension", {kind:"chat",chatId:"chat"}, ${value})`, runtime), /non-JSON|JSON object/);
  }
  await assert.rejects(vm.runInContext('const cyclic = {}; cyclic.self = cyclic; __operitChatExtension("writeExtension", {kind:"chat",chatId:"chat"}, cyclic)', runtime));
  assert.equal(calls, 0);
  assert.equal(runtime.__operitChatExtensionSequence, 0);
});

/** Retains original native errors and transport identities and clears callback slots on every failure. */
test("extension failures propagate unchanged and release callbacks", async () => {
  const failed = extensionRuntime(
    /** Reports an explicit canonical host rejection, including its original surrounding whitespace. */
    (context, id) => context[id](JSON.stringify({ message: " unchanged record guard error " }), true),
  );
  await assert.rejects(vm.runInContext('__operitChatExtension("deleteExtension", {kind:"chat",chatId:"chat"}, null)', failed), { message: " unchanged record guard error " });
  assert.equal(Object.hasOwn(failed, "__operit_chat_extension_1"), false);
  const original = new Error("native transport original");
  const broken = extensionRuntime(
    /** Rejects scheduling with the same transport exception rather than wrapping or replacing it. */
    () => { throw original; },
  );
  await assert.rejects(vm.runInContext('__operitChatExtension("readExtension", {kind:"chat",chatId:"chat"}, null)', broken), error => error === original);
  assert.equal(Object.hasOwn(broken, "__operit_chat_extension_1"), false);
  const malformed = extensionRuntime(
    /** Sends malformed callback bytes so the real JSON parser must reject them. */
    (context, id) => context[id]("not JSON", false),
  );
  await assert.rejects(vm.runInContext('__operitChatExtension("readExtension", {kind:"chat",chatId:"chat"}, null)', malformed));
  assert.equal(Object.hasOwn(malformed, "__operit_chat_extension_1"), false);
});

/** Uses two immutable native-bound fixture identities to test replacement isolation at the real JavaScript transport boundary. */
test("two execution fixtures cannot choose each other's namespace through target or value", async () => {
  const record = new Map();
  /** Creates a simulated record callback whose owner is closed over outside all JavaScript arguments. */
  function bound(owner) {
    return extensionRuntime(
      /** Models only owner dispatch and target validation, not the canonical Rust repository implementation. */
      (context, id, method, targetJson, valueJson) => {
        const target = JSON.parse(targetJson), value = JSON.parse(valueJson);
        const allowed = target.kind === "chat" ? ["kind", "chatId"] : ["kind", "chatId", "messageTimestamp", "variantIndex"];
        if (Object.keys(target).some(key => !allowed.includes(key))) {
          context[id](JSON.stringify({ message: "unknown target field" }), true); return;
        }
        if (method === "writeExtension") { record.set(owner, value); context[id](JSON.stringify(value), false); }
        else if (method === "readExtension") { context[id](JSON.stringify(record.has(owner) ? record.get(owner) : null), false); }
        else if (method === "deleteExtension") { context[id](JSON.stringify(record.delete(owner)), false); }
        else throw new Error("unknown fixture operation");
      },
    );
  }
  const a = bound("org.example.a"), b = bound("org.example.b");
  await vm.runInContext('__operitChatExtension("writeExtension", {kind:"chat",chatId:"chat"}, {actor:"a"})', a);
  await vm.runInContext('__operitChatExtension("writeExtension", {kind:"chat",chatId:"chat"}, {actor:"b"})', b);
  await vm.runInContext('__operitChatExtension("writeExtension", {kind:"chat",chatId:"chat"}, {owner:"org.example.b",namespace:"org.example.b",pluginId:"org.example.b",actor:"a-new"})', a);
  assert.deepEqual(plain(await vm.runInContext('__operitChatExtension("readExtension", {kind:"chat",chatId:"chat"}, null)', b)), { actor: "b" });
  for (const field of ["owner", "namespace", "pluginId"]) {
    await assert.rejects(vm.runInContext(`__operitChatExtension("readExtension", {kind:"chat",chatId:"chat",${field}:"org.example.b"}, null)`, a), { message: "unknown target field" });
  }
  assert.equal(await vm.runInContext('__operitChatExtension("deleteExtension", {kind:"chat",chatId:"chat"}, null)', a), true);
  assert.equal(await vm.runInContext('__operitChatExtension("readExtension", {kind:"chat",chatId:"chat"}, null)', a), null);
  assert.deepEqual(record.get("org.example.b"), { actor: "b" });
});

/** Checks runtime adapters require an explicitly captured canonical store and return input only after a successful void commit. */
test("metadata adapters require the captured runtime store and return committed input not another read", () => {
  const runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const provider = source("core/crates/runtime/application/src/services/ProviderRuntimeSupportService.rs");
  const support = source("core/crates/tool/services/src/runtime_support.rs");
  const methods = runtime.slice(runtime.indexOf("fn readChatExtension("), runtime.indexOf("fn openPluginChatMessage("));
  const captured = runtime.slice(runtime.indexOf("pub fn bindChatHistoryManager("), runtime.indexOf("fn runtimeBindings("));
  assert.doesNotMatch(methods, /ChatHistoryManager::default|chatRuntimeHolder|try_lock|\.lock\(/);
  assert.match(captured, /self\.chatHistoryManager\s*\.set\(manager\)/);
  assert.match(captured, /self\.chatHistoryManager\s*\.get\(\)/);
  assert.match(captured, /Tool runtime chat record store is not bound/);
  assert.match(captured, /Tool runtime chat record store is already bound/);
  assert.doesNotMatch(captured, /::default\(\)|defaultRuntime|getInstance|RuntimeStorePaths|chatRuntimeHolder|try_lock|\.lock\(/);
  const write = methods.slice(methods.indexOf("fn writeChatExtension("), methods.indexOf("fn deleteChatExtension("));
  assert.match(write, /writePluginExtension\(owner, &target, persistedValue\)[\s\S]*?\.map_err\([\s\S]*?\?;\s*Ok\(value\)/);
  assert.doesNotMatch(write, /readPluginExtension|serde_json::from_value|let stored/);
  assert.match(methods, /serde_json::Value::Object\(fields\) => Ok\(fields\.into_iter\(\)\.collect\(\)\)/);
  assert.match(write, /serde_json::Value::Object\(/);
  const dataDir = provider.slice(provider.indexOf("fn dataDir("), provider.indexOf("fn thinkingQualityLevel("));
  assert.match(dataDir, /self\.tool_handler\.getContext\(\)/);
  assert.match(dataDir, /storage\s*\.runtimeRootDir\(\)/);
  assert.doesNotMatch(dataDir, /ApiPreferences|defaultRuntime/);
  assert.match(support, /fn writeChatExtension\([\s\S]*?Result<operit_plugin_sdk::js_sdk::core::JsonObject, String>/);
});

/** Checks normal and draft resolution remain distinct and only the real selected API owner supplies runtime identity. */
test("configuration resolves owner-scoped extensions and requires a native-owned message snapshot", () => {
  const schema = source("core/crates/provider/services/src/runtime_support.rs");
  const runtime = source("core/crates/runtime/application/src/services/ProviderRuntimeSupportService.rs");
  const request = schema.slice(schema.indexOf("pub struct ChatConfigurationRequest"), schema.indexOf("pub struct ChatResourceRoute"));
  assert.doesNotMatch(request, /pub selection\s*:|ChatConfigurationBinding/);
  assert.match(request, /chatExtension: Option<JsonObject>/);
  assert.match(request, /messageExtension: Option<JsonObject>/);
  const result = schema.slice(schema.indexOf("pub struct ChatConfigurationResult"), schema.indexOf("impl ChatConfigurationResult"));
  assert.match(result, /pub messageExtension: JsonObject/);
  assert.match(result, /#\[serde\(skip\)\]\s*pub extensionOwner: String/);
  assert.doesNotMatch(runtime, /ChatConfigurationBinding|chat\.configuration\.binding|pub async fn (?:readBinding|writeBinding|deleteBinding)/);
  const normal = runtime.slice(runtime.indexOf("pub async fn resolve("), runtime.indexOf("pub async fn resolveDraft("));
  const draft = runtime.slice(runtime.indexOf("pub async fn resolveDraft("), runtime.indexOf("async fn invokeResolve("));
  assert.match(normal, /readChatExtension\(&self\.ownerPackage, &target\)/);
  assert.doesNotMatch(normal, /resolveDraft|unwrap_or_default/);
  assert.doesNotMatch(draft, /readChatExtension|ChatHistoryManager/);
  assert.match(runtime, /result\.extensionOwner = owner\.to_string\(\)/);
  assert.match(runtime, /extensionOwner is assigned only by the runtime/);
  assert.match(schema, /fn authenticated_descriptor\(raw: Value\)/);
});
/** Checks the shared route schema against its actual production constructor and target-side configuration consumer. */
test("Core route resumes the exact native-owned snapshot with matching generic fields", () => {
  const support = source("core/crates/tool/services/src/runtime_support.rs");
  const processing = source("core/crates/runtime/application/src/services/core/MessageProcessingDelegate.rs");
  const core = source("core/crates/runtime/application/src/services/ChatServiceCore.rs");
  const declaration = /pub struct CoreRouteResumeContext\s*\{([^}]+)\}/.exec(support);
  assert.notEqual(declaration, null);
  const fields = [...declaration[1].matchAll(/pub (\w+):/g)].map(match => match[1]).sort();
  const constructor = /CoreRouteResumeContext\s*\{([^}]+)\}/.exec(processing);
  assert.notEqual(constructor, null);
  const actual = [...constructor[1].matchAll(/^\s*(\w+):/gm)].map(match => match[1]).sort();
  assert.deepEqual(actual, fields);
  assert.doesNotMatch(declaration[1], /roleCardId|roleName|groupOrchestrationMode|groupParticipantNamesText|enableMemoryAutoUpdate/);
  assert.match(constructor[1], /participantId:\s*completionContextConfiguration\.profile\.id\.clone\(\)/);
  assert.match(constructor[1], /extensionOwner:\s*completionContextConfiguration\.extensionOwner\.clone\(\)/);
  assert.match(constructor[1], /messageExtension:\s*completionContextConfiguration\.messageExtension\.clone\(\)/);
  assert.match(core, /api\.extensionOwner\(\) != resumeContext\.extensionOwner/);
  assert.match(core, /messageExtension:\s*Some\(resumeContext\.messageExtension\.clone\(\)\)/);
  assert.match(core, /participantId:\s*Some\(resumeContext\.participantId\.clone\(\)\)/);
});