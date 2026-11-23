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
          /** Streams the original command until actual completion without a foreground wait. */
          async *runStream(request) {
            calls.push({ operation: 'observe', observation: { op: 'runstream', ...structuredClone(request) } });
            let nameToUse = request.sessionName;
            for (let generation = 1; sessions.get(`${request.terminalType}:${nameToUse}`)?.commandRunning; generation++) {
              nameToUse = `${request.sessionName}_generation_${generation}`;
            }
            const created = await this.create(nameToUse, request.terminalType);
            const session = [...sessions.values()].find(value => value.sessionId === created.sessionId);
            session.commandRunning = true;
            yield { type: 'start', ...created, command: request.command };
            yield { type: 'chunk', ...created, command: request.command, chunk: output, chunkIndex: 0, receivedChars: output.length };
            session.commandRunning = false;
            yield { type: 'completed', result: { ...created, command: request.command, output, exitCode: 0, timedOut } };
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
  assert.ok(!metadata.tools.some(tool => tool.name === 'terminal_wait'));
  assert.equal(fixture().tools.terminal_wait, undefined);
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
  f.context.Tools.System.terminal.runStream = async function*(request) {
    const profile = f.terminalInfo.profiles.find(value => value.id === request.profileId);
    f.calls.push({operation:'selectedProfile', profileId:request.profileId});
    yield* originalRun.call(this, {...request, sessionName:`${request.sessionName}_profile_${profile.id}`, terminalType:profile.terminalType});
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
  assert.equal(a.sessionId,again.sessionId);
  f.terminalInfo.defaultProfileId = 'ubuntu_b';
  assert.equal((await f.tools.terminal({command:'pwd'})).sessionId,b.sessionId);
  assert.deepEqual(f.calls.filter(call => call.operation === 'selectedProfile').map(call => call.profileId),
    ['ubuntu_a','ubuntu_b','ubuntu_a','ubuntu_b']);
  const discovered = await f.tools.list_terminals();
  assert.equal(discovered.profiles.length,3);
  assert.equal(discovered.defaultProfileId,'ubuntu_b');
});

/** Checks unknown and unavailable IDs fail before any execution reaches the host. */
test('invalid profile selection fails without starting a command', async () => {
  const f = managedFixture();
  await assert.rejects(f.tools.terminal({command:'id',profileId:'missing'}),/not registered/);
  f.terminalInfo.profiles[0].available = false;
  await assert.rejects(f.tools.terminal({command:'id'}),/unavailable/);
  assert.equal(f.calls.length,0);
});

/** Keeps terminal completion and large output independent of package foreground wait controls. */
test('profile execution streams session identity and output before returning the final result', async () => {
  const f = managedFixture({output:'x'.repeat(12001)});
  const first = await f.tools.terminal({command:'printf large'});
  assert.equal(first.status, 'completed');
  assert.deepEqual(f.intermediate.map(event => event.type), ['start', 'chunk']);
  assert.equal(f.intermediate[0].sessionId, first.sessionId);
  assert.equal(f.intermediate[1].chunk.length, 12001);
  assert.equal(f.files.get(first.output_saved_to).length,12001);
  const system = await f.tools.terminal({command:'id',profileId:'system'});
  assert.equal(system.terminal,'android-system');
  assert.ok(f.calls.filter(call => call.operation === 'observe').every(call => !Object.hasOwn(call.observation, 'waitMs')));
  const result = await f.tools.close({sessionId:first.sessionId});
  assert.equal(result.success,true);
  assert.notEqual((await f.tools.terminal({command:'pwd'})).sessionId,first.sessionId);
});

/** Prevents a running receipt from being mistaken for the final package result. */
test('an incomplete terminal stream fails explicitly instead of completing the package early', async () => {
  const f = managedFixture({timedOut:true});
  await assert.rejects(f.tools.terminal({command:'slow'}), /before command completion/);
});

/** Exercises a real pending async iterator and verifies package output arrives before final resolution. */
test('a slow command emits live output while its package promise remains pending', async () => {
  const f = managedFixture();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.runStream = async function*() {
    yield {type:'start',sessionId:'slow-session'};
    yield {type:'chunk',sessionId:'slow-session',chunk:'first\n'};
    await gate;
    yield {type:'chunk',sessionId:'slow-session',chunk:'second\n'};
    yield {type:'completed',result:{sessionId:'slow-session',platform:'android',terminal:'proot',terminalType:'bash',output:'first\nsecond\n',exitCode:0,timedOut:false}};
  };
  let completed = false;
  const running = f.tools.terminal({command:'slow'}).then(value => { completed = true; return value; });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(completed, false);
  assert.equal(f.intermediate[1].chunk, 'first\n');
  release();
  assert.equal((await running).output, 'first\nsecond\n');
  assert.equal(f.intermediate[2].chunk, 'second\n');
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
test('a pending stream stays alive and receives a full idle period after completion', async () => {
  const f = managedFixture();
  const original = f.context.Tools.System.terminal.runStream;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.runStream = async function*(request) {
    for await (const event of original.call(this,request)) {
      if (event.type === 'completed') await gate;
      yield event;
    }
  };
  const running = f.tools.terminal({command:'silent build'});
  await new Promise(resolve => setImmediate(resolve));
  f.setTime(3600000);
  await f.main.onClock();
  assert.equal(f.calls.filter(call => call.operation === 'closeIfIdle').length, 0);
  release();
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
  const original = f.context.Tools.System.terminal.runStream;
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  f.context.Tools.System.terminal.runStream = async function*(request) {
    for await (const event of original.call(this,request)) {
      if (event.type === 'chunk') await gate;
      yield event;
    }
  };
  const running = f.tools.terminal({command:'build'});
  await new Promise(resolve => setImmediate(resolve));
  idle.commandRunning = false;
  f.setTime(600000);
  await f.main.onClock();
  assert.equal(f.sessions.size, 1);
  assert.notEqual([...f.sessions.values()][0].sessionId, first.sessionId);
  release();
  await running;
  f.setTime(1200000);
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
