import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";
import { createSoftwareSettingsFixture } from "./software-settings-fixture.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";

const authenticatedOwner = "com.operit.character_cards", siblingOwner = "controlled.other_package";
// Explicit settings inputs are isolated host-boundary fixtures, not configured native-host evidence.
const directoryInput = plain(loadModule("tests/software-settings-input.ts").softwareSettingsTestInput);

/** Registers real main callbacks and providers over actual temporary disk IO plus explicit host-boundary inputs. */
async function environment(t) {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directoryInput);
  const chats = new Map(["policy-chat", "group-chat"].map(
    /** Defines actual controlled host records with an unrelated namespace that must never select this plugin's participant. */
    id => [id, { summary: { id, title: id, messageCount: 0, inputTokens: 0, outputTokens: 0, createdAt: "1760000000000", updatedAt: "1760000000000", isCurrent: false },
      extensions: new Map([[siblingOwner, { version: 1, selection: "card:sibling-only", private: { retained: id } }]]) }],
  ));
  const extensionCalls = [], hostFaults = new Map();
  /** Records a real controlled host operation and propagates its explicitly queued original failure. */
  function attempted(method, target) {
    extensionCalls.push({ method, target: plain(target), owner: authenticatedOwner });
    if (hostFaults.has(method)) { const error = hostFaults.get(method); hostFaults.delete(method); throw error; }
  }
  /** Requires the exact generic target ABI and one existing host record before accessing any namespace. */
  function chatTarget(target) {
    assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind"]); assert.equal(target.kind, "chat");
    assert.equal(typeof target.chatId, "string"); assert.notEqual(target.chatId.trim(), "");
    const chat = chats.get(target.chatId);
    if (chat === undefined) throw new Error("Unknown policy test host conversation: " + target.chatId);
    return chat;
  }
  const Chat = new Proxy({
    /** Performs the exact production record-existence lookup without creating a conversation or exposing namespace data. */
    async findChat(options) {
      assert.deepEqual(plain(options), { query: options.query, match: "exact", index: 0 }); attempted("findChat", options);
      const chat = chatTarget({ kind: "chat", chatId: options.query }); return { matchedCount: 1, chat: plain(chat.summary) };
    },
    /** Returns only the authenticated owner's JSON object; null means an existing record has no such namespace. */
    async readExtension(target) {
      attempted("readExtension", target); const chat = chatTarget(target);
      return chat.extensions.has(authenticatedOwner) ? plain(chat.extensions.get(authenticatedOwner)) : null;
    },
    /** Persists the exact supplied JSON object only in the authenticated namespace without plugin binding rules. */
    async writeExtension(target, value) {
      attempted("writeExtension", target); const chat = chatTarget(target);
      assert.ok(value !== null && typeof value === "object" && !Array.isArray(value));
      chat.extensions.set(authenticatedOwner, plain(value)); return plain(chat.extensions.get(authenticatedOwner));
    },
    /** Deletes only the authenticated namespace of an actual record and leaves sibling namespaces intact. */
    async deleteExtension(target) {
      attempted("deleteExtension", target); return chatTarget(target).extensions.delete(authenticatedOwner);
    },
    /** Enumerates every explicitly controlled host summary, not a file-backed binding mirror. */
    async listAll() {
      attempted("listAll", null); return { totalCount: chats.size, currentChatId: null, chats: [...chats.values()].map(
        /** Copies actual host summaries without exposing another owner's namespace. */
        chat => plain(chat.summary),
      ) };
    },
  }, {
    /** Rejects undeclared chat SDK methods rather than constructing successful empty host responses. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Unsupported policy test chat capability: " + String(property));
      return target[property];
    },
  });
  const Tools = new Proxy({ Files: disk.files, Chat, SoftwareSettings: settings.harness.globals.Tools.SoftwareSettings }, {
    /** Rejects all undeclared host capabilities instead of synthesizing a successful response. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Unsupported policy test host capability: " + String(property));
      return target[property];
    },
  });
  const entry = openPlugin({ ...disk, globals: { ...disk.globals, Tools } });
  const registrations = [
    ...entry.toolLifecycleHooks.map(
      /** Retains the actual main-registered lifecycle handler without invoking a second bootstrap. */
      definition => ({ kind: "lifecycle", entry: definition }),
    ),
    ...entry.toolPromptHooks.map(
      /** Retains the real prompt-policy registration sharing the same initialized business service. */
      definition => ({ kind: "prompt", entry: definition }),
    ),
  ];
  return { disk, registrations, chats, extensionCalls, settingsCalls: settings.calls, ...entry.main,
    /** Invokes the actual main-registered provider instead of injecting domain responses or directory defaults. */
    dispatchDomain(operation, input) { return entry.api(operation, input); },
    /** Supplies the generic native resolver's actual owner-scoped input from the controlled host record. */
    readChatExtension(chatId) { return Chat.readExtension({ kind: "chat", chatId }); },
    /** Schedules an exact original generic-host error for the next explicit invocation, never an automatic retry. */
    failNextHost(method, error) { assert.equal(hostFaults.has(method), false); hostFaults.set(method, error); },
  };
}

/** Resolves a full execution message marker through the actual production service before starting the tool-policy turn. */
async function resolveContext(environment, chatId, participantId) {
  const configuration = await environment.dispatchDomain("chat.configuration.resolve", {
    purpose: "execution", chatId, chatExtension: await environment.readChatExtension(chatId), messageExtension: null, participantId,
    promptFunctionType: "CHAT",
    defaultModelBinding: { providerId: directoryInput.models[0].providerId, modelId: directoryInput.models[0].modelId },
    defaultTtsConfigId: directoryInput.ttsConfigs[0].id,
  });
  assert.equal(configuration.profile.id, participantId);
  assert.deepEqual(plain(configuration.messageExtension.profile.toolAccess), plain(configuration.profile.toolAccess));
  return { chatId, participantId, extensionOwner: authenticatedOwner, messageExtension: plain(configuration.messageExtension) };
}

/** Persists explicit permissions and binding, then captures the complete resolved snapshot for one real test turn. */
async function bind(environment, chatId = "policy-chat") {
  await environment.dispatchDomain("chat.configuration.binding.write", { chatId, selection: "card:default" });
  await environment.dispatchDomain("character.update", { id: "default", changes: { toolAccessConfig: {
    enabled: true, allowedBuiltinTools: ["read_file", "use_package"], allowedPackages: ["approved_package"], allowedSkills: ["approved_skill"], allowedMcpServers: ["approved_server"],
  } } });
  const chat = environment.chats.get(chatId); assert.ok(chat);
  assert.deepEqual(chat.extensions.get(authenticatedOwner), { version: 1, selection: "card:default" });
  assert.deepEqual(chat.extensions.get(siblingOwner), { version: 1, selection: "card:sibling-only", private: { retained: chatId } });
  const state = JSON.parse(await readFile(path.join(environment.disk.directory, "character-memory", "state.json"), "utf8"));
  assert.equal(state.version, 3); assert.equal(Object.hasOwn(state, "chatBindings"), false);
  return resolveContext(environment, chatId, "default");
}

/** Constructs a lifecycle event with its explicit native-authenticated registration owner and frozen context. */
function event(toolName, context, parameters = {}, owner = authenticatedOwner) {
  return { containerPackageName: owner, eventName: "tool_call_intercept", eventPayload: { toolName, runtimeContext: context, parameters } };
}

/** Constructs a prompt event for one of the existing production visibility stages, not a synthetic tool entry point. */
function prompt(stage, context, availableTools, owner = authenticatedOwner) {
  return { containerPackageName: owner, eventPayload: { stage, availableTools, metadata: { executionContext: context } } };
}

/** Requires zero current-state reads or writes during already-snapshotted policy evaluation. */
function ioCounts(runtime) {
  return { disk: runtime.disk.calls.length, extensions: runtime.extensionCalls.length, settings: runtime.settingsCalls.length };
}

/** Proves both real execution and model visibility use the established concrete hook registrations. */
test("policy bootstrap registers exactly the existing lifecycle and prompt handlers", async t => {
  const runtime = await environment(t);
  assert.equal(runtime.registrations.length, 2);
  assert.equal(runtime.registrations[0].kind, "lifecycle"); assert.equal(runtime.registrations[0].entry.id, "participant-tool-execution"); assert.equal(runtime.registrations[0].entry.function, runtime.toolCallPolicy);
  assert.equal(runtime.registrations[1].kind, "prompt"); assert.equal(runtime.registrations[1].entry.id, "participant-tool-visibility"); assert.equal(runtime.registrations[1].entry.function, runtime.toolPromptPolicy);
  assert.deepEqual(ioCounts(runtime), { disk: 0, extensions: 0, settings: 0 });
  assert.equal(await runtime.toolCallPolicy({ eventName: "tool_execution_finished", eventPayload: {} }), undefined);
  assert.deepEqual(ioCounts(runtime), { disk: 0, extensions: 0, settings: 0 });
});

/** Runs production policy against a full production-resolved snapshot, not current stored card permissions. */
test("actual execution policy accepts only frozen builtin and registered source permissions", async t => {
  const runtime = await environment(t), context = await bind(runtime), before = ioCounts(runtime);
  for (const name of ["read_file", "approved_package:read", "approved_skill:task", "approved_server:request", "search", "proxy", "package_proxy"]) assert.equal((await runtime.toolCallPolicy(event(name, context))).action, "allow", name);
  for (const name of ["delete_file", "another_package:read", "approved_package_name_collision:read"]) {
    const decision = await runtime.toolCallPolicy(event(name, context));
    assert.equal(decision.action, "block", name); assert.equal(decision.reason, "Selected execution participant is not allowed to access tool: " + name);
  }
  assert.equal((await runtime.toolCallPolicy(event("use_package", context, { package_name: "approved_package" }))).action, "allow");
  assert.equal((await runtime.toolCallPolicy(event("use_package", context, { package_name: "another_package" }))).action, "block");
  await assert.rejects(runtime.toolCallPolicy(event("use_package", context, {})), /actual package_name/);
  await assert.rejects(runtime.toolCallPolicy(event("approved_package:read:extra", context)), /Invalid registered package tool name/);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Rejects missing, malformed, mismatched and foreign snapshots instead of manufacturing a participant or reading current bindings. */
test("invalid execution context and unowned or incomplete snapshot reject explicitly", async t => {
  const runtime = await environment(t), context = await bind(runtime), before = ioCounts(runtime);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, participantId: "other" })), /does not match the frozen message snapshot/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { participantId: "default" })), /executionContext.chatId is required/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, chatId: null })), /requires a real chat context/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, extensionOwner: siblingOwner })), /extensionOwner does not match/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", context, {}, siblingOwner)), /authenticated registered owner/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, messageExtension: [] })), /messageExtension/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, messageExtension: { version: 1 } })), /message marker selection/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...context, plugins: { [siblingOwner]: {} } })), /unexpected fields/);
  const absent = plain(context); delete absent.messageExtension;
  await assert.rejects(runtime.toolCallPolicy(event("read_file", absent)), /messageExtension is required/);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Filters actual host descriptor objects identically at both real production prompt stages. */
test("prompt visibility preserves exact host descriptors under the same frozen execution policy", async t => {
  const runtime = await environment(t), context = await bind(runtime), before = ioCounts(runtime);
  const entries = [
    { catalogEntryId: "0", name: "read_file", categoryName: "Builtin", description: "unchanged", parameters: "full" },
    { catalogEntryId: "1", name: "delete_file", categoryName: "Builtin", description: "denied", parameters: "full" },
    { catalogEntryId: "2", name: "approved_package:read", categoryName: "Package", description: "unchanged package", parameters: "full" },
    { catalogEntryId: "3", name: "other_package:read", categoryName: "Package", description: "denied package", parameters: "full" },
    { catalogEntryId: "4", name: "use_package", activationSource: "approved_skill", categoryName: "Activation", description: "approved", parameters: "full" },
    { catalogEntryId: "5", name: "use_package", activationSource: "other_skill", categoryName: "Activation", description: "denied", parameters: "full" },
  ];
  for (const stage of ["filter_tool_call_tools", "build_tool_prompt"]) {
    const result = await runtime.toolPromptPolicy(prompt(stage, context, entries));
    assert.deepEqual(plain(result.availableTools), [entries[0], entries[2], entries[4]]);
    for (const entry of result.availableTools) assert.ok(entries.some(
      /** Requires retained real host object identity, not a policy-created replacement tool. */
      original => original === entry,
    ));
  }
  await assert.rejects(runtime.toolPromptPolicy({ containerPackageName: authenticatedOwner, eventPayload: { stage: "filter_tool_call_tools", availableTools: entries, metadata: {} } }), /executionContext/);
  await assert.rejects(runtime.toolPromptPolicy(prompt("build_tool_prompt", { ...context, extensionOwner: siblingOwner }, entries)), /extensionOwner does not match/);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Freezes a real group member's complete policy and rejects a different or absent execution participant. */
test("group snapshots keep exact per-participant tool policy without consulting current membership", async t => {
  const runtime = await environment(t); await bind(runtime);
  const other = await runtime.dispatchDomain("character.create", { values: { name: "Actual second participant", toolAccessConfig: {
    enabled: true, allowedBuiltinTools: ["delete_file"], allowedPackages: [], allowedSkills: [], allowedMcpServers: [],
  } } });
  const group = await runtime.dispatchDomain("group.create", { values: { name: "Actual group", members: [{ characterCardId: other.id, orderIndex: 0 }, { characterCardId: "default", orderIndex: 1 }] } });
  assert.equal(group.members[0].characterCardId, other.id);
  await runtime.dispatchDomain("chat.configuration.binding.write", { chatId: "group-chat", selection: "group:" + group.id });
  const first = await resolveContext(runtime, "group-chat", "default"), second = await resolveContext(runtime, "group-chat", other.id);
  const before = ioCounts(runtime);
  assert.equal((await runtime.toolCallPolicy(event("read_file", first))).action, "allow");
  assert.equal((await runtime.toolCallPolicy(event("read_file", second))).action, "block");
  assert.equal((await runtime.toolCallPolicy(event("delete_file", second))).action, "allow");
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...second, participantId: null })), /frozen message snapshot/);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", { ...second, participantId: "not-member" })), /frozen message snapshot/);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Applies actual persisted mid-turn card and binding edits while keeping the already-open turn immutable. */
test("current role and binding changes cannot change an already-open turn's three policy entry points", async t => {
  const runtime = await environment(t), original = await bind(runtime);
  await runtime.dispatchDomain("character.update", { id: "default", changes: { toolAccessConfig: {
    enabled: true, allowedBuiltinTools: ["delete_file"], allowedPackages: [], allowedSkills: [], allowedMcpServers: [],
  } } });
  const next = await resolveContext(runtime, "policy-chat", "default");
  const deleted = await runtime.dispatchDomain("chat.configuration.binding.delete", { chatId: "policy-chat" });
  assert.equal(deleted.deleted, true);
  const chat = runtime.chats.get("policy-chat"); assert.equal(chat.extensions.has(authenticatedOwner), false);
  assert.equal(chat.extensions.get(siblingOwner).selection, "card:sibling-only");
  const before = ioCounts(runtime), entries = [{ name: "read_file" }, { name: "delete_file" }];
  assert.equal((await runtime.toolCallPolicy(event("read_file", original))).action, "allow");
  assert.equal((await runtime.toolCallPolicy(event("delete_file", original))).action, "block");
  assert.equal((await runtime.toolCallPolicy(event("read_file", next))).action, "block");
  assert.equal((await runtime.toolCallPolicy(event("delete_file", next))).action, "allow");
  for (const stage of ["filter_tool_call_tools", "build_tool_prompt"]) {
    assert.deepEqual(plain((await runtime.toolPromptPolicy(prompt(stage, original, entries))).availableTools), [entries[0]]);
    assert.deepEqual(plain((await runtime.toolPromptPolicy(prompt(stage, next, entries))).availableTools), [entries[1]]);
  }
  assert.deepEqual(ioCounts(runtime), before);
});

/** Treats a host-isolated null namespace as genuine non-participation, never as permission to read a current character. */
test("explicit owner non-participation and non-chat calls perform zero reads", async t => {
  const runtime = await environment(t), context = await bind(runtime), absent = { ...context, messageExtension: null }, before = ioCounts(runtime);
  assert.equal(await runtime.toolCallPolicy(event("read_file", absent)), undefined);
  assert.equal(await runtime.toolCallPolicy(event("read_file", null)), undefined);
  for (const stage of ["filter_tool_call_tools", "build_tool_prompt"]) assert.equal(await runtime.toolPromptPolicy(prompt(stage, absent, [{ name: "read_file" }])), undefined);
  await assert.rejects(runtime.toolCallPolicy(event("read_file", absent, {}, siblingOwner)), /authenticated registered owner/);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Exercises concurrent actual plugin policy calls with different snapshots; Rust task-local runtime execution is validated separately, not claimed by this unit test. */
test("concurrent snapshot policy evaluation does not leak the next turn's permissions", async t => {
  const runtime = await environment(t), first = await bind(runtime);
  await runtime.dispatchDomain("character.update", { id: "default", changes: { toolAccessConfig: {
    enabled: true, allowedBuiltinTools: ["delete_file"], allowedPackages: [], allowedSkills: [], allowedMcpServers: [],
  } } });
  const second = await resolveContext(runtime, "policy-chat", "default"), before = ioCounts(runtime), entries = [{ name: "read_file" }, { name: "delete_file" }];
  const results = await Promise.all([
    runtime.toolCallPolicy(event("read_file", first)), runtime.toolCallPolicy(event("read_file", second)),
    runtime.toolPromptPolicy(prompt("build_tool_prompt", first, entries)), runtime.toolPromptPolicy(prompt("filter_tool_call_tools", second, entries)),
  ]);
  assert.deepEqual(results.slice(0, 2).map(
    /** Compares each production decision independently rather than only checking some permission was granted. */
    result => result.action,
  ), ["allow", "block"]);
  assert.deepEqual(plain(results[2].availableTools), [entries[0]]); assert.deepEqual(plain(results[3].availableTools), [entries[1]]);
  assert.deepEqual(ioCounts(runtime), before);
});

/** Keeps existing real IO failures explicit in their actual service while proving policy does not reread that service. */
test("queued repository and host errors remain unconsumed by frozen policy and propagate on the real dependency read", async t => {
  const runtime = await environment(t), context = await bind(runtime), before = ioCounts(runtime);
  const diskFailure = new Error("snapshot-must-not-reread-disk"); runtime.disk.failNext("read", diskFailure);
  const hostFailure = new Error("snapshot-must-not-reread-binding"); runtime.failNextHost("readExtension", hostFailure);
  assert.equal((await runtime.toolCallPolicy(event("read_file", context))).action, "allow");
  assert.deepEqual(ioCounts(runtime), before);
  await assert.rejects(runtime.dispatchDomain("character.get", { id: "default" }), error => error === diskFailure);
  await assert.rejects(runtime.dispatchDomain("chat.configuration.binding.read", { chatId: context.chatId }), error => error === hostFailure);
});
