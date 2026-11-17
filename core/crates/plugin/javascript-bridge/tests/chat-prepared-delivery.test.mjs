import { installScopedHostRuntime, installGeneratedToolsRuntime } from '../../../../../tools/tests/fixtures/scoped_host_runtime.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
function runtime(native) {
  const context = vm.createContext({}), registry = installScopedHostRuntime(context);
  context.__operitNativeCallToolStructured = registry.binding((id, type, name, parameters) => {
    assert.equal(type, 'default');
    try { registry.settle(id, { success: true, data: native(name, JSON.parse(JSON.stringify(parameters))) }); }
    catch (error) { registry.settle(id, String(error), true); }
  });
  installGeneratedToolsRuntime(context); return context;
}
const plain = value => JSON.parse(JSON.stringify(value));
test('uniform message scheduling uses typed registered tools and actual owner-scoped receipts', async () => {
  const requests = [], message = { id: 'slot', chatId: 'chat', timestamp: 999999, status: 'scheduled', error: null };
  const c = runtime((name, parameters) => {
    requests.push({ name, parameters });
    const results = {
      chat_deliver_prepared: { chatId: 'chat', timestamp: 123, duplicate: false, notificationError: null },
      chat_schedule_message: { message }, chat_list_scheduled_messages: { messages: [message] },
      chat_cancel_scheduled_message: { id: 'slot', cancelled: true },
    };
    assert.ok(results[name], `Unexpected tool: ${name}`); return results[name];
  });
  const request = { id: 'slot', chatId: 'chat', title: 'Title', text: 'Hello', timestamp: 999999 };
  assert.equal(c.Tools.Chat.getDeliveryCapabilities, undefined);
  assert.equal(c.Tools.Chat.scheduleNotification, undefined);
  assert.deepEqual(plain(await vm.runInContext(`Tools.Chat.scheduleMessage(${JSON.stringify(request)})`, c)), { message });
  assert.deepEqual(plain(await c.Tools.Chat.listScheduledMessages()), { messages: [message] });
  assert.deepEqual(plain(await c.Tools.Chat.cancelScheduledMessage('slot')), { id: 'slot', cancelled: true });
  assert.deepEqual(requests, [
    { name: 'chat_schedule_message', parameters: { request } },
    { name: 'chat_list_scheduled_messages', parameters: {} },
    { name: 'chat_cancel_scheduled_message', parameters: { id: 'slot' } },
  ]); assert.equal(c.refs, 0);
});
test('native scheduling errors propagate and strict SDK calls reject incorrect arguments', async () => {
  let calls = 0; const c = runtime(() => { calls++; throw Error('Notification permission denied'); });
  await assert.rejects(vm.runInContext('Tools.Chat.scheduleMessage({ id: "slot", chatId: "chat", title: "Title", text: "Text", timestamp: 999999 })', c), /Notification permission denied/);
  assert.equal(c.refs, 0); const previous = calls;
  for (const expression of ['Tools.Chat.scheduleMessage()', 'Tools.Chat.cancelScheduledMessage("slot", "extra")', 'Tools.Chat.listScheduledMessages("extra")']) {
    await assert.rejects(async () => vm.runInContext(expression, c));
  } assert.equal(calls, previous);
});
