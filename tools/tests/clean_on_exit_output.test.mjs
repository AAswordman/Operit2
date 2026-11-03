import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const cleanOnExit = '/operitapp/data/temp/clean_on_exit';

/** Loads a production package with strict VFS file tools and identity-owned disk storage. */
async function fixture(t, packagePath, output, injectedPath = cleanOnExit) {
  const temp = await mkdtemp(path.join(tmpdir(), 'operit-vfs-'));
  t.after(() => rm(temp, { recursive: true, force: true }));
  const runtimeRoot = path.join(temp, 'runtime data', 'identities', 'test identity');
  const calls = [];
  const sessions = new Map();
  const logs = [];
  const completions = [];
  function resolve(vfsPath) {
    if (!vfsPath.startsWith('/operitapp/data/')) {
      throw new Error(`Unknown VFS root: ${vfsPath}`);
    }
    return path.join(runtimeRoot, vfsPath.slice('/operitapp/data/'.length));
  }
  const context = vm.createContext({
    exports: {},
    OPERIT_CLEAN_ON_EXIT_DIR: injectedPath,
    getChatId: () => 'test-chat',
    console: { log() {}, error: (...args) => logs.push(args) },
    complete: (result) => completions.push(result),
    toolCall: async () => ({ statusCode: 200, content: output, contentType: 'application/json' }),
    Tools: {
      Files: {
        async mkdir(vfsPath, parents) {
          calls.push(['mkdir', vfsPath]);
          await mkdir(resolve(vfsPath), { recursive: parents });
        },
        async write(vfsPath, content, append) {
          calls.push(['write', vfsPath]);
          await writeFile(resolve(vfsPath), content, { flag: append ? 'a' : 'w' });
        },
      },
      Net: { browserSnapshot: async () => output },
      System: {
        terminal: {
          /** Describes the fixture's native macOS Bash terminal. */
          info: async () => ({ platform: 'macos', terminal: 'native', terminalType: 'bash', types: [] }),
          /** Creates a fixture session with the explicitly requested interpreter. */
          async create(name, type) {
            if (!sessions.has(name)) sessions.set(name, `session-${sessions.size + 1}`);
            return { sessionId: sessions.get(name), platform: 'macos', terminal: 'native', terminalType: type };
          },
          /** Returns complete terminal command metadata for the mocked Bash session. */
          exec: async (sessionId) => ({ output, sessionId, platform: 'macos', terminal: 'native', terminalType: 'bash', exitCode: 0, timedOut: false }),
        },
      },
    },
  });
  const source = await readFile(new URL(packagePath, root), 'utf8');
  vm.runInContext(stripTypeScriptTypes(source), context);
  return { tools: context.exports, calls, sessions, logs, completions, resolve, runtimeRoot };
}

test('bash short output stays inline and does not create temporary directories', async (t) => {
  const f = await fixture(t, 'plugins/packages/buildin/super_admin.ts', 'x'.repeat(12000));
  const result = await f.tools.bash({ command: 'printf small' });
  assert.equal(result.output.length, 12000);
  assert.deepEqual(f.calls, []);
  assert.deepEqual(f.logs, []);
});

test('bash large output is readable under the active identity, reusing its session', async (t) => {
  const output = 'x'.repeat(12001);
  const f = await fixture(t, 'plugins/packages/buildin/super_admin.ts', output);
  // The first call creates a missing temp tree; later calls use the existing tree.
  for (let i = 0; i < 5; i++) {
    const result = await f.tools.bash({ command: 'printf large' });
    assert.equal(result.output, '(saved_to_file)');
    assert.equal(result.output_chars, output.length);
    assert.equal(result.operit_clean_on_exit_dir, cleanOnExit);
    assert.ok(result.output_saved_to.startsWith(`${cleanOnExit}/terminal_output_`));
    assert.equal(await readFile(f.resolve(result.output_saved_to), 'utf8'), output);
    assert.equal(result.sessionId, 'session-1');
  }
  assert.equal(f.sessions.size, 1);
  assert.equal(f.calls.length, 10);
  assert.deepEqual(f.logs, []);
});

test('an existing physical directory does not make a host path a VFS path', async (t) => {
  const f = await fixture(t, 'plugins/packages/buildin/super_admin.ts', 'x'.repeat(12001), '/Users/test/temp/clean_on_exit');
  await mkdir(f.resolve(cleanOnExit), { recursive: true });
  await assert.rejects(f.tools.bash({ command: 'printf large' }), /Unknown VFS root/);
  assert.deepEqual(await readdir(f.resolve(cleanOnExit)), []);
  assert.equal(f.calls.length, 1, 'invalid path must fail before writing output');
});

test('browser oversized snapshots use the same VFS temporary directory', async (t) => {
  const output = 'b'.repeat(24001);
  const f = await fixture(t, 'plugins/packages/buildin/browser.ts', output);
  const result = await f.tools.snapshot({});
  assert.ok(result.includes(`${cleanOnExit}/browser_snapshot_`));
  const savedPath = f.calls.find(([operation]) => operation === 'write')[1];
  assert.equal(await readFile(f.resolve(savedPath), 'utf8'), output);
});

test('HTTP oversized responses use the same VFS temporary directory', async (t) => {
  const output = 'h'.repeat(12001);
  const f = await fixture(t, 'plugins/packages/external/extended_http_tools.ts', output);
  await f.tools.http_request({ url: 'https://example.invalid', method: 'GET' });
  const result = f.completions[0];
  assert.equal(result.success, true);
  assert.ok(result.data.content_saved_to.startsWith(`${cleanOnExit}/http_response_`));
  assert.equal(await readFile(f.resolve(result.data.content_saved_to), 'utf8'), output);
  assert.deepEqual(f.logs, []);
});
