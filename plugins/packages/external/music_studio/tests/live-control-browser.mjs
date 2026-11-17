import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
const server=createServer(async(_req,res)=>{res.writeHead(200,{'Content-Type':'text/html;charset=utf-8'});res.end(await readFile('resources/studio.html'));});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const browser=await chromium.launch({channel:'chrome',headless:true});
try{
  const context=await browser.newContext({viewport:{width:1440,height:900}}),page=await context.newPage(),errors=[];
  page.on('pageerror',error=>errors.push(String(error)));
  const base=`http://127.0.0.1:${server.address().port}/`;
  await page.goto(`${base}?test=1`);await page.waitForFunction(()=>window.__musicTest);
  assert.equal(await page.evaluate(()=>typeof window.musicStudioControl),'undefined','bridge must be opt-in');
  assert.equal(await page.locator('.track-info input[type=range]').count(),0,'no duplicated gain faders in the arrangement labels');
  const original=await page.evaluate(()=>window.__musicTest.project());
  await page.goto(`${base}?control=1&test=1`);await page.waitForFunction(()=>window.musicStudioControl);
  const get=()=>page.evaluate(()=>window.musicStudioControl.get());
  const initial=await get();assert.equal(initial.projectId,original.id);assert.equal(initial.revision,original.revision);
  assert.deepEqual(await page.evaluate(()=>window.musicStudioControl.project()),original,'enabling collaboration cannot rewrite composition');
  const dry=await page.evaluate(async()=>{const c=window.musicStudioControl,s=await c.get();return c.dryRun({projectId:s.projectId,revision:s.revision,label:'检查改动但不提交',operations:[{type:'project.set',patch:{bpm:121}}]});});
  assert.equal(dry.dryRun,true);assert.equal(dry.revision,initial.revision);assert.deepEqual(await page.evaluate(()=>window.__musicTest.project()),original);
  const id=initial.tracks.find(t=>t.engine==='spectral').id;
  await page.evaluate(id=>window.musicStudioControl.view({type:'instrument',trackId:id}),id);
  assert.ok(await page.locator('.audition-panel').isVisible());assert.equal(await page.locator('[data-preview-pitch]').count(),24);
  const keyboard=await page.locator('.audition-panel').boundingBox(),editor=await page.locator('.editor').boundingBox();
  assert.ok(keyboard.y+keyboard.height<=editor.y+editor.height+1,'preview keys pinned in the device viewport');
  await page.locator('.synth-scroll').evaluate(el=>{el.scrollTop=el.scrollHeight;});
  assert.ok(await page.locator('.preview-keyboard').isVisible());
  assert.equal(await page.locator('.synth-help[open]').count(),0,'long explanations collapsed');
  // An actual UI transaction refreshes before the bridge returns, and one undo restores this round.
  const report=await page.evaluate(async id=>{const c=window.musicStudioControl,s=await c.get();return c.batch({projectId:s.projectId,revision:s.revision,label:'测试轨道增益调整',operations:[{type:'track.set',trackId:id,patch:{gain:.35}}]});},id);
  const edited=await page.evaluate(()=>window.__musicTest.project());assert.equal(edited.revision,original.revision+1);assert.equal(edited.tracks.find(t=>t.id===id).gain,.35);assert.equal(report.tracks[0].soundChanged,true);
  const undo=await page.evaluate(async()=>{const c=window.musicStudioControl,s=await c.get();return c.undo({projectId:s.projectId,revision:s.revision});});
  assert.deepEqual((await page.evaluate(()=>window.__musicTest.project())).tracks,original.tracks);assert.equal(undo.revision,edited.revision+1);
  const locked=await page.evaluate(async()=>{const c=window.musicStudioControl,s=await c.get();try{await c.transport({projectId:s.projectId,revision:s.revision,type:'play'});return null;}catch(e){return String(e);}});
  assert.match(locked,/AUDIO_LOCKED/);
  await page.evaluate(async()=>{const c=window.musicStudioControl,s=await c.get();await c.transport({projectId:s.projectId,revision:s.revision,type:'seek',bar:17});});
  assert.equal((await get()).playback.beat,32);assert.equal((await get()).playback.playing,false);
  await page.evaluate(id=>window.musicStudioControl.view({type:'piano',trackId:id,bar:17}),id);assert.ok(await page.locator('#piano').isVisible());
  assert.equal((await get()).playback.beat,32,'view must not alter playback position');
  const exported=await page.evaluate(()=>window.musicStudioControl.exportProject());assert.deepEqual(JSON.parse(exported.json),await page.evaluate(()=>window.__musicTest.project()));
  // Desktop and small-screen instrument UI remain compact with reachable preview keys.
  for(const [width,height] of [[1440,900],[390,844],[320,568],[844,390]]){
    await page.setViewportSize({width,height});await page.evaluate(id=>window.musicStudioControl.view({type:'instrument',trackId:id}),id);
    const dimensions=await page.evaluate(()=>({w:innerWidth,h:innerHeight,docW:document.documentElement.scrollWidth,docH:document.documentElement.scrollHeight,keys:document.querySelector('.preview-keyboard').getBoundingClientRect().toJSON(),range:document.querySelector('[data-param]').getBoundingClientRect().height}));
    assert.ok(dimensions.docW<=width+1&&dimensions.docH<=height+1);assert.ok(dimensions.keys.bottom<=height);assert.ok(dimensions.keys.height>=40&&dimensions.keys.height<=74);assert.equal(dimensions.range,40,'knobs stay usable instead of shrinking on small screens');
  }
  await page.goto(`${base}?control=1&test=1`);await page.waitForFunction(()=>window.musicStudioControl);
  assert.deepEqual((await page.evaluate(()=>window.musicStudioControl.project())).tracks,original.tracks,'actual current project survives reload');
  assert.deepEqual(errors,[]);console.log('PASS live webpage control: opt-in, same-project read/write, dry run, scoped view, atomic undo, locked-audio honesty, JSON export, pinned compact preview keyboard and persistence');
  await context.close();
}finally{await browser.close();await new Promise(r=>server.close(r));}
