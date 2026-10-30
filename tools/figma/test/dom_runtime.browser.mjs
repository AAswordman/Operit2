import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {resolve} from 'node:path';

const require=createRequire(import.meta.url);
const {chromium}=require(process.env.OPERIT_PLAYWRIGHT_MODULE ?? resolve('../../.ci-tools/browser/node_modules/playwright'));
const raw=async name=>(await readFile(new URL(`../../../apps/flutter/packages/operit_ui/lib/src/design/${name}`,import.meta.url),'utf8')).match(/r'''([\s\S]*?)'''/)[1];
const bootstrap=await raw('design_dom_runtime.dart');
const capture=bootstrap+'\n'+await raw('design_dom_capture.dart');
const browser=await chromium.launch({headless:true,...(process.env.OPERIT_BROWSER_EXECUTABLE?{executablePath:process.env.OPERIT_BROWSER_EXECUTABLE}:{})});
try {
  const page=await browser.newPage({viewport:{width:900,height:700}});
  await page.setContent(`<!doctype html><html><body>
    <p id="count">Count 0</p><button id="increment">Increment</button>
    <input id="input" maxlength="5"><output id="input-state"></output>
    <div id="holder"><p data-operit-design-id="stable">Original label</p></div>
    <div id="shadow"></div><iframe id="frame" srcdoc="<button id='nested'>Frame button</button>"></iframe>
    <script>
      let count=0;increment.onclick=()=>document.querySelector('#count').textContent='Count '+(++count);
      input.oninput=()=>document.querySelector('#input-state').textContent=input.value;
      const root=document.querySelector('#shadow').attachShadow({mode:'open'});
      root.innerHTML='<button id="shadow-button">Shadow button</button>';
      root.querySelector('button').onclick=()=>document.querySelector('#count').textContent='Shadow clicked';
    </script></body></html>`);
  await page.locator('#frame').evaluate(el=>new Promise(resolve=>el.contentDocument?.readyState==='complete'?resolve():el.addEventListener('load',resolve,{once:true})));
  const collect=async()=>JSON.parse(await page.evaluate(capture));
  const flatten=value=>{const all=[];const walk=n=>{all.push(n);(n.children??[]).forEach(walk);};walk(value.scene);return all;};
  let all=flatten(await collect());
  const byText=text=>all.find(n=>n.text===text);
  const byId=id=>all.find(n=>n.source?.domId===id);
  const target=n=>n.source.domRuntimeTarget;
  const request=async message=>page.evaluate(m=>window.__operitDesignDomRuntime.request(m),message);
  await request({operation:'event',target:target(byText('Increment')),event:{type:'tap'}});
  assert.equal(await page.locator('#count').textContent(),'Count 1');
  await request({operation:'event',target:target(byId('input')),event:{type:'input',text:'abcdef'}});
  assert.equal(await page.locator('#input-state').textContent(),'abcde');
  const original=byText('Original label'), id=target(original);
  await request({operation:'edit',target:id,properties:{text:'Edited label'}});
  assert.equal(await page.getByText('Edited label').textContent(),'Edited label');
  await page.locator('[data-operit-design-id="stable"]').evaluate(el=>{const replacement=document.createElement('p');replacement.dataset.operitDesignId='stable';replacement.textContent='New business label';el.replaceWith(replacement);});
  all=flatten(await collect());
  assert.equal(target(byText('Edited label')),id);
  await request({operation:'clear',target:id});
  assert.equal(await page.locator('[data-operit-design-id="stable"]').textContent(),'New business label');
  all=flatten(await collect());
  await request({operation:'edit',target:target(byId('increment')),properties:{width:240,fill:'#FF2200FF',radius:14}});
  assert.equal(await page.locator('#increment').evaluate(el=>getComputedStyle(el).width),'240px');
  assert.equal(await page.locator('#increment').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(255, 34, 0)');
  await page.locator('#increment').evaluate(el=>el.style.width='100px');
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('#increment')).width==='240px');
  await request({operation:'clear',target:target(byId('increment'))});
  assert.equal(await page.locator('#increment').evaluate(el=>getComputedStyle(el).width),'100px');
  all=flatten(await collect());
  await request({operation:'event',target:target(byText('Shadow button')),event:{type:'tap'}});
  assert.equal(await page.locator('#count').textContent(),'Shadow clicked');
  const nested=byText('Frame button');assert.ok(nested,'same-origin iframe target');
  await page.frameLocator('#frame').locator('#nested').evaluate(el=>el.onclick=()=>el.textContent='Frame clicked');
  await request({operation:'event',target:target(nested),event:{type:'tap'}});
  assert.equal(await page.frameLocator('#frame').locator('#nested').textContent(),'Frame clicked');
  await assert.rejects(request({operation:'edit',target:id,properties:{width:NaN}}));
  await assert.rejects(request({operation:'event',target:'missing',event:{type:'tap'}}));
  console.log('PASS DOM runtime: real click/input handlers, format limit, editable DOM, remount persistence, source-aware undo, shadow DOM and iframe actions');
} finally {await browser.close();}
