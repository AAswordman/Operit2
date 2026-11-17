import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
const server=createServer(async (_req,res) => { res.writeHead(200,{ 'Content-Type':'text/html;charset=utf-8' }); res.end(await readFile('resources/studio.html')); });
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const browser=await chromium.launch({ channel:'chrome',headless:true });
const url=`http://127.0.0.1:${server.address().port}/?test=1`;
const errors=[];
const near=(a,b,tolerance=.02)=>assert.ok(Math.abs(a-b)<tolerance,`${a} ≠ ${b}`);
try {
  await mkdir('.test-output',{ recursive:true });
  const context=await browser.newContext({ viewport:{ width:1440,height:900 } });
  const page=await context.newPage(); page.on('pageerror',e=>errors.push(String(e)));
  await page.goto(url); await page.waitForFunction(()=>window.__musicTest);
  const initial=await page.evaluate(()=>({ project:window.__musicTest.project(),state:window.__musicTest.state() }));
  const nav=()=>page.evaluate(()=>window.__musicTest.navigation());
  const editor=()=>page.evaluate(()=>window.__musicTest.editor());
  async function wheel(selector,modifier,dy,dx=0) {
    const r=await page.locator(selector).boundingBox();
    const point={ x:r.x+r.width*.65,y:r.y+Math.min(18,r.height/2) };
    await page.mouse.move(point.x,point.y);
    if (modifier) await page.keyboard.down(modifier);
    await page.mouse.wheel(dx,dy);
    if (modifier) await page.keyboard.up(modifier);
    await page.waitForTimeout(80); return point;
  }
  const before=await nav(), ruler=await page.locator('#ruler').boundingBox();
  const point=await wheel('#ruler','Control',-400);
  const after=await nav(); assert.ok(after.width>before.width*2);
  near((point.x-ruler.x)/before.width,(after.left+point.x-ruler.x)/after.width);
  await wheel('#ruler','Shift',180); assert.ok((await nav()).left>after.left+170);
  await wheel('#ruler','Meta',-120); assert.ok((await nav()).width>after.width);
  await wheel('#ruler',null,160); assert.ok((await nav()).top>0,'ordinary timeline wheel must scroll tracks');
  await page.locator('[data-action="zoom-fit"]').click(); near((await nav()).left,0); assert.equal((await nav()).zoom,0);
  // Virtualized canvases stay viewport-sized even at deep zoom.
  for(let i=0;i<8;i++) await page.locator('[data-action="zoom-in"]').click();
  const raster=await page.locator('[data-canvas]').first().evaluate(el=>({ width:el.width,viewport:document.querySelector('#timeline-scroll').clientWidth }));
  assert.ok(raster.width<=raster.viewport*2+2);
  await page.locator('[data-action="zoom-fit"]').click();
  await page.locator('[data-region]').first().click();
  const pv=(await editor()).viewport, piano=await page.locator('#piano').boundingBox();
  const anchor=await wheel('#piano','Control',-300), zoomed=(await editor()).viewport;
  assert.ok(zoomed.span<pv.span); near(zoomed.rows,pv.rows);
  const ax=(anchor.x-piano.x-42)/(piano.width-42);
  near(pv.start+pv.span*ax,zoomed.start+zoomed.span*ax,.0001);
  await wheel('#piano','Alt',-220); const pitchZoom=(await editor()).viewport; assert.ok(pitchZoom.rows<zoomed.rows); near(pitchZoom.span,zoomed.span);
  const ay=(anchor.y-piano.y)/piano.height;
  near(zoomed.top-zoomed.rows*ay,pitchZoom.top-pitchZoom.rows*ay,.0001);
  await wheel('#piano','Shift',80); assert.ok((await editor()).viewport.start>pitchZoom.start);
  const prior=(await editor()).viewport;
  await wheel('#piano',null,120); assert.ok((await editor()).viewport.top<prior.top);
  const middleBefore=(await editor()).viewport;
  await page.mouse.move(piano.x+piano.width/2,piano.y+piano.height/2); await page.mouse.down({ button:'middle' });
  await page.mouse.move(piano.x+piano.width/2+60,piano.y+piano.height/2-35,{ steps:6 }); await page.mouse.up({ button:'middle' });
  const middleAfter=(await editor()).viewport; assert.ok(middleAfter.start<middleBefore.start); assert.ok(middleAfter.top<middleBefore.top);
  assert.equal(await page.locator('#zoom').inputValue(),'free');
  assert.deepEqual(await page.evaluate(()=>window.__musicTest.project()),initial.project,'navigation must not write notes or revisions');
  assert.equal((await page.evaluate(()=>window.__musicTest.state())).playing,false); near((await page.evaluate(()=>window.__musicTest.state())).beat,initial.state.beat);
  // Context menu/knob areas are not intercepted by piano navigation.
  await page.locator('[data-action="region-device"]').click();
  const knob=page.locator('[data-param]').first(), knobBox=await knob.boundingBox();
  await page.mouse.move(knobBox.x+10,knobBox.y+5); await page.keyboard.down('Control'); await page.mouse.wheel(0,-200); await page.keyboard.up('Control');
  assert.equal((await editor()).tab,'synth');
  await context.close();
  console.log('PASS desktop: anchored Ctrl/⌘ wheel, Shift pan, Alt pitch zoom, trackpad pan, middle drag, bounded rasters and view-only state');

  const touchContext=await browser.newContext({ viewport:{ width:844,height:700 },hasTouch:true,isMobile:true,deviceScaleFactor:1 });
  const touch=await touchContext.newPage(); touch.on('pageerror',e=>errors.push(String(e)));
  await touch.goto(url); await touch.waitForFunction(()=>window.__musicTest);
  const cdp=await touchContext.newCDPSession(touch);
  const touchInitial=await touch.evaluate(()=>window.__musicTest.project());
  const touchNav=()=>touch.evaluate(()=>window.__musicTest.navigation());
  const touchEditor=()=>touch.evaluate(()=>window.__musicTest.editor());
  const send=(type,points)=>cdp.send('Input.dispatchTouchEvent',{ type,touchPoints:points.map((p,i)=>({ id:i+1,x:p.x,y:p.y,radiusX:6,radiusY:6,force:1 })) });
  async function gesture(a,b,nextA,nextB,end='touchEnd') {
    await send('touchStart',[a,b]);
    for(let i=1;i<=8;i++) {
      const mix=(from,to)=>({ x:from.x+(to.x-from.x)*i/8,y:from.y+(to.y-from.y)*i/8 });
      await send('touchMove',[mix(a,nextA),mix(b,nextB)]);
    }
    await send(end,[]); await touch.waitForTimeout(80);
  }
  const tr=await touch.locator('#timeline-scroll').boundingBox();
  const gutter=await touch.locator('.ruler-label').evaluate(el=>el.getBoundingClientRect().width);
  const cx=tr.x+gutter+(tr.width-gutter)/2,cy=tr.y+140;
  const tn=await touchNav();
  await gesture({ x:cx-55,y:cy },{ x:cx+55,y:cy },{ x:cx-110,y:cy-25 },{ x:cx+110,y:cy-25 });
  const pinched=await touchNav(); assert.ok(pinched.width>tn.width*1.8); assert.ok(pinched.top>15);
  assert.equal((await touchEditor()).open,false,'pinch over clips must not open the editor');
  const dragBefore=await touchNav();
  await gesture({ x:cx-55,y:cy },{ x:cx+55,y:cy },{ x:cx-95,y:cy-30 },{ x:cx+15,y:cy-30 });
  const dragAfter=await touchNav(); near(dragAfter.width,dragBefore.width,2); assert.ok(dragAfter.left>dragBefore.left+30); assert.ok(dragAfter.top>dragBefore.top+20);
  // A new stationary tap must still open its exact phrase after navigation.
  await touch.locator('#timeline-scroll').evaluate(el=>{ el.scrollLeft=0;el.scrollTop=0; });
  await touch.waitForTimeout(80);
  const clip=await touch.locator('[data-region]').first().boundingBox();
  await touch.touchscreen.tap(clip.x+clip.width/2,clip.y+clip.height/2);
  await touch.waitForFunction(()=>window.__musicTest.editor().open);
  const pbox=await touch.locator('#piano').boundingBox(), px=pbox.x+42+(pbox.width-42)/2,py=pbox.y+pbox.height/2;
  const pBefore=(await touchEditor()).viewport;
  await gesture({ x:px-55,y:py-30 },{ x:px+55,y:py+30 },{ x:px-100,y:py-55 },{ x:px+100,y:py+55 });
  const pAfter=(await touchEditor()).viewport; assert.ok(pAfter.span<pBefore.span*.7); assert.ok(pAfter.rows<pBefore.rows*.7);
  await gesture({ x:px-55,y:py-30 },{ x:px+55,y:py+30 },{ x:px-80,y:py-50 },{ x:px+30,y:py+10 });
  const translated=(await touchEditor()).viewport; near(translated.span,pAfter.span,.01); near(translated.rows,pAfter.rows,.01); assert.ok(translated.start>pAfter.start); assert.ok(translated.top<pAfter.top);
  // Cancelled captures cannot leak into later note editing.
  await gesture({ x:px-55,y:py-30 },{ x:px+55,y:py+30 },{ x:px-70,y:py-40 },{ x:px+70,y:py+40 },'touchCancel');
  assert.deepEqual(await touch.evaluate(()=>window.__musicTest.project()),touchInitial,'two-finger pinch/translation/cancel must never change project');
  const state=await touch.evaluate(()=>window.__musicTest.state()); assert.equal(state.playing,false); near(state.beat,0);
  await touch.screenshot({ path:'.test-output/touch-navigation.png' });
  // One-finger tap remains a real edit after cancelled gestures (capture on original canvas).
  await touch.touchscreen.tap(px,py);
  await touch.waitForFunction(rev=>window.__musicTest.project().revision>rev,touchInitial.revision);
  const edited=await touch.evaluate(()=>window.__musicTest.project());
  const originalCount=touchInitial.tracks.reduce((n,t)=>n+t.notes.length,0),editedCount=edited.tracks.reduce((n,t)=>n+t.notes.length,0);
  assert.equal(Math.abs(editedCount-originalCount),1);
  await touchContext.close();
  console.log('PASS touch: actual two-finger pinch + simultaneous pan, touch cancellation, no accidental editing/seek, stationary clip/note taps');
  assert.deepEqual(errors,[]); console.log('Navigation browser checks passed.');
} finally { await browser.close(); await new Promise(resolve=>server.close(resolve)); }
