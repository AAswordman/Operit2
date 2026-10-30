import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { bundle } from './runtime.mjs';

const context = vm.createContext({ module: { exports: {} } });
vm.runInContext(await bundle('../src/diff.ts'), context);
const { parseDiff, diffItems, changedRanges, visualWidth } = context.module.exports;
/** Detaches VM values for precise cross-realm contract comparisons. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

test('unified parsing preserves CRLF, both line numbers and explicit zero-length ranges', () => {
  const document = parseDiff('--- a/a\r\n+++ b/a\r\n@@ -2,2 +2,3 @@ symbol\r\n same\r\n-old\r\n+new\r\n+extra\r\n\\ No newline at end of file\r\n@@ -8,0 +10,1 @@\r\n+tail\r\n');
  assert.equal(document.hunks.length, 2);
  assert.equal(document.hunks[1].oldCount, 0);
  assert.equal(document.additions, 3);
  assert.equal(document.deletions, 1);
  assert.deepEqual(plain(document.hunks[0].rows.map(row => [row.oldLineNumber, row.newLineNumber, row.text])), [[2, 2, 'same'], [3, null, 'old'], [null, 3, 'new'], [null, 4, 'extra']]);
});

test('headerless fragments are explicit and do not turn file headers into changes', () => {
  const source = '--- a/a\n+++ b/a\n-old\n+new';
  assert.equal(parseDiff(source).hunks.length, 0);
  const document = parseDiff(source, true);
  assert.equal(document.hunks[0].header, '@@');
  assert.equal(document.additions, 1);
  assert.equal(document.deletions, 1);
  assert.equal(diffItems(document, false, null, null).some(item => item.type === 'hunk'), false);
});

test('intraline emphasis keeps shared prefix and suffix unpainted, including Unicode', () => {
  assert.deepEqual(plain(changedRanges('const old = true;', 'const next = true;')), [[{ start: 6, end: 9 }], [{ start: 6, end: 10 }]]);
  assert.deepEqual(plain(changedRanges('same', 'same')), [[], []]);
  assert.deepEqual(plain(changedRanges('a中b', 'a文b')), [[{ start: 1, end: 2 }], [{ start: 1, end: 2 }]]);
  assert.equal(visualWidth('a\t中😀'), 9);
});

test('split pairs unequal blocks and preserves missing sides without mixing context', () => {
  const items = diffItems(parseDiff('@@ -1,3 +1,4 @@\n same\n-old1\n-old2\n+new1\n+new2\n+new3'), true, null, null);
  assert.deepEqual(plain(items.map(item => [item.left?.text ?? null, item.right?.text ?? null])), [['same', 'same'], ['old1', 'new1'], ['old2', 'new2'], [null, 'new3']]);
});

test('folded gaps expand independently using current-file coordinates in both layouts', () => {
  const document = parseDiff('@@ -3,1 +3,1 @@\n-old\n+new\n@@ -6,1 +6,1 @@\n-old\n+new');
  const file = ['one', 'two', 'new', 'four', 'five', 'new'];
  for (const split of [false, true]) {
    const folded = diffItems(document, split, file, new Set());
    assert.equal(folded.filter(item => item.type === 'gap').length, 2);
    const one = diffItems(document, split, file, new Set([4]));
    assert.equal(one.filter(item => item.type === 'gap').length, 1);
    assert.ok(one.some(item => (split ? item.left : item.row)?.text === 'four'));
    const all = diffItems(document, split, file, null);
    assert.equal(all.some(item => item.type === 'gap'), false);
    const unavailable = diffItems(document, split, [], null);
    assert.equal(unavailable.filter(item => item.type === 'gap').length, 2);
  }
});
