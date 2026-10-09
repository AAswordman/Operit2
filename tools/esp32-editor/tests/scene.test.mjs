import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {simulatorViewState} from '../web/device-state.ts';
import {decodePacks, paintScene} from '../web/scene-renderer.ts';

function pack() {
  const asset = Buffer.from([69,83,73,49,2,1,2,4,50,0,0,0,0,0,248,255,224,7,255,31,0,128,1,0,2,3]);
  return Buffer.concat([Buffer.from('ESP1\x01\x03pet'),Buffer.from([asset.length,0]),asset]);
}
test('preview and native RGB565 golden agree: animation, alpha, anchor, clip', () => {
  const view = {active:true, revision:1,rect:{x:0,y:24,w:320,h:216},background:0,elapsed:0,
    layers:[{asset:'demo:pet',x:2,y:2,scale:2,anchor:'bc',frame:0,play:true,loop:true}],packs:[{id:'demo',data:pack().toString('base64')}]};
  const assets = decodePacks(view.packs); const strip = new Uint8Array(1280).fill(0x55);
  paintScene(view,assets,strip,24,2,0);assert.deepEqual([...strip.slice(0,8)],[0,248,0,248,0,0,0,0]);
  paintScene(view,assets,strip,24,2,50);assert.deepEqual([...strip.slice(0,8)],[224,7,224,7,16,0,16,0]);
  paintScene(view,assets,strip,24,2,100);assert.deepEqual([...strip.slice(0,4)],[0,248,0,248]);
  strip.fill(0x55);paintScene(view,assets,strip,0,2,50);assert(strip.every(b=>b===0x55));
});
test('real C strip painter preserves exit, critical system UI, input and RAM budgets', async () => {
  globalThis.window ??= {};
  const {default:create} = await import('../generated/ui.mjs');
  const ui = await create({wasmBinary:await readFile(new URL('../generated/ui.wasm',import.meta.url))});
  assert.equal(ui._simulator_init(),1);
  const actions=[]; ui.onAction = action => actions.push(action);
  const call=(name,types=[],args=[],values=[])=>typeof types==='string' ? ui.ccall('operit_ui_'+name,types,args,values) : ui.ccall('operit_ui_'+name,null,types,args);
  const screen=()=>JSON.parse(call('debug_snapshot','string'));
  let calls=0; const ticks=[];
  const painter=ui.addFunction((pointer,len,y,rows,tick)=>{
    assert.equal(len,1280);assert.equal(rows,2);calls++;ticks.push(tick);
    if(y>=24) ui.HEAPU8.fill(0x80,pointer,pointer+len);
  },'viiiiii');
  call('scene_painter',['number'],[painter]);call('set_scene',['number','number','number'],[1,1,100]);
  assert.equal(screen().page,'edge_scene');
  assert(screen().nodes.find(n=>n.id==='scene_exit'));
  for(let i=0;i<15;i++){call('set_scene',['number','number','number'],[1,1,100+i]);ui._operit_ui_pump(0);}
  assert.equal(calls,120);assert(ticks.every(t=>t===100),'timestamp freezes through a whole strip frame');
  assert(screen().staticBytes<=10240);assert.equal(screen().drawBytes,1280);
  ui._simulator_touch(100,50,1);ui._simulator_touch(100,50,0);assert(actions.includes('edge_scene_touch:100:50'));
  const count=actions.length;ui._simulator_touch(100,50,1);call('set_scene',['number','number','number'],[1,2,150]);ui._simulator_touch(100,50,0);assert.equal(actions.length,count,'scene change cancels the original gesture');
  ui._simulator_touch(298,12,1);for(let i=0;i<30;i++)ui._operit_ui_pump(0);ui._simulator_touch(298,12,0);assert(actions.includes('edge_scene_exit'),'local animation must not cancel the protected exit tap');
  assert.equal(call('debug_tap','number',['string'],['scene_exit']),1);assert(actions.includes('edge_scene_exit'));
  call('set_pairing_code',['string'],['123456']);assert.equal(screen().page,'Pairing');
  const before=calls;for(let i=0;i<20;i++)ui._operit_ui_pump(0);assert.equal(calls,before,'pairing owns the screen');
  call('set_pairing_code',['string'],['']);call('set_space_join_prompt',['string','number'],['批准空间申请',0]);assert.equal(screen().page,'Space');
  call('set_space_join_prompt',['string','number'],['',0]);call('set_scene',['number','number','number'],[0,2,0]);assert.equal(screen().page,'Chat');
  call('scene_painter',['number'],[0]);ui.removeFunction(painter);
});
test('Core demo uploads bounded chunks and retains original package hash', async () => {
  const {createHash}=await import('node:crypto');
  const original=await readFile(new URL('../../../plugins/packages/external/edge_pixel_pet/resources/demo.esp',import.meta.url));
  const {createRequire}=await import('node:module');const require=createRequire(import.meta.url);
  const {PACK_BYTES,PACK_SHA256,PACK_CHUNKS}=require('../../../plugins/packages/external/edge_pixel_pet/dist/assets.js');
  assert.equal(original.length,PACK_BYTES);assert.equal(createHash('sha256').update(original).digest('hex'),PACK_SHA256);
  assert.deepEqual(Buffer.concat(PACK_CHUNKS.map(c=>Buffer.from(c.data,'base64'))),original);
  for(const c of PACK_CHUNKS){assert(Buffer.byteLength(JSON.stringify({v:1,lease:'x'.repeat(36),upload:'x'.repeat(36),...c}))<=1024);assert(Buffer.from(c.data,'base64').length<=256);}
});

test('real browser state projection forwards native scene activation and stop', () => {
  const device={scene:{active:true,revision:42},paired:true,chat:{connected:true,chatId:'remote-chat'}};
  const view=simulatorViewState({ready:true,device});
  assert.deepEqual(view.scene,device.scene);assert.equal(view.connected,true);assert.equal(view.chat.chatId,'remote-chat');
  const stopped=simulatorViewState({ready:false,device:null});assert.equal(stopped.running,false);assert.equal(stopped.scene,undefined);
});
