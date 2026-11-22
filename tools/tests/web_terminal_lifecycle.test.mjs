import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', import.meta.url), 'utf8');
const workerSource = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_worker.ts', import.meta.url), 'utf8');

/** Connects the production filesystem channel to a manually dispatched OPFS producer. */
function filesystemFixture() {
  const context = vm.createContext({
    SharedArrayBuffer, Atomics, Int32Array, Uint8Array,
    MessagePack: {
      encode: value => new TextEncoder().encode(JSON.stringify(value)),
      decode: bytes => JSON.parse(new TextDecoder().decode(bytes)),
    },
    workerGlobal: { __operitHost: { terminalFilesystem: { read: name => `file:${name}` } } },
    errorMessage: error => String(error),
    setTimeout() { throw new Error('filesystem channel must never poll'); },
  });
  const client = source.slice(source.indexOf('  let terminalFilesystemBuffer:'), source.indexOf('  const textEncoder ='));
  const producer = workerSource.slice(workerSource.indexOf('function serviceTerminalFilesystem()'), workerSource.indexOf('/** Keeps filesystem I/O live'));
  vm.runInContext(stripTypeScriptTypes(`${client}
    terminalFilesystemBuffer = new SharedArrayBuffer(4096);
    const terminalFilesystemHeader = new Int32Array(terminalFilesystemBuffer, 0, 3);
    const terminalFilesystemPayload = new Uint8Array(terminalFilesystemBuffer, 12);
    ${producer}
    globalThis.call = callTerminalFilesystem;
    globalThis.service = serviceTerminalFilesystem;
    globalThis.header = terminalFilesystemHeader;
  `), context);
  return context;
}

test('filesystem channel serializes queued requests through producer notifications', async () => {
  const f = filesystemFixture();
  const first = f.call('read', ['one']), second = f.call('read', ['two']), third = f.call('read', ['three']);
  assert.equal(Atomics.load(f.header, 0), 1);
  f.service();
  assert.equal(await first, 'file:one');
  await new Promise(setImmediate);
  f.service();
  assert.equal(await second, 'file:two');
  await new Promise(setImmediate);
  f.service();
  assert.equal(await third, 'file:three');
  assert.equal(Atomics.load(f.header, 0), 0);
  assert.equal(Atomics.load(f.header, 2), 3);
});

test('filesystem owner failure wakes active and queued callers without releasing the dead channel', async () => {
  const f = filesystemFixture();
  const active = assert.rejects(f.call('read', ['active']), /owner stopped/);
  const queued = assert.rejects(f.call('read', ['queued']), /owner stopped/);
  Atomics.store(f.header, 0, -1);
  Atomics.notify(f.header, 0);
  await Promise.all([active, queued]);
  assert.equal(Atomics.load(f.header, 0), -1);
  await assert.rejects(f.call('read', ['later']), /owner stopped/);
});

test('worker Host wait uses one generation signal and never waits after a response races filesystem service', () => {
  const wait = workerSource.slice(workerSource.indexOf('function waitForMainHost('), workerSource.indexOf('/** Calls one UI-thread host method'));
  const header = new Int32Array(new SharedArrayBuffer(12)), control = new Int32Array(new SharedArrayBuffer(8));
  const waits = [];
  const context = vm.createContext({
    terminalFilesystemHeader: header, controlStateIndex: 0, control,
    serviceTerminalFilesystem() { Atomics.store(control, 0, 1); },
    Atomics: { load: Atomics.load, wait: (...args) => waits.push(args) },
  });
  vm.runInContext(stripTypeScriptTypes(`${wait}\nwaitForMainHost(control);`), context);
  assert.equal(waits.length, 0);
  Atomics.store(control, 0, 0);
  context.serviceTerminalFilesystem = () => {};
  context.Atomics.wait = (...args) => { waits.push(args); Atomics.store(control, 0, 1); };
  vm.runInContext('waitForMainHost(control);', context);
  assert.deepEqual(waits[0], [header, 2, 0], 'waiting must use the origin-wide signal with no timeout');
});

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

/** Exercises producer signals before and after arming, including silent close. */
test('browser command signals wake on output, completion and close without polling timers', async () => {
  const f = fixture();
  f.api.startPtyCommand(f.session.id, 'silent-until-output');
  const first = new Int32Array(new SharedArrayBuffer(4));
  f.api.armPtyCommand(f.session.id, first.buffer);
  assert.equal(Atomics.load(first, 0), 0);
  const token = f.session.activeCommand.token;
  f.output(`\x1b]1337;OPERIT_COMMAND_START;${token}\x07first`);
  assert.equal(Atomics.load(first, 0), 1);
  const raced = new Int32Array(new SharedArrayBuffer(4));
  f.api.armPtyCommand(f.session.id, raced.buffer);
  assert.equal(Atomics.load(raced, 0), 1, 'retained output must resolve registration immediately');
  f.api.pollPtyCommand(f.session.id);
  const completed = new Int32Array(new SharedArrayBuffer(4));
  f.api.armPtyCommand(f.session.id, completed.buffer);
  f.output(`\x1b]1337;OPERIT_COMMAND_END;${token};0;Lw==\x07`);
  await Promise.resolve();
  assert.equal(Atomics.load(completed, 0), 1);
  const closed = fixture();
  closed.api.startPtyCommand(closed.session.id, 'silent-command');
  const closing = new Int32Array(new SharedArrayBuffer(4));
  closed.api.armPtyCommand(closed.session.id, closing.buffer);
  closed.session.activeCommand.reject(new Error('closed'));
  await Promise.resolve();
  assert.equal(Atomics.load(closing, 0), 1);
  assert.equal(f.timers.length + closed.timers.length, 0);
});
