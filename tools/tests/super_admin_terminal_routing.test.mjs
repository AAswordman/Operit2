import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
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
  const terminalInfo = {
    platform,
    terminal: implementations[defaultType],
    terminalType: defaultType,
    types: Object.entries(implementations).map(([terminalType, terminal]) => ({ terminal, terminalType, available: true })),
  };
  const context = vm.createContext({
    exports: {},
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
  vm.runInContext(stripTypeScriptTypes(source('plugins/packages/buildin/super_admin.ts')), context);
  return {
    context, tools: context.exports, sessions, calls, files, errors, intermediate, terminalInfo,
    /** Changes the active chat to exercise session identity isolation. */
    setChatId(value) { chatId = value; },
  };
}


/** Exercises the shared profile tools independently of platform-specific names. */
test('one execution entry point advertises discovery without platform presets', () => {
  const metadata = JSON.parse(source('plugins/packages/buildin/super_admin.ts').match(/\/\* METADATA\s*([\s\S]*?)\*\//)[1]);
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
    yield* originalRun.call(this, {...request, terminalType:profile.terminalType});
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
