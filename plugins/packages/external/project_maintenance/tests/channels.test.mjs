import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {objectStorage} from '../../qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url),{ChannelCoordinator}=require('../dist/channels.js');
const message={key:'source',channel:'qq',accountId:'bot',messageId:'m',conversationId:'group',authorId:'user',text:'restart does not reset score',timestamp:'1',context:{scene:'group'}};
const session={key:'canonical-key',channel:'qq',accountId:'bot',scene:'group',conversationId:'group',title:'QQ group',chatId:'parent',contextChatIds:['member-chat'],lastMessageAt:100};
function fixture(){
 const storage=objectStorage(),calls=[],events=[],bindings=new Map([['parent',{projectPath:'/game',since:1}]]);
 let workspace='/game',failStatus=false,failCapture=false,peekGate,noticesGate,pending=true,source=message;
 globalThis.Tools={Storage:{objects:storage}};
 globalThis.ToolPkg={getLocalDataDir:()=>'/test-local',async callDependency(provider,method,payload){
  calls.push({provider,method,payload});if(provider.includes('wechat'))throw Error('WeChat not installed');
  switch(payload.action){
   case 'status':if(failStatus)throw Error('provider disabled');return {channel:'qq',service:'ready',capture:false,conversationIds:[],channelOnlyWhenNoAuto:false};
   case 'conversations':return {conversations:[session]};
   case 'open_chat':assert.equal(payload.conversationKey,session.key);return session;
   case 'subscribe':return {subscribed:payload.routes.length};
   case 'peek':if(peekGate)await peekGate;return {messages:pending?[source]:[]};
   case 'ack':events.push('ack');pending=false;return {acknowledged:true};
   case 'reply':events.push('reply');return {response:{}};
   case 'active_context':return payload.chatId==='member-chat'?{channel:'qq',scene:'group',chatId:payload.chatId,accountId:'bot',conversationId:'group',authorId:'user'}:null;
   default:throw Error('unexpected action '+payload.action);
  }
 }};
 const coordinator=new ChannelCoordinator({
  async chats(){return [{id:'parent',title:'真实 Bot 聊天',workspaceId:workspace?'w':null,workspacePath:workspace}];},
  async binding(id){return bindings.get(id)??null;},
  async workspace(){if(!workspace)throw Error('聊天尚未绑定工作区');return workspace;},
  async resetBinding(id,path){bindings.set(id,{projectPath:path,since:2});events.push('reset');},
  async capture(parent,path,value){events.push('capture');assert.equal(parent,'parent');assert.equal(path,workspace);assert.deepEqual(value,source);if(failCapture)throw Error('disk failed');},
  async notices(){if(noticesGate)await noticesGate;return [{eventId:'n',messageKey:'source',text:'done'}];},
  async status(scope){return {scope};},
 });
 return {coordinator,calls,events,bindings,workspace(value){workspace=value;},failStatus(value){failStatus=value;},failCapture(value){failCapture=value;},gate(value){peekGate=value;},noticeGate(value){noticesGate=value;},message(value){source=value;}};
}
test('maintenance derives routing from Bot and subscribes without rewriting global capture',async()=>{
 const f=fixture();await f.coordinator.tick('qq');assert.deepEqual(f.events,['capture','ack','reply']);
 assert.ok(f.calls.every(c=>c.provider==='com.operit.qqbot_bundle'&&c.method==='channel.v1'));
 assert.equal(f.calls.some(c=>c.payload.action==='configure'),false);
 assert.deepEqual(f.calls.find(c=>c.payload.action==='subscribe').payload,{action:'subscribe',consumerId:'com.operit.project_maintenance',routes:[{conversationKey:session.key,since:1}]});
 assert.equal(f.calls.find(c=>c.payload.action==='ack').payload.consumerId,'com.operit.project_maintenance');
 const [report]=await f.coordinator.status();assert.equal(report.enabled,true);assert.equal(report.sessions[0].title,'真实 Bot 聊天');
 await f.coordinator.tick('qq');assert.equal(f.events.filter(e=>e==='reply').length,1);
});
test('capture failure keeps the message pending and reports the error until recovery',async()=>{
 const f=fixture();f.failCapture(true);await f.coordinator.tick('qq');assert.deepEqual(f.events,['capture']);
 assert.match((await f.coordinator.status())[0].lastError,/disk failed/);f.failCapture(false);await f.coordinator.tick('qq');assert.deepEqual(f.events,['capture','capture','ack','reply']);
 assert.equal((await f.coordinator.status())[0].lastError,'');
});
test('missing providers clear stale connection evidence',async()=>{
 const f=fixture();await f.coordinator.tick('qq');f.failStatus(true);const [report]=await f.coordinator.status();assert.equal(report.service,'unknown');assert.equal(report.availability,'unavailable');
 assert.match(report.lastError,/disabled/);
});
test('disabling a software binding fences an in-flight peek before capture or acknowledgement',async()=>{
 const f=fixture();let resume;f.gate(new Promise(r=>resume=r));const work=f.coordinator.tick('qq');await new Promise(setImmediate);
 f.bindings.delete('parent');resume();await work;assert.deepEqual(f.events,[]);
 await f.coordinator.tick('qq');assert.deepEqual(f.calls.filter(c=>c.payload.action==='subscribe').at(-1).payload.routes,[]);
});
test('stopping fences a pending notification after asynchronous notice lookup',async()=>{
 const f=fixture();let resume;f.noticeGate(new Promise(r=>resume=r));const work=f.coordinator.tick('qq');await new Promise(setImmediate);
 f.coordinator.stop();resume();await work;assert.deepEqual(f.events,['capture','ack']);
});
test('unbinding while notifications load never sends back into the old route',async()=>{
 const f=fixture();let resume;f.noticeGate(new Promise(r=>resume=r));const work=f.coordinator.tick('qq');await new Promise(setImmediate);
 f.bindings.delete('parent');resume();await work;assert.deepEqual(f.events,['capture','ack']);
});
test('progress resolves the canonical Bot chat even from an isolated member chat',async()=>{
 const f=fixture();assert.equal(await f.coordinator.conversationStatus('unrelated'),null);
 const status=await f.coordinator.conversationStatus('member-chat');assert.equal(status.scope.parentChatId,'parent');assert.equal(status.scope.authorId,'user');
 assert.equal(await f.coordinator.isBotChat('member-chat'),true);assert.equal(await f.coordinator.isBotChat('parent'),true);
});
test('workspace changes reset the capture boundary and invalid sources never get acknowledged',async()=>{
 const f=fixture();f.workspace('/other-game');f.message({...message,accountId:'other'});await f.coordinator.tick('qq');
 assert.deepEqual(f.events,['reset']);assert.match((await f.coordinator.status())[0].lastError,/来源不一致/);
 f.workspace(null);await f.coordinator.tick('qq');assert.equal(f.calls.filter(c=>c.payload.action==='ack').length,0);
});
test('preparing a conversation reuses the Bot mapping and rejects invalid channels',async()=>{
 const f=fixture();assert.deepEqual(await f.coordinator.prepare('qq',session.key),{chatId:'parent'});
 await assert.rejects(f.coordinator.prepare('__proto__','key'),/Invalid channel/);
});
