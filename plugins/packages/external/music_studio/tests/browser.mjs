import { chromium } from "playwright";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile, mkdir, writeFile } from "node:fs/promises";
const server = createServer(async (_req, res) => { res.writeHead(200, { "Content-Type": "text/html;charset=utf-8" }); res.end(await readFile("resources/studio.html")); });
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const url = `http://127.0.0.1:${server.address().port}/?test=1`;
const browser = await chromium.launch({ channel: "chrome", headless: true });
const errors = [];
const skipLive = process.env.MUSIC_STUDIO_SKIP_LIVE === '1';
if (skipLive) console.warn('SKIP live AudioWorklet playback (MUSIC_STUDIO_SKIP_LIVE=1); UI and offline WASM/WAV checks still run.');
await mkdir(".test-output", { recursive: true });
try {
  for (const [width, height] of [[1440, 900], [1024, 768], [768, 1024], [390, 844], [320, 568], [844, 390]]) {
    const context = await browser.newContext({ viewport: { width, height } }); const page = await context.newPage(); page.on("pageerror", error => errors.push(String(error)));
    await page.goto(url); await page.waitForFunction(() => window.__musicTest);
    async function contained(label) {
      const metrics = await page.evaluate(() => ({ width: innerWidth, height: innerHeight, docW: document.documentElement.scrollWidth, docH: document.documentElement.scrollHeight, bodyW: document.body.scrollWidth, bodyH: document.body.scrollHeight, transport: document.querySelector(".transport").getBoundingClientRect().toJSON() }));
      assert.ok(metrics.docW <= width + 1 && metrics.bodyW <= width + 1, `${width} ${label}: horizontal page overflow ${JSON.stringify(metrics)}`);
      assert.ok(metrics.docH <= height + 1 && metrics.bodyH <= height + 1, `${width} ${label}: vertical page overflow ${JSON.stringify(metrics)}`);
      assert.ok(metrics.transport.bottom <= height + 1 && metrics.transport.top >= 0, `${width} ${label}: transport offscreen`);
    }
    await contained("initial");
    const ruler = await page.locator('#ruler').evaluate(el => ({width:el.clientWidth, childrenWidth:[...el.children].reduce((n,c)=>n+c.getBoundingClientRect().width,0)}));
    assert.ok(Math.abs(ruler.childrenWidth-ruler.width) < 2, 'ruler bars must align with fitted lanes');
    assert.equal(await page.locator(".side-panel").count(), 0); assert.equal(await page.locator("#editor-body").isVisible(), false); assert.equal(await page.locator("#overview-scope").count(), 0);
    await page.waitForFunction(() => { const c = document.querySelector("[data-canvas]"); return c && c.width > 1 && c.getContext("2d").getImageData(0, 0, 1, 1).data[3] > 0; });
    await page.screenshot({ path: `.test-output/layout-${width}x${height}.png` });
    await page.locator('[data-action="library"]').click(); assert.ok(await page.locator(".library").isVisible()); await contained("library");
    assert.equal(await page.locator(".library details").getAttribute("open"), null);
    await page.locator('[data-action="master"]').first().click(); assert.equal(await page.locator(".library").count(), 0); assert.ok(await page.locator(".master").isVisible());
    assert.equal(await page.locator(".master details[open]").count(), 0); await contained("master");
    await page.locator('[data-action="close-panel"]').last().click();
    assert.equal(await page.locator('.editor').count(), 0, 'no persistent bottom editor rail');
    await page.locator('[data-device="midi_t4"]').click();
    assert.ok(await page.locator('.synth-panel').isVisible()); assert.equal(await page.locator('.device-disclosure[open]').count(), 0);
    assert.equal(await page.locator('[data-tab="piano"]').count(), 0, 'device inspector must not contain unrelated piano tabs');
    if (width < 1200) assert.equal(await page.locator('.arrangement-pane').isVisible(), false);
    else {
      assert.ok(await page.locator('.arrangement-pane').isVisible());
      const boxes = await page.evaluate(() => ({ arrangement: document.querySelector('.arrangement-pane').getBoundingClientRect().toJSON(), editor: document.querySelector('.editor').getBoundingClientRect().toJSON() }));
      assert.ok(boxes.editor.left >= boxes.arrangement.right - 1, 'device belongs to the right, not below');
      assert.ok(Math.abs(boxes.editor.top - boxes.arrangement.top) < 1);
    }
    await contained('device');
    await page.screenshot({ path: `.test-output/device-${width}x${height}.png` });
    await page.locator('[data-action="close-editor"]').last().click();
    assert.equal(await page.locator('.editor').count(), 0);
    const clip = page.locator('[data-region-track="midi_t4"]').first();
    await clip.scrollIntoViewIfNeeded();
    const timelineBefore = await page.locator('#timeline-scroll').evaluate(el => ({ left: el.scrollLeft, top: el.scrollTop }));
    await clip.click();
    assert.ok(await page.locator('#piano').isVisible()); assert.equal(await page.locator('.arrangement-pane').isVisible(), false);
    const opened = await page.evaluate(() => window.__musicTest.editor());
    assert.equal(opened.selected, 'midi_t4'); assert.equal(opened.tab, 'piano'); assert.ok(opened.region.end > opened.region.start);
    assert.equal(opened.page, Math.floor(opened.region.start / 2), 'piano opens at the clicked region, not bar 1');
    if (width < 600) assert.equal(opened.visibleBars, 1, 'phone defaults to a usable one-bar note grid');
    const canvas = await page.locator('#piano').boundingBox(); assert.ok(canvas.height > 150, `piano should get a full workspace: ${canvas.height}`);
    await contained('piano');
    await page.waitForFunction(() => { const c = document.querySelector('#piano'); return c.width > 1 && c.getContext('2d').getImageData(0, 0, 1, 1).data[3] > 0; });
    await page.screenshot({ path: `.test-output/piano-${width}x${height}.png` });
    // Pointer editing must also stay local (including the narrowest phone viewport).
    const beforeClick = await page.evaluate(() => window.__musicTest.project());
    await page.locator('#piano').click({ position: { x: 42+(canvas.width-42)*.18, y: canvas.height-3 } });
    await page.waitForFunction(revision => window.__musicTest.project().revision > revision, beforeClick.revision);
    const afterClick = await page.evaluate(() => window.__musicTest.project());
    const oldIds = new Set(beforeClick.tracks.find(t => t.id === 'midi_t4').notes.map(n => n.id));
    const added = afterClick.tracks.find(t => t.id === 'midi_t4').notes.filter(n => !oldIds.has(n.id));
    assert.equal(added.length, 1); assert.ok(added[0].start >= opened.region.start && added[0].start+added[0].duration <= opened.region.end);
    assert.deepEqual(afterClick.tracks.filter(t => t.id !== 'midi_t4'), beforeClick.tracks.filter(t => t.id !== 'midi_t4'));
    await page.evaluate(async () => { const p = window.__musicTest.project(); await window.__musicTest.request({ action: 'undo', projectId: p.id, revision: p.revision }); });
    await page.waitForFunction(revision => window.__musicTest.project().revision > revision, afterClick.revision);
    assert.deepEqual((await page.evaluate(() => window.__musicTest.project())).tracks, beforeClick.tracks);
    if (width === 1440) {
      const original = await page.evaluate(() => window.__musicTest.project());
      const region = opened.region;
      const outside = p => p.tracks.find(t => t.id === region.trackId).notes.filter(n => n.start < region.start || n.start >= region.end);
      await page.locator('#pattern').selectOption('arpeggio');
      await page.locator('[data-action="generate"]').click();
      await page.waitForFunction(revision => window.__musicTest.project().revision > revision, original.revision);
      const edited = await page.evaluate(() => window.__musicTest.project());
      assert.deepEqual(outside(edited), outside(original), 'fill must preserve all other phrases');
      assert.deepEqual(edited.tracks.filter(t => t.id !== region.trackId), original.tracks.filter(t => t.id !== region.trackId));
      const generated = edited.tracks.find(t => t.id === region.trackId).notes.filter(n => !original.tracks.find(t => t.id === region.trackId).notes.some(old => old.id === n.id));
      assert.ok(generated.length > 0 && generated.every(n => n.start >= region.start && n.start+n.duration <= region.end));
      await page.evaluate(async () => { const p = window.__musicTest.project(); await window.__musicTest.request({ action: 'undo', projectId: p.id, revision: p.revision }); });
      await page.waitForFunction(revision => window.__musicTest.project().revision > revision, edited.revision);
      assert.deepEqual((await page.evaluate(() => window.__musicTest.project())).tracks, original.tracks);
    }
    await page.locator('[data-action="close-editor"]').last().click();
    assert.ok(await page.locator('.arrangement-pane').isVisible()); assert.equal(await page.locator('.editor').count(), 0);
    const timelineAfter = await page.locator('#timeline-scroll').evaluate(el => ({ left: el.scrollLeft, top: el.scrollTop }));
    assert.ok(Math.abs(timelineAfter.left-timelineBefore.left) < 2 && Math.abs(timelineAfter.top-timelineBefore.top) < 2, 'return must preserve timeline position');
    assert.equal(await page.evaluate(() => document.activeElement.dataset.region), opened.region.key, 'keyboard focus returns to the phrase');
    await page.locator('[data-action="project-menu"]').click(); await page.locator('[data-action="settings"]').click(); assert.ok(await page.locator("#setting-name").isVisible()); assert.equal(await page.locator(".project-menu").count(), 0); await contained("settings");
    await page.keyboard.press("Escape");
    await page.locator('[data-action="scope"]').click(); assert.ok(await page.locator("#overview-scope").isVisible()); await contained("scope");
    if (width === 1440 && !skipLive) {
      await page.locator('[data-action="play"]').click(); await page.waitForFunction(() => window.__musicTest.state().playing, null, { timeout: 10000 });
      assert.equal((await page.evaluate(() => window.__musicTest.state())).playing, true);
      await page.locator('[data-action="library"]').click(); assert.equal((await page.evaluate(() => window.__musicTest.state())).playing, true);
      await page.locator('[data-action="close-panel"]').last().click(); await page.locator('[data-action="stop"]').click();
      // Seek into a later MIDI block and verify real stereo analysers, then cross short loop boundaries.
      await page.locator('#section-jump').selectOption('64');
      await page.locator('[data-action="play"]').click(); await page.waitForTimeout(900);
      const live = await page.evaluate(() => window.__musicTest.state());
      assert.ok(live.beat > 64 && live.beat < 68 && live.stereo.every(x => x > 0.0001)); assert.equal(live.dropped, 0);
      await page.locator('[data-action="stop"]').click();
      await page.evaluate(async () => { const p = window.__musicTest.project(); await window.__musicTest.request({action:'batch',projectId:p.id,revision:p.revision,operations:[{type:'project.set',patch:{loop:{enabled:true,start:64,end:66}}}]}); });
      await page.waitForFunction(() => window.__musicTest.project().loop.enabled);
      await page.locator('[data-action="play"]').click(); await page.waitForTimeout(850);
      const loopBeats = []; for (let i=0;i<12;i++) { loopBeats.push((await page.evaluate(() => window.__musicTest.state())).beat); await page.waitForTimeout(90); }
      assert.ok(loopBeats.every(b => b >= 64 && b < 66)); assert.ok(Math.max(...loopBeats)-Math.min(...loopBeats) > 1, 'loop playhead must not freeze');
      await page.locator('[data-action="stop"]').click();
      await page.evaluate(async () => { const p = window.__musicTest.project(); await window.__musicTest.request({action:'batch',projectId:p.id,revision:p.revision,operations:[{type:'project.set',patch:{loop:{enabled:false,start:0,end:320}}}]}); });
      await page.waitForFunction(() => !window.__musicTest.project().loop.enabled);
    }
    if (width === 1440) {
      const renderStart = Date.now();
      const rendered = await page.evaluate(() => window.__musicTest.render(true));
      await writeFile('.test-output/midi-demo.wav', Buffer.from(rendered.audio, 'base64')); delete rendered.audio;
      assert.ok(rendered.peak > 0.1 && rendered.peak < 0.99 && rendered.bytes > 100000);
      assert.equal(rendered.dropped, 0); assert.ok(Math.abs(rendered.duration - (320 * 631580 / 1e6 + 3.2)) < 0.001);
      assert.ok(rendered.analysis.rms.every(x => x > 0.01));
      assert.ok(rendered.analysis.sideRms > 0.002 && rendered.analysis.correlation < 0.999);
      assert.ok(rendered.analysis.midRms > rendered.analysis.sideRms, 'mono fold-down must preserve body');
      assert.equal(rendered.analysis.sections.length, 10);
      assert.ok(rendered.analysis.sections.every(s => s.rms > .005), 'all MIDI navigation blocks must contain audible signal');
      console.log("WAV render verified", rendered, 'render seconds', (Date.now()-renderStart)/1000);
      await writeFile('.test-output/midi-demo-analysis.json', JSON.stringify(rendered, null, 2));
      // Effects parameters are collapsed and can still be edited after disclosure.
      await page.locator('[data-effects="midi_t4"]').click(); assert.equal(await page.locator(".fx-details[open]").count(), 0);
      await page.locator(".fx-details summary").first().click(); assert.ok(await page.locator('.fx-details input').first().isVisible());
      await page.locator('[data-action="close-editor"]').last().click();
      // AI/model updates preserve open state, selected tab, and timeline scroll.
      await page.locator('[data-action="zoom-in"]').click(); await page.locator('#timeline-scroll').evaluate(el => { el.scrollLeft = 100; });
      await page.evaluate(async () => { const p = window.__musicTest.project(); await window.__musicTest.request({ action: "batch", projectId: p.id, revision: p.revision, operations: [{ type: "project.set", patch: { name: "AI updated" } }] }); });
      await page.waitForFunction(() => document.querySelector('#session-name').textContent === 'AI updated');
      assert.equal(await page.locator('#editor-body').isVisible(), false);
    }
    // Sound-design controls are visible and persist real edits on every viewport.
    await page.locator('[data-device="midi_t4"]').click();
    assert.equal(await page.locator('.oscillator-card').count(), 2);
    assert.ok(await page.getByLabel('A 波形', { exact: true }).isVisible());
    assert.ok(await page.getByLabel('B 波形', { exact: true }).isVisible());
    const originalSynth = await page.evaluate(() => window.__musicTest.project().tracks.find(t => t.id === 'midi_t4').synth);
    await page.getByLabel('B 波形', { exact: true }).selectOption('square');
    await page.waitForFunction(() => window.__musicTest.project().tracks.find(t => t.id === 'midi_t4').synth.waveB === 'square');
    for (const [key, value] of [['unison', 8], ['unisonB', 7], ['subLevel', .5], ['haasMs', -12.5], ['bassMono', 150]]) {
      const slider = page.locator(`[data-param="${key}"][data-group="synth"]`);
      await slider.evaluate((el, value) => { el.value = String(value); el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); }, value);
      await page.waitForFunction(({ key, value }) => window.__musicTest.project().tracks.find(t => t.id === 'midi_t4').synth[key] === value, { key, value });
    }
    await contained('sound design');
    const synthOverflow = await page.locator('.synth-panel').evaluate(el => ({ width: el.clientWidth, scroll: el.scrollWidth }));
    assert.ok(synthOverflow.scroll <= synthOverflow.width + 1, `synth overflow ${JSON.stringify(synthOverflow)}`);
    await page.locator('#editor-body').evaluate(el => { el.scrollTop = 0; });
    await page.screenshot({ path: `.test-output/synth-${width}x${height}.png` });
    await page.reload(); await page.waitForFunction(() => window.__musicTest);
    const saved = await page.evaluate(() => window.__musicTest.project().tracks.find(t => t.id === 'midi_t4').synth);
    assert.equal(saved.waveB, 'square'); assert.equal(saved.unison, 8); assert.equal(saved.unisonB, 7); assert.equal(saved.haasMs, -12.5); assert.equal(saved.bassMono, 150);
    await page.evaluate(async synth => { const p = window.__musicTest.project(); await window.__musicTest.request({ action: 'batch', projectId: p.id, revision: p.revision, operations: [{ type: 'synth.set', trackId: 'midi_t4', patch: synth }] }); }, originalSynth);
    await page.waitForFunction(() => window.__musicTest.project().tracks.find(t => t.id === 'midi_t4').synth.unison !== 8);
    await page.locator('[data-region-track="midi_t8"]').first().click();
    assert.ok(await page.locator('#piano').isVisible());
    await page.locator('[data-tab="drums"]').click();
    assert.ok(await page.locator('.drum-grid').isVisible());
    const drumContext = await page.evaluate(() => window.__musicTest.editor());
    assert.equal(drumContext.selected, 'midi_t8'); assert.ok(drumContext.region);
    await contained('drum phrase');
    await page.keyboard.press('Escape'); assert.equal(await page.locator('.editor').count(), 0);
    await page.locator('[data-action="help"]').click(); assert.ok(await page.locator('.help-panel').isVisible());
    await page.keyboard.press('Escape'); assert.equal(await page.locator('.editor').count(), 0);
    console.log(`PASS responsive DAW ${width}×${height}`); await context.close();
  }
  assert.deepEqual(errors, []); console.log("No browser runtime errors.");
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
