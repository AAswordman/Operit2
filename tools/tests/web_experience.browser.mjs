import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { mkdir, stat, writeFile } from 'node:fs/promises';
import { resolve, relative, extname, sep } from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.OPERIT_PLAYWRIGHT_MODULE ?? resolve('.ci-tools/browser/node_modules/playwright'));
const artifacts = resolve(process.env.OPERIT_BROWSER_ARTIFACTS ?? '.ci-artifacts/web-experience');
const webRoot = resolve(process.argv[2] ?? 'apps/flutter/app/build/web');
const contentTypes = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.ttf': 'font/ttf', '.otf': 'font/otf', '.woff2': 'font/woff2' };

/** Serves the actual build output with the runtime's required browser isolation headers. */
async function localServer() {
  const server = createServer(async (request, response) => {
    try {
      const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      let path = resolve(webRoot, `.${pathname}`);
      const local = relative(webRoot, path);
      if (local === '..' || local.startsWith(`..${sep}`)) {
        response.writeHead(403).end();
        return;
      }
      if ((await stat(path)).isDirectory()) path = resolve(path, 'index.html');
      response.writeHead(200, {
        'Content-Type': contentTypes[extname(path)] ?? 'application/octet-stream',
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
        'Cross-Origin-Resource-Policy': 'same-origin',
      });
      createReadStream(path).pipe(response);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  return { server, base: `http://127.0.0.1:${server.address().port}` };
}

/** Confirms the real application is interactive after any published announcement is acknowledged. */
async function waitForApplication(page) {
  await page.waitForFunction(() => {
    const text = document.body.innerText;
    return text.includes('Navigation') || text.includes('我知道了');
  }, null, { timeout: 120_000 });
  const announcement = page.getByRole('button', { name: /^我知道了/ });
  if (await announcement.count() > 0) {
    await page.getByRole('button', { name: '我知道了', exact: true }).click({ timeout: 60_000 });
  }
  await page.waitForFunction(() => {
    const text = document.body.innerText;
    return text.includes('Navigation') && text.includes('展开输入框');
  }, null, { timeout: 120_000 });
  await page.keyboard.press('Tab');
}

/** Runs onboarding, real plugin calls, configured reload, and the theme renderer/export. */
async function main() {
  await mkdir(artifacts, { recursive: true });
  const serving = process.env.OPERIT_WEB_BASE_URL ? null : await localServer();
  const base = process.env.OPERIT_WEB_BASE_URL ?? serving.base;
  const browser = await chromium.launch({ headless: true, ...(process.env.OPERIT_BROWSER_EXECUTABLE ? { executablePath: process.env.OPERIT_BROWSER_EXECUTABLE } : {}) });
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, locale: 'zh-CN', acceptDownloads: true });
  const messages = [];
  const errors = [];
  const badResponses = [];
  const cdnRequests = [];
  const report = { base, messages, errors, badResponses, cdnRequests };
  await context.route(/https:\/\/(?:www\.gstatic\.com\/flutter-canvaskit|fonts\.gstatic\.com)\//, route => {
    cdnRequests.push(route.request().url());
    return route.abort();
  });
  /** Records page and worker errors as deployment failures. */
  function observe(page) {
    page.on('pageerror', error => errors.push(String(error)));
    page.on('console', message => {
      messages.push({ type: message.type(), text: message.text() });
      if (message.text().startsWith('Browser verification RPC:')) console.log(message.text());
      if (message.type() === 'error') errors.push(message.text());
    });
    page.on('response', response => {
      if (response.status() >= 400) badResponses.push({ url: response.url(), status: response.status() });
    });
  }
  const page = await context.newPage();
  observe(page);
  try {
    await page.goto(new URL('/', base).href, { timeout: 120_000 });
    await page.evaluate(() => globalThis.__operitRuntimeReady);
    await page.waitForSelector('flutter-view', { timeout: 60_000 });
    await page.locator('flt-semantics-placeholder').evaluate(element => element.click());
    await page.getByRole('button', { name: '开始', exact: true }).click();
    await page.getByRole('button', { name: /我已阅读|同意|继续/ }).click({ timeout: 20_000 });
    await page.getByRole('button', { name: '确认', exact: true }).click();
    for (let index = 0; index < 10; index++) await page.keyboard.press('Tab');
    await page.waitForFunction(() => document.body.innerText.includes('启动方式'), null, { timeout: 120_000 });
    await page.getByRole('button', { name: '跳过', exact: true }).click();
    await waitForApplication(page);
    console.log('Configured application is interactive; checking the real plugin registry.');
    report.registry = await page.evaluate(async () => {
      let sequence = 0;
      /** Calls the exact compact Core ABI used by generated Dart proxy clients. */
      async function call(target, method, args = {}) {
        console.log(`Browser verification RPC: ${target}.${method}`);
        let timer;
        const response = await Promise.race([
          globalThis.__operitRuntime.call(globalThis.MessagePack.encode([`browser-test-${++sequence}`, target, method, args])),
          new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`Core call timed out: ${target}.${method}`)), 60_000); }),
        ]).finally(() => clearTimeout(timer));
        const result = globalThis.MessagePack.decode(response);
        if (result[0] !== 0) throw new Error(JSON.stringify(result));
        return result[1];
      }
      const target = 'core/application.packageManager';
      await call('core/application', 'initMcpPlugins');
      // Registry methods require canonical readiness; the UI starts and awaits the same owner.
      const issues = await call(target, 'getToolPkgLoadIssues');
      const packages = Object.keys(await call(target, 'getAvailablePackages'));
      const apis = await call(target, 'getToolPkgPublicApis', { packageName: 'com.operit.character_cards' });
      const characters = await call(target, 'invokeToolPkgPublicApi', { packageName: 'com.operit.character_cards', methodName: 'character.list', payload: {} });
      return { issues, packages, apis, characters };
    });
    assert.deepEqual(report.registry.issues, []);
    for (const name of ['com.operit.character_cards', 'com.operit.workflow', 'character_memory']) assert.ok(report.registry.packages.includes(name), `Missing package ${name}`);
    assert.ok(report.registry.apis.includes('character.list'));
    assert.ok(Array.isArray(report.registry.characters));
    await waitForApplication(page);
    report.appText = await page.locator('body').innerText();
    assert.ok(!report.appText.includes('Operit2 encountered an error'));
    await page.screenshot({ path: resolve(artifacts, 'configured-app.png'), timeout: 30_000 });
    report.storageOwnership = await page.evaluate(async () => {
      const locks = await navigator.locks.query();
      return locks.held.filter(lock => lock.name.endsWith('.runtime.storage'));
    });
    assert.equal(report.storageOwnership.length, 1);
    assert.equal(report.storageOwnership[0].mode, 'exclusive');
    console.log('Plugin invocation passed; checking configured reload.');
    report.reloads = [];
    for (let index = 0; index < 3; index++) {
      await page.reload();
      await page.evaluate(() => globalThis.__operitRuntimeReady);
      await page.waitForSelector('flutter-view', { timeout: 60_000 });
      await page.locator('flt-semantics-placeholder').evaluate(element => element.click());
      await waitForApplication(page);
      const text = await page.locator('body').innerText();
      assert.ok(!text.includes('Operit2 encountered an error'));
      assert.equal(await page.evaluate(() => crossOriginIsolated), true);
      report.reloads.push(text);
      console.log(`Configured reload ${index + 1}/3 passed.`);
    }
    report.reloadText = report.reloads.at(-1);
    await page.screenshot({ path: resolve(artifacts, 'configured-reload.png'), timeout: 30_000 });
    console.log('Configured reload is interactive; checking theme preview and export.');
    const theme = await context.newPage();
    observe(theme);
    await theme.goto(new URL('/theme/', base).href, { timeout: 120_000 });
    await theme.waitForFunction(() => document.getElementById('receipt')?.textContent?.trim().startsWith('{'), null, { timeout: 120_000 });
    report.themeReceipt = JSON.parse(await theme.locator('#receipt').textContent());
    const download = theme.waitForEvent('download');
    await theme.getByRole('button', { name: '导出 ToolPkg', exact: true }).click();
    await (await download).saveAs(resolve(artifacts, 'theme.toolpkg'));
    assert.ok((await stat(resolve(artifacts, 'theme.toolpkg'))).size > 1000);
    await theme.screenshot({ path: resolve(artifacts, 'theme-preview.png'), timeout: 30_000 });
    await theme.close();
    console.log('Theme export passed; checking the standalone data rescue runtime.');
    await page.goto(new URL('/data-rescue.html', base).href, { timeout: 120_000 });
    await page.evaluate(() => globalThis.__operitDataRescueReady);
    report.dataRescue = await page.evaluate(async () => {
      let timer;
      const response = await Promise.race([
        globalThis.__operitDataRescue(JSON.stringify({ operation: 'configure' })),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Maintenance ownership handoff timed out')), 60_000); }),
      ]).finally(() => clearTimeout(timer));
      return JSON.parse(response);
    });
    assert.equal(report.dataRescue.ok, true, JSON.stringify(report.dataRescue));
    const closed = await page.evaluate(async () => JSON.parse(await globalThis.__operitDataRescue(JSON.stringify({ operation: 'close' }))));
    assert.equal(closed.ok, true, JSON.stringify(closed));
    assert.deepEqual(cdnRequests, []);
    assert.deepEqual(badResponses, []);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ packages: report.registry.packages, publicApiCount: report.registry.apis.length, characters: report.registry.characters.length, themeReceipt: report.themeReceipt, errors, badResponses, cdnRequests }, null, 2));
  } catch (error) {
    report.failure = String(error);
    await page.screenshot({ path: resolve(artifacts, 'failure.png'), timeout: 10_000 }).catch(() => {});
    throw error;
  } finally {
    await writeFile(resolve(artifacts, 'report.json'), JSON.stringify(report, null, 2));
    await browser.close();
    if (serving) await new Promise(resolve => serving.server.close(resolve));
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; });
