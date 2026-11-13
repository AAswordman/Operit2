import assert from 'node:assert/strict';
import test from 'node:test';
import { loadModule, plain } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

/** Drives the production screen callbacks with retained Compose state and an isolated VFS. */
function screen(language = 'zh-CN') {
  const states = new Map(), refs = new Map(), files = new Map(), toasts = [], exports = [];
  let tree;
  const module = loadModule('../../external/worldbook/src/ui/regex_manager/index.ui.ts', {
    getLang: () => language,
    ToolPkg: { getConfigDir: () => '/config',
      async callDependency() { return []; } },
    Tools: { Files: {
      async exists(path) { return { exists: files.has(path) }; }, async mkdir() {},
      async read(path) { return { content: files.get(path) }; },
      async write(path, content) { files.set(path, content); },
    } },
  });
  const ctx = {
    Modifier: { onSizeChanged: onSizeChanged => ({ onSizeChanged }) },
    UI: new Proxy({}, { get: (_target, type) => (props, children = []) => ({ type, props, children }) }),
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), value => states.set(key, value)];
    },
    useRef(key, initial) { if (!refs.has(key)) refs.set(key, { current: initial }); return refs.get(key); },
    showToast(value) { toasts.push(value); },
    async saveFile(value) { exports.push(value); },
  };
  /** Re-renders the actual screen without discarding its retained form state. */
  function render() { tree = module.default(ctx); return tree; }
  /** Enumerates the actual screen tree to select stable action keys. */
  function nodes(node = tree) { return [node, ...node.children.flatMap(child => nodes(child))]; }
  /** Runs an event and drains its asynchronous save before rendering the next frame. */
  async function event(key, method = 'onClick', value) {
    const node = nodes().find(item => item.props.key === key);
    assert.ok(node, `Missing UI node: ${key}`);
    node.props[method](value);
    await new Promise(resolve => setImmediate(resolve));
    render();
  }
  render();
  return { files, toasts, exports, event, nodes, render,
    resize(width) { tree.props.modifier.onSizeChanged({ width, height: 800 }); render(); },
    setLanguage(value) { language = value; render(); },
    async load() { await tree.props.onLoad(); render(); },
    rules() { return JSON.parse(files.get('/config/regex_scripts.json')); } };
}

test('rule editor saves current fields, previews without writes, and exports stored rules', async () => {
  const ui = screen();
  await ui.load();
  await ui.event('new');
  await ui.event('name', 'onValueChange', 'Hide tags');
  await ui.event('find', 'onValueChange', '/hidden/g');
  await ui.event('replace', 'onValueChange', 'visible');
  ui.resize(1280);
  assert.equal(ui.nodes().find(node => node.props.key === 'regex-editor').type, 'Row');
  ui.resize(360);
  assert.equal(ui.nodes().find(node => node.props.key === 'regex-editor').type, 'Column');
  assert.equal(ui.nodes().find(node => node.props.key === 'replace').props.value, 'visible');
  await ui.event('test-source', 'onValueChange', 'hidden hidden');
  await ui.event('test');
  assert.ok(ui.nodes().some(node => node.props.text === '结果：visible visible'));
  assert.equal(ui.files.size, 0);
  await ui.event('save');
  const [saved] = ui.rules();
  assert.equal(saved.scriptName, 'Hide tags');
  assert.equal(saved.markdownOnly, true);
  await ui.event('export');
  assert.deepEqual(JSON.parse(ui.exports[0].content), [saved]);
  await ui.event(`edit-${saved.id}`);
  await ui.event('find', 'onValueChange', '/[/');
  await ui.event('save');
  assert.deepEqual(ui.rules(), [saved]);
  assert.ok(ui.toasts.some(text => /SyntaxError/.test(text)));
  assert.ok(ui.nodes().some(node => node.props.key === 'find' && node.props.value === '/[/'));
});

test('imports are atomic and list actions retain execution order and require explicit deletion', async () => {
  const ui = screen();
  await ui.load();
  await ui.event('open-import');
  const source = ['First', 'Second'].map(scriptName => ({ scriptName, findRegex: '/x/g', replaceString: 'y', placement: [2] }));
  await ui.event('import-json', 'onValueChange', JSON.stringify([...source, { findRegex: '/[/' }]));
  await ui.event('import');
  assert.equal(ui.files.size, 0);
  await ui.event('import-json', 'onValueChange', JSON.stringify(source));
  await ui.event('import');
  const [first, second] = ui.rules();
  await ui.event(`up-${second.id}`);
  assert.deepEqual(ui.rules().map(rule => rule.id), [second.id, first.id]);
  await ui.event(`toggle-${first.id}`);
  assert.equal(ui.rules()[1].disabled, true);
  await ui.event(`delete-${first.id}`);
  assert.equal(ui.rules().length, 2);
  await ui.event(`delete-${first.id}`);
  assert.deepEqual(plain(ui.rules()).map(rule => rule.id), [second.id]);
});

test('editor language changes cover labels, validation, previews and statuses without translating user data', async () => {
  const ui = screen('en-US');
  await ui.load();
  assert.ok(ui.nodes().some(node => node.props.text === 'Regex rules'));
  assert.ok(ui.nodes().some(node => node.props.text === 'No regex rules yet. Add or import a rule to get started.'));
  await ui.event('new');
  assert.equal(ui.nodes().find(node => node.props.key === 'name').props.value, 'New regex');
  const chrome = ui.nodes().flatMap(node => [node.props.text, node.props.label]).filter(value => typeof value === 'string');
  assert.ok(chrome.every(value => !/\p{Script=Han}/u.test(value)));
  await ui.event('save');
  assert.match(ui.toasts.at(-1), /search pattern cannot be empty/);
  await ui.event('name', 'onValueChange', '我的规则');
  await ui.event('find', 'onValueChange', '/x/g');
  await ui.event('replace', 'onValueChange', '原文');
  await ui.event('test-source', 'onValueChange', 'x');
  await ui.event('test');
  assert.ok(ui.nodes().some(node => node.props.text === 'Result: 原文'));
  ui.setLanguage('zh-CN');
  assert.ok(ui.nodes().some(node => node.props.text === '正则规则'));
  assert.equal(ui.nodes().find(node => node.props.key === 'name').props.value, '我的规则');
  assert.ok(ui.nodes().some(node => node.props.text === '结果：原文'));
  await ui.event('save');
  assert.equal(ui.toasts.at(-1), '正则规则已保存');
  ui.setLanguage('en-US');
  assert.ok(ui.nodes().some(node => node.props.text === 'Enabled · Display text · Global'));
  assert.equal(ui.rules()[0].scriptName, '我的规则');
  assert.equal(ui.rules()[0].replaceString, '原文');
  await ui.event('open-import');
  await ui.event('import-json', 'onValueChange', '{broken');
  await ui.event('import');
  assert.match(ui.toasts.at(-1), /Invalid regex JSON/);
  assert.doesNotMatch(ui.toasts.at(-1), /\p{Script=Han}/u);
});
