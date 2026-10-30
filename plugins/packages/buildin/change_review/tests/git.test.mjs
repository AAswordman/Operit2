import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { mkdtemp, writeFile, rename, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { bundle } from './runtime.mjs';

const context = vm.createContext({ module: { exports: {} } });
vm.runInContext(await bundle('../src/git.ts'), context);
const api = context.module.exports;
const execute = promisify(execFile);

/** Runs real Git as argument vectors using the same structured contract as production. */
async function run(workspace, program, args) {
  try {
    const result = await execute(program, args, { cwd: workspace, encoding: 'utf8' });
    return { ...result, exitCode: 0, workingDirectory: workspace };
  } catch (error) {
    if (typeof error.code !== 'number') throw error;
    return { exitCode: error.code, stdout: error.stdout, stderr: error.stderr, workingDirectory: workspace };
  }
}

test('porcelain rename keeps the destination and consumes the original filename', () => {
  const rows = api.parseStatus('R  new\tname.ts\0old name.ts\0?? untracked.txt\0UU conflict.txt\0');
  assert.equal(rows[0].path, 'new\tname.ts');
  assert.equal(rows[0].originalPath, 'old name.ts');
  assert.equal(rows[1].untracked, true);
  assert.equal(rows[2].untracked, false);
  assert.equal(rows[2].status, 'U');
});

test('machine numstat retains tabs and rename destinations without trimming filenames', () => {
  const stats = api.parseNumstat('2\t1\t space\tname \0-\t-\timage.bin\0' + '3\t0\t\0old.ts\0new.ts\0');
  assert.equal(stats.get(' space\tname ').additions, 2);
  assert.equal(stats.get('image.bin').deletions, 0);
  assert.equal(stats.get('new.ts').additions, 3);
});

test('real repository preserves branch comparison when lazily selecting a diff', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'operit-review-'));
  try {
    /** Executes setup commands and fails immediately on any unexpected exit code. */
    async function command(...args) {
      const result = await run(root, 'git', args);
      assert.equal(result.exitCode, 0, result.stderr);
    }
    await command('init', '-b', 'main');
    await command('config', 'user.email', 'test@example.invalid');
    await command('config', 'user.name', 'Review test');
    await writeFile(path.join(root, 'a file.ts'), 'one\ntwo\n');
    await command('add', '.');
    const unborn = await api.loadChanges(root, 'git', null, run);
    assert.equal(unborn[0].unborn, true);
    const initial = await api.loadFileDiff(root, unborn[0], async () => 'one\ntwo\n', run);
    assert.equal(initial.additions, 2);
    assert.deepEqual(Array.from(await api.loadChanges(root, 'branch', null, run)), []);
    await command('commit', '-m', 'base');
    await command('switch', '-c', 'feature');
    await writeFile(path.join(root, 'a file.ts'), 'one\nbranch\n');
    await command('add', '.'); await command('commit', '-m', 'branch change');
    await writeFile(path.join(root, 'a file.ts'), 'one\nworktree\n');
    await writeFile(path.join(root, 'new.txt'), 'untracked\n');
    const branch = await api.loadChanges(root, 'branch', 'main', run);
    const automatic = await api.loadChanges(root, 'branch', null, run);
    assert.equal(automatic[0].base, 'refs/heads/main');
    assert.equal(branch.length, 1);
    const selected = await api.loadFileDiff(root, branch[0], () => { throw new Error('Unexpected untracked read'); }, run);
    assert.match(selected.diffContent, /\+branch/);
    assert.doesNotMatch(selected.diffContent, /worktree/);
    const working = await api.loadChanges(root, 'git', 'main', run);
    assert.equal(working.length, 2);
    const tracked = working.find(file => file.path === 'a file.ts');
    const worktree = await api.loadFileDiff(root, tracked, () => { throw new Error('Unexpected untracked read'); }, run);
    assert.match(worktree.diffContent, /\+worktree/);
    const untracked = await api.loadFileDiff(root, working.find(file => file.untracked), async () => 'untracked\n', run);
    assert.equal(untracked.additions, 1);
    assert.match(untracked.diffContent, /@@ -0,0 \+1,1 @@/);
    await command('restore', 'a file.ts');
    await rename(path.join(root, 'a file.ts'), path.join(root, 'renamed.ts'));
    await command('add', '-A');
    const renamed = (await api.loadChanges(root, 'git', 'main', run)).find(file => file.status === 'R');
    assert.equal(renamed.path, 'renamed.ts');
    assert.equal(renamed.originalPath, 'a file.ts');
  } finally {
    assert.equal(path.dirname(path.resolve(root)), path.resolve(tmpdir()));
    assert.ok(path.basename(root).startsWith('operit-review-'));
    await rm(root, { recursive: true, force: true });
  }
});

test('command failure remains a visible error and does not try another executable or ref', async () => {
  let calls = 0;
  await assert.rejects(api.loadChanges('/workspace', 'git', 'main', async () => {
    calls++;
    return { exitCode: 128, stdout: '', stderr: 'not a git repository' };
  }), /not a git repository/);
  assert.equal(calls, 1);
});
