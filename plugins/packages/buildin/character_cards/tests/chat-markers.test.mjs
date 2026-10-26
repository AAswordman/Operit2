import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const markers = loadModule("src/chat-markers.ts"), domain = loadModule("src/domain.ts");

/** Supplies a complete explicitly controlled current execution profile, never a production directory replacement. */
function executionProfile() {
  return { id: "removed-card", name: "原始身份", avatarUri: "resources/avatar.png", introPrompt: "原始角色提示", userPreferencesText: "# USER\n原始用户档案", openingStatement: "原始开场白",
    modelBinding: { providerId: "old-provider", modelId: "old-model" }, ttsConfigId: "old-voice",
    toolAccess: { enabled: true, allowedBuiltinTools: ["read_file"], allowedPackages: ["one-package"], allowedSkills: ["one-skill"], allowedMcpServers: ["one-server"] },
    resources: [{ key: "character:removed-card", readable: true, writable: true }, { key: "shared:old-resource", readable: true, writable: false }] };
}
/** Constructs the exact currently locked resolver request without any Core-owned selection token. */
function request(chatExtension) {
  return { purpose: "execution", chatId: "draft-not-persisted", chatExtension, participantId: null, promptFunctionType: "CHAT",
    defaultModelBinding: { providerId: "controlled-provider", modelId: "controlled-model" }, defaultTtsConfigId: "controlled-voice" };
}
/** Supplies explicit controlled model, voice and source records solely for the real disk-service test. */
function controlledDirectories(calls) {
  return {
    /** Returns the single explicitly scheduled model directory fixture. */
    async listModels() {
      calls.push("models");
      return [{ providerId: "controlled-provider", providerName: "Controlled provider", providerTypeId: "controlled-type", endpoint: "controlled://model", modelId: "controlled-model",
        capabilities: { directImage: false, directAudio: false, directVideo: false, toolCall: true }, pricing: null }];
    },
    /** Returns the complete explicitly scheduled voice configuration fixture. */
    async listTtsConfigs() {
      calls.push("tts");
      return [{ id: "controlled-voice", name: "Controlled voice", providerType: "controlled", endpoint: "controlled://voice", apiKey: "fixture-only", model: "controlled-tts", voice: "controlled-voice", responseFormat: "wav", speed: 1,
        httpMethod: "POST", requestBody: "{}", contentType: "application/json", headers: [], responsePipeline: [], createdAt: 1700000000000, updatedAt: 1700000000000 }];
    },
    /** Returns explicit controlled source catalogs without implementing a production provider. */
    async readToolCatalog() {
      calls.push("tools");
      return { builtinTools: [{ name: "read_file", displayName: "Controlled read", description: "Fixture only" }], packages: [], skills: [], mcpServers: [] };
    },
  };
}

/** Checks exact chat versions and lossless updates of other actual fields in the authenticated namespace. */
test("chat marker helpers validate explicit versions and preserve unrelated namespace fields when changing selection", () => {
  const extension = { version: 1, selection: "card:first", notes: { title: "保留", records: [null, 42, true] }, state: "opaque" };
  const encoded = plain(markers.encodeChatMarker("group:second", extension));
  assert.deepEqual(encoded, { ...extension, selection: "group:second" }); assert.equal(extension.selection, "card:first");
  assert.deepEqual(plain(markers.decodeChatMarker(encoded)), { version: 1, selection: "group:second" });
  assert.deepEqual(plain(markers.encodeChatMarker("card:first", null)), { version: 1, selection: "card:first" });
  for (const invalid of [null, {}, { version: 2, selection: "card:first" }, { version: 1 }, { version: 1, selection: "unknown:first" },
    { version: 1, selection: "card:first", groupId: "1" }, { version: 1, selection: "card:first", illegal: undefined }]) {
    assert.throws(() => markers.decodeChatMarker(invalid));
  }
});

/** Verifies the historical snapshot is complete and rejects missing voice, policies, resources or duplicate participants. */
test("runtime execution data preserves complete profiles and refuse partial or contradictory identity snapshots", () => {
  const profile = executionProfile();
  const marker = { version: 1, selection: "card:removed-card", promptFunctionType: "VOICE", primaryOwnerKey: "character:removed-card", profile, participants: [plain(profile)] };
  const encoded = plain(markers.encodeExecutionData(marker)); assert.deepEqual(encoded, marker);
  assert.deepEqual(plain(markers.decodeExecutionData(encoded)), marker);
  const badVoice = plain(marker); delete badVoice.profile.ttsConfigId;
  const badResource = plain(marker); delete badResource.participants[0].resources[0].writable;
  const badPolicy = plain(marker); delete badPolicy.profile.toolAccess.allowedSkills;
  const mismatched = plain(marker); mismatched.profile.name = "另一身份";
  const duplicate = plain(marker); duplicate.participants.push(plain(profile));
  const missingOwner = plain(marker); delete missingOwner.primaryOwnerKey;
  const forbiddenOwner = { ...plain(marker), primaryOwnerKey: "shared:old-resource" };
  for (const invalid of [badVoice, badResource, badPolicy, mismatched, duplicate, missingOwner, forbiddenOwner, { ...marker, version: 0 }, { ...marker, profile: null }]) {
    assert.throws(() => markers.decodeExecutionData(invalid));
  }
});

/** The public request cannot carry frozen prompt, model, voice or permission configuration. */
test("resolver rejects historical configuration inputs before reading current storage", () => {
  const input = request({ version: 1, selection: "card:default" });
  for (const field of ["executionData", "messageExtension"]) {
    assert.throws(() => domain.parseChatConfigurationRequest({ ...input, [field]: { profile: executionProfile() } }), /not a supported field/);
  }
});

/** Exercises the actual file service with a creation draft and strictly disallowed host chat capabilities. */
test("draft resolver consumes supplied chatExtension and returns a complete persisted-domain send snapshot without querying a nonexistent chat", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), calls = [], files = loadModule("src/storage/database.ts", disk.globals);
  const repository = await files.DatabaseCharacterRepository.open(controlledDirectories(calls));
  const service = loadModule("src/service-runtime.ts", disk.globals).createCharacterCardsService(repository);
  const result = plain(await service.dispatchDomain("chat.configuration.resolve", request({ version: 1, selection: "card:default" }, null)));
  assert.equal(result.contextKey, "card:default"); assert.equal(result.profile.id, "default"); assert.equal(result.participants.length, 1);
  assert.deepEqual(result.profile.modelBinding, { providerId: "controlled-provider", modelId: "controlled-model" });
  assert.equal(result.profile.ttsConfigId, "controlled-voice");
  assert.deepEqual(plain(markers.decodeExecutionData(result.executionData)).profile, result.profile);
  assert.deepEqual(calls.sort(), ["models", "tts"]);
  await assert.rejects(() => service.dispatchDomain("chat.configuration.resolve", request(null, null)), /./);
  await assert.rejects(() => service.dispatchDomain("chat.configuration.resolve", request({ version: 1, selection: "card:missing" }, null)), /Character not found: missing/);
});

/** Rejects the removed Core selection field and every absent mandatory extension field at the public domain boundary. */
test("resolver request rejects old selection and missing extension fields rather than deriving a binding from active state", () => {
  const valid = request({ version: 1, selection: "card:default" }, null);
  assert.throws(() => domain.parseChatConfigurationRequest({ ...valid, selection: "card:default" }), /not a supported field/);
  const missingChat = plain(valid); delete missingChat.chatExtension;
  assert.throws(() => domain.parseChatConfigurationRequest(missingChat));
});
