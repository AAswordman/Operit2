import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {inspect} = createRequire(import.meta.url)('../dist/inspection.js');
const ticket = {id:'t',parentChatId:'parent',projectPath:'/game',agentId:'agent',stage:'interrupted'};
const ports = {now:()=>100,claim:async()=>({ticketId:'t'}),list:async()=>({schemaVersion:1,agents:[{id:'agent',parentChatId:'parent',status:'interrupted'}]}),status:async()=>({chatId:'agent',isIdle:true,isProcessing:false})};
test('inspection reads only the linked agent and reports its host state and reservation',async()=>{
 const result=await inspect(ticket,ports);
 assert.deepEqual(result,{ticketId:'t',observedAt:100,claim:'this_ticket',agentId:'agent',managed:'interrupted',native:'idle'});
});
test('unknown spawn and dependency errors never imply absence or completion',async()=>{
 let statusCalls=0; const broken={...ports,claim:async()=>{throw Error('x')},list:async()=>{throw Error('x')},status:async()=>{statusCalls++;throw Error('x')}};
 const missing=await inspect({...ticket,agentId:null},broken);
 assert.equal(missing.claim,'unknown'); assert.equal(missing.managed,'unknown_spawn'); assert.equal(missing.native,'unknown'); assert.equal(statusCalls,0);
 const linked=await inspect(ticket,broken); assert.equal(linked.managed,'unavailable'); assert.equal(linked.native,'unknown');
});
test('foreign, malformed, and inconsistent states are not accepted as evidence',async()=>{
 const other=await inspect(ticket,{...ports,claim:async()=>({ticketId:'other'}),list:async()=>({schemaVersion:1,agents:[{id:'agent',parentChatId:'another',status:'completed'}]}),status:async()=>({chatId:'agent',isIdle:true,isProcessing:true})});
 assert.equal(other.claim,'other_ticket'); assert.equal(other.managed,'not_found'); assert.equal(other.native,'unknown');
 const invalid=await inspect(ticket,{...ports,list:async()=>({schemaVersion:1,agents:[{id:'agent',parentChatId:'parent',status:'forged'}]}),status:async()=>({chatId:'other',isIdle:true,isProcessing:false})});
 assert.equal(invalid.managed,'unavailable'); assert.equal(invalid.native,'unknown');
});
