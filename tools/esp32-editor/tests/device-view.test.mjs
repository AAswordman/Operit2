import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {DeviceView} from '../web/device-view.ts';
import {simulatorViewState} from '../web/device-state.ts';
import {ScenePreview} from '../web/scene-renderer.ts';

async function fixture() {
  globalThis.window ??= {};
  const {default: create} = await import('../generated/ui.mjs');
  const ui = await create({wasmBinary: await readFile(new URL('../generated/ui.wasm', import.meta.url))});
  assert.equal(ui._simulator_init(), 1);
  const actions = [], receipts = [];
  ui.onAction = action => actions.push(action);
  const view = new DeviceView(ui, new ScenePreview(), (...result) => receipts.push(result));
  const inspect = () => JSON.parse(ui.ccall('operit_ui_debug_snapshot', 'string', [], []));
  return {ui, view, inspect, actions, receipts};
}

test('browser adapter forwards history cursors to shared C UI lazy paging', async () => {
  const {ui, view, actions, inspect} = await fixture();
  view.update(simulatorViewState({ready: true, device: {sessionId: 'one', paired: true,
    chat: {connected: true, hasOlder: true, hasNewer: false,
      messages: Array.from({length: 12}, (_, i) => ({sender: 'user', text: '消息 ' + i}))}}}));
  assert(inspect().chatScrollLine > 0);
  for (let i = 0; i < 8; i++) ui.ccall('operit_ui_debug_swipe', 'number', ['string'], ['down']);
  assert(actions.includes('edge_history_older'), 'history flag must reach C, not stop at browser state');
  actions.length = 0;
  view.update(simulatorViewState({ready: true, device: {sessionId: 'one', paired: true,
    chat: {connected: true, hasOlder: false, hasNewer: false}}}));
  for (let i = 0; i < 8; i++) ui.ccall('operit_ui_debug_swipe', 'number', ['string'], ['down']);
  assert.deepEqual(actions, []);
});

test('stopped device clears C pairing, approval, chat and plugin state without old data', async () => {
  const {ui, view, inspect} = await fixture();
  view.update(simulatorViewState({ready: true, device: {sessionId: 'one', paired: true,
    pairingCode: '123456', spaceJoinPrompt: '申请加入空间',
    plugins: {items: [{id: 'old', name: '旧插件'}]},
    chat: {connected: true, messages: [{sender: 'user', text: 'OLD_MESSAGE'}]}}}));
  ui.ccall('operit_ui_set_chat_draft', null, ['string'], ['保留草稿']);
  assert.equal(inspect().page, 'Chat');
  view.update(simulatorViewState({ready: false, device: null}));
  const stopped = inspect();
  assert.equal(stopped.pairingCode, '');
  assert(!['Pairing', 'Space', 'edge_scene'].includes(stopped.page));
  assert(!stopped.nodes.some(node => ['edge_space_approve', 'edge_space_reject'].includes(node.id)));
  assert(!JSON.stringify(stopped).includes('OLD_MESSAGE'));
  assert(!JSON.stringify(stopped).includes('旧插件'));
  assert.equal(ui.ccall('operit_ui_debug_tap', 'number', ['string'], ['sidebar_toggle']), 1);
  assert.equal(ui.ccall('operit_ui_debug_tap', 'number', ['string'], ['sidebar_settings']), 1);
  assert.equal(inspect().nodes.find(node => node.id === 'link_status').text, '未配对');
  ui._operit_ui_set_connection(1, 1);
  ui.ccall('operit_ui_submit_chat', null, [], []);
  assert.equal(ui.ccall('operit_ui_chat_draft', 'string', [], []), '保留草稿');
});

test('each browser sees a receipt once and accepts reused receipt ids in a new process', async () => {
  const first = await fixture(), second = await fixture();
  const state = simulatorViewState({ready: true, device: {sessionId: 'one',
    chatSendResult: {id: 1, sendId: 'browser-send', ok: true}}});
  for (const browser of [first, second]) {
    browser.view.update(state); browser.view.update(state);
    assert.deepEqual(browser.receipts, [[true, undefined, 'browser-send']]);
    browser.view.update({...state, sessionId: 'two'});
    assert.equal(browser.receipts.length, 2);
  }
});

test('stop invalidates pending scene fetch and a restarted process can reuse a scene revision', async t => {
  const original = globalThis.fetch;
  t.after(() => { globalThis.fetch = original; });
  let resolve;
  let calls = 0;
  globalThis.fetch = () => { calls++; return new Promise(done => { resolve = done; }); };
  const {ui, view, inspect} = await fixture();
  const active = sessionId => ({sessionId, running: true, scene: {active: true, revision: 1}});
  view.update(active('one'));
  assert.equal(calls, 1);
  view.update({running: false});
  resolve({ok: true, json: async () => ({active: true, revision: 1, elapsed: 0, packs: [], layers: []})});
  await new Promise(done => setImmediate(done));
  assert.notEqual(inspect().page, 'edge_scene');
  view.update(active('two'));
  assert.equal(calls, 2);
  resolve({ok: true, json: async () => ({active: true, revision: 1, elapsed: 0, packs: [], layers: []})});
  await new Promise(done => setImmediate(done));
  assert.equal(inspect().page, 'edge_scene');
  // Miss the stopped poll: identical summary still fetches the new process's scene.
  view.update(active('three'));
  assert.equal(calls, 3);
  view.update({running: false});
  assert.notEqual(inspect().page, 'edge_scene');
});

test('scene preview retries a transient null snapshot at the same revision', async t => {
  const original = globalThis.fetch;
  t.after(() => { globalThis.fetch = original; });
  let calls = 0;
  globalThis.fetch = async () => { calls++; return {ok: true, json: async () => null}; };
  const {view} = await fixture();
  const state = {sessionId: 'one', running: true, scene: {active: true, revision: 1}};
  view.update(state);
  await new Promise(done => setImmediate(done));
  view.update(state);
  assert.equal(calls, 2);
});

test('browser adapter forwards real conversation metadata, loading, error and selected identity',async()=>{
  const {ui,view,inspect,actions}=await fixture();ui._operit_ui_set_connection(1,1);
  const tap=id=>ui.ccall('operit_ui_debug_tap','number',['string'],[id]);
  const state={sessionId:'one',paired:true,chat:{connected:true,chatId:'a',conversationsLoading:true,conversations:[]}};
  view.update(state);tap('sidebar_toggle');tap('sidebar_chat');view.update(state);
  assert.equal(inspect().nodes.find(n=>n.id==='conversations_empty').text,'读取对话列表…');
  state.chat.conversationsLoading=false;state.chat.conversations=[{id:'a',title:'当前对话'},{id:'b',title:'其他对话'}];view.update(state);
  assert.equal(inspect().nodes.find(n=>n.id==='conversation_0').selected,true);
  tap('conversation_1');assert.equal(actions.at(-1),'edge_select:b');
  state.chat.conversationsError='对话已被删除';view.update(state);assert.equal(inspect().nodes.find(n=>n.id==='error').text,'对话已被删除');
  tap('dismiss_error');view.update(state);assert(!inspect().nodes.some(n=>n.id==='error'),'polling the same error must leave refresh reachable');
  tap('conversations_refresh');assert.equal(actions.at(-1),'edge_conversations_refresh');
  state.chat.conversationsError=null;state.chat.chatId='b';view.update(state);assert.equal(inspect().page,'Chat');
});


test('browser projects the device-space controller and clears stale candidates on stop', async () => {
  const {ui,view,inspect,actions}=await fixture();
  const tap=id=>ui.ccall('operit_ui_debug_tap','number',['string'],[id]);
  tap('sidebar_toggle');tap('sidebar_settings');tap('settings_space');tap('space_pair');
  view.update(simulatorViewState({ready:true,device:{deviceSpace:{phase:2,status:'选择 Core',candidates:['真实 Core']}}}));
  assert.equal(inspect().nodes.find(n=>n.id==='space_peer_0').text,'真实 Core');
  tap('space_peer_0');assert.equal(actions.at(-1),'edge_pair_candidate:0');
  view.update(simulatorViewState({ready:true,device:{deviceSpace:{phase:3,status:'输入六位码'}}}));
  assert.equal(inspect().page,'PairCodeEntry');assert.equal(tap('pair_digit_0'),1);
  view.update(simulatorViewState({ready:false}));
  assert(!JSON.stringify(inspect()).includes('真实 Core'));
  assert.equal(inspect().page,'SpaceManage');
});

test('complete speech input reaches its owner while C receives only a display preview',async()=>{
  const {ui}=await fixture();const received=[];
  const view=new DeviceView(ui,new ScenePreview(),()=>{},text=>received.push(text));
  const text='长语音识别结果🙂'.repeat(400)+'最终标记';
  view.update({sessionId:'long',running:true,paired:true,speech:{resultId:1,text}});
  ui._operit_ui_set_connection(1,1);ui.ccall('operit_ui_submit_chat',null,[],[]);
  const preview=ui.ccall('operit_ui_chat_draft','string',[],[]);
  assert.deepEqual(received,[text]);assert(Buffer.byteLength(preview)<=511);
  assert(text.startsWith(preview));assert(!preview.includes('�'));
});
