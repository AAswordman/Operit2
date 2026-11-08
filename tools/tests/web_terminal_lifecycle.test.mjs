import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', import.meta.url), 'utf8');

/** Runs the production browser command protocol against a controllable virtual serial port. */
function fixture() {
  const commands = source.slice(source.indexOf('  function linuxVmSession('), source.indexOf('  /** Redraws the single PTY progress line'));
  const api = source.slice(source.indexOf('      startPtyCommand('), source.indexOf('      /** Records terminal dimensions requested for one browser-local Linux VM terminal. */'));
  const sends = [], timers = [];
  const session = {
    id: 'web-terminal', state: 'running', emulator: { serial_send_bytes: (...args) => sends.push(args) },
    activeCommand: null, pendingCommands: [], managedExecution: null, screen: [], workingDir: '/',
    output: new Uint8Array(4096), outputLength: 0,
  };
  const context = vm.createContext({
    TextEncoder, TextDecoder, Uint8Array, session,
    textEncoder: new TextEncoder(), textDecoder: new TextDecoder(),
    linuxVmSessions: new Map([[session.id, session]]), linuxVmOutputLimit: 4 * 1024 * 1024,
    linuxVmCommandIndex: 0,
    bytesToBase64: bytes => Buffer.from(bytes).toString('base64'),
    base64ToBytes: text => new Uint8Array(Buffer.from(text, 'base64')),
    clearTimeout() {}, setTimeout(callback, delay) { timers.push({ callback, delay }); return timers.length; },
  });
  vm.runInContext(stripTypeScriptTypes(`${commands}\nconst api = {${api}}; globalThis.api = api;`), context);
  return { session, sends, timers, api: context.api,
    output(text) {
      for (const byte of new TextEncoder().encode(text)) {
        context.byte = byte;
        vm.runInContext('consumeLinuxVmCommandByte(session, session.activeCommand, byte)', context);
      }
    },
  };
}

test('browser managed execution returns immediately, streams output and retains the original receipt', async () => {
  const f = fixture();
  assert.equal(f.api.startPtyCommand(f.session.id, 'sleep 30; printf finished'), undefined);
  assert.equal(f.timers.length, 0, 'session-owned commands must have no cancellation timer');
  const token = f.session.activeCommand.token;
  f.output(`\x1b]1337;OPERIT_COMMAND_START;${token}\x07starting\n`);
  assert.equal(new TextDecoder().decode(f.api.pollPtyCommand(f.session.id).output), 'starting\n');
  assert.equal(f.api.sessionState(f.session.id).commandRunning, true);
  assert.throws(() => f.api.startPtyCommand(f.session.id, 'cd /'), /busy/);
  f.output(`finished\n\x1b]1337;OPERIT_COMMAND_END;${token};7;${Buffer.from('/workspace').toString('base64')}\x07`);
  await Promise.resolve();
  const receipt = f.api.pollPtyCommand(f.session.id).result;
  assert.equal(receipt.exitCode, 7);
  assert.equal(receipt.timedOut, false);
  assert.match(receipt.output, /starting\nfinished/);
  const screen = f.api.sessionState(f.session.id);
  assert.equal(screen.workingDir, '/workspace');
  assert.equal(screen.commandRunning, false);
  assert.match(screen.content, /finished/);
  assert.equal(f.sends.length, 1, 'polls must never send a marker command or Ctrl+C');
});

test('browser session closure rejects the retained command and bounded capture keeps running', async () => {
  const f = fixture();
  f.api.startPtyCommand(f.session.id, 'long-running');
  const command = f.session.activeCommand;
  command.output = new Uint8Array(4 * 1024 * 1024);
  command.outputLength = command.output.length;
  f.output(`\x1b]1337;OPERIT_COMMAND_START;${command.token}\x07more`);
  assert.ok(command.outputLength < command.output.length);
  assert.equal(f.api.sessionState(f.session.id).commandRunning, true);
  command.reject(new Error('session closed'));
  await Promise.resolve();
  assert.match(f.api.pollPtyCommand(f.session.id).error, /session closed/);
  assert.equal(f.timers.length, 0);
});
