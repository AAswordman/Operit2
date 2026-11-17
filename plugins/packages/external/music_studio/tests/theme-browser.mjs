import { chromium, webkit } from 'playwright';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { demoProject }=await load('src/shared/composition.ts');
const fixture=demoProject('field-of-hopes-and-dreams');
const html=await readFile('resources/studio.html');
const server=createServer((_req,res)=>{res.writeHead(200,{'Content-Type':'text/html;charset=utf-8'});res.end(html);});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const url=`http://127.0.0.1:${server.address().port}/?test=1`;
await mkdir('.test-output',{recursive:true});
function luminance(rgb) {
  const c=rgb.match(/[\d.]+/g).slice(0,3).map(Number).map(x=>{x/=255;return x<=.04045?x/12.92:((x+.055)/1.055)**2.4;});
  return c[0]*.2126+c[1]*.7152+c[2]*.0722;
}
function contrast(a,b){const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);}
async function assertTheme(page,mode) {
  await page.waitForFunction(mode=>document.documentElement.dataset.theme===mode,mode);
  const colors=await page.evaluate(()=>{
    const body=getComputedStyle(document.body),canvas=document.querySelector('[data-canvas]');
    const pixel=canvas.getContext('2d').getImageData(2,2,1,1).data;
    const probe=document.createElement('span');probe.style.color='var(--muted)';document.body.append(probe);
    const muted=getComputedStyle(probe).color;probe.remove();
    return {bg:body.backgroundColor,fg:body.color,muted,scheme:getComputedStyle(document.documentElement).colorScheme,pixel:[...pixel],w:innerWidth,h:innerHeight,sw:document.documentElement.scrollWidth,sh:document.documentElement.scrollHeight};
  });
  assert.equal(colors.scheme,mode);assert.ok(contrast(colors.bg,colors.fg)>=7,JSON.stringify(colors));
  assert.ok(contrast(colors.bg,colors.muted)>=4.5,JSON.stringify(colors));
  assert.ok(colors.sw<=colors.w+1&&colors.sh<=colors.h+1);
  await page.waitForFunction(mode=>{
    const canvas=document.querySelector('[data-canvas]');if(!canvas.getBoundingClientRect().width)return true;
    const pixels=canvas.getContext('2d').getImageData(0,2,canvas.width,1).data;
    let sum=0;for(let i=0;i<pixels.length;i+=4)sum+=pixels[i]+pixels[i+1]+pixels[i+2];
    const average=sum/(canvas.width*3);return mode==='dark'?average<150:average>180;
  },mode);
}
try {
  for(const [name,type] of [['chromium',chromium],['webkit',webkit]]) {
    const browser=await type.launch(name==='chromium'?{channel:'chrome',headless:true}:{headless:true});
    try {
      // Preview follows live system changes without changing the user's project or input.
      const preview=await browser.newContext({viewport:{width:1440,height:900},colorScheme:'light'});
      const page=await preview.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
      await page.goto(url);await page.waitForFunction(()=>window.__musicTest);await assertTheme(page,'light');
      const before=await page.evaluate(()=>window.__musicTest.project());
      await page.locator('[data-action="library"]').click();
      const input=await page.locator('input[type="search"]').count()?page.locator('input[type="search"]').first():page.locator('.library input').first();
      await input.fill('piano');
      await page.evaluate(()=>{window.__themeRoot=document.querySelector('#app').firstElementChild;window.__themeInput=document.querySelector('.library input');});
      for(const mode of ['dark','light','dark']) {
        await page.emulateMedia({colorScheme:mode});await assertTheme(page,mode);
        assert.deepEqual(await page.evaluate(()=>window.__musicTest.project()),before);
        assert.ok(await page.evaluate(()=>window.__themeRoot===document.querySelector('#app').firstElementChild&&window.__themeInput===document.querySelector('.library input')));
        assert.equal(await input.inputValue(),'piano');
      }
      await page.emulateMedia({contrast:'more'});await assertTheme(page,'dark');
      await page.emulateMedia({contrast:'no-preference'});
      await page.screenshot({path:`.test-output/${name}-studio-dark-library.png`});
      await preview.close();
      // Host brightness has priority even if the OS has the opposite appearance.
      for(const width of [1440,390]) {
        const context=await browser.newContext({viewport:{width,height:width===1440?900:844},colorScheme:'light'});
        await context.addInitScript(({fixture,width})=>{
          const snapshot={project:fixture,projects:[{id:fixture.id,name:fixture.name,revision:fixture.revision}],commands:[],receipts:[]};
          window.__hostSnapshot=snapshot;window.__hostRequests=[];
          window.MusicHost={getTheme:()=>width===1440?new Promise(resolve=>{window.__resolveInitialTheme=resolve;}):Promise.resolve({brightness:'dark'}),request:async value=>{window.__hostRequests.push(value.action);return value.action==='sync'?{}:snapshot;},saveFile:async()=>''};
        },{fixture,width});
        const hosted=await context.newPage();hosted.on('pageerror',e=>errors.push(String(e)));
        await hosted.goto(url);await hosted.waitForFunction(()=>document.querySelector('[data-canvas]')?.width>1);
        if(width===1440) {
          await hosted.evaluate(()=>{window.MusicStudioTheme.apply({brightness:'dark'});window.__resolveInitialTheme({brightness:'light'});});
        }
        await assertTheme(hosted,'dark');
        await hosted.evaluate(()=>window.MusicStudioTheme.apply({brightness:'invalid'}));await assertTheme(hosted,'dark');
        await hosted.locator('[data-region-track="midi_t4"]').first().click();
        await hosted.waitForFunction(()=>document.querySelector('#piano')?.width>1);
        const before=await hosted.evaluate(()=>{
          window.__themePiano=document.querySelector('#piano');
          return {project:window.__hostSnapshot.project,title:document.querySelector('#editor-title').textContent,scroll:document.querySelector('#timeline-scroll').scrollTop};
        });
        async function pianoPixels(){return hosted.locator('#piano').evaluate(c=>{const p=c.getContext('2d').getImageData(c.width-2,0,1,c.height).data;let total=0;for(let i=0;i<p.length;i+=4)total+=p[i]+p[i+1]+p[i+2];return total/(c.height*3);});}
        const dark=await pianoPixels();assert.ok(dark<100);
        await hosted.screenshot({path:`.test-output/${name}-studio-dark-piano-${width}.png`});
        await hosted.evaluate(()=>window.MusicStudioTheme.apply({brightness:'light'}));await assertTheme(hosted,'light');
        await hosted.waitForFunction(()=>{const c=document.querySelector('#piano'),p=c.getContext('2d').getImageData(c.width-2,0,1,1).data;return p[0]>200;});
        assert.ok(await pianoPixels()>200);
        await hosted.emulateMedia({colorScheme:'dark'});await assertTheme(hosted,'light');
        assert.deepEqual(await hosted.evaluate(()=>({project:window.__hostSnapshot.project,title:document.querySelector('#editor-title').textContent,scroll:document.querySelector('#timeline-scroll').scrollTop})),before);
        assert.ok(await hosted.evaluate(()=>window.__themePiano===document.querySelector('#piano')));
        assert.ok(await hosted.evaluate(()=>window.__hostRequests.every(action=>['get','sync'].includes(action))));
        await hosted.locator('[data-action="project-menu"]').click();
        await hosted.evaluate(()=>window.MusicStudioTheme.apply({brightness:'dark'}));await assertTheme(hosted,'dark');
        assert.ok(await hosted.locator('.project-menu').isVisible());
        await hosted.screenshot({path:`.test-output/${name}-studio-dark-menu-${width}.png`});
        await hosted.locator('button[data-action="close-menu"]').click();
        await hosted.locator('[data-action="close-editor"]').first().click();
        await hosted.locator('[data-device="midi_t4"]').click();
        assert.ok(await hosted.locator('.preview-key.white').first().isVisible());
        const keys=await hosted.locator('.preview-key.white').first().evaluate(el=>{const s=getComputedStyle(el);return {bg:s.backgroundColor,fg:s.color};});
        assert.ok(luminance(keys.bg)<.1&&contrast(keys.bg,keys.fg)>4.5,JSON.stringify(keys));
        await hosted.screenshot({path:`.test-output/${name}-studio-dark-device-${width}.png`});
        console.log(`PASS ${name} ${width}: host override, live DOM/Canvas theme, editor and project preserved`);
        await context.close();
      }
      assert.deepEqual(errors,[]);console.log(`PASS ${name}: system theme, contrast, input preserved`);
    } finally {await browser.close();}
  }
} finally {await new Promise(resolve=>server.close(resolve));}
