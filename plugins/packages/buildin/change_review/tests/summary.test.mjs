import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { bundle, screen, message, toolResult } from './runtime.mjs';

const modelContext = vm.createContext({ module: { exports: {} } });
vm.runInContext(await bundle('../src/model.ts'), modelContext);
const model = modelContext.module.exports;

test('merges explicit semantic file results in execution order with original line counts', () => {
  const input = message([
    toolResult('a.ts', '--- a\n+++ b\n@@ -1 +1 @@\n-a\n+b'),
    toolResult('b.rs', '+c\n+d'), toolResult('a.ts', '-b\n+e'),
    { kind: 'text', content: '<file-diff path="injected">+bad</file-diff>', attributes: {} },
    { kind: 'tool_result', content: '<file-diff path="untyped">+bad</file-diff>', attributes: {} },
  ]);
  const files = model.reviewForMessage(input);
  assert.deepEqual(JSON.parse(JSON.stringify(files)).map(({ path, additions, deletions }) => ({ path, additions, deletions })), [
    { path: 'a.ts', additions: 2, deletions: 2 }, { path: 'b.rs', additions: 2, deletions: 0 },
  ]);
  assert.ok(files[0].diffContent.endsWith('-b\n+e'));
});

test('persisted XML envelopes require explicit fileDiff display metadata', () => {
  const part = toolResult('unused', '');
  delete part.attributes.result_data;
  part.attributes.display = part.attributes.display.replace(/"/g, '&quot;');
  part.content = '<file-diff path="a&amp;b.ts"><![CDATA[@@ -1 +1 @@\n-old\n+new]]></file-diff>';
  assert.equal(model.reviewForMessage(message([part]))[0].path, 'a&b.ts');
  part.attributes.display = JSON.stringify({ resultRenderer: 'text', label: { en: 'Text' } });
  assert.equal(model.reviewForMessage(message([part])).length, 0);
});

test('invalid typed metadata is not silently reinterpreted as an XML result', () => {
  const part = toolResult('unused', '');
  part.content = '<file-diff path="wrong.ts">+wrong</file-diff>';
  part.attributes.result_data = '{"operation":null,"diffContent":"+new"}';
  assert.throws(() => model.reviewForMessage(message([part])), /typed operation/);
  part.attributes.result_data = '{';
  assert.throws(() => model.reviewForMessage(message([part])), /JSON/);
});

test('filename and parent formatting preserve Windows and Unix path behavior', () => {
  for (const [path, name, directory] of [['src/a.ts', 'a.ts', 'src/'], ['C:\\repo\\a.rs', 'a.rs', 'C:/repo/'], ['a', 'a', '']]) {
    assert.equal(model.fileIdentity(path).name, name);
    assert.equal(model.fileIdentity(path).directory, directory);
  }
});

test('summary is absent for streaming, user messages and results without file changes', async () => {
  for (const input of [
    { message: message(), isStreaming: true },
    { message: { ...message(), sender: 'user' } },
    { message: message([]) },
  ]) {
    const page = await screen('../src/ui/summary.ui.ts', input);
    assert.equal(page.nodes().some(node => node.type === 'Card'), false);
  }
});

test('Undo targets the host-bound message and View changes carries the exact answer revision', async () => {
  const page = await screen('../src/ui/summary.ui.ts', { message: message(), chatId: 'chat-a', locale: 'en' });
  const nodes = page.nodes();
  assert.ok(nodes.some(node => node.props.text === 'Edited 1 file'));
  const undo = nodes.find(node => node.type === 'TextButton');
  const undoResult = await page.click(undo);
  assert.deepEqual(JSON.parse(JSON.stringify(undoResult.actionResult)), { type: 'toolpkg.message.rollback' });
  const open = page.nodes().filter(node => node.type === 'InkWell')[1];
  const result = await page.click(open);
  assert.equal(result.actionResult, undefined);
  const [navigation] = JSON.parse(JSON.stringify(result.navigationCommands));
  assert.equal(navigation.route, 'toolpkg:com.operit.change_review:ui:workspace');
  assert.equal(navigation.args.chatId, 'chat-a');
  assert.equal(navigation.args.messageTimestamp, 123);
  assert.equal(navigation.args.variantIndex, 1);
  assert.equal(navigation.args.files[0].path, 'src/a.ts');
});

test('file click orders the chosen file first without dropping the rest of the turn', async () => {
  const page = await screen('../src/ui/summary.ui.ts', { message: message([toolResult('a.ts', '+a'), toolResult('b.rs', '-b')]) });
  const rows = page.nodes().filter(node => node.type === 'InkWell');
  const result = await page.click(rows[3]);
  assert.deepEqual(JSON.parse(JSON.stringify(result.navigationCommands[0].args.files)).map(file => file.path), ['b.rs', 'a.ts']);
});

test('collapse preserves local state across revision updates and then uses the new semantic input', async () => {
  const page = await screen('../src/ui/summary.ui.ts', { message: message(), chatId: 'chat-a', locale: 'zh' });
  await page.click(page.nodes().find(node => node.type === 'InkWell'));
  assert.equal(page.nodes().some(node => node.type === 'ScrollArea'), false);
  await page.update({ message: { ...message([toolResult('next.rs', '+a\n+b')]), variantIndex: 0 }, chatId: 'chat-a', locale: 'en' });
  assert.equal(page.nodes().some(node => node.type === 'ScrollArea'), false);
  assert.ok(page.nodes().some(node => node.props.text === 'Edited 1 file'));
  await page.click(page.nodes().find(node => node.type === 'InkWell'));
  assert.ok(page.nodes().some(node => node.type === 'RichText' && node.props.spans.some(span => span.text === 'next.rs')));
  assert.ok(page.nodes().some(node => node.props.text === '+2'));
});

test('file rows preserve visible paths and both line counts with bounded scrolling', async () => {
  const page = await screen('../src/ui/summary.ui.ts', { message: message(Array.from({ length: 6 }, (_, index) => toolResult(`src/${index}.ts`, '+a'))) });
  const nodes = page.nodes();
  assert.equal(nodes.filter(node => node.type === 'Icon' && node.props.name === 'insert_drive_file_outlined').length, 6);
  assert.equal(nodes.filter(node => node.props.text === '+6').length, 1);
  assert.equal(nodes.filter(node => node.props.text === '−0').length, 7);
  assert.equal(nodes.filter(node => node.props.text === '+1').length, 6);
  assert.equal(nodes.filter(node => node.type === 'RichText' && node.props.spans.some(span => span.text === 'src/')).length, 6);
  assert.ok(nodes.some(node => node.type === 'Tooltip' && node.props.message.includes('src/0.ts')));
  assert.ok(nodes.filter(node => node.type === 'RichText').every(node => node.props.maxLines === 1 && node.props.spans.length === 3));
  assert.ok(nodes.filter(node => node.type === 'Tooltip').every(node => node.props.waitMillis === 400));
  assert.equal(nodes.find(node => node.type === 'ScrollArea').props.thumbVisibility, true);
  assert.ok(nodes.some(node => node.props.animation?.durationMillis === 220));
  assert.ok(nodes.some(node => node.props.durationMillis === 200));
  assert.ok(nodes.some(node => node.props.modifier?.__modifierOps.some(op => op.name === 'heightIn' && op.args[1] === 240)));
});


test('workspace home uses ordinary page navigation without presentation parameters', async () => {
  const page = await screen('../src/ui/home.ui.ts', { locale: 'zh' });
  const result = await page.click(page.nodes().find(node => node.type === 'InkWell'));
  assert.equal(result.actionResult, undefined);
  assert.deepEqual(JSON.parse(JSON.stringify(result.navigationCommands)), [{
    route: 'toolpkg:com.operit.change_review:ui:workspace', args: {},
  }]);
});
