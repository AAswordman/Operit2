import { writeFile } from 'node:fs/promises';
import { screen, message, toolResult } from '../tests/runtime.mjs';

const fixtures = {};
for (const brightness of ['light', 'dark']) {
  for (const locale of ['zh', 'en']) {
    for (const count of [1, 6, 12]) {
      const page = await screen('../src/ui/summary.ui.ts', { locale, chatId: 'chat-a',
        message: message(Array.from({ length: count }, (_, index) => toolResult(`src/file${index}.ts`, '@@ -1 +1,2 @@\n-old\n+new\n+second'))) },
        { theme: { brightness, colors: {} } });
      fixtures[`${brightness}-${locale}-${count}-expanded`] = page.fixture.latest.tree;
      await page.click(page.nodes().find(node => node.type === 'InkWell'));
      fixtures[`${brightness}-${locale}-${count}-collapsed`] = page.fixture.latest.tree;
    }
  }
}
await writeFile(new URL('../../../../../apps/flutter/app/test/fixtures/change_review_summary.json', import.meta.url), JSON.stringify(fixtures));

const workspaceFixtures = {};
const basicDiff = '@@ -1,3 +1,3 @@\n const value = 1;\n-old value\n+new value\n tail';
const longDiff = '@@ -1,3 +1,4 @@\n const value = 1;\n-' + 'old_value '.repeat(18) + '\n+' + 'new_value '.repeat(24) + '\n+second\n tail';
const gapDiff = '@@ -5,2 +5,2 @@\n-old\n+new\n context\n@@ -35,2 +35,2 @@\n second\n-before\n+after';
const samples = [
  { name: 'unified', diff: basicDiff },
  { name: 'split', diff: longDiff, split: true },
  { name: 'wrap', diff: longDiff, wrap: true },
  { name: 'split-wrap', diff: longDiff, split: true, wrap: true },
  { name: 'gap', diff: gapDiff },
  { name: 'split-gap', diff: gapDiff, split: true },
  { name: 'full', diff: gapDiff, full: true },
  { name: 'split-full', diff: gapDiff, full: true, split: true },
  { name: 'full-off', diff: gapDiff, fullOff: true },
  { name: 'english', diff: basicDiff, locale: 'en' },
  { name: 'narrow', diff: basicDiff, width: 400 },
  { name: 'scaled', diff: basicDiff, scale: 1.5 },
  { name: 'empty', empty: true },
  { name: 'tree', diff: basicDiff, pane: 'tree' },
  { name: 'preview', diff: basicDiff, pane: 'tree', preview: { path: '/app/workspaces/test/readme.txt', content: 'tree file body' } },
];
workspaceFixtures.samples = samples;
for (const brightness of ['light', 'dark']) {
 for (const sample of samples) {
  const page = await screen('../src/ui/workspace.ui.ts', {
    locale: sample.locale ?? 'zh', size: { width: sample.width ?? 800, height: 600 }, split: sample.split ?? false, wrap: sample.wrap ?? false,
    pane: sample.pane ?? 'changes', ...(sample.preview ? { preview: sample.preview } : {}),
    ...(sample.pane || sample.full || sample.fullOff ? { workspacePath: '/app/workspaces/test' } : {}), input: { files: sample.empty ? [] : [
      { path: 'src/a.ts', diffContent: sample.diff, additions: sample.diff === longDiff ? 2 : sample.diff === gapDiff ? 2 : 1, deletions: sample.diff === gapDiff ? 2 : 1 },
    ] },
  }, { theme: { brightness, colors: {} }, tools: { Files: { list: async () => ({ entries: [
    { name: 'readme.txt', isDirectory: false, size: 14, lastModified: '' },
  ] }), read: async () => ({ content: sample.full || sample.fullOff ? Array.from({ length: 40 }, (_, index) => `line ${index + 1}`).join('\n') : 'tree file body' }) }, SoftwareSettings: { exec: async args => JSON.stringify({ vfsPath: args[4] + '/' + args[5], nativePath: args[4] + '/' + args[5] }) } } });
  await page.settle();
  if (sample.full || sample.fullOff) {
    await page.click(page.nodes().find(node => node.type === 'Tooltip' && node.props.message === '完整展示整个文件 (展开未更改区域)').children[0]);
    await page.settle();
    if (sample.fullOff) {
      await page.click(page.nodes().find(node => node.type === 'Tooltip' && node.props.message === '仅显示改动附近的行 (收起未更改区域)').children[0]);
      await page.settle();
    }
  }
  if (sample.preview) {
    await page.click(page.nodes().find(node => node.type === 'InkWell' && node.props.key === sample.preview.path));
    await page.settle();
  }
  workspaceFixtures[`${brightness}-${sample.name}`] = page.fixture.latest.tree;
 }
}
const directoryFiles = ['src/chat/ChatArea.dart', 'src/chat/ChatInput.dart', 'src/workspace/WorkspacePanel.dart', 'src/workspace/WorkspaceSession.dart', 'README.md'].map(path => ({
  path, diffContent: basicDiff, additions: 1, deletions: 1,
}));
for (const brightness of ['light', 'dark']) {
  const page = await screen('../src/ui/workspace.ui.ts', { input: { files: directoryFiles }, split: true, wrap: true, size: { width: 800, height: 600 } }, { theme: { brightness, colors: {} } });
  await page.settle();
  workspaceFixtures[`${brightness}-directory-tree`] = page.fixture.latest.tree;
  /** Exercises the same drag actions used by the native pointer gesture region. */
  async function drag(key, deltaX) {
    const node = page.nodes().find(node => node.props.key === key);
    const gesture = node.props.modifier.__modifierOps.find(part => part.name === 'dragGestures').args[0];
    await page.context.__operit_dispatch_compose_dsl_action({ actionId: gesture.onDragStart.__actionId, payload: { x: 6, y: 100 } });
    await page.context.__operit_dispatch_compose_dsl_action({ actionId: gesture.onDrag.__actionId, payload: { deltaX, deltaY: 0, x: 6 + deltaX, y: 100 } });
    await page.settle();
  }
  await drag('sidebar-splitter', -70);
  await drag('diff-splitter', 70);
  workspaceFixtures[`${brightness}-resized-tree`] = page.fixture.latest.tree;
}
await writeFile(new URL('../../../../../apps/flutter/app/test/fixtures/change_review_workspace.json', import.meta.url), JSON.stringify(workspaceFixtures));

const home = await screen('../src/ui/home.ui.ts', { locale: 'zh' });
await writeFile(new URL('../../../../../apps/flutter/app/test/fixtures/change_review_home.json', import.meta.url), JSON.stringify(home.fixture.latest.tree));
