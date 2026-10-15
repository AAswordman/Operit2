import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import { installScopedHostRuntime, installGeneratedToolsRuntime, domainToolResult } from './fixtures/scoped_host_runtime.mjs';

/** Uses freshly generated production Tools and the real toolCall structured transport. */
function fixture() {
  const sandbox = vm.createContext({});
  const registry = installScopedHostRuntime(sandbox);
  const calls = [];
  sandbox.__operitNativeCallToolStructured = registry.binding((requestId, type, name, parameters) => {
    calls.push({ requestId, type, name, parameters });
  });
  installGeneratedToolsRuntime(sandbox);
  vm.runInContext(`
    var originalToolCall = toolCall;
    var toolCallInvocations = [];
    toolCall = function(name, parameters) {
      toolCallInvocations.push({name, parameters});
      return originalToolCall.apply(this, arguments);
    };
  `, sandbox);
  return { sandbox, registry, calls, run: code => vm.runInContext(code, sandbox) };
}

/** Exercises the actual generated full-list binding and preserves all supplied native summaries. */
test('Chat.listAll explicitly requests complete records while listChats retains its limit', async () => {
  const chats = Array.from({ length: 251 },
    /** Supplies a distinct native summary for every persisted conversation in the fixture. */
    (_, index) => ({ id: 'chat-' + index, title: 'Chat ' + index, messageCount: 0,
      createdAt: '1700000000000', updatedAt: '1700000000000', isCurrent: false,
      inputTokens: 0, outputTokens: 0 }),
  );
  for (const [expression, parameters, records] of [
    ['Tools.Chat.listAll()', { list_all: 'true' }, chats],
    ['Tools.Chat.listChats({limit:7})', { limit: 7 }, chats.slice(0, 7)],
  ]) {
    const f = fixture(), promise = f.run(expression);
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0].name, 'list_chats');
    assert.deepEqual(structuredClone(f.calls[0].parameters), parameters);
    const result = { totalCount: chats.length, currentChatId: null, chats: records };
    f.registry.settle(f.calls[0].requestId, { success: true, data: result }, false);
    assert.deepEqual(structuredClone(await promise), result);
    assert.equal(f.sandbox.refs, 0);
  }
});

/** Tests execution through toolCall, rather than merely finding its name in generated source. */
test('all former Runtime operations invoke toolCall and the ordinary structured host once', async () => {
  const cases = [
    ["Tools.Models.listSummaries()", 'models_list_summaries', {}, []],
    ["Tools.Tts.listConfigs()", 'tts_list_configs', {}, []],
    ["Tools.Tts.getCurrentConfigId()", 'tts_get_current_config_id', {}, 'current'],
    ["Tools.Tts.setCurrentConfigId('tts')", 'tts_set_current_config_id', {id:'tts'}, 'tts'],
    ["Tools.Theme.listConfigs()", 'theme_list_configs', {}, []],
    ["Tools.Theme.applyConfig('theme')", 'theme_apply_config', {id:'theme'}, {id:'theme'}],
    ["Tools.Catalog.readToolSources()", 'catalog_read_tool_sources', {}, {builtinTools:[],packages:[],skills:[],mcpServers:[]}],
    ["Tools.Chat.sendMessage({chatId:'chat',input:{text:'message',attachments:[]}})", 'chat_send_message', {request:{chatId:'chat',input:{text:'message',attachments:[]}}}, {chatId:'chat'}],
    ["Tools.Chat.cancel('chat')", 'chat_cancel', {chatId:'chat'}, {chatId:'chat',cancelRequested:true}],
    ["Tools.Chat.readExtension({kind:'chat',chatId:'chat'})", 'chat_read_extension', {target:{kind:'chat',chatId:'chat'}}, null],
    ["Tools.Chat.writeExtension({kind:'chat',chatId:'chat'},{nested:[null,false]})", 'chat_write_extension', {target:{kind:'chat',chatId:'chat'},value:{nested:[null,false]}}, {nested:[null,false]}],
    ["Tools.Chat.deleteExtension({kind:'chat',chatId:'chat'})", 'chat_delete_extension', {target:{kind:'chat',chatId:'chat'}}, false],
    ["Tools.Storage.request({op:'close',handle:'database'})", 'storage_request', {request:{op:'close',handle:'database'}}, null],
  ];
  for (const [expression, name, parameters, result] of cases) {
    const f = fixture();
    const promise = f.run(expression);
    assert.equal(f.calls.length, 1, expression);
    assert.equal(f.sandbox.toolCallInvocations.length, 1, expression);
    assert.equal(f.sandbox.toolCallInvocations[0].name, name);
    assert.equal(f.calls[0].type, 'default');
    assert.equal(f.calls[0].name, name);
    assert.deepEqual(structuredClone(f.calls[0].parameters), parameters);
    f.registry.settle(f.calls[0].requestId, {success:true,data:domainToolResult(name,result,parameters)}, false);
    assert.deepEqual(structuredClone(await promise), domainToolResult(name, result, parameters));
    assert.equal(f.sandbox.refs, 0);
  }
});

/** Streaming resource operations use the same toolCall entry and preserve lazy admission. */
test('stream open, next and close each invoke toolCall without another native transport', async () => {
  const f = fixture();
  f.run("var stream = Tools.Chat.sendMessageStreaming({chatId:'chat'});");
  assert.equal(f.calls.length, 0);
  const pulling = f.run('stream.next()');
  assert.equal(f.calls[0].name, 'chat_observe');
  assert.equal(f.calls[0].parameters.observation.op, 'open');
  f.registry.settle(f.calls[0].requestId, {success:true,data:{op:'open',handle:'observation'}}, false);
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls[1].name, 'chat_observe');
  assert.equal(f.calls[1].parameters.observation.op, 'next');
  f.registry.settle(f.calls[1].requestId, {success:true,data:{op:'next',event:{type:'part',parts:[]}}}, false);
  await pulling;
  const closing = f.run('stream.return()');
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls[2].name, 'chat_observe');
  assert.equal(f.calls[2].parameters.observation.op, 'close');
  f.registry.settle(f.calls[2].requestId, {success:true,data:{op:'close'}}, false);
  await closing;
  assert.equal(f.sandbox.toolCallInvocations.length, 3);
  assert.equal(f.sandbox.refs, 0);
});

/** Prevents reintroducing a separate SDK tool-name catalog or inline registration implementations. */
test('Tools bindings and ordinary registrations have one canonical tool catalog', () => {
  const source = path => readFileSync(new URL('../../' + path, import.meta.url), 'utf8');
  const bindings = source('core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs');
  const registration = source('core/crates/tool/services/src/tools/ToolRegistration.rs');
  const generator = source('core/crates/plugin/codegen/src/runtime_bindings.rs');
  assert.doesNotMatch(bindings, /RuntimeToolName|JsToolTarget/);
  assert.doesNotMatch(registration, /registerStructuredTools|registerSharedTool|TypedToolExecutor/);
  assert.doesNotMatch(generator, /binding\.runtime|JsToolTarget/);
  const toolTypes = source('core/crates/plugin/sdk/src/js_sdk/tool_types.rs');
  for (const name of ['storage_request','chat_send_message','chat_observe','chat_cancel','chat_read_extension','chat_write_extension','chat_delete_extension','models_list_summaries','tts_list_configs','tts_get_current_config_id','tts_set_current_config_id','theme_list_configs','theme_apply_config','catalog_read_tool_sources']) {
    assert.doesNotMatch(toolTypes, new RegExp(`pub ${name}: serde_json::Value`));
  }
});
