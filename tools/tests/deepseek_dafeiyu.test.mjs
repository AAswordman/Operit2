import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync, existsSync } from 'node:fs';
import test from 'node:test';
const require = createRequire(import.meta.url);
const plugin = require('../../plugins/packages/external/deepseek_dafeiyu/dist/main.js');
const manifest = JSON.parse(readFileSync(new URL('../../plugins/packages/external/deepseek_dafeiyu/manifest.json', import.meta.url)));

test('pet settings reject invalid geometry before making host calls', async () => {
  const old = globalThis.ToolPkg;
  let calls = 0;
  globalThis.ToolPkg = { ui: { floatingWindows: { capabilities() { calls++; } } } };
  try {
    for (const patch of [{ width: NaN }, { width: 400 }, { x: Infinity }, { clickThrough: 'yes' }]) {
      await assert.rejects(plugin.openPet({ ...plugin.DEFAULTS, ...patch }));
    }
    assert.equal(calls, 0);
  } finally { globalThis.ToolPkg = old; }
});

test('opening reports missing permission without implicitly asking or closing another session', async () => {
  const old = globalThis.ToolPkg;
  const calls = [];
  globalThis.ToolPkg = { ui: { floatingWindows: { async capabilities() {
    calls.push('capabilities'); return { supported: true, permissionGranted: false };
  } } } };
  try {
    await assert.rejects(plugin.openPet(plugin.DEFAULTS), /授权/);
    assert.deepEqual(calls, ['capabilities']);
  } finally { globalThis.ToolPkg = old; }
});

test('opening replaces only the pet ID, and passes animation resources and settings to its live route', async () => {
  const old = globalThis.ToolPkg;
  const calls = [];
  globalThis.ToolPkg = {
    async readResource(key) { return `vfs:/pet/${key}`; },
    ui: { floatingWindows: {
      async capabilities() { return { supported: true, permissionGranted: true, scope: 'application' }; },
      async list() { return [{ id: 'dafeiyu' }, { id: 'unrelated' }]; },
      async close(id) { calls.push(['close', id]); },
      async open(request) { calls.push(['open', request]); return request; },
    } },
  };
  try {
    const result = await plugin.openPet({ ...plugin.DEFAULTS, width: 192, clickThrough: true });
    assert.deepEqual(calls[0], ['close', 'dafeiyu']);
    assert.equal(calls.length, 2);
    assert.equal(result.route, plugin.PET_ROUTE);
    assert.equal(result.options.width, 192);
    assert.equal(result.options.clickThrough, true);
    assert.equal(result.state.sprites.eat, 'vfs:/pet/dafeiyu_eat');
    assert.equal(result.state.settings.width, 192);
    for (const asset of manifest.resources) {
      assert.ok(existsSync(new URL(`../../plugins/packages/external/deepseek_dafeiyu/${asset.path}`, import.meta.url)));
    }
  } finally { globalThis.ToolPkg = old; }
});

test('registration creates separate settings and pet routes without creating a window', () => {
  const old = globalThis.ToolPkg;
  const routes = [], navigation = [];
  globalThis.ToolPkg = {
    registerUiRoute(route) { routes.push(route); },
    registerNavigationEntry(entry) { navigation.push(entry); },
  };
  try {
    assert.equal(plugin.registerToolPkg(), true);
    assert.deepEqual(routes.map(route => route.route), [plugin.SETTINGS_ROUTE, plugin.PET_ROUTE]);
    assert.equal(navigation[0].route, plugin.SETTINGS_ROUTE);
    assert.equal(navigation[0].surface, 'toolbox');
  } finally { globalThis.ToolPkg = old; }
});
