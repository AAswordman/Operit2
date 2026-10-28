import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url);
const base=new URL('../../../../../plugins/packages/external/edge_pixel_pet/dist/',import.meta.url);
async function actor(failUpload=false) {
  const calls=[], handlers=new Map();
  const sceneCall=async(node,action,args={})=> {
    calls.push({node,action,args});
    if(action==='capabilities') return {protocol:1,input:{local:true},cache:{packs:[]}};
    if(action==='pack.begin') return {upload:'upload'};
    if(action==='pack.chunk'&&failUpload) throw Error('link failure');
    if(action==='config.apply') return {configured:true,persistent:true,revision:7};
    return {};
  };
  const context=vm.createContext({exports:{},getEnv:()=> 'edge',
    ToolPkg:{ipc:{on:(channel,handler)=>handlers.set(channel,handler),call:(channel,payload)=>handlers.get(channel)(payload)}},
  });
  context.require=name=>name==='./assets'?require(fileURLToPath(new URL('assets.js',base))):
    name==='./protocol'?{sceneCall}:name==='./pet'?pet:null;
  const load=async file=> {const exports={}; const source=await readFile(new URL(file,base),'utf8');
    new vm.Script('(function(require,exports){'+source+'\n})').runInContext(context)(context.require,exports);return exports;};
  const pet=await load('pet.js');const main=await load('main.js');const tools=await load('packages/edge_pixel_pet.js');
  return {tools,main,calls};
}
test('configuration package uploads then saves on device without runtime hooks or leases', async () => {
  const a=await actor();
  assert.equal(a.main.on_edge_event,undefined);
  const result=await a.tools.apply_configuration({node_id:'edge'});
  assert.equal(result.revision,7);
  assert(a.calls.some(call=>call.action==='pack.commit'));
  assert.equal(a.calls.at(-1).action,'config.apply');
  assert.equal(a.calls.at(-1).args.enabled,true);
  assert(a.calls.every(call=>call.args.lease===undefined));
  assert(!a.calls.some(call=>/lease|events|scene\.set/.test(call.action)));
  await a.tools.clear_configuration({node_id:'edge'});
  assert.equal(a.calls.at(-1).action,'config.clear');
});
test('failed asset transfer is aborted and never publishes the configuration', async () => {
  const a=await actor(true);
  await assert.rejects(a.tools.apply_configuration(),/link failure/);
  assert.equal(a.calls.at(-1).action,'pack.abort');
  assert(!a.calls.some(call=>call.action==='config.apply'));
});
test('connection test only reads native capabilities', async () => {
  const a=await actor();
  assert.equal((await a.main.test_connection()).passed,true);
  assert.deepEqual(a.calls.map(call=>call.action),['capabilities']);
});
