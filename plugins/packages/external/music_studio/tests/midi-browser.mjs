import { chromium, webkit } from 'playwright';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
const html = await readFile('resources/studio.html');
const server = createServer((_req, res) => { res.writeHead(200, { 'Content-Type': 'text/html;charset=utf-8' }); res.end(html); });
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/?test=1`;
try {
  await mkdir('.test-output', { recursive: true });
  for (const [engine, type] of [['chromium', chromium], ['webkit', webkit]]) {
    if (process.env.MUSIC_STUDIO_BROWSER && process.env.MUSIC_STUDIO_BROWSER !== engine) continue;
    const browser = await type.launch(engine === 'chromium' ? { channel: 'chrome', headless: true } : { headless: true });
    try {
      for (const [width, height] of [[1440, 900], [390, 844]]) {
        const context = await browser.newContext({ viewport: { width, height } }), page = await context.newPage(), errors = [];
        page.on('pageerror', e => errors.push(String(e)));
        await page.goto(url); await page.waitForFunction(() => window.__musicTest);
        const original = await page.evaluate(() => window.__musicTest.project());
        assert.equal(original.name, 'THE WORLD REVOLVING'); assert.equal(original.tracks.length, 19);
        assert.equal(original.tracks.reduce((n, t) => n + t.notes.length, 0), 11410);
        await page.screenshot({ path: `.test-output/${engine}-midi-default-${width}.png` });
        await page.locator('[data-action="library"]').click();
        await page.locator('.library .panel-disclosure summary').click();
        assert.equal(await page.locator('[data-template]').count(), 2);
        assert.ok(await page.locator('[data-template="the-world-revolving"]').isVisible());
        assert.ok(await page.locator('[data-template="field-of-hopes-and-dreams"]').isVisible());
        await page.locator('[data-action="close-panel"]').last().click();
        await page.locator('[data-action="project-menu"]').click();
        assert.ok(await page.getByText('新建示范工程', { exact: true }).isVisible());
        assert.equal(await page.locator('.project-menu [data-template]').count(), 2);
        assert.ok(await page.locator('.project-menu [data-action="new"]').isVisible());
        await page.screenshot({ path: `.test-output/${engine}-new-demo-menu-${width}.png` });
        await page.locator('.project-menu [data-template="field-of-hopes-and-dreams"]').click();
        await page.waitForFunction(() => window.__musicTest.project().name === 'The Field of Hopes and Dreams');
        assert.equal(await page.locator('.project-menu').count(), 0, 'creating a demo closes the project menu');
        const next = await page.evaluate(() => window.__musicTest.project());
        assert.equal(next.tracks.length, 17); assert.equal(next.tracks.reduce((n, t) => n + t.notes.length, 0), 5275);
        const snapshot = await page.evaluate(() => window.__musicTest.snapshot());
        assert.equal(snapshot.projects.length, 2); assert.ok(snapshot.projects.some(p => p.id === original.id));
        await page.reload(); await page.waitForFunction(() => window.__musicTest);
        assert.deepEqual(await page.evaluate(() => window.__musicTest.project()), next, 'selected MIDI persists on restart');
        const bounds = await page.evaluate(() => ({ w: innerWidth, h: innerHeight, sw: document.documentElement.scrollWidth, sh: document.documentElement.scrollHeight }));
        assert.ok(bounds.sw <= bounds.w + 1 && bounds.sh <= bounds.h + 1, JSON.stringify(bounds));
        await page.screenshot({ path: `.test-output/${engine}-field-of-hopes-${width}.png` });
        if (engine === 'chromium' && width === 1440) {
          const result = await page.evaluate(() => window.__musicTest.render(true));
          await writeFile('.test-output/field-of-hopes-and-dreams.wav', Buffer.from(result.audio, 'base64')); delete result.audio;
          assert.equal(result.dropped, 0); assert.ok(result.peak > .1 && result.peak < .99);
          assert.ok(Math.abs(result.duration - (336 * 480001 / 1e6 + 3.2)) < 1e-6);
          assert.ok(result.analysis.rms.every(v => v > .01));
          assert.ok(result.analysis.sections.every(s => s.rms > .002), 'complete arrangement must render through its final block');
          await writeFile('.test-output/field-of-hopes-analysis.json', JSON.stringify(result, null, 2));
          console.log('PASS second MIDI complete WAV:', result);
        }
        await page.locator('[data-action="project-menu"]').click();
        await page.locator('.project-menu [data-template="the-world-revolving"]').click();
        await page.waitForFunction(() => window.__musicTest.project().name === 'THE WORLD REVOLVING', null, { timeout: 5000 }).catch(async e => { console.error(await page.locator('.notice').allTextContents()); throw e; });
        assert.equal(await page.locator('.project-menu').count(), 0);
        const final = await page.evaluate(() => window.__musicTest.snapshot());
        assert.equal(final.projects.length, 3); assert.ok(final.projects.some(p => p.id === original.id));
        await page.reload(); await page.waitForFunction(() => window.__musicTest);
        assert.equal((await page.evaluate(() => window.__musicTest.snapshot())).projects.length, 3, 'large demo catalog must survive reload');
        if (width === 1440) {
          const legacyContext = await browser.newContext(), legacyPage = await legacyContext.newPage();
          await legacyPage.addInitScript(store => {
            if (!sessionStorage.getItem('migration-seeded')) {
              localStorage.setItem('operit.music.studio.preview.v1', JSON.stringify(store));
              sessionStorage.setItem('migration-seeded', '1');
            }
          }, { version: 1, activeId: original.id, projects: [original] });
          await legacyPage.goto(url); await legacyPage.waitForFunction(() => window.__musicTest);
          assert.deepEqual(await legacyPage.evaluate(() => window.__musicTest.project()), original, 'legacy preview data must open unchanged');
          await legacyPage.evaluate(async () => {
            const p = window.__musicTest.project();
            await window.__musicTest.request({ action: 'batch', projectId: p.id, revision: p.revision, operations: [{ type: 'project.set', patch: { name: 'Migrated session' } }] });
          });
          assert.equal(await legacyPage.evaluate(() => localStorage.getItem('operit.music.studio.preview.v1')), null);
          await legacyPage.reload(); await legacyPage.waitForFunction(() => window.__musicTest);
          const migrated = await legacyPage.evaluate(() => window.__musicTest.project());
          assert.equal(migrated.name, 'Migrated session'); assert.deepEqual(migrated.tracks, original.tracks);
          await legacyContext.close();
        }
        assert.deepEqual(errors, []); await context.close();
        console.log(`PASS ${engine} ${width}: two templates, all notes, persistence and no overflow`);
      }
    } finally { await browser.close(); }
  }
} finally { await new Promise(resolve => server.close(resolve)); }
