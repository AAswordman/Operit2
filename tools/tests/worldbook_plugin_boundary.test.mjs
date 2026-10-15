import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

/** Executes the production prompt hook with independent storage fixtures. */
function runtime() {
  const exports = {};
  const entries = [
    { id: 'global', name: 'Global', content: 'global lore', always_active: true, inject_target: 'system' },
    { id: 'a', name: 'A', content: 'role A lore', always_active: true, inject_target: 'system', character_card_id: 'card-a' },
    { id: 'b', name: 'B', content: 'role B lore', always_active: true, inject_target: 'system', character_card_id: 'card-b' },
  ];
  const context = vm.createContext({ exports, console, require(name) {
    if (name.includes('index.ui.js')) return {};
    if (name.includes('worldbook_storage')) return { async readWorldBookEntries() { return entries; } };
    if (name.includes('worldbook_variables')) return {
      async syncWorldBookVariableContext() { return {}; },
      renderWorldBookContent(text) { return text; },
    };
    throw new Error('Unexpected dependency: ' + name);
  } });
  vm.runInContext(readFileSync(new URL('../../plugins/packages/external/worldbook/dist/main.js', import.meta.url), 'utf8'), context);
  return exports;
}

function event(participantId) {
  return { eventName: 'after_compose_system_prompt', eventPayload: {
    chatId: 'chat-a', systemPrompt: 'base', metadata: {
      executionContext: { chatId: 'chat-a', participantId },
    },
  } };
}

test('worldbook uses the actual execution participant rather than native role lookup', async () => {
  const hooks = runtime();
  const result = await hooks.systemPromptHook(event('card-a'));
  assert.match(result.systemPrompt, /global lore/);
  assert.match(result.systemPrompt, /role A lore/);
  assert.doesNotMatch(result.systemPrompt, /role B lore/);
});

test('no participant activates only unbound entries, never a guessed active character', async () => {
  const hooks = runtime();
  const result = await hooks.systemPromptHook(event(null));
  assert.match(result.systemPrompt, /global lore/);
  assert.doesNotMatch(result.systemPrompt, /role [AB] lore/);
});

test('mismatched execution chat is rejected instead of using another chat role', async () => {
  const request = event('card-a');
  request.eventPayload.metadata.executionContext.chatId = 'chat-b';
  await assert.rejects(runtime().systemPromptHook(request), /does not match/);
});

test('worldbook declares and calls its character-plugin dependency, not a host business API', () => {
  const manifest = JSON.parse(readFileSync(new URL('../../plugins/packages/external/worldbook/manifest.json', import.meta.url), 'utf8'));
  assert.ok(manifest.requires.some(entry => entry.id === 'com.operit.character_cards'));
  const service = readFileSync(new URL('../../plugins/packages/external/worldbook/src/shared/worldbook_service.ts', import.meta.url), 'utf8');
  assert.match(service, /ToolPkg\.callDependency/);
  assert.doesNotMatch(service, /Tools\.Chat\.listCharacterCards/);
});
