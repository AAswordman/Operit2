import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

/** Runs the generated production plugin with recorded generic SDK calls. */
function runtime(send = async () => ({ sent: true })) {
  const calls = [];
  const results = [];
  const context = vm.createContext({
    exports: {}, console, setTimeout, clearTimeout,
    complete: value => results.push(value),
    Tools: { Chat: {
      async sendMessage(request) { calls.push(request); return send(request); },
      async getMessages() { return { messages: [{ sender: 'ai', content: 'answer', timestamp: 1 }] }; },
    } },
  });
  vm.runInContext(readFileSync(new URL('../../plugins/.out/external/extended_chat.js', import.meta.url), 'utf8'), context);
  return { tools: context.exports, calls, results };
}

test('external chat sends original input and explicit runtime/participant using the generic SDK', async () => {
  const { tools, calls, results } = runtime();
  await tools.chat_with_agent({ message: ' original input ', chat_id: 'chat-a', runtime: 'floating', participant_id: 'speaker-a', notify_reply: true });
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [{
    kind: 'submit', chatId: 'chat-a', runtime: 'floating',
    input: { text: ' original input ', attachments: [], replyToMessageTimestamp: null },
    turn: { kind: 'execute', participantId: 'speaker-a' }, notifyReply: true,
  }]);
  assert.equal(results[0].success, true);
  assert.equal(tools.list_character_cards, undefined);
});

test('invalid or retired inputs fail before dispatch instead of fabricating a role binding', async () => {
  for (const extra of [
    { runtime: 'unknown' }, { chat_id: '' }, { participant_id: ' ' },
    { timeout: -1 }, { character_card_name: 'old role' }, { persist_turn: false },
  ]) {
    const { tools, calls, results } = runtime();
    await tools.chat_with_agent({ message: 'hello', chat_id: 'chat-a', runtime: 'main', ...extra });
    assert.equal(results[0].success, false);
    assert.equal(calls.length, 0);
  }
});

test('chat transcript formatting consumes generic sender metadata', async () => {
  const { tools, results } = runtime();
  await tools.read_messages({ chat_id: 'chat-a' });
  assert.equal(results[0].success, true);
  assert.equal(results[0].data.text, '[1] ai:\nanswer');
});
