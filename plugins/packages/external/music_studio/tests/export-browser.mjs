import { chromium, webkit } from 'playwright';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { load } from './helpers.mjs';
const { default: screen }=await load('src/ui/screen.ts');
const { demoProject }=await load('src/shared/composition.ts');
const fixture=demoProject('field-of-hopes-and-dreams');
const html=await readFile('resources/studio.html');
const server=createServer((_req,res)=>{res.writeHead(200,{'Content-Type':'text/html;charset=utf-8'});res.end(html);});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
await mkdir('.test-output/exports',{recursive:true});
const url=`http://127.0.0.1:${server.address().port}/`;
try {
  for(const [name,type] of [['chromium',chromium],['webkit',webkit]]) {
    const browser=await type.launch(name==='chromium'?{channel:'chrome',headless:true}:{headless:true});
    try {
      const context=await browser.newContext(),page=await context.newPage();
      const saved=[],errors=[];let mode='saved',bridge;
      page.on('pageerror',e=>errors.push(String(e)));
      const ctx={
        createWebViewController:()=>({addJavascriptInterface(_name,value){bridge=value;}}),
        useRef:(_key,current)=>({current}),useEffect(){},useState:(_key,value)=>[value,()=>{}],
        Theme:{getCurrent:()=>({brightness:'light'})},UI:{Box:props=>({props}),Text:()=>({})},
        async saveFile(options){
          // File-selector boundary only: the production screen builds the request;
          // the Flutter owner service has separate byte-exact decode/selector tests.
          if(mode==='cancel')return {cancelled:true,path:null};
          if(mode==='error')throw Error('Disk full');
          const bytes=Buffer.from(options.contentBase64,'base64');
          const path=resolve('.test-output/exports',`${name}-${options.suggestedName}`);
          await writeFile(path,bytes);saved.push({path,bytes,options});
          return {cancelled:false,path};
        },
      };
      globalThis.ToolPkg={readResource:async()=>'/studio.html'};
      await screen(ctx).props.onLoad();delete globalThis.ToolPkg;
      await page.exposeFunction('__saveNativeFile',async(name,content)=>mode==='null'?null:bridge.saveFile([name,content]));
      await context.addInitScript(fixture=>{
        const snapshot={project:fixture,projects:[{id:fixture.id,name:fixture.name,revision:fixture.revision}],commands:[],receipts:[]};
        window.MusicHost={getTheme:async()=>({brightness:'light'}),request:async()=>snapshot,saveFile:(...args)=>window.__saveNativeFile(...args)};
      },fixture);
      await page.goto(url);
      const openMenu=async()=>{if(!await page.locator('.project-menu').count())await page.locator('[data-action="project-menu"]').click();};
      await openMenu();
      await page.locator('[data-action="export-json"]').click();
      await page.waitForFunction(()=>document.querySelector('.notice')?.textContent.includes('工程已导出'));
      assert.deepEqual(JSON.parse((await readFile(saved[0].path)).toString('utf8')),fixture);
      for(const [next,message] of [['cancel','已取消导出'],['null','未返回有效保存结果'],['error','Disk full']]) {
        mode=next;
        await openMenu();
        await page.locator('[data-action="export-json"]').click();
        await page.waitForFunction(message=>document.querySelector('.notice')?.textContent.includes(message),message);
        assert.equal(saved.length,1);
      }
      mode='saved';
      // The complete remixed MIDI crosses the old export limit and exercises the
      // real HTML -> WAV worker -> WebView callback -> system save API contract.
      await openMenu();
      await page.locator('[data-action="render"]').click();
      await page.waitForFunction(()=>document.querySelector('.notice')?.textContent.includes('WAV 已保存'),null,{timeout:120000});
      const wav=saved.at(-1);assert.equal(saved.length,2);
      assert.equal(wav.bytes.toString('ascii',0,4),'RIFF');assert.equal(wav.bytes.toString('ascii',8,12),'WAVE');
      assert.equal(wav.bytes.readUInt32LE(4)+8,wav.bytes.length);
      assert.ok(wav.bytes.length>24000000);
      assert.deepEqual(await readFile(wav.path),wav.bytes);
      assert.equal(wav.options.mimeType,'audio/wav');assert.deepEqual(wav.options.extensions,['wav']);
      assert.deepEqual(errors,[]);
      console.log(`PASS ${name}: exact project JSON and ${wav.bytes.length}-byte full WAV saved through native API; null, cancellation and write failure never report success`);
      await context.close();
    } finally {await browser.close();}
  }
} finally {await new Promise(done=>server.close(done));}
