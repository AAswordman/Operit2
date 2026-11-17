import test from 'node:test';
import assert from 'node:assert/strict';
import { load } from './helpers.mjs';
const { default: screen } = await load('src/ui/screen.ts');
const web = await load('web/host.ts');

async function nativeBridge(saveFile) {
  let bridge;
  const ctx={
    createWebViewController:()=>({addJavascriptInterface(_name,value){bridge=value;}}),
    useRef:(_key,current)=>({current}),useEffect(){},useState:(_key,value)=>[value,()=>{}],
    Theme:{getCurrent:()=>({brightness:'light'})},saveFile,
    UI:{Box:props=>({props}),Text:()=>({})},
  };
  globalThis.ToolPkg={readResource:async()=>'/studio.html'};
  try { await screen(ctx).props.onLoad(); } finally { delete globalThis.ToolPkg; }
  return bridge;
}

test('native export calls the system save API with binary content, WAV metadata and its selected path',async()=>{
  const content=Buffer.alloc(19*1024*1024,0xf1).toString('base64'); // exceeds the old 24M-character limit
  let calls=0;
  const bridge=await nativeBridge(async options=>{
    calls++;assert.equal(options.contentBase64,content);assert.equal(options.content,undefined);
    assert.deepEqual(options.extensions,['wav']);assert.equal(options.mimeType,'audio/wav');
    assert.equal(options.suggestedName,'project_r73.wav');
    return {cancelled:false,path:'/chosen/project_r73.wav'};
  });
  assert.deepEqual(await bridge.saveFile(['project_r73.wav',content]),{cancelled:false,path:'/chosen/project_r73.wav'});
  assert.equal(calls,1);
});

test('host bridge preserves cancellation, rejects invalid payloads and never accepts a null save result',async()=>{
  let calls=0;
  const bridge=await nativeBridge(async options=>{calls++;assert.deepEqual(options.extensions,['json']);return {cancelled:true,path:null};});
  assert.deepEqual(await bridge.saveFile(['project.operitmusic.json','e30=']),{cancelled:true,path:null});
  for (const [name,value] of [['../escape.wav','e30='],['ok.wav','%%%?'],['ok.wav','a']]) await assert.rejects(bridge.saveFile([name,value]),/无效/);
  assert.equal(calls,1);
  for(const result of [null,undefined,{}, {cancelled:false,path:null},{cancelled:false,path:' '}]) {
    const broken=await nativeBridge(async()=>result);
    await assert.rejects(broken.saveFile(['ok.wav','e30=']),/未返回/);
  }
  const failure=await nativeBridge(async()=>{throw Error('Disk full');});
  await assert.rejects(failure.saveFile(['ok.wav','e30=']),/Disk full/);
});

test('web export validates the host response and never reports null or cancelled paths as saved',async()=>{
  const bytes=new TextEncoder().encode('{"name":"混音🌍"}');
  try {
    globalThis.window={MusicHost:{saveFile:async(name,content)=>{
      assert.equal(name,'project.operitmusic.json');assert.deepEqual(Buffer.from(content,'base64'),Buffer.from(bytes));
      return {cancelled:false,path:'/chosen/混音.operitmusic.json'};
    }}};
    assert.equal(await web.saveFile('project.operitmusic.json',bytes),'/chosen/混音.operitmusic.json');
    for(const result of [null,undefined,'null','/legacy/path',{}, {cancelled:false,path:null}]) {
      window.MusicHost.saveFile=async()=>result;
      await assert.rejects(web.saveFile('project.operitmusic.json',bytes),/未返回/);
    }
    window.MusicHost.saveFile=async()=>({cancelled:true,path:null});
    await assert.rejects(web.saveFile('project.operitmusic.json',bytes),/已取消/);
    window.MusicHost.saveFile=async()=>{throw Error('Write failed');};
    await assert.rejects(web.saveFile('project.operitmusic.json',bytes),/Write failed/);
  } finally {delete globalThis.window;}
});
