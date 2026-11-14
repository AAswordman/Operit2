import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { installScopedHostRuntime, installGeneratedToolsRuntime } from './fixtures/scoped_host_runtime.mjs';

/** Exercises the generated SDK and version gates over the production structured transport. */
function engine(version = '1.0.0', respond) {
  const calls = [];
  const context = vm.createContext({state: 'android'});
  const registry = installScopedHostRuntime(context);
  context.__operitNativeCallToolStructured = registry.binding((requestId, type, name, params) => {
    calls.push({type, name, params: structuredClone(params)});
    queueMicrotask(async () => {
      try {
        const data = await respond(name, params);
        registry.settle(requestId, {success: true, data}, false);
      } catch (error) {
        registry.settle(requestId, {success: false, message: error.message}, false);
      }
    });
  });
  installGeneratedToolsRuntime(context, version);
  context.__operitGetCallState = () => ({params: {__operit_toolpkg_api_version: context.version, __operit_package_state: context.state}});
  vm.runInContext('__operitInstallV1Runtime(globalThis)', context);
  return {context, calls, run: code => vm.runInContext(code, context)};
}

/** Supplies native-shaped receipts without executing Android device commands. */
function host(options = {}) {
  let next = 0;
  return async (name, params) => {
    if (name === 'create_terminal_session') {
      if (options.createError) throw new Error(options.createError);
      assert.equal(params.type, 'shell');
      return {sessionId: 'android-shell-' + (++next), sessionName: params.session_name,
        platform: 'android', terminal: 'android-system', terminalType: 'shell', isNewSession: true};
    }
    if (name === 'execute_in_terminal_session') {
      if (options.execError) throw new Error(options.execError);
      return {sessionId: params.session_id, command: params.command, platform: 'android',
        terminal: 'android-system', terminalType: 'shell', output: options.output ?? 'raw dumpsys output\n',
        exitCode: options.exitCode ?? 0, timedOut: options.timedOut ?? false};
    }
    if (name === 'close_terminal_session') {
      if (options.closeError) throw new Error(options.closeError);
      return {sessionId: params.session_id, success: !options.closeFailure, message: 'closed'};
    }
    throw new Error('Unexpected tool: ' + name);
  };
}

for (const version of ['1.0.0', '1.0.1']) {
  test(`${version}: legacy shell selects Android system shell, preserves command/output and closes`, async () => {
    const h = engine(version, host());
    const command = "dumpsys usagestats | head -20; echo '/sdcard/a b'";
    h.context.command = command;
    const receipt = await h.run('Tools.System.shell(command)');
    assert.equal(receipt.output, 'raw dumpsys output\n');
    assert.equal(receipt.command, command);
    assert.equal(receipt.exitCode, 0);
    assert.equal(String(receipt), receipt.output);
    assert.deepEqual(h.calls.map(c => c.name), ['create_terminal_session', 'execute_in_terminal_session', 'close_terminal_session']);
    assert.equal(h.calls[1].params.command, command);
    assert.equal(h.calls[1].params.timeout_ms, 20000);
    assert.equal(h.calls[2].params.session_id, h.calls[1].params.session_id);
    assert.equal(h.run('typeof Tools.System.terminal'), 'object');
    assert.equal(h.run('typeof Tools.System.terminal.create'), 'function');
  });
}

test('nonzero exit, timeout and host rejection throw and still close the owned session', async () => {
  for (const [options, message] of [
    [{exitCode: 1, output: 'Permission denied'}, /code 1: Permission denied/],
    [{timedOut: true, output: 'partial'}, /timed out: partial/],
    [{execError: 'Shizuku disconnected'}, /Shizuku disconnected/],
    [{execError: 'original failure', closeError: 'cleanup failure'}, /original failure/],
  ]) {
    const h = engine('1.0.0', host(options));
    await assert.rejects(h.run('Tools.System.shell("pm disable-user --user 0 example.app")'), message);
    assert.equal(h.calls.at(-1).name, 'close_terminal_session');
    assert.equal(h.calls.filter(c => c.name === 'execute_in_terminal_session').length, 1);
  }
});

test('cleanup failure cannot report success; creation failure never executes or closes another session', async () => {
  for (const options of [{closeError: 'close rejected'}, {closeFailure: true}]) {
    const h = engine('1.0.0', host(options));
    await assert.rejects(h.run('Tools.System.shell("echo ok")'), /close rejected|cleanup failed/);
  }
  const h = engine('1.0.0', host({createError: 'permission unavailable'}));
  await assert.rejects(h.run('Tools.System.shell("echo ok")'), /permission unavailable/);
  assert.equal(h.calls.length, 1);
});

test('invalid arguments and non-Android calls fail before admission; v2 cannot reuse a retained alias', async () => {
  const h = engine('1.0.0', host());
  for (const value of ['null', '42', '""', '"  "', '"a\\0b"']) {
    await assert.rejects(h.run(`Tools.System.shell(${value})`), /nonblank command/);
  }
  h.context.state = 'linux';
  await assert.rejects(h.run('Tools.System.shell("echo ok")'), /requires Android/);
  h.run('var retainedShell = Tools.System.shell');
  h.context.version = '2.0.0';
  assert.equal(h.run('typeof Tools.System.shell'), 'undefined');
  assert.throws(() => h.run('retainedShell("echo ok")'), /no implementation|requires/);
  assert.equal(h.calls.length, 0);
});

test('concurrent calls own distinct sessions and close the matching session', async () => {
  const h = engine('1.0.0', host());
  await h.run('Promise.all([Tools.System.shell("echo one"), Tools.System.shell("echo two")])');
  const creates = h.calls.filter(c => c.name === 'create_terminal_session');
  assert.equal(new Set(creates.map(c => c.params.session_name)).size, 2);
  const executed = h.calls.filter(c => c.name === 'execute_in_terminal_session').map(c => c.params.session_id).sort();
  const closed = h.calls.filter(c => c.name === 'close_terminal_session').map(c => c.params.session_id).sort();
  assert.deepEqual(closed, executed);
  assert.equal(new Set(closed).size, 2);
});

/** Uses the downloaded package unchanged when available; no market code is vendored. */
test('marketplace Gentle Guardian preserves shell text and recognizes permission failures', {
  skip: !process.env.OPERIT_MARKET_GUARDIAN_DIR,
}, async () => {
  const file = join(process.env.OPERIT_MARKET_GUARDIAN_DIR, 'packages/gentle_guardian_tools.js');
  const code = readFileSync(file, 'utf8');
  const start = code.indexOf('async function execShell(cmd)');
  const end = code.indexOf('\nasync function hideApp', start);
  assert.ok(start > 0 && end > start);
  const h = engine('1.0.0', host());
  h.run(code.slice(start, end));
  const receipt = await h.run('execShell("dumpsys usagestats")');
  assert.equal(receipt.success, true);
  const extractor = code.slice(code.indexOf('function extractShellText(out)'), code.indexOf('// 解析', code.indexOf('function extractShellText(out)')));
  h.run(extractor);
  h.context.shellOutput = receipt.output;
  assert.equal(h.run('extractShellText(shellOutput)'), 'raw dumpsys output\n');
  const denied = engine('1.0.0', host({exitCode: 1, output: 'Permission denied'}));
  denied.run(code.slice(start, end));
  const failure = await denied.run('execShell("pm disable-user --user 0 example.app")');
  assert.equal(failure.success, false);
  assert.match(failure.message, /Permission denied/);
  assert.equal(denied.calls.filter(c => c.name === 'execute_in_terminal_session').length, 1);
});
