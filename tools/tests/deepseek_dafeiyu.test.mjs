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

test('opening an existing pet updates its live state without resetting dragged coordinates or closing it', async () => {
  const old = globalThis.ToolPkg;
  const calls = [];
  globalThis.ToolPkg = {
    async readResource(key) { return `vfs:/pet/${key}`; },
    ui: { floatingWindows: {
      async capabilities() { return { supported: true, permissionGranted: true }; },
      async list() { return [{ id: 'dafeiyu' }, { id: 'unrelated' }]; },
      async update(id, options, state) { calls.push(['update', id, options, state]); return {id}; },
      async show(id) { calls.push(['show', id]); },
      async close() { throw Error('Must keep the live session'); },
    } },
  };
  try {
    await plugin.openPet({ ...plugin.DEFAULTS, width: 192, animated: false, clickThrough: true });
    assert.equal(calls.length, 2);
    const [, id, options, state] = calls[0];
    assert.equal(id, 'dafeiyu');
    assert.equal(options.width, 192);
    assert.equal(options.clickThrough, true);
    assert.equal(options.draggable, true);
    assert.equal(options.x, undefined);
    assert.equal(options.y, undefined);
    assert.equal(state.settings.animated, false);
    assert.deepEqual(calls[1], ['show', 'dafeiyu']);
  } finally { globalThis.ToolPkg = old; }
});

test('opening a new pet enables host dragging and supplies resources', async () => {
  const old = globalThis.ToolPkg;
  globalThis.ToolPkg = {
    async readResource(key) { return `vfs:/pet/${key}`; },
    ui: { floatingWindows: {
      async capabilities() { return { supported: true, permissionGranted: true }; },
      async list() { return []; },
      async open(request) { return request; },
    } },
  };
  try {
    const result = await plugin.openPet(plugin.DEFAULTS);
    assert.equal(result.route, plugin.PET_ROUTE);
    assert.equal(result.options.draggable, true);
    assert.equal(result.state.sprites.eat, 'vfs:/pet/dafeiyu_eat');
    for (const asset of manifest.resources) assert.ok(existsSync(new URL(`../../plugins/packages/external/deepseek_dafeiyu/${asset.path}`, import.meta.url)));
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

test('settings controls persist and update a live pet immediately, coalescing edits during an outstanding save', async () => {
  const old = { ToolPkg: globalThis.ToolPkg, PluginConfig: globalThis.PluginConfig };
  const states = new Map(), refs = new Map(), calls = [];
  const store = { settings: { ...plugin.DEFAULTS } };
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let flushes = 0;
  globalThis.PluginConfig = {
    async use() { return store; },
    async flush() { if (++flushes === 1) await gate; },
  };
  globalThis.ToolPkg = {
    ui: { floatingWindows: {
      async capabilities() { return { supported: true, permissionGranted: true, scope: 'system' }; },
      async list() { return [{ id: 'dafeiyu' }]; },
      async update(id, options, state) { calls.push({ id, options, state }); },
    } },
  };
  const ctx = {
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), value => states.set(key, value)];
    },
    useRef(key, initial) {
      if (!refs.has(key)) refs.set(key, { current: initial });
      return refs.get(key);
    },
    useEffect() {},
    MaterialTheme: { colorScheme: { onSurfaceVariant: '#000' } },
    UI: new Proxy({}, { get: (_, type) => (props, children = []) => ({ type, props, children }) }),
  };
  function* nodes(node) { yield node; for (const child of node.children) yield* nodes(child); }
  const settle = () => new Promise(resolve => setImmediate(resolve));
  try {
    await plugin.settingsPage(ctx).props.onLoad();
    let tree = plugin.settingsPage(ctx);
    const switches = [...nodes(tree)].filter(node => node.type === 'Switch');
    switches[0].props.onCheckedChange(false);
    await settle(); // First persistence is still pending while another control changes.
    switches[1].props.onCheckedChange(true);
    release();
    await settle(); await settle();
    assert.equal(calls.at(-1).state.settings.animated, false);
    assert.equal(calls.at(-1).state.settings.walk, true);
    assert.equal(store.settings.animated, false);
    assert.equal(store.settings.walk, true);
    assert.equal(calls.at(-1).options.x, undefined);
    tree = plugin.settingsPage(ctx);
    [...nodes(tree)].find(node => node.type === 'Slider').props.onValueChange(256);
    await new Promise(resolve => setTimeout(resolve, 100));
    assert.equal(calls.at(-1).options.width, 256);
    assert.equal(store.settings.width, 256);
    assert.match(states.get('status'), /实时应用/);
  } finally {
    release();
    Object.assign(globalThis, old);
  }
});
