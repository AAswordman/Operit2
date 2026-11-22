import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('../../core/crates/plugin/javascript-bridge/src/javascript/AndroidUtils.script.js', import.meta.url), 'utf8');

/** Loads the complete production lazy library with distinguishable Android and Linux transports. */
function engine(options = {}) {
  const calls = [];
  const logs = [];
  let sequence = 0;
  const context = vm.createContext({
    console: { error: (...args) => logs.push(args) },
    __operitCanonicalTools: { System: { terminal: {
      async create(name, type) {
        calls.push({ op: 'create', name, type });
        if (options.createError) throw new Error(options.createError);
        return { sessionId: 'android-' + (++sequence) };
      },
      async exec(id, command, timeout) {
        calls.push({ op: 'exec', id, command, timeout });
        if (options.execError) throw new Error(options.execError);
        return { output: options.output ?? 'raw Android output\n', exitCode: options.exitCode ?? 0, timedOut: options.timedOut ?? false };
      },
      async close(id) {
        calls.push({ op: 'close', id });
        if (options.closeError) throw new Error(options.closeError);
        return { success: !options.closeFailure, message: 'close refused' };
      },
    } } },
    Tools: { System: { terminal: {
      create() { throw new Error('public v1 facade selects Linux'); },
      hiddenExec() { throw new Error('hiddenExec selects proot'); },
    } } },
  });
  context.__operitExpose = (name, value) => { context[name] = value; };
  vm.runInContext(script, context);
  return { calls, logs, run: code => vm.runInContext(code, context) };
}

test('AndroidUtils selects Android shell and preserves command, timeout and raw output', async () => {
  const h = engine();
  const command = "dumpsys usagestats | head -20; echo '/sdcard/a b'";
  h.run('globalThis.command = ' + JSON.stringify(command));
  assert.equal(await h.run('executeShell(command, 42000)'), 'raw Android output\n');
  assert.deepEqual(h.calls.map(call => call.op), ['create', 'exec', 'close']);
  assert.equal(h.calls[0].type, 'shell');
  assert.equal(h.calls[1].command, command);
  assert.equal(h.calls[1].timeout, 42000);
  assert.equal(h.calls[2].id, h.calls[1].id);
});

test('public Android helpers use system-shell output and keep installation timeout', async () => {
  const h = engine({ output: '16\n' });
  assert.equal(await h.run('new SystemManager().getProperty("ro.build.version.sdk")'), '16');
  assert.equal(await h.run('new PackageManager().install("/sdcard/app.apk")'), '16\n');
  const commands = h.calls.filter(call => call.op === 'exec');
  assert.equal(commands[0].command, 'getprop ro.build.version.sdk');
  assert.equal(commands[0].timeout, 15000);
  assert.equal(commands[1].command, 'pm install -r /sdcard/app.apk');
  assert.equal(commands[1].timeout, 60000);
});

test('permission denial, transport failure and timeout reject and close the owned session', async () => {
  for (const [options, error] of [
    [{ exitCode: 1, output: 'Permission denied' }, /code 1: Permission denied/],
    [{ timedOut: true, output: 'partial' }, /timed out: partial/],
    [{ execError: 'Shizuku disconnected' }, /Shizuku disconnected/],
  ]) {
    const h = engine(options);
    await assert.rejects(h.run('executeShell("dumpsys usagestats")'), error);
    assert.equal(h.calls.filter(call => call.op === 'exec').length, 1);
    assert.equal(h.calls.at(-1).op, 'close');
    assert.equal(h.calls.at(-1).id, h.calls[1].id);
  }
});

test('creation failure leaves other sessions alone; cleanup failure remains observable', async () => {
  const unavailable = engine({ createError: 'Shizuku unavailable' });
  await assert.rejects(unavailable.run('executeShell("id")'), /Shizuku unavailable/);
  assert.deepEqual(unavailable.calls.map(call => call.op), ['create']);
  for (const options of [{ closeError: 'close rejected' }, { closeFailure: true }]) {
    const h = engine(options);
    await assert.rejects(h.run('executeShell("id")'), /close rejected|cleanup failed/);
  }
  const denied = engine({ execError: 'command denied', closeError: 'close rejected' });
  await assert.rejects(denied.run('executeShell("id")'), /command denied/);
  assert.equal(denied.logs.length, 1);
  assert.match(denied.logs[0][1].message, /close rejected/);
});

test('concurrent Android helper calls own distinct sessions and close the matching session', async () => {
  const h = engine();
  await h.run('Promise.all([executeShell("echo one"), executeShell("echo two")])');
  const created = h.calls.filter(call => call.op === 'create');
  assert.equal(new Set(created.map(call => call.name)).size, 2);
  assert.ok(created.every(call => call.type === 'shell'));
  const executed = h.calls.filter(call => call.op === 'exec').map(call => call.id).sort();
  const closed = h.calls.filter(call => call.op === 'close').map(call => call.id).sort();
  assert.deepEqual(closed, executed);
});
