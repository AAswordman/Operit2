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
          /** Streams startup and output, retaining the invocation until the host completes it. */
          async *runStream(request) {
            calls.push({ operation: 'observe', observation: { op: 'runStream', ...structuredClone(request) } });
            const created = await this.open(request);
            const session = [...sessions.values()].find(value => value.sessionId === created.sessionId);
            const completion = new Promise(resolve => { session.finish = resolve; });
            session.receipt = { ...created, command: request.command, output, exitCode: 0, timedOut: false,
              internalMetadata: {command: request.command} };
            yield {type:'start', ...created, command:request.command};
            yield {type:'chunk', ...created, command:request.command, chunk:output, chunkIndex:0, receivedChars:output.length};
            if (timedOut) await completion;
            session.commandRunning = false;
            yield {type:'completed', result:session.receipt};
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
test('one execution entry point shares tools with platform-selected path advice', () => {
  const metadata = JSON.parse(source('plugins/packages/buildin/super_admin/src/tools.ts').match(/\/\* METADATA\s*([\s\S]*?)\*\//)[1]);
  assert.equal(metadata.tools.find(tool => tool.name === 'usage_advice').advice, true);
  assert.deepEqual(metadata.states.map(state => state.id), ['windows', 'linux', 'macos', 'android', 'ios', 'ohos', 'web']);
  for (const state of metadata.states) {
    assert.equal(state.condition, `platform.${state.id}`);
    assert.equal(state.inheritTools, true);
    assert.equal(state.tools.length, 1);
    const advice = state.tools[0];
    assert.equal(advice.name, 'platform_paths');
    assert.equal(advice.advice, true);
    assert.deepEqual(advice.parameters, []);
    const platformPaths = [...advice.description.en.matchAll(/\/virtual\/([a-z]+)\//g)].map(match => match[1]);
    assert.ok(platformPaths.every(platform => platform === state.id));
  }
  assert.ok(metadata.tools.some(tool => tool.name === 'terminal'));
  assert.ok(metadata.tools.some(tool => tool.name === 'list_terminals'));
  assert.ok(metadata.tools.some(tool => tool.name === 'close'));
  assert.ok(!metadata.tools.some(tool => tool.name === 'terminal_wait'));
  assert.equal(fixture().tools.terminal_wait, undefined);
  assert.match(metadata.tools.find(tool => tool.name === 'terminal').description.en, /package_wait/);
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
  const originalRun = f.context.Tools.System.terminal.runStream;
  const originalOpen = f.context.Tools.System.terminal.open;
  f.context.Tools.System.terminal.runStream = async function*(request) {
    const profile = f.terminalInfo.profiles.find(value => value.id === request.profileId);
    f.calls.push({operation:'selectedProfile', profileId:request.profileId});
    yield* originalRun.call(this, {...request, sessionName:`${request.sessionName}_profile_${profile.id}`, terminalType:profile.terminalType});
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

/** Preserves exact-profile identity and large output while the package runtime owns waiting. */
test('profile execution streams session identity and output without a terminal wait budget', async () => {
  const f = managedFixture({output:'x'.repeat(12001)});
  const first = await f.tools.terminal({command:'printf large'});
  assert.equal(first.status, 'completed');
  assert.equal(Object.hasOwn(first, 'output'), false);
  assert.equal(f.files.get(first.output_saved_to).length,12001);
  const system = await f.tools.terminal({command:'id',profileId:'system'});
  assert.equal(system.terminal,'android-system');
  assert.ok(f.calls.filter(call => call.operation === 'observe').every(call =>
    call.observation.op === 'runStream' && !Object.hasOwn(call.observation, 'waitMs')));
  const result = await f.tools.close({sessionId:first.sessionId});
  assert.equal(result.success,true);
  assert.notEqual((await f.tools.terminal({command:'pwd'})).sessionId,first.sessionId);
});

/** Keeps one invocation alive and interactive until the original process completes. */
test('a running command streams progress and completes without another terminal invocation', async () => {
  const f = managedFixture({timedOut:true});
  let settled = false;
  const running = f.tools.terminal({command:'read answer'}).then(result => { settled = true; return result; });
  await new Promise(setImmediate);
  assert.equal(settled, false);
  const {sessionId} = f.intermediate[0];
  assert.equal(f.intermediate[0].type, 'start');
  assert.equal(f.intermediate[1].type, 'chunk');
  assert.equal((await f.tools.get_screen({sessionId})).commandRunning, true);
  await f.tools.input({sessionId,input:'answer',control:'enter'});
  f.sessions.get(sessionId).finish();
  const completed = await running;
  assert.equal(completed.status, 'completed');
  assert.equal(completed.exitCode, 0);
  assert.equal(f.calls.filter(call => call.operation === 'observe').length, 1);
});

/** Keeps SDK scripts and internal metadata out of streamed events and final AI receipts. */
test('terminal AI results project execution data instead of forwarding SDK receipts', async () => {
  const command = `cat <<'SCRIPT'\n${'PRIVATE_SCRIPT_BODY\n'.repeat(2000)}SCRIPT`;
  for (const output of ['ok\n', '雪🌍\n'.repeat(4000)]) {
    const f = managedFixture({output});
    const first = await f.tools.terminal({command});
    const session = f.sessions.get(first.sessionId);
    assert.equal(session.receipt.command, command, 'The SDK receipt must retain the original script');
    for (const result of [first, ...f.intermediate]) {
      assert.equal(Object.hasOwn(result, 'command'), false);
      assert.equal(Object.hasOwn(result, 'internalMetadata'), false);
      assert.equal(JSON.stringify(result).includes('PRIVATE_SCRIPT_BODY'), false);
      assert.equal(result.sessionId, first.sessionId);
      assert.equal(result.platform, session.platform);
      assert.equal(result.terminal, session.terminal);
      assert.equal(result.terminalType, session.terminalType);
      if (result.type) continue;
      assert.equal(Object.hasOwn(result, 'output'), false);
      if (output.length > 12000) {
        assert.equal(result.output_chars, output.length);
        assert.equal(f.files.get(result.output_saved_to), output);
      }
    }
    assert.equal(f.intermediate.filter(event => event.type === 'chunk').map(event => event.chunk).join(''), output);
    assert.equal(session.receipt.output, output, 'The SDK receipt must retain its complete output');
    assert.equal(first.status, 'completed');
    assert.equal(first.exitCode, 0);
    assert.equal(first.timedOut, false);
    assert.equal(f.calls.filter(call => call.operation === 'observe').length, 1);
  }
});

/** Delivers each chunk once across early reads and completion, including repeated text and Unicode. */
test('completion metadata does not replay streamed output before or after foreground expiry', async () => {
  for (const chunks of [[], ['same\n', 'same\n', '雪🌍\r\n'], ['short output']]) {
    const output = chunks.join('');
    const f = managedFixture({output, timedOut:true});
    const original = f.context.Tools.System.terminal.runStream;
    f.context.Tools.System.terminal.runStream = async function*(request) {
      for await (const event of original.call(this, request)) {
        if (event.type !== 'chunk') { yield event; continue; }
        let receivedChars = 0;
        for (const [chunkIndex, chunk] of chunks.entries()) {
          receivedChars += chunk.length;
          yield {...event, chunk, chunkIndex, receivedChars};
        }
      }
    };
    const running = f.tools.terminal({command:'test output'});
    await new Promise(setImmediate);
    const receivedBeforeCompletion = f.intermediate.splice(0);
    const session = [...f.sessions.values()][0];
    session.finish();
    const result = await running;
    const delivered = [...receivedBeforeCompletion, ...f.intermediate];
    assert.equal(delivered.filter(event => event.type === 'chunk').map(event => event.chunk).join(''), output);
    assert.deepEqual(delivered.filter(event => event.type === 'chunk').map(event => event.chunkIndex), chunks.map((_, index) => index));
    assert.equal(Object.hasOwn(result, 'output'), false);
    assert.equal(session.receipt.output, output);
    assert.equal(result.status, 'completed');
    assert.equal(result.exitCode, 0);
  }
});

/** Rejects an incomplete observation and releases leases without inventing a successful result. */
test('a stream without completion fails and releases its session for idle cleanup', async () => {
  const f = managedFixture();
  const original = f.context.Tools.System.terminal.runStream;
  f.context.Tools.System.terminal.runStream = async function*(request) {
    for await (const event of original.call(this, request)) {
      if (event.type !== 'completed') yield event;
    }
  };
  await assert.rejects(f.tools.terminal({command:'pwd'}), /without a completion result/);
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Preserves process failure codes and host errors while releasing the startup family lease. */
test('terminal streams preserve exit codes and startup failures', async () => {
  const f = managedFixture();
  const original = f.context.Tools.System.terminal.runStream;
  f.context.Tools.System.terminal.runStream = async function*() { throw new Error('Host launch failed'); };
  await assert.rejects(f.tools.terminal({command:'pwd'}), /Host launch failed/);
  f.context.Tools.System.terminal.runStream = async function*(request) {
    for await (const event of original.call(this, request)) {
      if (event.type === 'completed') event.result.exitCode = 7;
      yield event;
    }
  };
  assert.equal((await f.tools.terminal({command:'exit 7'})).exitCode, 7);
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 0);
});

/** Preserves real command output even when it intentionally prints the submitted script. */
test('terminal projection does not strip script text from actual stdout', async () => {
  const command = 'printf "SCRIPT_OUTPUT"';
  const f = managedFixture({output:command});
  const result = await f.tools.terminal({command});
  assert.equal(Object.hasOwn(result, 'output'), false);
  assert.equal(f.intermediate.find(event => event.type === 'chunk').chunk, command);
  assert.equal(Object.hasOwn(result, 'command'), false);
});

/** A pending original command must not prevent another tool from sending the input it needs. */
test('input and screen remain independent from a pending command result wait', async () => {
  const f = managedFixture({timedOut:true});
  let completed = false;
  const running = f.tools.terminal({command:'read answer'}).then(value => { completed = true; return value; });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(completed, false);
  const {sessionId} = f.intermediate[0];
  const other = f.tools.terminal({command:'another command'});
  await new Promise(setImmediate);
  assert.equal(f.sessions.size, 2, 'A streaming command must release its startup family lease');
  await f.tools.get_screen({sessionId});
  await f.tools.input({sessionId,input:'answer',control:'enter'});
  for (const session of f.sessions.values()) session.finish();
  assert.equal((await running).status, 'completed');
  await other;
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
test('a streaming command stays protected and receives a full idle period after completion', async () => {
  const f = managedFixture({timedOut:true});
  const running = f.tools.terminal({command:'silent build'});
  await new Promise(setImmediate);
  f.setTime(3600000);
  await f.main.onClock();
  assert.equal(f.calls.filter(call => call.operation === 'closeIfIdle').length, 0);
  [...f.sessions.values()][0].finish();
  await running;
  f.setTime(4199999);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  f.setTime(4200000);
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
  await f.tools.get_screen({sessionId:busy.sessionId});
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
