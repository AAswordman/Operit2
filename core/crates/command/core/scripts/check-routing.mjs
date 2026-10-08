import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";

/** Reads a command source relative to this scope-local static routing checker. */
function source(file) {
  return readFileSync(new URL(`../src/commands/${file}`, import.meta.url), "utf8");
}

/** Reads a mounted runtime source for strict retirement registration assertions. */
function runtimeSource(file) {
  return readFileSync(new URL(`../../../runtime/application/src/${file}`, import.meta.url), "utf8");
}

/** Reads the existing SDK implementation to verify its actual generic catalog contract. */
function sdkSource(file) {
  return readFileSync(new URL(`../../../tool/services/src/tools/packTool/${file}`, import.meta.url), "utf8");
}

/** Extracts one named Rust function body for scope-local source contract assertions. */
function functionSource(text, name) {
  const signature = new RegExp(`\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${name}(?:\\s*<|\\s*\\()`);
  const start = signature.exec(text);
  assert.ok(start, `Rust function ${name} must exist`);
  const open = text.indexOf("{", start.index);
  assert.notEqual(open, -1, `${name} must have a function body`);
  let depth = 1;
  let cursor = open + 1;
  for (; cursor < text.length && depth > 0; cursor += 1) {
    if (text[cursor] === "{") depth += 1;
    if (text[cursor] === "}") depth -= 1;
  }
  assert.equal(depth, 0, `${name} must have balanced source braces`);
  return text.slice(start.index, cursor);
}

const router = source("mod.rs");
const catalog = source("catalog.rs");
const delegated = source("delegated.rs");
const plugin = source("plugin.rs");
const entry = source("../lib.rs");
const output = source("../output.rs");
const sdk = sdkSource("RuntimePackageManager.rs");
const sdkCatalog = sdkSource("ToolPkgCoreCommandCatalog.rs");
const runRoot = functionSource(router, "run_core_command");
const runBuiltin = functionSource(router, "run_core_owned_command");
const help = functionSource(router, "print_core_usage");
const load = functionSource(delegated, "load");
const execute = functionSource(delegated, "execute");
const list = functionSource(plugin, "list_plugin_commands");
const explicit = functionSource(plugin, "execute_plugin_command");
const readySnapshot = functionSource(sdk, "readySnapshot");
const sdkExecute = functionSource(sdk, "executeResolvedToolPkgCoreCommand");

const builtinBlock = /const BUILTIN_ROOTS:[\s\S]*?= &\[([\s\S]*?)\];/.exec(catalog);
assert.ok(builtinBlock, "Core must declare one directory containing only its inherent handlers");
const builtins = [...builtinBlock[1].matchAll(/\("([a-z-]+)", BuiltinRoot::([A-Za-z]+)\)/g)]
  .map(([, name, handler]) => ({ name, handler }));
const builtinNames = new Set(builtins.map(({ name }) => name));
assert.equal(builtins.length, 19, "All 19 existing inherent Core command families must remain mounted");
assert.equal(builtinNames.size, builtins.length, "Inherent Core directory entries must be unique");

assert.match(router, /^mod catalog;$/m);
assert.match(delegated, /use super::catalog::\{resolve_root, RootCommandOwner, BUILTIN_ROOTS\};/);
assert.match(runRoot, /let registry = ReadyCommandRegistry::load\(&application\.toolHandler\)\.await\?;/);
assert.equal([...router.matchAll(/ReadyCommandRegistry::load\(/g)].length, 1);
assert.ok(runRoot.indexOf("ReadyCommandRegistry::load") < runRoot.indexOf("registry.resolve"));
assert.match(runRoot, /match registry\.resolve\(&args\[0\]\)\?/);
assert.match(runRoot, /RootCommandOwner::Plugin\(selected\) =>\s*(?:\{\s*)?registry\.execute\(&selected, &args\[1\.\.\], output\)\.await/);
assert.match(runRoot, /RootCommandOwner::Core\(root\) =>\s*\{\s*run_core_owned_command\(application, &registry, root, &args\[1\.\.\], output\)\.await/);
assert.equal([...runRoot.matchAll(/registry\.resolve\(/g)].length, 1, "Each root invocation resolves exactly once");
assert.match(runRoot, /if args\.is_empty\(\)\s*\{\s*return print_core_usage\(&registry, output\);/);
assert.doesNotMatch(runRoot, /try_|\.or_else\(|\.unwrap_or|match .*\.execute|execute[^;]+\.is_err\(/);
assert.doesNotMatch(router, /^\s*mod (people|tag|memory);$/m);
assert.doesNotMatch(router, /(?:people|tag|memory)::run_[a-z_]+_command/);
for (const file of ["people.rs", "tag.rs", "memory.rs"]) {
  assert.equal(existsSync(new URL(`../src/commands/${file}`, import.meta.url)), false, `${file} must be physically retired, not just unmounted`);
}

const retiredPreferences = [
  "ActivePromptManager", "CharacterCardManager", "CharacterGroupCardManager",
  "SharedMemoryStoreManager", "PromptTagManager", "PersonaCardChatHistoryManager",
  "CharacterCardBilingualData", "CharacterCardToolAccessResolver",
];
const preferenceModules = runtimeSource("data/preferences/mod.rs");
for (const module of retiredPreferences) {
  assert.equal(existsSync(new URL(`../../../runtime/application/src/data/preferences/${module}.rs`, import.meta.url)), false, `${module} must be physically retired`);
  assert.doesNotMatch(preferenceModules, new RegExp(`\\b${module}\\b`, "u"), `${module} must not remain registered or re-exported`);
}
for (const file of ["services/MemoryManagementService.rs", "services/core/ChatMemoryOwnerResolver.rs"]) {
  assert.equal(existsSync(new URL(`../../../runtime/application/src/${file}`, import.meta.url)), false, `${file} must be physically retired`);
}
assert.doesNotMatch(runtimeSource("services/mod.rs"), /\bmod\s+MemoryManagementService\b/u);
assert.doesNotMatch(runtimeSource("services/core/mod.rs"), /\bmod\s+ChatMemoryOwnerResolver\b/u);
assert.match(preferenceModules, /pub mod ThemeConfigManager;/u);
assert.match(preferenceModules, /pub use ThemeConfigManager::\*;/u);

assert.match(delegated, /struct ReadyCommandRegistry\s*\{\s*manager: RuntimePackageManager,\s*catalog: ToolPkgCoreCommandCatalog,/);
assert.equal([...delegated.matchAll(/RuntimePackageManager::readySnapshot\(/g)].length, 1);
assert.match(load, /RuntimePackageManager::readySnapshot\(\s*tool_handler\.getOrCreatePackageManager\(\),?\s*\)\s*\.await\?/);
assert.match(load, /let catalog = manager\.getToolPkgCoreCommandCatalog\(false\)\?;/);
assert.match(load, /Ok\(Self \{ manager, catalog \}\)/);
assert.doesNotMatch(load, /\.clone\(|\.lock\(/, "Only the SDK may clone a registry after its readiness barrier");
assert.match(readySnapshot, /readiness\.wait_until_ready\(\)\.await\?;/);
assert.match(readySnapshot, /manager\.packageRegistryReadiness\.require_ready\(\)\?;/);
assert.ok(readySnapshot.indexOf("wait_until_ready") < readySnapshot.indexOf("Ok(manager.clone())"));
assert.match(delegated, /resolve_root\(command_name, \|name, builtin_names\|[\s\S]*?self\.catalog\.resolve\(name, builtin_names\)/);
assert.match(execute, /self\.manager\s*\.executeResolvedToolPkgCoreCommand\(handler, args, json_mode\)/);
assert.equal([...delegated.matchAll(/\.executeResolvedToolPkgCoreCommand\(/g)].length, 1);
assert.doesNotMatch(execute, /\.resolve\(|\.getOrCreatePackageManager\(|readySnapshot\(|\.clone\(/);
assert.match(sdkExecute, /let command = resolved\.declaration\(\);/);
assert.doesNotMatch(sdkExecute, /\.resolve\(|getToolPkgCoreCommandCatalog\(|readySnapshot\(/);
for (const text of [router, catalog, delegated, plugin]) {
  assert.doesNotMatch(text, /root_command_owner|\.executeToolPkgCoreCommand\(|\.getToolPkgCoreCommands\(/);
  assert.doesNotMatch(text, /#\[cfg\([^\]]*(?:target_arch|target_os)/);
}
assert.match(catalog, /Resolve: FnOnce\(&str, &\[&str\]\) -> Result<Option<Plugin>, String>/);
assert.match(catalog, /let plugin = resolve_plugin\(name, &builtin_names\)\?;/);
assert.match(catalog, /match \(builtin, plugin\)/);
assert.match(catalog, /\(Some\(_\), Some\(_\)\) => Err/);
assert.match(catalog, /\(None, None\) => Err/);
assert.match(sdkCatalog, /\.filter\(\|registration\| registration\.info\.name\.eq_ignore_ascii_case\(name\)\)/);
assert.match(sdkCatalog, /if !registration\.enabled[\s\S]*?plugin command is disabled/);
assert.match(sdkCatalog, /duplicate registered plugin command/);
assert.match(sdkCatalog, /collides with builtin Core command/);

assert.match(plugin, /run_plugin_command\(\s*application: &OperitApplication,\s*registry: &ReadyCommandRegistry,/);
assert.match(plugin, /"commands" => list_plugin_commands\(registry, output\)/);
assert.match(plugin, /"exec" => execute_plugin_command\(registry, &args\[1\.\.\], output\)\.await/);
assert.match(list, /let commands = registry\.plugin_commands\(\)\?;/);
assert.match(explicit, /super::delegated::run_delegated_command\(registry, command_name, &args\[1\.\.\], output\)\.await/);
assert.doesNotMatch(list + explicit, /getOrCreatePackageManager|package_manager\(|\.lock\(|\.clone\(|ReadyCommandRegistry::load/);
assert.match(delegated, /provider\(selected, args, output\.isJsonMode\(\)\)\.await\?/);
assert.match(delegated, /let json = result\.json\.ok_or_else\(/);
assert.match(delegated, /output\.setJsonStdout\(json\)/);
assert.match(delegated, /output\.push_stdout\(result\.stdout\)/);
assert.match(delegated, /output\.push_stderr\(result\.stderr\)/);

const coreRoutes = new Map([
  ["Extension", /extension::run_extension_command\(application, args, output\)/],
  ["Tool", /tool::run_tool_command\(application, args, output\)\.await/],
  ["Package", /package::run_package_command\(application, args, output\)\.await/],
  ["Plugin", /plugin::run_plugin_command\(application, registry, args, output\)\.await/],
  ["Skill", /skill::run_skill_command\(application, args, output\)/],
  ["Mcp", /mcp::run_mcp_command\(application, args, output\)/],
  ["Market", /market::run_market_command\(application, args, output\)/],
  ["Host", /host::run_host_command\(application\.hostManager\.clone\(\), args, output\)/],
  ["Log", /log::run_log_command\(args, output\)/],
  ["LocalModels", /local_models::run_local_models_command\(application, args, output\)/],
  ["Prefs", /prefs::run_prefs_command\(application\.hostManager\.clone\(\), args, output\)/],
  ["Approval", /approval::run_approval_command\(application\.hostManager\.clone\(\), args, output\)/],
  ["Model", /model::run_model_command\(application\.hostManager\.clone\(\), args, output\)/],
  ["Chat", /chat::run_chat_command\(application, args, output\)/],
  ["Workspace", /workspace::run_workspace_command\(application, args, output\)\.await/],
  ["Storage", /storage::run_storage_command\(application, args, output\)/],
  ["Stt", /stt::run_stt_command\(application, args, output\)/],
  ["Update", /update::run_update_command\(args, output\)/],
  ["Usage", /usage::run_usage_command\(application, args, output\)/],
]);
for (const { name, handler } of builtins) {
  const arm = new RegExp(`BuiltinRoot::${handler} =>\\s*(?:\\{\\s*)?(${coreRoutes.get(handler).source})`);
  assert.match(runBuiltin, arm, `${name} must keep its original Core handler and arguments`);
}
assert.equal([...runBuiltin.matchAll(/^\s*BuiltinRoot::[A-Za-z]+ =>/gm)].length, builtins.length);
assert.doesNotMatch(runBuiltin, /_ =>|command_name: &str|"[a-z-]+" =>/);
assert.match(runBuiltin, /if args\.first\(\)\.map\(String::as_str\) == Some\("scope"\)/);
assert.match(runBuiltin, /if let Some\(kind\) = root\.scope_alias_kind\(\)/);
assert.match(runBuiltin, /return extension::run_scope_command\(application, kind, &args\[1\.\.\], output\)/);
assert.match(catalog, /Self::Plugin \| Self::Package \| Self::Skill \| Self::Mcp => Some\(self\.name\(\)\)/);
assert.match(help, /let commands = registry\.plugin_commands\(\)\?;/);
assert.match(help, /\.chain\(commands\.iter\(\)\.map\(\|command\| command\.name\.as_str\(\)\)\)/);
assert.match(help, /format!\("operit2 <\{\}>"\s*, root_names\.join\("\|"\)\)/);
assert.match(help, /command\.usage, command\.description, command\.containerPackageName/);
assert.match(help, /"registeredCommands": commands/);
assert.doesNotMatch(help, /operit2 <[a-z-]+\|[^\"]*>"/);
assert.match(list, /command\.usage, command\.description, command\.containerPackageName/);
assert.match(entry, /commands::run_core_command\(application, &commandArgs, &mut output\)\.await\?;\s*output\.finalizeJson\(\)\?/);
assert.match(output, /\.jsonStdoutDocument\s*\.take\(\)\s*\.ok_or_else\(/);
assert.match(output, /self\.stdout = serde_json::to_string\(&stdout\)/);
assert.match(output, /self\.stderr\.clear\(\)/);

/** Builds explicit immutable catalog fixtures; these checks are not a Rust runtime integration test. */
function registration(name, owner, enabled = true) {
  return Object.freeze({ name, owner, enabled, handler: `${owner}.handler`, usage: `operit2 ${name} <show>`, description: `Registered ${name}` });
}

/** Models the verified SDK directory contract with one exact name lookup and explicit ownership errors. */
function resolveFixture(name, registrations, resolutions) {
  const canonical = name.trim();
  assert.ok(canonical.length > 0 && !/[\/\s]/u.test(canonical), `invalid root command name: ${name}`);
  resolutions.count += 1;
  const key = canonical.toLowerCase();
  const candidates = registrations.filter((command) => command.name.toLowerCase() === key);
  const builtin = builtinNames.has(key);
  if (builtin && candidates.length > 0) throw new Error(`plugin command /${canonical} collides with builtin Core command: ${canonical}`);
  if (candidates.length > 1) throw new Error(`duplicate registered plugin command /${canonical}`);
  if (candidates.length === 1 && !candidates[0].enabled) throw new Error(`plugin command is disabled: /${canonical} (${candidates[0].owner})`);
  if (candidates.length === 1) return Object.freeze({ type: "plugin", token: candidates[0] });
  if (builtin) return Object.freeze({ type: "core", name: key });
  throw new Error(`unknown root command: ${canonical}`);
}

/** Models one selected token execution and the provider's explicit text or JSON output contract. */
function executeFixture(token, args, jsonMode, provider) {
  const result = provider(token, args, jsonMode);
  if (jsonMode) {
    assert.notEqual(result.json, undefined, `plugin command /${token.name} did not return a JSON result`);
    return { stdout: JSON.stringify(result.json), stderr: "" };
  }
  return { stdout: result.stdout, stderr: result.stderr };
}

const dynamicName = `new-root-${randomUUID()}`;
const dynamicRoots = [dynamicName, "plan", "goal", "custom.automation"];
const registrations = Object.freeze(dynamicRoots.map((name) => registration(name, `fixture.${name}`)));
let fixtureChecks = 0;
for (const name of dynamicRoots) {
  const resolutions = { count: 0 };
  const owner = resolveFixture(name.toUpperCase(), registrations, resolutions);
  assert.equal(owner.type, "plugin");
  assert.equal(owner.token.name, name);
  assert.equal(resolutions.count, 1);
  for (const jsonMode of [false, true]) {
    const args = Object.freeze(["show", "record with spaces", ""]);
    let executions = 0;
    const document = { ok: true, owner: owner.token.owner, record: { id: "record with spaces", value: "" } };
    const result = executeFixture(owner.token, args, jsonMode, (selected, actualArgs, actualJsonMode) => {
      executions += 1;
      assert.equal(selected, owner.token, "The original immutable handler token must execute");
      assert.equal(actualArgs, args, "Full unmodified subcommand arguments must reach the provider");
      assert.equal(actualJsonMode, jsonMode);
      return { stdout: "exact provider output\n", stderr: "exact provider diagnostic\n", json: document };
    });
    assert.equal(executions, 1);
    assert.equal(resolutions.count, 1, "Execution must not resolve again");
    assert.deepEqual(result, jsonMode
      ? { stdout: JSON.stringify(document), stderr: "" }
      : { stdout: "exact provider output\n", stderr: "exact provider diagnostic\n" });
    fixtureChecks += 1;
  }
}
for (const { name } of builtins) {
  const resolutions = { count: 0 };
  assert.deepEqual(resolveFixture(name, registrations, resolutions), { type: "core", name });
  assert.equal(resolutions.count, 1);
  fixtureChecks += 1;
}
for (const [commands, error] of [
  [[registration(dynamicName, "disabled.owner", false)], /plugin command is disabled/],
  [[registration(dynamicName, "first.owner"), registration(dynamicName.toUpperCase(), "second.owner")], /duplicate registered plugin command/],
  [[registration(dynamicName, "enabled.owner"), registration(dynamicName, "disabled.owner", false)], /duplicate registered plugin command/],
]) {
  const resolutions = { count: 0 };
  assert.throws(() => resolveFixture(dynamicName, commands, resolutions), error);
  assert.equal(resolutions.count, 1);
  fixtureChecks += 1;
}
for (const enabled of [true, false]) {
  const collisionName = builtins[0].name;
  const resolutions = { count: 0 };
  assert.throws(() => resolveFixture(collisionName, [registration(collisionName, "collision.owner", enabled)], resolutions), /collides with builtin Core command/);
  assert.equal(resolutions.count, 1);
  fixtureChecks += 1;
}
for (const name of [`unregistered-${randomUUID()}`, `${dynamicName}-extra`]) {
  const resolutions = { count: 0 };
  assert.throws(() => resolveFixture(name, registrations, resolutions), /unknown root command/);
  assert.equal(resolutions.count, 1);
  fixtureChecks += 1;
}
for (const name of ["", "  ", "/root", "root with spaces"]) {
  const resolutions = { count: 0 };
  assert.throws(() => resolveFixture(name, registrations, resolutions), /invalid root command name/);
  assert.equal(resolutions.count, 0);
  fixtureChecks += 1;
}
for (const jsonMode of [false, true]) {
  const originalError = new Error("original provider storage error");
  let executions = 0;
  assert.throws(() => executeFixture(registrations[0], [], jsonMode, () => {
    executions += 1;
    throw originalError;
  }), (error) => error === originalError);
  assert.equal(executions, 1);
  fixtureChecks += 1;
}
let emptyExecutions = 0;
assert.deepEqual(executeFixture(registrations[0], [], false, (_, args) => {
  emptyExecutions += 1;
  assert.deepEqual(args, []);
  return { stdout: "registered usage\n", stderr: "" };
}), { stdout: "registered usage\n", stderr: "" });
assert.equal(emptyExecutions, 1);
fixtureChecks += 1;
let missingJsonExecutions = 0;
assert.throws(() => executeFixture(registrations[0], [], true, () => {
  missingJsonExecutions += 1;
  return { stdout: '{"ok":true}', stderr: "" };
}), /did not return a JSON result/);
assert.equal(missingJsonExecutions, 1);
fixtureChecks += 1;
const structuredError = { ok: false, error: { code: "record_not_found", message: "exact error" } };
assert.deepEqual(executeFixture(registrations[0], [], true, () => ({ stdout: "ignored text", stderr: "exact error", json: structuredError })), { stdout: JSON.stringify(structuredError), stderr: "" });
fixtureChecks += 1;
const discoveredRoots = [...builtinNames, ...registrations.map((command) => command.name)];
for (const name of dynamicRoots) assert.ok(discoveredRoots.some((root) => root === name));
fixtureChecks += 1;

const rustTests = [
  [catalog, [
    "arbitrary_registered_roots_resolve_once",
    "registered_lookup_is_exact_and_case_insensitive",
    "disabled_registration_is_not_a_core_command",
    "duplicate_registration_is_an_error",
    "builtin_plugin_collision_is_an_error",
    "ambiguous_owner_is_never_executed",
    "inherent_core_commands_keep_their_handlers",
    "unknown_root_is_an_error",
    "invalid_names_do_not_resolve",
    "catalog_errors_are_preserved_once",
    "scope_aliases_preserve_original_kinds",
  ]],
  [delegated, [
    "selected_handler_executes_once_with_exact_arguments",
    "empty_subcommands_reach_selected_handler",
    "provider_errors_are_propagated_once",
    "json_requires_the_selected_provider_document",
    "structured_provider_error_documents_are_preserved",
  ]],
];
for (const [text, tests] of rustTests) {
  for (const name of tests) assert.match(text, new RegExp(`#\\[test\\]\\s*fn ${name}\\(`), `${name} must remain present`);
}
console.log(`Routing static contracts passed: one ready snapshot, immutable selected-token execution, ${builtins.length} inherent Core routes, dynamic registered help, original scope aliases and JSON finalization.`);
console.log(`Generic catalog fixture checks passed: ${fixtureChecks} cases including a runtime-generated root, plan/goal, disabled, duplicate, collisions, unknown, single execution, JSON and exact errors.`);
console.log(`Rust unit test coverage present: ${rustTests.reduce((count, [, tests]) => count + tests.length, 0)} tests; Rust tests were not run and plugin production services were not exercised.`);
