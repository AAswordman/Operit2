import assert from 'node:assert/strict';
import test from 'node:test';
import { screen } from './runtime.mjs';

const review = { path: 'src/a.ts', diffContent: '@@ -3 +3 @@\n-old\n+new', additions: 1, deletions: 1 };

/** Invokes any callback through its real compiled action address. */
async function dispatch(page, node, property, payload) {
  assert.ok(node?.props[property]?.__actionId, `Missing ${property} on ${node?.type}`);
  await page.context.__operit_dispatch_compose_dsl_action({ actionId: node.props[property].__actionId, payload });
  await page.settle();
}

/** Reads the clickable native child of an exact tooltip. */
function control(page, message) {
  return page.nodes().find(node => node.type === 'Tooltip' && node.props.message === message)?.children[0];
}

/** Reads a splitter's serialized gesture callbacks without depending on modifier placement. */
function splitter(page, key) {
  const node = page.nodes().find(node => node.props.key === key);
  assert.equal(node?.props.width, 12, 'Resize handles retain a forgiving hit target');
  return node.props.modifier.__modifierOps.find(part => part.name === 'dragGestures').args[0];
}

/** Sends pointer deltas through the plugin's actual compiled callback. */
async function drag(page, key, deltaX, start = true) {
  if (start) {
    await page.context.__operit_dispatch_compose_dsl_action({ actionId: splitter(page, key).onDragStart.__actionId, payload: { x: 6, y: 100 } });
  }
  await page.context.__operit_dispatch_compose_dsl_action({ actionId: splitter(page, key).onDrag.__actionId, payload: { deltaX, deltaY: 0, x: 6 + deltaX, y: 100 } });
  await page.settle();
}

/** Models two mounted Git repositories and the generic host capabilities, without a native review model. */
function host({ deferredDiff, read } = {}) {
  const calls = [], clipboard = [];
  return { calls, clipboard, tools: {
    Files: {
      list: async path => ({ entries: path === '/workspace' ? [{ name: 'a', isDirectory: true }, { name: 'b', isDirectory: true }] : [{ name: '.git', isDirectory: true }] }),
      read: read ?? (async () => ({ content: 'first\nsecond\nnew\nlast\n' })),
    },
    System: { setClipboardText: async value => { clipboard.push(value); } },
    SoftwareSettings: { exec: async args => {
      calls.push(Array.from(args));
      const command = args[2];
      if (command === 'resolve-path') {
        const value = args[3] === '--from' ? `${args[4]}/${args[5]}` : args.at(-1);
        return JSON.stringify({ vfsPath: value, nativePath: value });
      }
      if (command === 'open-path') return JSON.stringify({ opened: true });
      assert.equal(command, 'exec');
      const workspace = args[3], git = args.slice(6);
      let stdout = '';
      if (git[0] === 'rev-parse') stdout = git.includes('--show-toplevel') ? workspace + '\n' : 'head\n';
      else if (git[0] === 'for-each-ref') stdout = 'refs/heads/main\0 \0\nrefs/heads/feature\0*\0\n';
      else if (git.includes('status')) stdout = ' M same.ts\0';
      else if (git.includes('--name-status')) stdout = 'M\0same.ts\0';
      else if (git.includes('--numstat')) stdout = '1\t1\tsame.ts\0';
      else if (git.includes('--unified=3')) {
        if (deferredDiff) return deferredDiff(workspace, args);
        stdout = `@@ -1 +1 @@\n-old\n+${workspace.endsWith('/a') ? 'repo-a' : 'repo-b'}`;
      } else throw new Error(`Unexpected Git invocation: ${git}`);
      return JSON.stringify({ exitCode: 0, stdout, stderr: '', workingDirectory: workspace });
    } },
  } };
}

test('workspace mounts the original review controls with plugin-owned diff and source menus', async () => {
  const page = await screen('../src/ui/workspace.ui.ts', { locale: 'zh', input: { files: [
    { path: 'src/a.ts', diffContent: '@@ -1 +1 @@\n-old\n+new', additions: 1, deletions: 1 },
  ] } });
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  assert.ok(page.nodes().some(node => node.type === 'MenuAnchor'));
  assert.ok(page.nodes().some(node => node.type === 'Tooltip' && node.props.message === '分栏视图 (Split)'));
});

test('equal filenames from different mounts retain distinct selections and branch bases', async () => {
  const native = host();
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [] } }, { tools: native.tools });
  await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'repo-a'));
  const second = page.nodes().find(node => node.type === 'InkWell' && node.props.key === JSON.stringify(['/workspace/b', 'same.ts']));
  await page.click(second); await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'repo-b'));
  assert.ok(!page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'repo-a'));
  const sourceMenu = page.nodes().find(node => node.type === 'MenuAnchor' && node.props.tooltip === '切换变更对比源');
  await dispatch(page, sourceMenu, 'onSelect', 1);
  const diffs = native.calls.filter(args => args.includes('--unified=3'));
  assert.ok(diffs.at(-1).includes('refs/heads/main...HEAD'));
});

test('late selected-file replies cannot replace a newer selection', async () => {
  let release;
  const native = host({ deferredDiff: async workspace => {
    if (workspace.endsWith('/a')) await new Promise(resolve => { release = resolve; });
    return JSON.stringify({ exitCode: 0, stdout: `@@ -1 +1 @@\n-old\n+${workspace}`, stderr: '' });
  } });
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [] } }, { tools: native.tools });
  await page.settle();
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === JSON.stringify(['/workspace/b', 'same.ts'])));
  await page.settle();
  release(); await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === '/workspace/b'));
  assert.ok(!page.nodes().some(node => node.type === 'CodeText' && node.props.text === '/workspace/a'));
});

test('a selected-file failure remains visible but cannot poison another file', async () => {
  const native = host({ deferredDiff: async workspace => JSON.stringify({ exitCode: workspace.endsWith('/a') ? 128 : 0,
    stdout: workspace.endsWith('/a') ? '' : '@@ -1 +1 @@\n-old\n+healthy-file', stderr: 'read failed' }) });
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [] } }, { tools: native.tools });
  await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'Text' && node.props.text.includes('read failed')));
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === JSON.stringify(['/workspace/b', 'same.ts'])));
  await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'healthy-file'));
  assert.ok(!page.nodes().some(node => node.type === 'Text' && node.props.text.includes('read failed')));
});

test('full-file display reads lazily, compact gaps are inert and toggling off discards full text', async () => {
  let reads = 0;
  const native = host({ read: async () => { reads++; return { content: 'first\nsecond\nnew\nlast' }; } });
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [review] } }, { tools: native.tools });
  await page.settle();
  assert.equal(reads, 0);
  assert.ok(page.nodes().some(node => node.type === 'Icon' && node.props.name === 'unfold_less'));
  await page.click(control(page, '完整展示整个文件 (展开未更改区域)')); await page.settle();
  assert.equal(reads, 1);
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'first'));
  await page.click(control(page, '仅显示改动附近的行 (收起未更改区域)')); await page.settle();
  assert.ok(!page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'first'));
});

test('new review input switches the source without resetting layout choices', async () => {
  const native = host();
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [] } }, { tools: native.tools });
  await page.settle();
  await page.click(control(page, '分栏视图 (Split)')); await page.settle();
  await page.update({ input: { files: [review] } }); await page.settle();
  assert.ok(page.nodes().some(node => node.props.text === '上一轮变更'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  assert.ok(!page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'repo-a'));
  assert.equal(page.nodes().find(node => node.type === 'LazyColumn' && node.props.viewport)?.props.viewport.variableExtent, true);
});

test('ten thousand diff lines materialize only a bounded variable-height JS window', async () => {
  const file = { ...review, diffContent: '@@ -0,0 +1,10000 @@\n' + Array.from({ length: 10000 }, (_, index) => `+line-${index}`).join('\n') };
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [file] } });
  await page.settle();
  assert.ok(page.nodes().filter(node => node.type === 'CodeText').length < 50);
  const list = page.nodes().find(node => node.type === 'LazyColumn' && node.props.viewport);
  const viewport = list.props.viewport;
  assert.equal(viewport.variableExtent, true);
  await page.context.__operit_dispatch_compose_dsl_action({ actionId: viewport.onRangeChanged.__actionId, payload: { firstIndex: 5000, lastIndex: 5040 } });
  assert.equal(page.nodes().filter(node => node.type === 'CodeText').length, 40);
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'line-4999'));
});

test('all external opener choices dispatch generic host intents and copy actual native paths', async () => {
  const native = host();
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [review] } }, { tools: native.tools });
  await page.settle();
  for (const index of [1, 2, 3, 5]) {
    await dispatch(page, page.nodes().find(node => node.type === 'MenuAnchor' && node.props.tooltip === '选择打开方式'), 'onSelect', index);
  }
  const opened = native.calls.filter(args => args[2] === 'open-path');
  assert.deepEqual(opened.map(args => args.slice(4)), [['reveal'], ['program', 'code', '-g'], ['terminal']]);
  assert.deepEqual(native.clipboard, ['/workspace/src/a.ts']);
  assert.deepEqual(page.errors, []);
  assert.match(page.toasts[0], /已复制路径/);
});

test('absolute message resources remain usable without a bound workspace', async () => {
  const native = host();
  native.tools.SoftwareSettings.exec = async args => {
    native.calls.push(Array.from(args));
    assert.equal(args[2], 'resolve-path');
    if (args[3] === '--from') {
      assert.equal(args[4], '');
      assert.equal(args[5], 'D:\\repo\\a.ts');
    }
    return JSON.stringify({ vfsPath: '/mnt/windows/d/repo/a.ts', nativePath: 'D:\\repo\\a.ts' });
  };
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [{ ...review, path: 'D:\\repo\\a.ts' }] } }, { tools: native.tools });
  await page.settle();
  await dispatch(page, page.nodes().find(node => node.type === 'MenuAnchor' && node.props.tooltip === '选择打开方式'), 'onSelect', 5);
  assert.deepEqual(native.clipboard, ['D:\\repo\\a.ts']);
  assert.deepEqual(page.errors, []);
});

test('filter, file navigation, sidebar restoration and width drag retain plugin state', async () => {
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [review, { ...review, path: 'src/b.ts', diffContent: '@@ -1 +1 @@\n-old\n+second-file' }] } });
  await page.settle();
  await page.click(page.nodes().find(node => node.type === 'IconButton' && node.props.tooltip === '下一个改动 (N)'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'second-file'));
  await page.click(page.nodes().find(node => node.type === 'IconButton' && node.props.tooltip === '上一个改动 (P)'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  await dispatch(page, page.nodes().find(node => node.type === 'TextField'), 'onValueChange', 'b.ts');
  assert.ok(!page.nodes().some(node => node.type === 'InkWell' && node.props.key === 'src/a.ts'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'second-file'));
  await drag(page, 'sidebar-splitter', 70);
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 290));
  assert.ok(!page.nodes().some(node => node.props.message === '关闭文件区'));
  const collapse = page.nodes().filter(node => node.type === 'IconButton' && node.props.tooltip === '收起文件区');
  assert.equal(collapse.length, 1, 'Only the preview toolbar owns the sidebar toggle');
  await page.click(collapse[0]);
  assert.ok(!page.nodes().some(node => node.type === 'TextField'));
  await page.click(page.nodes().find(node => node.type === 'IconButton' && node.props.tooltip === '展开文件区'));
  assert.equal(page.nodes().find(node => node.type === 'TextField').props.value, 'b.ts');
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 290));
  assert.deepEqual(page.errors, []);
});

test('changed files form shared collapsible directories and filtering reveals matching descendants', async () => {
  const files = ['src/features/chat/a.ts', 'src/features/chat/b.ts', 'src/model/a.ts', 'readme.md'].map(path => ({ ...review, path }));
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files } });
  assert.equal(page.nodes().filter(node => node.type === 'Text' && node.props.text === 'src').length, 1);
  assert.equal(page.nodes().filter(node => node.type === 'Text' && node.props.text === 'a.ts').length, 2);
  assert.ok(!page.nodes().some(node => node.type === 'Text' && node.props.text === 'src/features/chat'));
  const folderKey = 'change-dir:' + JSON.stringify(['', 'src/features']);
  await page.click(page.nodes().find(node => node.props.key === folderKey)); await page.settle();
  assert.ok(!page.nodes().some(node => node.props.key === files[0].path));
  assert.ok(page.nodes().some(node => node.props.key === files[2].path));
  await dispatch(page, page.nodes().find(node => node.type === 'TextField'), 'onValueChange', 'features/chat/b.ts');
  assert.ok(page.nodes().some(node => node.props.key === files[1].path));
  assert.ok(!page.nodes().some(node => node.props.key === files[0].path));
  await dispatch(page, page.nodes().find(node => node.type === 'TextField'), 'onValueChange', '');
  assert.ok(!page.nodes().some(node => node.props.key === files[0].path), 'Filtering preserves the previous collapse state');
  await page.click(page.nodes().find(node => node.type === 'IconButton' && node.props.tooltip === '下一个改动 (N)')); await page.settle();
  assert.ok(page.nodes().some(node => node.props.key === files[1].path), 'Next-file navigation reveals the selected ancestor chain');
  assert.deepEqual(page.errors, []);
});

test('sidebar can shrink, clamps to the viewport and starts the next drag at its rendered width', async () => {
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [review] } });
  await drag(page, 'sidebar-splitter', -1000);
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 120));
  await drag(page, 'sidebar-splitter', 1000);
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 500));
  await page.update({ size: { width: 400, height: 600 } }); await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 239));
  await drag(page, 'sidebar-splitter', -20);
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 219));
  await drag(page, 'sidebar-splitter', -20, false);
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 199));
  assert.deepEqual(page.errors, []);
});

test('rapid sidebar pointer updates accumulate before the next host render', async () => {
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [review] } });
  const gesture = splitter(page, 'sidebar-splitter');
  await page.context.__operit_dispatch_compose_dsl_action({ actionId: gesture.onDragStart.__actionId, payload: { x: 6, y: 100 } });
  await Promise.all([30, 40, 30].map(deltaX => page.context.__operit_dispatch_compose_dsl_action({
    actionId: gesture.onDrag.__actionId, payload: { deltaX, deltaY: 0, x: 6, y: 100 },
  })));
  await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'Column' && node.props.width === 320));
  assert.deepEqual(page.errors, []);
});

test('split comparison widths follow dragging, retain both sides and preserve the ratio on resize', async () => {
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: [review] }, split: true, wrap: true });
  /** Reads the paired row without mistaking toolbar rows for code panes. */
  function pair() {
    return page.nodes().find(node => node.type === 'Row' && node.props.width && node.children.length === 3 && node.children[1].props.width === 1);
  }
  assert.equal(pair().children[0].props.modifier.__modifierOps.find(part => part.name === 'width').args[0], 289);
  await drag(page, 'diff-splitter', 80);
  const widths = pair().children.filter((_, index) => index !== 1).map(node => node.props.modifier.__modifierOps.find(part => part.name === 'width').args[0]);
  assert.deepEqual(widths, [369, 209]);
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'old'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  await page.update({ size: { width: 1100, height: 600 } }); await page.settle();
  const resized = pair().children[0].props.modifier.__modifierOps.find(part => part.name === 'width').args[0];
  assert.ok(Math.abs(resized / 878 - 369 / 578) < 0.000001);
  await drag(page, 'diff-splitter', -10000);
  assert.equal(pair().children[0].props.modifier.__modifierOps.find(part => part.name === 'width').args[0], 96);
  assert.deepEqual(page.errors, []);
});

test('tree directories are cached and ordinary previews stay plugin-owned', async () => {
  const native = host(), listings = [];
  native.tools.Files.list = async path => {
    listings.push(path);
    return { entries: path === '/workspace' ? [{ name: 'src', isDirectory: true }, { name: '.hidden', isDirectory: false }] : [{ name: 'a.ts', isDirectory: false }] };
  };
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [review] } }, { tools: native.tools });
  await page.settle();
  await page.click(control(page, '切换至项目工作树')); await page.settle();
  assert.ok(!page.nodes().some(node => node.props.text === '.hidden'));
  for (let index = 0; index < 3; index++) {
    await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === '/workspace/src')); await page.settle();
  }
  assert.equal(listings.filter(path => path === '/workspace/src').length, 1);
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === '/workspace/src/a.ts')); await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.filePath === '/workspace/src/a.ts' && node.props.text === 'first\nsecond\nnew\nlast\n'));
  assert.deepEqual(page.errors, []);
});

test('tree previews use selectable syntax text with the selected file grammar', async () => {
  const html = '<!-- header\ncomment -->\n<html lang="zh-CN">\n</html>\n';
  const script = '// note\nconst answer = 42;\n';
  const native = host({ read: async path => ({ content: path.endsWith('.html') ? html : script }) });
  native.tools.Files.list = async () => ({ entries: [
    { name: 'index.html', isDirectory: false }, { name: 'main.ts', isDirectory: false },
  ] });
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [review] } }, { tools: native.tools });
  await page.settle();
  await page.click(control(page, '切换至项目工作树')); await page.settle();
  for (const [name, content] of [['index.html', html], ['main.ts', script]]) {
    await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === `/workspace/${name}`));
    await page.settle();
    const selection = page.nodes().find(node => node.type === 'SelectionContainer');
    assert.equal(selection?.children[0].type, 'CodeText');
    assert.equal(selection.children[0].props.filePath, `/workspace/${name}`);
    assert.equal(selection.children[0].props.text, content);
  }
  await page.click(control(page, '切换至变更文件列表')); await page.settle();
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === review.path));
  await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  assert.deepEqual(page.errors, []);
});

test('late previews cannot reopen over a newly selected diff, and read errors remain visible', async () => {
  let release;
  const native = host({ read: async path => {
    if (path.endsWith('slow.ts')) { await new Promise(resolve => { release = resolve; }); return { content: 'obsolete-preview' }; }
    throw new Error('preview read denied');
  } });
  native.tools.Files.list = async () => ({ entries: [{ name: 'slow.ts', isDirectory: false }, { name: 'denied.ts', isDirectory: false }] });
  const page = await screen('../src/ui/workspace.ui.ts', { workspacePath: '/workspace', input: { files: [review] } }, { tools: native.tools });
  await page.settle();
  await page.click(control(page, '切换至项目工作树')); await page.settle();
  const pending = page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === '/workspace/slow.ts'));
  await new Promise(resolve => setImmediate(resolve));
  await page.click(control(page, '切换至变更文件列表')); await page.settle();
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === review.path));
  release(); await pending; await page.settle();
  assert.ok(!page.nodes().some(node => node.props.text === 'obsolete-preview'));
  assert.ok(page.nodes().some(node => node.type === 'CodeText' && node.props.text === 'new'));
  await page.click(control(page, '切换至项目工作树')); await page.settle();
  await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === '/workspace/denied.ts')); await page.settle();
  assert.ok(page.nodes().some(node => node.type === 'Text' && node.props.text.includes('preview read denied')));
  assert.deepEqual(page.errors, []);
});
