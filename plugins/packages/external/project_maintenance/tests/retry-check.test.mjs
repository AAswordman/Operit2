import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {prepareRetry}=createRequire(import.meta.url)('../dist/retry-check.js');
function fixture(){
 const calls=[]; let parentStatus='completed',children=[],host={chatId:'a',isIdle:true,isProcessing:false};
 return {calls,setParent:s=>parentStatus=s,setChildren:c=>children=c,setHost:h=>host=h,ports:{
  async list(parent){calls.push(['list',parent]);return {schemaVersion:1,agents:parent==='p'?[{id:'a',parentChatId:'p',status:parentStatus}]:children};},
  async close(parent,id){calls.push(['close',parent,id]);parentStatus='closed';return {id,status:'closed'};},
  async status(id){calls.push(['status',id]);return host;},
 }};
}
test('retry fences a completed idle agent before allowing a reservation transfer',async()=>{
 const f=fixture(); assert.equal(await prepareRetry('p','a',f.ports),true);
 assert.deepEqual(f.calls.map(x=>x[0]),['list','list','close','status']);
 assert.equal(await prepareRetry('p','a',f.ports),true); assert.equal(f.calls.filter(x=>x[0]==='close').length,2);
});
test('running or uncertain agents and unclosed descendants block retry',async()=>{
 const f=fixture(); f.setParent('interrupted'); assert.equal(await prepareRetry('p','a',f.ports),false);
 f.setParent('completed'); f.setChildren([{id:'child',parentChatId:'a',status:'running'}]); assert.equal(await prepareRetry('p','a',f.ports),false);
 assert.equal(f.calls.some(x=>x[0]==='close'),false);
 f.setChildren([]); f.setHost({chatId:'a',isIdle:false,isProcessing:true}); assert.equal(await prepareRetry('p','a',f.ports),false);
});
test('dependency errors, foreign responses and unconfirmed closure block retry',async()=>{
 const f=fixture(); f.ports.close=async()=>({id:'foreign',status:'closed'}); assert.equal(await prepareRetry('p','a',f.ports),false);
 f.ports.close=async()=>{throw Error('offline')}; assert.equal(await prepareRetry('p','a',f.ports),false);
});
