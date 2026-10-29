// Regression test: terminal_jobs used to hand a session NAME to the host's `screen`, which
// resolves sessions by ID, so every call reported `exists: false` with
// "Terminal session does not exist: super_admin_default_session_bash_<chatId>".
// The tool must redeem the name for an id through `create` (create-or-get) first.
//
// The module under test is the compiled plugin (what actually ships), produced by
// `tsc -p plugins/packages/buildin/tsconfig.json` into plugins/.out/buildin.
import test from 'node:test';
import assert from 'node:assert';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const COMPILED = fileURLToPath(new URL('../../../.out/buildin/super_admin.js', import.meta.url));

const skip = !existsSync(COMPILED)
  ? 'compiled plugin missing: run tsc -p plugins/packages/buildin/tsconfig.json first'
  : false;

/** Installs a fake Tools global and records the calls the plugin makes. */
function stubTools({ isNewSession = false, screenFails = false } = {}) {
  const calls = [];
  // getChatId is injected by the plugin runtime as a bare global, not a module import.
  globalThis.getChatId = () => 'chat-1';
  globalThis.Tools = {
    System: {
      getChatId: () => 'chat-1',
      terminal: {
        create: async (name, type) => {
          calls.push({ op: 'create', name, type });
          return { sessionId: `id-for-${type}`, sessionName: name, isNewSession, platform: 'android' };
        },
        screen: async (sessionId) => {
          calls.push({ op: 'screen', sessionId });
          if (screenFails) throw new Error('boom');
          return { content: 'line-a\nline-b\n', rows: 2, cols: 6, commandRunning: false, terminalType: 'shell' };
        },
      },
    },
  };
  return calls;
}

test('terminal_jobs redeems the session name for an id before reading the screen', { skip }, async () => {
  const calls = stubTools();
  const plugin = require(COMPILED);
  const out = await plugin.terminal_jobs({});

  const creates = calls.filter((c) => c.op === 'create');
  const screens = calls.filter((c) => c.op === 'screen');

  assert.equal(creates.length, 3, 'each default session must be redeemed through create');
  assert.equal(screens.length, 3, 'all three default sessions must be read');
  for (const screen of screens) {
    assert.ok(
      String(screen.sessionId).startsWith('id-for-'),
      `screen must receive an id, not a name; got: ${screen.sessionId}`,
    );
  }
  for (const job of out.jobs) {
    assert.equal(job.exists, true, `session should exist: ${JSON.stringify(job)}`);
    assert.equal(job.error, undefined, `no error expected: ${JSON.stringify(job)}`);
  }
});

test('terminal_jobs admits that redeeming a name may create the session', { skip }, async () => {
  const calls = stubTools({ isNewSession: true });
  const plugin = require(COMPILED);
  const out = await plugin.terminal_jobs({});

  assert.ok(calls.every((c) => c.op === 'create' || c.op === 'screen'));
  for (const job of out.jobs) {
    assert.equal(job.exists, false, 'a session created by this query must not be reported as pre-existing');
    assert.equal(job.createdByQuery, true);
  }
  assert.match(out.note, /原本不存在/);
});

test('an explicit sessionId skips the name lookup entirely', { skip }, async () => {
  const calls = stubTools();
  const plugin = require(COMPILED);
  const out = await plugin.terminal_jobs({ sessionId: 'bg-session-id' });

  assert.equal(calls.filter((c) => c.op === 'create').length, 0, 'an explicit id must not trigger create');
  assert.deepEqual(calls, [{ op: 'screen', sessionId: 'bg-session-id' }]);
  assert.equal(out.jobs.length, 1);
});

test('a failing screen keeps the error instead of pretending the session exists', { skip }, async () => {
  stubTools({ screenFails: true });
  const plugin = require(COMPILED);
  const out = await plugin.terminal_jobs({ sessionId: 'bg-session-id' });

  assert.equal(out.jobs[0].exists, false);
  assert.match(out.jobs[0].error, /boom/);
});
