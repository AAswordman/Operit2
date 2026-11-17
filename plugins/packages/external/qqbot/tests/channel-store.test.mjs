import test from 'node:test';
import assert from 'node:assert/strict';
import {objectStorage} from './storage.mjs';
import {load,plain} from './harness.mjs';
function fixture(){
 const storage=objectStorage();let at=1000;
 const {ChannelStore,conversationKey}=load('channel-store',{Tools:{Storage:{objects:storage}},ToolPkg:{getLocalDataDir:()=>'/test'},Date:{now:()=>at++}});
 const store=new ChannelStore();
 const msg=(id,author='alice',scene='group',account='bot')=>({key:JSON.stringify([account,scene,id]),channel:'qq',accountId:account,conversationId:'same-id',messageId:String(id),authorId:author,text:`message-${id}`,timestamp:'1',context:{scene,raw:{token:'must-not-appear-in-history'}}});
 return {store,storage,conversationKey,msg,restart:()=>new ChannelStore()};
}
test('independent durable consumers never steal messages and retained routes deduplicate beyond history',async()=>{
 const f=fixture(),key=f.conversationKey('qq','bot','group','same-id');
 await f.store.observe(f.msg('initial'),'group','Group');
 for(const consumerId of ['maintenance','other'])await f.store.subscribe(consumerId,[{conversationKey:key,since:0}],'qq','bot');
 for(let i=0;i<85;i++)await f.store.observe(f.msg(i),'group','Group');
 assert.equal((await f.store.history(key,50)).messages.length,50);
 const first=(await f.store.peek('maintenance',100))[0];
 await f.store.ack('maintenance',first.key);
 assert.equal((await f.store.peek('other',100)).length,85);
 const restart=f.restart();await restart.observe(f.msg(0),'group','Group');
 assert.equal((await restart.peek('maintenance',100)).length,84);
 await restart.subscribe('maintenance',[],'qq','bot');
 assert.equal((await restart.peek('maintenance',100)).length,0);
 assert.equal((await restart.peek('other',100)).length,85);
 assert.equal((await restart.replyRoute(first.key)).conversationId,'same-id');
});
test('history is bounded, readable, non-consuming, and separates account, scene and sender',async()=>{
 const f=fixture(),key=f.conversationKey('qq','bot','group','same-id');
 for(let i=0;i<90;i++)await f.store.observe({...f.msg(i,i%2?'bob':'alice'),text:'x'.repeat(8000)},'group','Group');
 await f.store.observe(f.msg(0,'alice','c2c'),'c2c','Private');
 await f.store.observe(f.msg(0,'alice','group','other'),'group','Other account');
 const history=await f.store.history(key,50,undefined,'alice');
 assert.ok(history.messages.every(m=>m.authorId==='alice'));
 assert.ok(history.messages.reduce((n,m)=>n+m.text.length,0)<=12000);
 assert.equal(history.truncated,true);assert.doesNotMatch(JSON.stringify(history),/must-not-appear|token/);
 assert.equal((await f.store.history(f.conversationKey('qq','bot','c2c','same-id'))).messages.length,1);
 assert.equal((await f.store.conversations('qq','bot')).length,2);
 assert.deepEqual(plain(await f.store.history(key,50)),plain(await f.store.history(key,50)));
 await assert.rejects(f.store.subscribe('consumer',[{conversationKey:key,since:0}],'qq','other'),/account/);
 await assert.rejects(f.store.history(key,51),/bounds/);
});
test('subscription boundaries remove stale deliveries and failed commits can be retried without losing messages',async()=>{
 const f=fixture(),key=f.conversationKey('qq','bot','group','same-id');
 await f.store.observe(f.msg('initial'),'group','Group');await f.store.subscribe('maintainer',[{conversationKey:key,since:0}],'qq','bot');
 f.storage.fail(true);await assert.rejects(f.store.observe(f.msg('retry'),'group','Group'),/disk/);
 f.storage.fail(false);await f.store.observe(f.msg('retry'),'group','Group');
 assert.equal((await f.store.peek('maintainer')).length,1);
 await f.store.subscribe('maintainer',[{conversationKey:key,since:99999}],'qq','bot');
 assert.equal((await f.store.peek('maintainer')).length,0);
});
