import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { plain } from "./runtime.mjs";

/** Compares real registered presentation and execution APIs after durable role binding changes. */
test("composer model selection matches execution for fixed, global, model-only and grouped conversations", async t => {
  const runtime = await selectorEnvironment(t);
  const { models } = await runtime.tools.Models.listSummaries();
  const { configs } = await runtime.tools.Tts.listConfigs();
  models.push({ ...models[0], providerId: "fixed-provider", providerName: "Fixed provider", modelId: "fixed-model" });
  /** Supplies two explicit model catalog records at the test host boundary. */
  runtime.tools.Models.listSummaries = async () => ({ models: plain(models) });
  const global = { providerId: models[0].providerId, modelId: models[0].modelId };
  const fixed = { providerId: models[1].providerId, modelId: models[1].modelId };
  const card = await runtime.api("character.create", { values: { name: "Fixed role", chatModelBindingMode: "FIXED_MODEL", chatProviderId: fixed.providerId, chatModelId: fixed.modelId } });
  const follower = await runtime.api("character.create", { values: { name: "Global role" } });
  /** Calls the authentic host-only presentation handler for this real bound conversation. */
  const display = async binding => plain(await runtime.api("chat.model.selection", { chatId: "selector-chat", defaultModelBinding: binding }));
  /** Resolves the same live records through actual message execution, including provider validation. */
  const execution = async participantId => plain(await runtime.api("chat.configuration.resolve", {
    purpose: "execution", chatId: "selector-chat", chatExtension: null, participantId,
    promptFunctionType: "CHAT", defaultModelBinding: global, defaultTtsConfigId: configs[0].id,
  }));
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id });
  const selected = await display(global);
  assert.equal(selected.participants[0].followsGlobal, false);
  assert.deepEqual(selected.participants[0].binding, fixed);
  assert.deepEqual(selected.participants[0].binding, (await execution(null)).profile.modelBinding);
  assert.deepEqual((await display({ providerId: "changed-global", modelId: "cheap-model" })).participants[0].binding, fixed);

  await runtime.api("character.update", { id: card.id, changes: { chatModelBindingMode: "FOLLOW_GLOBAL", chatProviderId: null, chatModelId: null } });
  assert.equal((await display(global)).participants[0].followsGlobal, true);
  assert.deepEqual((await display(global)).participants[0].binding, (await execution(null)).profile.modelBinding);
  await runtime.api("character.update", { id: card.id, changes: { chatModelBindingMode: "FIXED_MODEL", chatProviderId: fixed.providerId, chatModelId: fixed.modelId } });
  const group = await runtime.api("group.create", { values: { name: "Mixed group", members: [{ characterCardId: card.id, orderIndex: 4 }, { characterCardId: follower.id, orderIndex: 1 }] } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "group:" + group.id });
  const grouped = await display(global);
  assert.deepEqual(grouped.participants.map(participant => participant.id), [follower.id, card.id]);
  for (const participant of grouped.participants) assert.deepEqual(participant.binding, (await execution(participant.id)).profile.modelBinding);

  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "model:chat" });
  assert.deepEqual((await display(global)).participants[0].binding, (await execution(null)).profile.modelBinding);
});

/** Presentation reads only the demanded role records and never requires speech, memory or tool catalogs. */
test("registered model presentation avoids execution catalogs and shares ordinary model mode for unassociated chats", async t => {
  const runtime = await selectorEnvironment(t);
  const { models } = await runtime.tools.Models.listSummaries();
  const global = { providerId: models[0].providerId, modelId: models[0].modelId };
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  /** Fails any execution catalog request made during display-only model resolution. */
  runtime.tools.Tts.listConfigs = async () => { throw new Error("Display must not read TTS"); };
  runtime.tools.Models.listSummaries = async () => { throw new Error("Display must not read model catalogs"); };
  runtime.tools.Catalog.readToolSources = async () => { throw new Error("Display must not read tools"); };
  const selection = await runtime.api("chat.model.selection", { chatId: "selector-chat", defaultModelBinding: global });
  assert.equal(selection.participants.length, 1);

  assert.deepEqual(plain(await runtime.api("chat.model.selection", { chatId: "unbound-chat", defaultModelBinding: global })), {
    participants: [{ id: "model:chat", name: "仅使用模型", binding: global, followsGlobal: true }],
  });
  await runtime.seedBinding("selector-chat", { version: 999, selection: "card:default" });
  await assert.rejects(runtime.api("chat.model.selection", { chatId: "selector-chat", defaultModelBinding: global }), /version/);
});
