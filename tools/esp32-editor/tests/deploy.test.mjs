import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import {packLayout,crc32} from '../src/layout/package-layout.mts';
import {defaultProject} from '../src/layout/default-project.mts';
import {deployRoute} from '../src/api/deploy-api.mts';
import {aiRoute,previewCodeEdits} from '../src/api/ai-api.mts';
const listen=server=>new Promise(resolve=>server.listen(0,'127.0.0.1',()=>resolve('http://127.0.0.1:'+server.address().port)));
test('compact package has stable header, CRC and no JSON/parser requirement on device',()=>{
 const data=packLayout(defaultProject()),view=new DataView(data.buffer);
 assert.equal(new TextDecoder().decode(data.slice(0,4)),'OUI2');assert.equal(view.getUint32(4,true),data.length-16);
 assert.equal(view.getUint32(8,true),crc32(data.slice(16)));assert.equal(data[16],8);assert.ok(data.length<4096);
 assert.throws(()=>packLayout({...defaultProject(),enabled:false}));
});
test('fixed renderer rejects both layout deployments without contacting a device', async t => {
 let requests=0;
 const device=http.createServer((_req,res)=>{requests++;res.end('{}');});
 const address=await listen(device);
 const server=http.createServer((req,res)=>deployRoute(req,res,new URL(req.url,'http://localhost')));
 const base=await listen(server);t.after(()=>{device.close();server.close();});
 for(const route of ['layout','usb-layout']) {
  const result=await fetch(base+'/api/deploy/'+route,{method:'POST',body:JSON.stringify({address,port:'not-a-port',document:defaultProject()})});
  assert.equal(result.status,422);assert.match((await result.json()).error,/不支持动态布局/);
 }
 assert.equal(requests,0);
});
test('independent AI tool loop reads source, handles split UTF8 and validates operations',async t=>{
 let calls=0;const upstream=http.createServer(async(req,res)=>{let body='';for await(const chunk of req)body+=chunk;const input=JSON.parse(body);assert.equal(req.headers.authorization,'Bearer memory-key');calls++;
 const message=calls===1?{role:'assistant',content:null,tool_calls:[{id:'source',type:'function',function:{name:'read_source',arguments:JSON.stringify({path:'tools/esp32-editor/src/layout/routes.mts'})}}]}:{role:'assistant',content:JSON.stringify({summary:'修改首页按钮',operations:[{op:'update',id:'home_apps',changes:{text:'Open'}}]})};
 if(calls===2)assert.equal(input.messages.at(-1).role,'tool');const data=Buffer.from(JSON.stringify({choices:[{message}]}));for(let i=0;i<data.length;i++)res.write(data.subarray(i,i+1));res.end();});
 const endpoint=await listen(upstream),server=http.createServer((req,res)=>aiRoute(req,res,new URL(req.url,'http://localhost'))),base=await listen(server);t.after(()=>{upstream.close();server.close();});
 const result=await fetch(base+'/api/ai/develop',{method:'POST',body:JSON.stringify({endpoint,model:'test',key:'memory-key',task:'修改按钮',context:{document:defaultProject()}})});
 assert.equal(result.status,200);const proposal=await result.json();assert.equal(proposal.summary,'修改首页按钮');assert.equal(proposal.operations[0].id,'home_apps');assert.equal(calls,2);
 await assert.rejects(previewCodeEdits([{path:'tools/esp32-editor/src/layout/routes.mts',revision:'stale',find:'anything',replace:''}]),/变化/);
});
