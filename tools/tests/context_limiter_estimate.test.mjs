import assert from 'node:assert/strict';
import test from 'node:test';
import { loadModule } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

/** Boots the actual limiter registrations with explicit persisted settings. */
function plugin(enabled = true) {
  const hooks = {};
  const main = loadModule('../../external/context_limiter/src/main.ts', {
    getEnv: key => key === 'CTX_LIMITER_FLOOR_LIMIT' ? '1' : String(enabled),
    ToolPkg: {
      registerPromptFinalizeHook: hook => { hooks.send = hook.function; },
      registerPromptEstimateFinalizeHook: hook => { hooks.estimate = hook.function; },
      registerInputMenuTogglePlugin() {},
    },
    console: { log() {} },
  });
  main.registerToolPkg();
  return hooks;
}

/** Gives both request paths the same history with structured tool boundaries. */
function history() {
  return [
    { kind: 'SYSTEM', content: 'rules' },
    { kind: 'USER', content: 'old question' },
    { kind: 'ASSISTANT', content: 'old answer' },
    { kind: 'USER', content: 'latest question' },
    { kind: 'TOOL_CALL', content: 'call', toolName: 'search' },
    { kind: 'TOOL_RESULT', content: 'result', toolName: 'search' },
  ];
}

/** Counts the same retained floors for estimates and sends without changing stored history. */
test('limiter estimates use the real floor transformation and preserve tool turns', () => {
  const hooks = plugin();
  const original = history();
  const event = { eventPayload: { preparedHistory: original } };
  const send = structuredClone(hooks.send(event));
  const estimate = structuredClone(hooks.estimate(event));
  assert.deepEqual(estimate, send);
  assert.deepEqual(estimate.preparedHistory.map(turn => turn.content), ['rules', 'latest question', 'call', 'result']);
  assert.equal(original.length, 6);
  assert.equal(estimate.preparedHistory[2].toolName, 'search');
});

/** Leaves independent estimates unchanged when the actual limiter setting is disabled. */
test('disabled limiter contributes nothing to either request path', () => {
  const hooks = plugin(false);
  const event = { eventPayload: { preparedHistory: history() } };
  assert.equal(hooks.send(event), null);
  assert.equal(hooks.estimate(event), null);
});
