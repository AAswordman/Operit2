import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const manifest = JSON.parse(await readFile(new URL('../generated/manifest.json', import.meta.url)));
const options = {skip: manifest.renderer !== 'mini' ? 'Run npm run build first' : false};
async function fixture() {
  globalThis.window ??= {};
  const {default: create} = await import('../generated/ui.mjs');
  const ui = await create({wasmBinary: await readFile(new URL('../generated/ui.wasm', import.meta.url))});
  const actions = []; ui.onAction = value => actions.push(value);
  assert.equal(ui._simulator_init(),1);
  const call = (name,types=[],args=[]) => ui.ccall('operit_ui_'+name,null,types,args);
  const inspect = () => JSON.parse(ui.ccall('operit_ui_debug_snapshot','string',[],[]));
  const tap = id => ui.ccall('operit_ui_debug_tap','number',['string'],[id]);
  const pump = () => {for(let i=0;i<20;i++) ui._operit_ui_pump(20);};
  const frame = () => {pump(); const ptr=ui._simulator_frame(); return Buffer.from(ui.HEAPU8.slice(ptr,ptr+320*240*2));};
  pump();return {ui,actions,call,inspect,tap,pump,frame};
}

test('mini UI uses under 10 KiB static RAM, a 1280-byte strip, and no UI heap',options,async()=>{
  const {ui,inspect}=await fixture();const state=inspect();
  assert.equal(state.renderer,'mini');assert(state.staticBytes<10*1024);
  assert.equal(state.drawBytes,1280);assert.equal(state.heapBytes,0);
  for(const name of ['used','total','free','largest','peak'])assert.equal(ui['_simulator_heap_'+name](),0);
  assert.equal(ui._simulator_stack_size(),12*1024);
});
test('pairing digits are validated and take priority over approvals and old errors',options,async()=>{
  const {call,inspect}=await fixture();
  call('set_paired',['number'],[1]);call('action_error',['string'],['旧错误']);
  call('set_space_join_prompt',['string','number'],['申请加入空间\n测试空间',0]);
  call('set_pairing_code',['string'],['001234']);
  let state=inspect();assert.equal(state.page,'Pairing');assert.equal(state.pairingCode,'001234');
  assert(state.nodes.some(n=>n.id==='pairing_code'&&n.text==='001234'));
  assert(!state.nodes.some(n=>n.id==='edge_space_approve'||n.id==='error'));
  call('set_pairing_code',['string'],['board: 123456']);
  state=inspect();assert.equal(state.pairingCode,'');assert.equal(state.page,'Space');
});
test('approval buttons emit service actions, do not locally approve, and reject repeat taps while busy',options,async()=>{
  const {call,inspect,tap,actions}=await fixture();
  call('set_space_join_prompt',['string','number'],['申请加入空间\n胖鱼\n测试空间',0]);
  assert.equal(tap('edge_space_approve'),1);assert.deepEqual(actions,['edge_space_approve']);
  assert.equal(inspect().page,'Space');
  call('set_space_join_prompt',['string','number'],['',1]);
  assert.equal(inspect().page,'Space');assert.equal(tap('edge_space_approve'),0);assert.equal(tap('edge_space_reject'),0);
  call('set_space_join_prompt',['string','number'],['申请加入空间',0]);
  assert.equal(tap('edge_space_reject'),1);assert.deepEqual(actions,['edge_space_approve','edge_space_reject']);
  call('set_space_join_prompt',['string','number'],['',0]);assert.equal(inspect().page,'Pairing');
});
test('touch requires press/release on the same target and cancels across state changes',options,async()=>{
  const {ui,call,inspect,actions}=await fixture();
  const n=inspect().nodes.find(n=>n.id==='pair_listen'),x=n.rect.x+10,y=n.rect.y+10;
  ui._simulator_touch(x,y,1);assert.deepEqual(actions,[]);
  ui._simulator_touch(x+180,y,0);assert.deepEqual(actions,[]);
  ui._simulator_touch(x,y,1);call('set_pairing_code',['string'],['654321']);
  ui._simulator_touch(x,y,0);assert.deepEqual(actions,[]);
  ui._simulator_touch(x,y,1);ui._simulator_touch(x,y,0);assert.deepEqual(actions,['edge_pair']);
});
test('Chinese and six-digit codes are painted as distinct real glyphs',options,async()=>{
  const {call,frame}=await fixture();
  call('set_paired',['number'],[1]);
  const chars=['中','文','申','请','加','入','空','间','批','准','拒','绝'];const images=[];
  for(const c of chars){call('set_chat_screen',['string'],[c]);images.push(frame().toString('hex'));}
  assert.equal(new Set(images).size,chars.length);
  call('set_pairing_code',['string'],['000000']);const first=frame();
  call('set_pairing_code',['string'],['123456']);assert.notDeepEqual(frame(),first);
});
test('text is bounded on whole UTF-8 characters and diagnostic JSON escapes control characters',options,async()=>{
  const {call,inspect,frame}=await fixture();call('set_paired',['number'],[1]);
  call('set_chat_screen',['string'],['中'.repeat(1000)+'"\\\n']);
  assert(!inspect().nodes.find(n=>n.id==='chat_text').text.includes('\ufffd'));assert.equal(frame().length,320*240*2);
  call('set_space_join_prompt',['string','number'],['"\\\n\t'+String.fromCharCode(1)+'文',0]);
  assert.equal(inspect().nodes.find(n=>n.id==='space_join_body').text,'"\\\n\t'+String.fromCharCode(1)+'文');
});
test('unchanged state does not continuously redraw or allocate',options,async()=>{
  const {ui,call,pump}=await fixture();call('set_pairing_code',['string'],['123456']);pump();
  const generation=ui._simulator_generation();
  for(let i=0;i<2000;i++){
    call('set_pairing_code',['string'],['123456']);
    call('set_space_join_prompt',['string','number'],['',0]);
    ui._operit_ui_pump(20);
  }
  assert.equal(ui._simulator_generation(),generation);assert.equal(ui._simulator_heap_used(),0);
});
test('unpair requires a confirmation before forwarding the existing service action',options,async()=>{
  const {call,tap,inspect,actions}=await fixture();call('set_paired',['number'],[1]);
  assert.equal(tap('open_status'),1);assert.equal(tap('edge_unpair'),1);
  assert.equal(inspect().page,'Unpair');assert.deepEqual(actions,[]);
  assert.equal(tap('unpair_cancel'),1);assert.deepEqual(actions,[]);
  tap('edge_unpair');tap('unpair_confirm');assert.deepEqual(actions,['edge_unpair']);
});
test('host keyboard submission retains draft on failure, prevents double send, clears on success',options,async()=>{
  const {call,ui,actions,inspect}=await fixture();call('set_paired',['number'],[1]);
  call('set_chat_draft',['string'],['你好']);call('submit_chat');assert.deepEqual(actions,[]);
  call('set_connection',['number','number'],[1,1]);call('submit_chat');call('submit_chat');
  assert.deepEqual(actions,['edge_send']);
  assert.equal(ui.ccall('operit_ui_chat_draft','string',[],[]),'你好');
  call('chat_send_result',['number','string'],[0,'失败']);
  assert.equal(ui.ccall('operit_ui_chat_draft','string',[],[]),'你好');
  call('action_error',['string'],['']);call('submit_chat');assert.equal(actions.length,2);
  call('chat_send_result',['number','string'],[1,'']);
  assert.equal(ui.ccall('operit_ui_chat_draft','string',[],[]),'');
  assert.equal(inspect().nodes.find(n=>n.id==='edge_send').enabled,false);
});
test('repeated pairing and approval redraws stay within the checked C stack and fixed state',options,async()=>{
  const {ui,call,tap,pump,inspect}=await fixture();const before=inspect().staticBytes;
  for(let i=0;i<300;i++){
    call('set_pairing_code',['string'],[String(i).padStart(6,'0')]);pump();
    call('set_pairing_code',['string'],['']);
    call('set_space_join_prompt',['string','number'],['申请加入空间\n测试设备',0]);tap('edge_space_approve');pump();
    call('set_space_join_prompt',['string','number'],['',0]);pump();
  }
  assert.equal(inspect().staticBytes,before);assert.equal(ui._simulator_heap_used(),0);
  assert(ui._simulator_stack_free()>0);
});

test('frequent changing chat updates cannot starve lower screen strips',options,async()=>{
  const {ui,call,pump}=await fixture();call('set_paired',['number'],[1]);pump();
  call('set_chat_screen',['string'],['开始']);
  const before=ui._simulator_generation();
  for(let i=0;i<40;i++) {call('set_chat_screen',['string'],['更新 '+i]);ui._operit_ui_pump(20);}
  assert(ui._simulator_generation()-before>=240,'must flush at least two complete frames');
});

test('mini native UI source cannot reference malloc, LVGL, or an entire-frame buffer',options,async()=>{
  const source=await readFile(new URL('../../../apps/esp32/ui_port/operit_mini_ui.c',import.meta.url),'utf8');
  assert(!/\b(?:malloc|calloc|realloc|lv_init|lv_obj_create)\s*\(/.test(source));
  assert(!source.includes('#include "lvgl.h"'));
  assert(source.includes('_Static_assert'));
});

test('failed space approval remains reviewable and displays the backend error',options,async()=>{
  const {call,inspect,tap,actions}=await fixture();
  call('set_space_join_prompt',['string','number'],['申请加入空间',0]);
  call('action_error',['string'],['审批失败，请重试']);
  assert.equal(inspect().page,'Space');
  assert.equal(inspect().nodes.find(n=>n.id==='space_join_error').text,'审批失败，请重试');
  assert.equal(tap('edge_space_approve'),1);assert.deepEqual(actions,['edge_space_approve']);
});

test('real chat row reaches the painted and inspected mini buffer without a transcript allocation',options,async()=>{
  const {call,inspect,frame}=await fixture();call('set_paired',['number'],[1]);
  call('set_message',['number','number','string'],[0,1,'ESP32-R2']);
  call('set_message',['number','number','string'],[1,0,'ESP32-R2-OK']);
  call('finish_messages',['number'],[2]);call('set_chat_screen',['string'],['']);
  assert.equal(inspect().nodes.find(n=>n.id==='chat_text').text,'You: ESP32-R2\nAI: ESP32-R2-OK');
  const replyFrame=frame();
  call('set_chat_screen',['string'],['401 Unauthorized']);
  assert.equal(inspect().nodes.find(n=>n.id==='chat_text').text,'401 Unauthorized');
  assert.notDeepEqual(frame(),replyFrame);
  call('finish_messages',['number'],[0]);call('set_chat_screen',['string'],['输入消息开始对话']);
  assert.equal(inspect().nodes.find(n=>n.id==='chat_text').text,'输入消息开始对话');
  assert(inspect().staticBytes<10*1024);assert.equal(inspect().heapBytes,0);
});

test('station Wi-Fi status is independent of an authenticated serial Space connection',options,async()=>{
  const {call,inspect,tap}=await fixture();
  call('set_paired',['number'],[1]);call('set_connection',['number','number'],[0,1]);
  assert.equal(tap('open_status'),1);
  assert.equal(inspect().nodes.find(n=>n.id==='wifi').text,'Wi-Fi 未连接');
  assert.equal(inspect().nodes.find(n=>n.id==='link_status').text,'已连接');
  call('set_connection',['number','number'],[1,1]);
  assert.equal(inspect().nodes.find(n=>n.id==='wifi').text,'Wi-Fi 已连接');
  call('set_connection',['number','number'],[0,1]);
  assert.equal(inspect().nodes.find(n=>n.id==='wifi').text,'Wi-Fi 未连接');
});


test('leaving a Space requires local confirmation and does not revoke pairing',options,async()=>{
  const {call,tap,inspect,actions}=await fixture();call('set_paired',['number'],[1]);
  tap('open_status');assert.equal(tap('space_leave'),1);
  assert.equal(inspect().page,'LeaveSpace');assert.deepEqual(actions,[]);
  assert.equal(tap('space_leave_cancel'),1);assert.equal(inspect().page,'Status');
  assert.deepEqual(actions,[]);
  tap('space_leave');assert.equal(tap('space_leave_confirm'),1);
  assert.deepEqual(actions,['edge_space_leave']);assert.equal(inspect().page,'Status');
  assert.equal(inspect().nodes.find(n=>n.id==='link_status').text,'已配对 / 离线');
  assert(inspect().staticBytes<10*1024);assert.equal(inspect().heapBytes,0);
});


test('bounded chat window scrolls locally then lazily requests another page',options,async()=>{
  const {ui,call,inspect,frame,actions}=await fixture();call('set_paired',['number'],[1]);
  call('set_chat_history',['number','number'],[1,0]);
  for(let i=0;i<12;i++)call('set_message',['number','number','string'],[i,i%2,'消息 '+i]);
  call('finish_messages',['number'],[12]);
  const tail=inspect();assert(tail.chatCachedLines>=12);assert(tail.chatScrollLine>0);
  assert(tail.staticBytes<10*1024);assert.equal(tail.heapBytes,0);assert(tail.chatCachedBytes<2048);
  const before=frame();assert.equal(call('debug_swipe',['string'],['down']),1);
  assert(inspect().chatScrollLine<tail.chatScrollLine);assert.notDeepEqual(frame(),before);assert.deepEqual(actions,[]);
  for(let i=0;i<4;i++)call('debug_swipe',['string'],['down']);
  assert(actions.includes('edge_history_older'));
  call('navigate_apps',[],[]);assert.equal(call('debug_swipe',['string'],['down']),0);
});


test('scrolling past newest history does not jump back to the top or issue a no-op load',options,async()=>{
  const {call,inspect,actions}=await fixture();call('set_paired',['number'],[1]);
  call('set_chat_history',['number','number'],[0,0]);
  call('set_message',['number','number','string'],[0,0,Array.from({length:15},(_,n)=>'P'+n).join('\n')]);
  call('finish_messages',['number'],[1]);
  const tail=inspect().chatScrollLine;assert(tail>0);
  for(let n=0;n<3;n++)call('debug_swipe',['string'],['up']);
  assert.equal(inspect().chatScrollLine,tail);assert.deepEqual(actions,[]);
  for(let n=0;n<5;n++)call('debug_swipe',['string'],['down']);
  assert.equal(inspect().chatScrollLine,0);assert.deepEqual(actions,[]);
});
