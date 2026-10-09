import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url);
const base=new URL('../../../plugins/packages/external/edge_pixel_pet/dist/',import.meta.url);
async function actor() {
  let writes=0, fail=false;
  const config={version:1,state:{taps:0,sessions:{}}}, calls=[], handlers=new Map();
  const sceneCall=async(node,action,args={})=> {
    calls.push({node,action,args});
    if(action==='capabilities') return {protocol:1,events:{push:true},cache:{packs:[]}};
    if(action==='lease.open') return {lease:'12345678-1234-1234-1234-123456789abc',cursor:0,ttlMs:60000};
    if(action==='pack.begin') return {upload:'upload'};
    if(action==='lease.renew') return {ttlMs:60000};
    return {};
  };
  const context=vm.createContext({exports:{},getEnv:()=> 'edge',
    PluginConfig:{use:async()=>config,flush:async()=>{if(fail){fail=false;throw new Error('disk unavailable');}writes++;}},
    ToolPkg:{ipc:{on:(channel,handler)=>handlers.set(channel,handler),call:(channel,payload)=>handlers.get(channel)(payload)}},
  });
  context.require=name=>name==='./assets'?require(fileURLToPath(new URL('assets.js',base))):
    name==='./protocol'?{sceneCall}:name==='./pet'?pet:null;
  const load=async file=> {const exports={}; const source=await readFile(new URL(file,base),'utf8');
    new vm.Script('(function(require,exports){'+source+'\n})').runInContext(context)(context.require,exports);return exports;};
  const pet=await load('pet.js');const main=await load('main.js');const tools=await load('packages/edge_pixel_pet.js');
  return {tools,main,config,calls,get writes(){return writes;},failNext(){fail=true;}};
}
function payload(stream,seq,action='target.tap',lostBefore=0) {
  return {eventPayload:{chatId:'chat',nodeId:'edge',batch:{v:1,source:'display.scene',stream,next:seq,lostBefore,
    events:[{seq,action,data:action==='target.tap'?{x:144,y:122,target:'pet'}:{}}]}}};
}
test('Core actor: idle/status/renew never poll input or write config; replay/exit ACK deduplicates',async()=> {
  const a=await actor(), started=await a.tools.start_pet({node_id:'edge'});
  const lease=a.config.state.sessions.edge.lease;
  assert.equal(started.taps,0);assert.equal(a.calls.find(c=>c.action==='lease.open').args.notify.packageName,'com.operit.edge_pixel_pet');
  const writes=a.writes;
  assert.equal((await a.main.test_connection()).passed,true);
  assert.equal((await a.main.test_tool_call()).passed,true);
  assert(a.calls.slice(-2).every(call=>call.action==='capabilities'));
  assert.equal(a.writes,writes);
  const calls=a.calls.length;
  await a.tools.pet_status({node_id:'edge'});assert.equal(a.writes,writes);assert.equal(a.calls.length,calls);
  await a.tools.renew_pet({node_id:'edge'});assert.equal(a.writes,writes);assert.equal(a.calls.at(-1).action,'lease.renew');
  const ack=await a.main.on_edge_event(payload(lease,1));assert.equal(ack.accepted,true);assert.equal(ack.next,1);
  assert.equal(a.config.state.taps,1);
  const persisted=a.writes;
  await a.main.on_edge_event(payload(lease,1));assert.equal(a.config.state.taps,1);assert.equal(a.writes,persisted);
  assert.equal((await a.main.on_edge_event(payload('wrong',2))).accepted,false);
  await a.main.on_edge_event(payload(lease,2,'system.exit'));
  const exitedWrites=a.writes,exitedCalls=a.calls.length;
  const exited=await a.tools.renew_pet({node_id:'edge'});assert.equal(exited.closed,'system.exit');
  assert.equal(a.calls.length,exitedCalls);assert.equal(a.writes,exitedWrites);
  await a.main.on_edge_event(payload(lease,2,'system.exit'));assert.equal(a.writes,exitedWrites);
  assert(!a.calls.some(c=>c.action==='events.poll'),'not even renamed input polling');
});
test('Core ACK requires persistence; failure rolls back and retry counts once; loss is explicit',async()=> {
  const a=await actor();await a.tools.start_pet({node_id:'edge'});
  const lease=a.config.state.sessions.edge.lease;
  a.failNext();await assert.rejects(a.main.on_edge_event(payload(lease,1)),/disk unavailable/);
  assert.equal(a.config.state.taps,0);assert.equal(a.config.state.sessions.edge.cursor,0);
  await a.main.on_edge_event(payload(lease,1));assert.equal(a.config.state.taps,1);
  await a.main.on_edge_event(payload(lease,8,'target.tap',6));
  assert.equal(a.config.state.taps,2);assert.equal(a.config.state.sessions.edge.lostEvents,true);
  const writes=a.writes;await a.main.on_edge_event(payload(lease,8,'target.tap',6));assert.equal(a.writes,writes);
});

test('generic listener rejects an unrelated producer or action without ACK or mutation', async()=> {
  const a=await actor(); await a.tools.start_pet({node_id:'edge'});
  const stream=a.config.state.sessions.edge.lease, writes=a.writes;
  const sensor=payload(stream,1,'sensor.sample'); sensor.eventPayload.batch.source='sensor.environment';
  assert.equal((await a.main.on_edge_event(sensor)).accepted,false);
  assert.equal((await a.main.on_edge_event(payload(stream,1,'constructor'))).accepted,false);
  assert.equal((await a.main.on_edge_event(payload(stream,1,'unknown.action'))).accepted,false);
  assert.equal(a.writes,writes); assert.equal(a.config.state.taps,0);
});
