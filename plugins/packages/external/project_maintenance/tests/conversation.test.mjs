import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url); let state, calls=[];
globalThis.ToolPkg={ipc:{async call(_channel,request){calls.push(request);return structuredClone(state);}}};
const {maintenanceConversation}=require('../dist/conversation.js');
const event=(chatId,history=[{kind:'SYSTEM',content:'base'},{kind:'USER',content:'进度怎么样？'}])=>({eventName:'before_send_to_model',eventPayload:{chatId,preparedHistory:history}});
test('conversation receives only its structured status without modifying the original history',async()=>{
 state={bound:true,total:1,tickets:[{id:'ticket-1',stage:'review',updatedAt:10}]}; calls=[];
 const input=event('parent'); const result=await maintenanceConversation(input);
 assert.equal(calls[0].chatId,'parent'); assert.equal(input.eventPayload.preparedHistory[0].content,'base');
 assert.match(result.preparedHistory[0].content,/ticket-1/); assert.match(result.preparedHistory[0].content,/review/);
 assert.match(result.preparedHistory[0].content,/不要猜测/); assert.equal(result.preparedHistory[1].content,'进度怎么样？');
});
test('unrelated conversations do not receive maintenance context',async()=>{
 state={bound:false,total:0,tickets:[]}; assert.equal(await maintenanceConversation(event('unrelated')),null);
});
