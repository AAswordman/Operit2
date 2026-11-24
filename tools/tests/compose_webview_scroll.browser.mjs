import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(new URL('../../plugins/packages/external/openui/package.json', import.meta.url));
const { chromium } = require('playwright');
const source = readFileSync(new URL('../../apps/flutter/app/lib/ui/features/packages/screens/ComposeWebViewScrollBridge.dart', import.meta.url), 'utf8');
const script = source.match(/r'''([\s\S]*?)'''/)[1];

/** Runs the production page bridge in Chromium with native DOM scrolling. */
test('nested page scrolling retains input until both page scroll boundaries', async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 600, height: 400 } });
    await page.setContent('<style>body{margin:0}#outer{height:200px;overflow:auto}#inner{height:100px;overflow:auto}#content{height:500px}#tail{height:700px}</style><div id="outer"><div id="inner"><div id="content"></div></div><div id="tail"></div></div>');
    await page.evaluate(() => {
      window.messages = [];
      window.__ComposeDslWebViewHostBridgeChannel__ = {
        /** Records one-way input without introducing an RPC response. */
        postMessage(raw) { window.messages.push(JSON.parse(raw)); },
      };
    });
    await page.addScriptTag({ content: script });
    await page.addScriptTag({ content: script });
    await page.mouse.move(40, 40);
    await page.mouse.wheel(0, 50);
    await page.waitForFunction(() => document.getElementById('inner').scrollTop === 50);
    assert.deepEqual(await page.evaluate(() => window.messages), []);
    await page.evaluate(() => { document.getElementById('inner').scrollTop = 400; });
    await page.mouse.wheel(0, 50);
    await page.waitForFunction(() => document.getElementById('outer').scrollTop > 0);
    assert.deepEqual(await page.evaluate(() => window.messages), []);
    await page.evaluate(() => { document.getElementById('outer').scrollTop = 600; });
    await page.mouse.wheel(0, 70);
    await page.waitForFunction(() => window.messages.length === 1);
    assert.equal(await page.evaluate(() => window.messages[0].payload.delta), 70);
    await page.evaluate(() => {
      document.getElementById('inner').scrollTop = 0;
      document.getElementById('outer').scrollTop = 0;
    });
    await page.mouse.wheel(0, -60);
    await page.waitForFunction(() => window.messages.length === 2);
    assert.equal(await page.evaluate(() => window.messages[1].payload.delta), -60);
    await page.mouse.wheel(80, 0);
    await page.evaluate(() => new Promise(requestAnimationFrame));
    assert.equal(await page.evaluate(() => window.messages.length), 2);
    await page.close();
  } finally {
    await browser.close();
  }
});

/** Exercises touch overflow, wheel units and frame coalescing with real DOM geometry. */
test('touch, partial wheel overflow and batched input preserve exact unused deltas', async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 400, height: 300 } });
    await page.setContent('<style>body{margin:0}#scroll{height:100px;overflow:auto;line-height:20px}#content{height:500px}</style><div id="scroll"><div id="content"></div></div>');
    await page.evaluate(() => {
      window.messages = [];
      window.__ComposeDslWebViewHostBridgeChannel__ = {
        /** Records native host delivery. */
        postMessage(raw) { window.messages.push(JSON.parse(raw)); },
      };
    });
    await page.addScriptTag({ content: script });
    const messages = await page.evaluate(async () => {
      const scroll = document.getElementById('scroll');
      const target = document.getElementById('content');
      scroll.scrollTop = 390;
      target.dispatchEvent(new WheelEvent('wheel', { deltaY: 40, bubbles: true, cancelable: true }));
      await new Promise(requestAnimationFrame);
      scroll.scrollTop = 400;
      target.dispatchEvent(new WheelEvent('wheel', { deltaY: 2, deltaMode: 1, bubbles: true, cancelable: true }));
      target.dispatchEvent(new WheelEvent('wheel', { deltaY: 5, bubbles: true, cancelable: true }));
      await new Promise(requestAnimationFrame);
      /** Dispatches one-finger page input in screen coordinates. */
      function touch(type, y) {
        const point = new Touch({ identifier: 1, target, clientY: y, screenY: y, screenX: 10 });
        target.dispatchEvent(new TouchEvent(type, { touches: type === 'touchend' ? [] : [point], bubbles: true, cancelable: true }));
      }
      touch('touchstart', 100);
      touch('touchmove', 70);
      touch('touchend', 70);
      return window.messages;
    });
    assert.deepEqual(messages.map(message => [message.payload.phase, message.payload.delta]), [
      ['wheel', 30], ['wheel', 45], ['start', 0], ['move', 30], ['end', 0],
    ]);
    assert.ok(messages.every(message => message.payload.viewportHeight === 300));
    await page.close();
  } finally {
    await browser.close();
  }
});
