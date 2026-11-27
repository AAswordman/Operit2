import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
const require = createRequire(import.meta.url);
let ts;
try { ts = require('../../../../../.ci-tools/typescript/node_modules/typescript'); }
catch { ts = require('../../../../../edge/boards/esp32/editor/node_modules/typescript'); }
const compile = path => ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
  compilerOptions: {module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020},
}).outputText;
const assets = {exports:{}};
runInNewContext(compile('../src/assets.ts'),{module:assets,exports:assets.exports});
const packs = assets.exports.PACKS;
function fixture(nodes = [], fail = '') {
  const calls = [], module = {exports:{}};
  const caps = {protocol:1,configuration:{transient:true},animation:{local:true,maxFrames:16},formats:{pack:'ESP1',compressedAsset:'ESR1',paletteMax:64},
    limits:{packBytes:8192,cacheBytes:16384,packs:2,assetsPerPack:4},screen:{width:320,height:240},reserved:{h:24},cache:{packs:[]}};
  async function sceneCall(node,method,args={}) {
    calls.push({node,method,args});
    if(method===fail) throw Error('failed '+method);
    if(method==='capabilities')return caps;
    if(method==='pack.begin')return {upload:'token'};
    if(method==='config.read')return {layers:[{asset:'other:pet'}]};
    return {};
  }
  runInNewContext(compile('../src/driver.ts'),{module,exports:module.exports,require(name){return name==='./assets'?assets.exports:{sceneCall};},
    toolCall:async()=>JSON.stringify({currentNodeId:'local',nodes})});
  return {driver:module.exports,calls,caps};
}
const node = (id,overrides={})=>({nodeId:id,displayName:id,platform:'esp32',model:'ESP32-2432S028',reachable:true,...overrides});
test('discovers only directly reachable ESP32 Space nodes and verifies capabilities',async()=>{
  const f=fixture([node('local'),node('esp'),node('esp'),node('phone',{platform:'android-edge'}),node('offline',{reachable:false}),node('relay',{relayHops:2})]);
  const result=await f.driver.discoverDevices();
  assert.equal(JSON.stringify(result.devices.map(d=>d.target.nodeId)), '["esp"]');
  assert.equal(f.calls.length,1);
});
test('failed capability probe is reported and disconnected nodes cannot be selected',async()=>{
  const f=fixture([node('esp')],'capabilities');
  const result=await f.driver.discoverDevices();assert.equal(result.devices.length,0);assert.match(result.issues[0].message,/failed capabilities/);
  await assert.rejects(f.driver.connectDevice('missing'),/离线/);
});
test('uploads only the next mood at native pixel scale and reuses its cached pack',async()=>{
  const f=fixture();await f.driver.sendMood({nodeId:'esp',label:'Cat'},'thinking');
  assert.equal(f.calls.filter(c=>c.method==='pack.commit').length,1);
  const apply=f.calls.at(-1);assert.equal(apply.method,'config.show');assert.equal(apply.args.layers[0].asset,'task-cat-v2-thinking:cat');
  assert.equal(apply.args.layers[0].play,true);assert.equal(apply.args.layers[0].scale,3);
  assert(f.calls.every(c=>JSON.stringify({v:1,...c.args}).length<=1024));
  const pack=packs.find(p=>p.moods.includes('thinking'));
  f.caps.cache.packs=[{id:pack.id,sha256:pack.sha256}];f.calls.length=0;
  await f.driver.sendMood({nodeId:'esp',label:'Cat'},'thinking');
  assert.equal(f.calls.length,2);assert.equal(f.calls.at(-1).args.layers[0].asset,'task-cat-v2-thinking:cat');
});
test('reclaims only owned resources and keeps an active scene until new assets commit',async()=>{
  const f=fixture(); f.caps.cache.packs=[{id:'task-cat-v1-0',sha256:'old'},{id:'another-plugin',sha256:'other'}];
  await f.driver.sendMood({nodeId:'esp',label:'Cat'},'approval');
  assert.deepEqual(f.calls.filter(c=>c.method==='pack.remove').map(c=>c.args.packId),['task-cat-v1-0']);
  assert(f.calls.findIndex(c=>c.method==='pack.commit')<f.calls.findIndex(c=>c.method==='config.show'));
});
test('failed upload aborts without displaying incomplete resources',async()=>{
  const f=fixture([],'pack.chunk');await assert.rejects(f.driver.sendMood({nodeId:'esp',label:'Cat'},'idle'),/pack.chunk/);
  assert.equal(f.calls.at(-1).method,'pack.abort');assert(!f.calls.some(c=>c.method==='config.show'));
});
test('disconnect leaves another plugin scene intact',async()=>{
  const f=fixture();await f.driver.releaseScene({nodeId:'esp',label:'Cat'});
  assert.deepEqual(f.calls.map(c=>c.method),['config.read']);
});
test('compressed original assets obey limits, hashes and complete five-frame sampling',async()=>{
  const {createHash}=await import('node:crypto');
  const {decodePacks}=await import('../../../../../edge/boards/esp32/editor/web/scene-renderer.ts');
  const expected={idle:12,working:10,thinking:8,approval:15,completed:7,error:11};
  for(const p of packs){
    const bytes=Buffer.concat(p.chunks.map(c=>Buffer.from(c.data,'base64')));
    assert.equal(bytes.length,p.bytes);assert(bytes.length<=8192);
    assert.equal(createHash('sha256').update(bytes).digest('hex'),p.sha256);
    assert.equal(bytes.subarray(0,4).toString(),'ESP1');assert.equal(bytes[4],1);
    assert.equal(bytes.subarray(6,9).toString(),'cat');
    const data=bytes.subarray(11);assert.equal(bytes.readUInt16LE(9),data.length);
    assert.equal(data.subarray(0,4).toString(),'ESR1');assert.equal(data[4],77);assert.equal(data[5],67);
    assert.equal(data[6],expected[p.moods[0]]);assert(data[7]<=64);assert.equal(data.readUInt16LE(8),333);
    const asset=decodePacks([{id:p.id,data:bytes.toString('base64')}]).get(`${p.id}:cat`);
    assert.equal(asset.indices.length,77*67*expected[p.moods[0]]);
    assert(asset.indices.every(index=>index<asset.palette.length));
  }
  assert.equal(packs.length,6);
});

test('incomplete or undersized capability replies cannot select a device',async()=>{
  for(const change of [caps=>delete caps.configuration.transient,caps=>delete caps.limits.packBytes,caps=>caps.limits.cacheBytes=8192,caps=>delete caps.formats.compressedAsset,caps=>delete caps.reserved.h]){
    const f=fixture([node('esp')]);change(f.caps);
    await assert.rejects(f.driver.connectDevice('esp'),/容量/);
  }
});
