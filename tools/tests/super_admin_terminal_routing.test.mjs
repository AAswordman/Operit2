import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { loadSuperAdmin } from './fixtures/super_admin_runtime.mjs';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const androidImplementations = { bash: 'proot', shell: 'android-system' };

/** Reads current production source rather than generated package artifacts. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Loads Super Admin with strict, explicitly typed terminal tools and complete host results. */
function fixture({ platform = 'android', defaultType = 'bash', implementations = androidImplementations, output = 'ok\n', timedOut = false, closeError } = {}) {
  const sessions = new Map();
  const calls = [];
  const files = new Map();
  const errors = [];
  const intermediate = [];
  let chatId = 'chat-a';
  let sessionSequence = 0;
  let now = 0;
  const terminalInfo = {
    platform,
    terminal: implementations[defaultType],
    terminalType: defaultType,
    types: Object.entries(implementations).map(([terminalType, terminal]) => ({ terminal, terminalType, available: true })),
  };
  const context = vm.createContext({
    exports: {},
    Date: class extends Date { /** Supplies a deterministic idle clock. */ static now() { return now; } },
    OPERIT_CLEAN_ON_EXIT_DIR: '/operitapp/data/temp/clean_on_exit',
    /** Supplies the active chat identity without a platform-dependent branch. */
    getChatId() { return chatId; },
    /** Captures the live package execution output separately from its final return value. */
    sendIntermediateResult(event) { intermediate.push(structuredClone(event)); },
    console: {
      /** Keeps command logging out of test output. */
      log() {},
      /** Records errors without changing their propagation. */
      error(...args) { errors.push(args); },
    },
    Tools: {
      Files: {
        /** Records creation of the virtual temporary output directory. */
        async mkdir(path, parents) { calls.push({ operation: 'mkdir', path, parents }); },
        /** Saves oversized command output inside the mocked virtual filesystem. */
        async write(path, content, append) {
          assert.equal(append, false);
          files.set(path, content);
        },
      },
      System: {
        terminal: {
          /** Returns a bounded foreground receipt while the session owns unfinished execution. */
          async run(request) {
            calls.push({ operation: 'observe', observation: { op: 'run', ...structuredClone(request) } });
            const created = await this.open(request);
            const session = [...sessions.values()].find(value => value.sessionId === created.sessionId);
            session.commandRunning = timedOut;
            session.receipt = { ...created, command: request.command, output, exitCode: timedOut ? -1 : 0, timedOut };
            return session.receipt;
          },
          /** Models a fresh PTY process rather than the legacy reusable session API. */
          async open(request) {
            const terminalType = request.terminalType;
            if (!Object.hasOwn(implementations, terminalType)) throw new Error(`Unsupported terminal type: ${terminalType}`);
            const session = { sessionId: `session-${++sessionSequence}`, sessionName: request.sessionName,
              platform, terminal: implementations[terminalType], terminalType, commandRunning: true, isNewSession: true };
            sessions.set(session.sessionId, session);
            return {...session};
          },
          /** Reads the original execution without submitting another command. */
          async wait(sessionId, waitMs) {
            calls.push({ operation: 'wait', sessionId, waitMs });
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            assert.ok(session);
            return { ...session.receipt, timedOut: session.commandRunning, exitCode: session.commandRunning ? -1 : 0 };
          },
          /** Returns capabilities whose default may differ from the selected session. */
          async info() { return terminalInfo; },
          /** Requires exact interpreter selection and keys reusable sessions by name and type. */
          async create(name, terminalType) {
            calls.push({ operation: 'create', name, terminalType });
            if (!Object.hasOwn(implementations, terminalType)) {
              throw new Error(`Unsupported terminal type: ${terminalType}`);
            }
            const key = `${terminalType}:${name}`;
            const isNewSession = !sessions.has(key);
            if (isNewSession) {
              sessions.set(key, { sessionId: `session-${++sessionSequence}`, sessionName: name, platform, terminal: implementations[terminalType], terminalType });
            }
            return { ...sessions.get(key), isNewSession };
          },
          /** Closes only the requested session and preserves host failures. */
          async close(sessionId) {
            calls.push({ operation: 'close', sessionId });
            if (closeError) throw closeError;
            for (const [key, session] of sessions) {
              if (session.sessionId === sessionId) {
                sessions.delete(key);
                return { sessionId, success: true, message: 'Session closed' };
              }
            }
            throw new Error(`Terminal session not found: ${sessionId}`);
          },
          /** Atomically enforces identity and host idleness for cleanup. */
          async closeIfIdle(sessionId, sessionName) {
            calls.push({ operation: 'closeIfIdle', sessionId, sessionName });
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            assert.ok(session);
            assert.ok(session.sessionName === sessionName || /^\d+$/.test(session.sessionName.slice(`${sessionName}_generation_`.length)) && session.sessionName.startsWith(`${sessionName}_generation_`));
            if (session.commandRunning) throw new Error('Command in progress');
            return this.close(sessionId);
          },
          /** Returns host state without marking the plugin's automatic checks as user activity. */
          async screen(sessionId) {
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            if (!session) throw new Error('Session does not exist');
            return { ...session, rows: 24, cols: 80, content: output };
          },
          /** Models successful interactive input independently of command execution. */
          async input() { return 'Input accepted'; },
          /** Returns the actual selected session's identity with the command output. */
          async exec(...args) {
            const [sessionId, command] = args;
            calls.push({ operation: 'exec', args });
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            assert.ok(session, 'Execution must target an explicitly created session');
            return { ...session, command, output, exitCode: 0, timedOut };
          },
          /** Uses the cancellable observation transport for foreground commands. */
          async execStreaming(...args) {
            const [sessionId, command] = args;
            calls.push({ operation: 'execStreaming', args });
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            assert.ok(session, 'Execution must target an explicitly created session');
            return { ...session, command, output, exitCode: 0, timedOut };
          },
        },
      },
    },
  });
  const runtime = loadSuperAdmin(context);
  return {
    context, ...runtime, sessions, calls, files, errors, intermediate, terminalInfo,
    /** Advances the clock without sleeping. */
    setTime(value) { now = value; },
    /** Changes the active chat to exercise session identity isolation. */
    setChatId(value) { chatId = value; },
  };
}


/** Exercises the shared profile tools independently of platform-specific names. */
test('one execution entry point advertises discovery without platform presets', () => {
  const metadata = JSON.parse(source('plugins/packages/buildin/super_admin/src/tools.ts').match(/\/\* METADATA\s*([\s\S]*?)\*\//)[1]);
  assert.equal(metadata.states, undefined);
  assert.ok(metadata.tools.some(tool => tool.name === 'terminal'));
  assert.ok(metadata.tools.some(tool => tool.name === 'list_terminals'));
  assert.ok(metadata.tools.some(tool => tool.name === 'close'));
  assert.ok(metadata.tools.some(tool => tool.name === 'terminal_wait'));
  assert.equal(typeof fixture().tools.terminal_wait, 'function');
  assert.deepEqual(metadata.tools.find(tool => tool.name === 'terminal').parameters.map(parameter => parameter.name), ['command', 'profileId']);
  for (const name of ['powershell', 'bash', 'zsh', 'shell']) assert.ok(!metadata.tools.some(tool => tool.name === name));
});

/** Builds independent profiles sharing Bash to catch accidental interpreter-only selection. */
function profiles() {
  return [
    {id:'ubuntu_a',name:'Ubuntu A',terminal:'proot',terminalType:'bash',available:true},
    {id:'ubuntu_b',name:'Ubuntu B',terminal:'proot',terminalType:'bash',available:true},
    {id:'system',name:'System shell',terminal:'android-system',terminalType:'shell',available:true},
  ];
}

/** Creates a production-plugin fixture whose mock host routes by exact profile ID. */
function managedFixture(options = {}) {
  const f = fixture(options);
  f.terminalInfo.defaultProfileId = 'ubuntu_a';
  f.terminalInfo.profiles = profiles();
  // The fixture's old interpreter model remains useful for session lifetime and output handling.
  const originalRun = f.context.Tools.System.terminal.run;
  const originalOpen = f.context.Tools.System.terminal.open;
  f.context.Tools.System.terminal.run = async function(request) {
    const profile = f.terminalInfo.profiles.find(value => value.id === request.profileId);
    f.calls.push({operation:'selectedProfile', profileId:request.profileId});
    return originalRun.call(this, {...request, sessionName:`${request.sessionName}_profile_${profile.id}`, terminalType:profile.terminalType});
  };
  f.context.Tools.System.terminal.open = async function(request) {
    if (request.terminalType !== undefined) return originalOpen.call(this, request);
    const profile = f.terminalInfo.profiles.find(value => value.id === request.profileId);
    return originalOpen.call(this, {...request, sessionName:`${request.sessionName}_profile_${profile.id}`, terminalType:profile.terminalType});
  };
  return f;
}

/** Verifies default changes affect new commands and separate same-shell configurations keep context isolated. */
test('default and explicit profiles select separate same-interpreter sessions', async () => {
  const f = managedFixture();
  const a = await f.tools.terminal({command:'pwd'});
  const b = await f.tools.terminal({command:'pwd',profileId:'ubuntu_b'});
  const again = await f.tools.terminal({command:'pwd',profileId:'ubuntu_a'});
  assert.notEqual(a.sessionId,b.sessionId);
  assert.notEqual(a.sessionId,again.sessionId);
  assert.equal(again.context_preserved,false);
  f.terminalInfo.defaultProfileId = 'ubuntu_b';
  assert.notEqual((await f.tools.terminal({command:'pwd'})).sessionId,b.sessionId);
  assert.deepEqual(f.calls.filter(call => call.operation === 'selectedProfile').map(call => call.profileId),
    ['ubuntu_a','ubuntu_b','ubuntu_a','ubuntu_b']);
  const discovered = await f.tools.list_terminals();
  assert.equal(discovered.profiles.length,3);
  assert.equal(discovered.defaultProfileId,'ubuntu_b');
});

/** Persistent interactive programs remain protected even after long periods without tool activity. */
test('interactive open preserves the live shell until explicit close', async () => {
  const f = managedFixture();
  const interactive = await f.tools.terminal_open({profileId:'ubuntu_a'});
  assert.equal(interactive.mode,'interactive');
  assert.equal(interactive.context_preserved,true);
  const command = await f.tools.terminal({command:'pwd',profileId:'ubuntu_a'});
  assert.notEqual(interactive.sessionId,command.sessionId);
  f.setTime(600000); await f.hooks[0].function();
  assert.equal(f.sessions.size,1);
  assert.equal([...f.sessions.values()][0].sessionId,interactive.sessionId);
  await f.tools.input({sessionId:interactive.sessionId,input:'c',control:'ctrl'});
  await f.tools.close({sessionId:interactive.sessionId});
  assert.equal(f.sessions.size,0);
});

/** Checks unknown and unavailable IDs fail before any execution reaches the host. */
test('invalid profile selection fails without starting a command', async () => {
  const f = managedFixture();
  await assert.rejects(f.tools.terminal({command:'id',profileId:'missing'}),/not registered/);
  f.terminalInfo.profiles[0].available = false;
  await assert.rejects(f.tools.terminal({command:'id'}),/unavailable/);
  assert.equal(f.calls.length,0);
});

/** Preserves exact-profile identity and large output with a bounded foreground wait. */
test('profile execution returns session identity and output with a bounded wait', async () => {
  const f = managedFixture({output:'x'.repeat(12001)});
  const first = await f.tools.terminal({command:'printf large'});
  assert.equal(first.status, 'completed');
  assert.equal(f.files.get(first.output_saved_to).length,12001);
  const system = await f.tools.terminal({command:'id',profileId:'system'});
  assert.equal(system.terminal,'android-system');
  assert.ok(f.calls.filter(call => call.operation === 'observe').every(call => call.observation.waitMs === 1000));
  const result = await f.tools.close({sessionId:first.sessionId});
  assert.equal(result.success,true);
  assert.notEqual((await f.tools.terminal({command:'pwd'})).sessionId,first.sessionId);
});

/** Keeps screen, input and original-result observation available after foreground waiting expires. */
test('a running command returns control and can be observed without submitting it again', async () => {
  const f = managedFixture({timedOut:true});
  const running = await f.tools.terminal({command:'read answer'});
  assert.equal(running.status, 'running');
  assert.equal(running.exitCode, null);
  assert.equal(running.commandRunning, true);
  assert.equal((await f.tools.get_screen({sessionId:running.sessionId})).commandRunning, true);
  await f.tools.input({sessionId:running.sessionId,input:'answer',control:'enter'});
  assert.equal((await f.tools.terminal_wait({sessionId:running.sessionId})).status, 'running');
  [...f.sessions.values()][0].commandRunning = false;
  const completed = await f.tools.terminal_wait({sessionId:running.sessionId});
  assert.equal(completed.status, 'completed');
  assert.equal(completed.exitCode, 0);
  assert.equal(f.calls.filter(call => call.operation === 'observe').length, 1);
  assert.ok(f.calls.filter(call => call.operation === 'wait').every(call => call.waitMs === 1000));
});

/** Keeps SDK scripts and internal metadata out of every AI receipt, including waits and saved output. */
test('terminal AI results project execution data instead of forwarding SDK receipts', async () => {
  const command = `cat <<'SCRIPT'\n${'PRIVATE_SCRIPT_BODY\n'.repeat(2000)}SCRIPT`;
  for (const output of ['ok\n', '雪🌍\n'.repeat(4000)]) {
    for (const timedOut of [false, true]) {
      const f = managedFixture({output, timedOut});
      const run = f.context.Tools.System.terminal.run;
      f.context.Tools.System.terminal.run = async function(request) {
        const receipt = await run.call(this, request);
        receipt.internalMetadata = {command: request.command};
        return receipt;
      };
      const first = await f.tools.terminal({command});
      const session = f.sessions.get(first.sessionId);
      assert.equal(session.receipt.command, command, 'The SDK receipt must retain the original script');
      const waiting = await f.tools.terminal_wait({sessionId:first.sessionId});
      session.commandRunning = false;
      const completed = await f.tools.terminal_wait({sessionId:first.sessionId});
      for (const result of [first, waiting, completed]) {
        assert.equal(Object.hasOwn(result, 'command'), false);
        assert.equal(Object.hasOwn(result, 'internalMetadata'), false);
        assert.equal(JSON.stringify(result).includes('PRIVATE_SCRIPT_BODY'), false);
        assert.equal(result.sessionId, first.sessionId);
        assert.equal(result.platform, session.platform);
        assert.equal(result.terminal, session.terminal);
        assert.equal(result.terminalType, session.terminalType);
        if (output.length > 12000) {
          assert.equal(result.output, '(saved_to_file)');
          assert.equal(result.output_chars, output.length);
          assert.equal(f.files.get(result.output_saved_to), output);
        } else {
          assert.equal(result.output, output);
        }
      }
      assert.equal(first.status, timedOut ? 'running' : 'completed');
      assert.equal(first.exitCode, timedOut ? null : 0);
      assert.equal(completed.status, 'completed');
      assert.equal(completed.exitCode, 0);
      assert.equal(completed.timedOut, false);
      assert.equal(f.calls.filter(call => call.operation === 'observe').length, 1);
    }
  }
});

/** Preserves real command output even when it intentionally prints the submitted script. */
test('terminal projection does not strip script text from actual stdout', async () => {
  const command = 'printf "SCRIPT_OUTPUT"';
  const f = managedFixture({output:command});
  const result = await f.tools.terminal({command});
  assert.equal(result.output, command);
  assert.equal(Object.hasOwn(result, 'command'), false);
});

/** A pending original command must not prevent another tool from sending the input it needs. */
test('input and screen remain independent from a pending command result wait', async () => {
  const f = managedFixture();
  const started = await f.tools.terminal({command:'read answer'});
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const original = f.context.Tools.System.terminal.wait;
  f.context.Tools.System.terminal.wait = async function(...args) {
    await gate; return original.apply(this, args);
  };
  let completed = false;
  const running = f.tools.terminal_wait({sessionId:started.sessionId}).then(value => { completed = true; return value; });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(completed, false);
  await f.tools.get_screen({sessionId:started.sessionId});
  await f.tools.input({sessionId:started.sessionId,input:'answer',control:'enter'});
  release();
  assert.equal((await running).status, 'completed');
});

/** Proves cleanup runs from the durable Host hook without another AI tool invocation. */
test('the plugin closes an idle session at ten minutes and recreates its context on next use', async () => {
  const f = managedFixture();
  assert.equal(f.hooks.length, 1);
  assert.equal(f.hooks[0].trigger.intervalMs, 60000);
  const first = await f.tools.terminal({command:'pwd'});
  f.setTime(599999);
  await f.hooks[0].function();
  assert.equal(f.sessions.size, 1);
  f.setTime(600000);
  await f.hooks[0].function();
  assert.equal(f.sessions.size, 0);
  assert.equal(f.calls.filter(call => call.operation === 'closeIfIdle').length, 1);
  assert.notEqual((await f.tools.terminal({command:'pwd'})).sessionId, first.sessionId);
});

/** Restarts the inactivity deadline for commands, screen reads and interactive input. */
test('terminal interactions reset only the used session deadline', async () => {
  const f = managedFixture();
  const a = await f.tools.terminal({command:'pwd'});
  const b = await f.tools.terminal({command:'pwd',profileId:'ubuntu_b'});
  f.setTime(500000);
  await f.tools.get_screen({sessionId:a.sessionId});
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  assert.equal([...f.sessions.values()][0].sessionId, a.sessionId);
  assert.ok(f.calls.some(call => call.operation === 'closeIfIdle' && call.sessionId === b.sessionId));
  f.setTime(1000000);
  await f.tools.input({sessionId:a.sessionId,input:'pwd'});
  f.setTime(1500000);
  await f.tools.terminal({command:'pwd'});
  f.setTime(2099999);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(2100000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Keeps a silent running command protected even after many inactivity intervals. */
test('a returned running command stays protected and receives a full observed idle period', async () => {
  const f = managedFixture({timedOut:true});
  await f.tools.terminal({command:'silent build'});
  f.setTime(3600000);
  await f.main.onClock();
  assert.equal(f.calls.filter(call => call.operation === 'closeIfIdle').length, 0);
  [...f.sessions.values()][0].commandRunning = false;
  await f.main.onClock();
  f.setTime(4199999);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(4200000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(4800000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Ensures one long command does not indefinitely retain completed sessions in the same chat. */
test('an idle generation is reclaimed while another generation is running', async () => {
  const f = managedFixture();
  const first = await f.tools.terminal({command:'pwd'});
  const idle = [...f.sessions.values()][0];
  idle.commandRunning = true;
  const running = await f.tools.terminal({command:'build'});
  const busy = [...f.sessions.values()].find(session => session.sessionId === running.sessionId);
  busy.commandRunning = true;
  idle.commandRunning = false;
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  assert.notEqual([...f.sessions.values()][0].sessionId, first.sessionId);
  busy.commandRunning = false;
  await f.tools.terminal_wait({sessionId:busy.sessionId});
  f.setTime(1200000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(1800000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Protects an in-flight interaction and counts idleness from its completion. */
test('cleanup does not interrupt a pending screen read', async () => {
  const f = managedFixture();
  const result = await f.tools.terminal({command:'pwd'});
  const original = f.context.Tools.System.terminal.screen;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.screen = async function(sessionId) {
    await gate;
    return original.call(this,sessionId);
  };
  f.setTime(500000);
  const reading = f.tools.get_screen({sessionId:result.sessionId});
  await new Promise(resolve => setImmediate(resolve));
  f.setTime(1100000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  release();
  await reading;
  f.context.Tools.System.terminal.screen = original;
  f.setTime(1699999);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(1700000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Reproduces the diagnostic's pending cleanup screen and checks both profiles can still execute. */
test('a pending idle screen probe cannot block commands in either profile', async () => {
  const f = managedFixture();
  const first = await f.tools.terminal({command:'pwd'});
  const screen = f.context.Tools.System.terminal.screen;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.screen = async function(sessionId) {
    if (sessionId === first.sessionId) await gate;
    return screen.call(this, sessionId);
  };
  f.setTime(600000);
  const cleanup = f.main.onClock();
  await new Promise(setImmediate);
  const completed = [];
  const commands = ['ubuntu_a', 'ubuntu_b'].map(profileId =>
    f.tools.terminal({command:'echo alive', profileId}).then(result => {
      completed.push(profileId);
      return result;
    }));
  try {
    await new Promise(setImmediate);
    assert.deepEqual([...completed].sort(), ['ubuntu_a', 'ubuntu_b']);
  } finally {
    release();
    await Promise.all([cleanup, ...commands]);
  }
  assert.ok(![...f.sessions.values()].some(session => session.sessionId === first.sessionId),
    'New processes do not reuse the idle process observed by this probe');
});

/** Reproduces a suspended close without letting it hold a different profile's session lease. */
test('a pending idle close only delays acquisition of its own session family', async () => {
  const f = managedFixture();
  const first = await f.tools.terminal({command:'pwd'});
  const close = f.context.Tools.System.terminal.closeIfIdle;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.closeIfIdle = async function(...args) {
    await gate;
    return close.apply(this, args);
  };
  f.setTime(600000);
  const cleanup = f.main.onClock();
  await new Promise(setImmediate);
  const completed = [];
  const commands = ['ubuntu_a', 'ubuntu_b'].map(profileId =>
    f.tools.terminal({command:'echo alive', profileId}).then(result => {
      completed.push(profileId);
      return result;
    }));
  try {
    await new Promise(setImmediate);
    assert.deepEqual([...completed], ['ubuntu_b']);
  } finally {
    release();
    await Promise.all([cleanup, ...commands]);
  }
  assert.ok(![...f.sessions.values()].some(session => session.sessionId === first.sessionId));
});

/** Keeps a delayed observation single-flight while allowing another session's reclamation. */
test('overlapping idle sweeps do not duplicate a pending probe or delay another profile cleanup', async () => {
  const f = managedFixture();
  const a = await f.tools.terminal({command:'pwd'});
  const b = await f.tools.terminal({command:'pwd',profileId:'ubuntu_b'});
  const screen = f.context.Tools.System.terminal.screen;
  const probes = [];
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.screen = async function(sessionId) {
    probes.push(sessionId);
    if (sessionId === a.sessionId) await gate;
    return screen.call(this, sessionId);
  };
  f.setTime(600000);
  const firstSweep = f.main.onClock();
  const secondSweep = f.main.onClock();
  try {
    await new Promise(setImmediate);
    assert.equal(probes.filter(id => id === a.sessionId).length, 1);
    assert.equal(probes.filter(id => id === b.sessionId).length, 1);
    assert.ok(![...f.sessions.values()].some(session => session.sessionId === b.sessionId));
  } finally {
    release();
    await Promise.all([firstSweep, secondSweep]);
  }
});

/** Invalidates old probes by activity revision even when the session becomes idle again. */
test('a delayed idle probe cannot close a session used and released since its snapshot', async () => {
  const f = managedFixture();
  const first = await f.tools.terminal({command:'pwd'});
  const screen = f.context.Tools.System.terminal.screen;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.screen = async function(sessionId) {
    await gate;
    return screen.call(this, sessionId);
  };
  f.setTime(600000);
  const cleanup = f.main.onClock();
  await new Promise(setImmediate);
  try {
    await f.tools.input({sessionId:first.sessionId,input:'c',control:'ctrl'});
    f.setTime(1200000);
  } finally {
    release();
    await cleanup;
  }
  assert.equal(f.sessions.size, 1, 'The stale screen must not authorize closing the reused session');
  f.context.Tools.System.terminal.screen = screen;
  await f.main.onClock();
  assert.equal(f.sessions.size, 0, 'A fresh probe can reclaim it after the new idle period');
});

/** Releases the close barrier even when the Host rejects closing the session. */
test('a failed idle close releases waiting acquisition and preserves the original error', async () => {
  const f = managedFixture();
  const first = await f.tools.terminal({command:'pwd'});
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.closeIfIdle = async function() {
    await gate;
    throw new Error('Host refused close');
  };
  f.setTime(600000);
  const cleanup = f.main.onClock();
  await new Promise(setImmediate);
  const command = f.tools.terminal({command:'echo alive'});
  release();
  await cleanup;
  assert.notEqual((await command).sessionId, first.sessionId);
  assert.match(f.errors[0][0], /Host refused close/);
  f.context.Tools.System.terminal.closeIfIdle = async function() {
    throw new Error('Host refused close');
  };
  f.setTime(1200000);
  await f.main.onClock();
  assert.equal(f.errors.length, 2, 'Each of the two distinct sessions reports its own close failure once');
  f.setTime(1800000);
  await f.main.onClock();
  assert.equal(f.errors.length, 2, 'Repeated failures on the same sessions are not logged again');
});

/** Checks independent host activity after this plugin's stream has already completed. */
test('a host-running command is not closed and restarts idleness when it finishes', async () => {
  const f = managedFixture();
  await f.tools.terminal({command:'pwd'});
  const session = [...f.sessions.values()][0];
  session.commandRunning = true;
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  session.commandRunning = false;
  f.setTime(1200000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(1800000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Keeps conditional-close rejection visible without escalating to an unconditional close. */
test('a command starting between observation and close is preserved', async () => {
  const f = managedFixture();
  await f.tools.terminal({command:'pwd'});
  f.context.Tools.System.terminal.closeIfIdle = async () => { throw new Error('Command in progress'); };
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  assert.equal(f.calls.some(call => call.operation === 'close'), false);
  assert.match(f.errors[0][0], /Command in progress/);
});

/** Ensures plugin ownership excludes terminals created directly in the workspace. */
test('manual terminals are excluded and explicit close removes the cleanup record', async () => {
  const f = managedFixture();
  const manual = await f.context.Tools.System.terminal.create('workspace-tab', 'bash');
  const owned = await f.tools.terminal({command:'pwd'});
  await f.tools.close({sessionId:owned.sessionId});
  f.setTime(600000);
  await f.main.onClock();
  assert.deepEqual([...f.sessions.values()].map(session => session.sessionId), [manual.sessionId]);
  assert.equal(f.calls.some(call => call.operation === 'closeIfIdle'), false);
});

/** Keeps the new container discoverable while preserving all existing tool names. */
test('the ToolPkg manifest owns the existing super_admin tool namespace', () => {
  const manifest = JSON.parse(source('plugins/packages/buildin/super_admin/manifest.json'));
  assert.equal(manifest.main, 'dist/main.js');
  assert.deepEqual(manifest.subpackages, [{id:'super_admin',entry:'dist/tools.js'}]);
  assert.equal(manifest.enabled_by_default, true);
});
