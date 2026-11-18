import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {objectStorage} from '../../qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url), {dispatch,consoleRequest}=require('../dist/service.js');
const maintenance='com.operit.project_maintenance';
const row={key:'wx-conversation',channel:'wechat',accountId:'wx-account',conversationId:'peer',scene:'c2c',title:'Test peer',chatId:null,lastMessageAt:8};
const incoming={key:'wechat:wx-account:peer:m',channel:'wechat',accountId:'wx-account',conversationId:'peer',authorId:'peer',messageId:'m',timestamp:'8',text:'hi',context:{contextToken:'private-token',items:[{type:2,image_item:{media:{aes_key:'private-key'}}}]}};
const source={conversation:row,messages:[{messageId:'m',authorName:'',text:'hi',receivedAt:8,timestamp:'8'}],truncated:false,latestInboundId:'m'};
const storage=objectStorage();
function fixture(){
  const calls=[], messages=[...source.messages];
  globalThis.Tools={Storage:{objects:storage}};
  globalThis.CryptoJS={binary:{randomHex:()=> '1234567'}};
  globalThis.ToolPkg={getLocalDataDir:()=>'/device',async callDependency(provider,method,payload){
    calls.push({provider,method,payload});
    if(!provider.includes('wechat'))throw Error(`ToolPkg container not found: ${provider}`);
    switch(payload.action){
      case 'status':return {channel:'wechat',service:'ready'};
      case 'conversations':return {conversations:[row]};
      case 'history':return {...source,messages:messages.filter(message=>payload.before===undefined||message.receivedAt<payload.before).slice(-(payload.limit??20))};
      case 'peek':return {messages:[incoming]};
      case 'subscribe':return {subscribed:1};
      case 'reply':messages.push({messageId:'out',authorName:'机器人',text:payload.text,receivedAt:10,timestamp:'10',direction:'outgoing',delivery:'accepted'});return {response:{token:'private-opaque'}};
      case 'ack':return {acknowledged:true};
      default:throw Error('Unexpected action');
    }
  }};
  return calls;
}
test('optional providers fail independently without hiding the working connector', async()=>{
  fixture();
  const data=await dispatch({action:'dashboard'},maintenance);
  assert.deepEqual(data.connectors.map(c=>[c.channel,c.available]),[['qq',false],['wechat',true],['feishu',false]]);
  assert.deepEqual(data.conversations,[row]);
});
test('authenticates consumer from the host and strips raw message secrets',async()=>{
  const calls=fixture();
  await dispatch({action:'subscribe',channel:'wechat',consumerId:'spoof',routes:[{conversationKey:row.key,since:0}]},maintenance);
  const data=await dispatch({action:'peek',channel:'wechat',conversationKey:row.key,consumerId:maintenance},'other.plugin');
  await dispatch({action:'ack',channel:'wechat',key:incoming.key,consumerId:'spoof'},maintenance);
  assert.deepEqual(calls.map(call=>call.payload.consumerId),[maintenance,'other.plugin',maintenance]);
  assert.deepEqual(data.messages[0].context,{scene:'c2c',media:[{kind:'image'}],plainText:false});
  assert.doesNotMatch(JSON.stringify(data),/private-token|private-key/);
  await assert.rejects(dispatch({action:'manage',channel:'wechat',command:'start'},'other.plugin'),/未知/);
  await assert.rejects(dispatch({action:'configure',channel:'wechat'},'other.plugin'),/权限/);
});
test('manual reply verifies the selected message and merges outgoing records',async()=>{
  const calls=fixture();
  await storage.db.commit([{op:'put',collection:'history',key:row.key,value:[{direction:'outgoing',authorName:'控制台',messageId:'legacy',receivedAt:9,timestamp:'9',text:'old reply'}]}]);
  const value=await dispatch({channel:'wechat',action:'history',conversationKey:row.key},maintenance);
  assert.equal(value.messages[0].replyKey,incoming.key);
  await assert.rejects(consoleRequest({action:'console_reply',channel:'wechat',conversationKey:row.key,key:'wrong',text:'reply'}),/不在近期/);
  const result=await consoleRequest({action:'console_reply',channel:'wechat',conversationKey:row.key,key:incoming.key,text:'reply'});
  assert.deepEqual(result,{response:null});
  const history=await dispatch({channel:'wechat',action:'history',conversationKey:row.key},maintenance);
  assert.equal(history.messages.at(-1).direction,'outgoing');
  assert.equal(history.messages.at(-1).text,'reply');
  assert.equal(history.messages.at(-1).delivery,'accepted');
  assert.equal(history.messages.at(-1).replyKey,undefined);
  assert.deepEqual(history.messages.map(item=>item.messageId),['m','legacy','out']);
  const page=await dispatch({channel:'wechat',action:'history',conversationKey:row.key,limit:1},maintenance);
  assert.equal(page.latestReplyKey,incoming.key);
  assert.equal(page.messages[0].messageId,'out');
  assert.equal((await dispatch({channel:'wechat',action:'history',conversationKey:row.key,before:10,limit:1},maintenance)).messages[0].messageId,'legacy');
  assert.equal(calls.filter(item=>item.payload.action==='reply').length,1);
});
test('maintenance vendors the exact dependency client',()=>{
  assert.equal(readFileSync(new URL('../src/api.ts',import.meta.url),'utf8'),readFileSync(new URL('../../project_maintenance/src/dependencies/connector_hub/api.ts',import.meta.url),'utf8'));
});
