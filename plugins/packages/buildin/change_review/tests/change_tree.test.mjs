import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { bundle } from './runtime.mjs';

const context = vm.createContext({ module: { exports: {} } });
vm.runInContext(await bundle('../src/change_tree.ts'), context);
const { buildChangeTree, changeTreeRows, fileKey } = context.module.exports;

/** Creates a minimal semantic change without filesystem dependencies. */
function file(path, root) {
  return { path, diffContent: '+changed', additions: 1, deletions: 0, ...(root === undefined ? {} : { root }) };
}

test('directories precede files and common parents are stored once', () => {
  const tree = buildChangeTree([file('src/z.ts'), file('readme.md'), file('src/ui/b.ts'), file('src/a.ts')]);
  assert.deepEqual(Array.from(changeTreeRows(tree, new Set()), row => [row.depth, row.node.name]), [
    [0, 'src'], [1, 'ui'], [2, 'b.ts'], [1, 'a.ts'], [1, 'z.ts'], [0, 'readme.md'],
  ]);
  assert.equal(changeTreeRows(tree, new Set([tree[0].id])).length, 2);
});

test('equal paths in separate Git roots remain separate trees and selections', () => {
  const first = file('src/a.ts', '/workspace/a'), second = file('src/a.ts', '/workspace/b');
  const tree = buildChangeTree([first, second]);
  assert.deepEqual(Array.from(tree, node => node.name), ['a', 'b']);
  assert.notEqual(fileKey(first), fileKey(second));
  const rows = changeTreeRows(tree, new Set([tree[0].id]));
  assert.equal(rows.filter(row => row.node.kind === 'file').length, 1);
  assert.equal(rows.at(-1).node.id, fileKey(second));
});

test('Windows and absolute resource paths display segments without changing their original identity', () => {
  for (const path of ['D:\\repo\\src\\a.ts', '/mnt/repo/src/a.ts']) {
    const rows = changeTreeRows(buildChangeTree([file(path)]), new Set());
    assert.equal(rows.at(-1).node.name, 'a.ts');
    assert.equal(rows.at(-1).node.id, path);
    assert.equal(rows.at(-1).node.file.path, path);
    assert.equal(rows.at(-1).parents.length, 3);
  }
  assert.equal(buildChangeTree([]).length, 0);
});
