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
    assert.match([...nodes(plugin.settingsPage(ctx))].find(node => node.type === 'Image').props.uri, /dafeiyu_still$/);
    [...nodes(plugin.settingsPage(ctx))].find(node => node.type === 'Switch').props.onCheckedChange(true);
    await settle(); await settle();
    tree = plugin.settingsPage(ctx);
    [...nodes(tree)].find(node => node.type === 'TextButton' && node.props.text === '碎碎念').props.onClick();
    tree = plugin.settingsPage(ctx);
    const preview = [...nodes(tree)].find(node => node.type === 'Image');
    assert.match(preview.props.uri, /dafeiyu_talk$/);
    [...nodes(tree)].find(node => node.type === 'TextButton' && node.props.text === '碎碎念').props.onClick();
    assert.notEqual([...nodes(plugin.settingsPage(ctx))].find(node => node.type === 'Image').props.animationKey, preview.props.animationKey);
  } finally {
    release();
    Object.assign(globalThis, old);
  }
});

test('interactions play the full real clip, replay from the beginning, and preserve playback during resizing', () => {
  const old = { setTimeout: globalThis.setTimeout, clearTimeout: globalThis.clearTimeout };
  const states = new Map(), refs = new Map(), effects = new Map(), timers = new Map();
  let timerId = 0;
  globalThis.setTimeout = (callback, delay) => {
    const id = ++timerId; timers.set(id, { callback, delay }); return id;
  };
  globalThis.clearTimeout = id => timers.delete(id);
  const images = Object.fromEntries(['idle', 'happy', 'eat', 'walk', 'talk', 'still'].map(name => [name, `/pet/${name}`]));
  states.set('settings', { ...plugin.DEFAULTS });
  states.set('sprites', images);
  const ctx = {
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), value => states.set(key, value)];
    },
    useRef(key, initial) {
      if (!refs.has(key)) refs.set(key, { current: initial });
      return refs.get(key);
    },
    useEffect(key, setup, deps) {
      const previous = effects.get(key);
      if (previous && JSON.stringify(previous.deps) === JSON.stringify(deps)) return;
      previous?.cleanup();
      effects.set(key, { deps, cleanup: setup() });
    },
    Modifier: { tapGestures: callbacks => callbacks },
    UI: new Proxy({}, { get: (_, type) => (props, children = []) => ({ type, props, children }) }),
  };
  const image = tree => tree.children.find(node => node.type === 'Image');
  const info = JSON.parse(readFileSync(new URL('../../plugins/packages/external/deepseek_dafeiyu/resources/animation-info.json', import.meta.url)));
  try {
    let tree = plugin.pet(ctx);
    for (const [handler, action] of [['onTap', 'happy'], ['onDoubleTap', 'eat'], ['onLongPress', 'talk']]) {
      image(tree).props.modifier[handler]();
      tree = plugin.pet(ctx);
      const first = image(tree).props;
      assert.equal(first.uri, images[action]);
      assert.equal(timers.size, 1);
      assert.equal([...timers.values()][0].delay, info.actions[action].duration_ms);
      const reset = [...timers.values()][0].callback;
      states.set('settings', { ...plugin.DEFAULTS, width: 256 });
      tree = plugin.pet(ctx);
      assert.equal(image(tree).props.animationKey, first.animationKey);
      assert.equal([...timers.values()][0].callback, reset);
      image(tree).props.modifier[handler]();
      tree = plugin.pet(ctx);
      assert.notEqual(image(tree).props.animationKey, first.animationKey);
      assert.equal(timers.size, 1, 'Replay must cancel the previous reset');
    }
    [...timers.values()][0].callback();
    tree = plugin.pet(ctx);
    assert.equal(image(tree).props.uri, images.idle);
    states.set('settings', { ...plugin.DEFAULTS, walk: true });
    tree = plugin.pet(ctx);
    assert.equal(image(tree).props.uri, images.walk);
    states.set('settings', { ...plugin.DEFAULTS, animated: false });
    assert.equal(image(plugin.pet(ctx)).props.uri, images.still);
    assert.equal(manifest.resources.find(resource => resource.key === 'dafeiyu_talk').path, 'resources/talk.webp');
  } finally {
    for (const effect of effects.values()) effect.cleanup();
    Object.assign(globalThis, old);
  }
});
