import { readSdkScript } from "../runtime/compose_scripts.mjs";
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const require = createRequire(new URL('plugins/packages/buildin/workflow/package.json', root));
const ts = require('typescript');
const packagePath = 'plugins/packages/buildin/daily_life/';
const dataDirectory = '/app/data/plugin_data/space/com.operit.daily_life';
const storePath = `${dataDirectory}/reminders.preferences.json`;
const reminderKeyPrefix = 'reminder_';
const startTime = Date.parse('2030-01-01T00:00:00Z');

/** Reads the current production source instead of generated plugin artifacts. */
function source(relativePath) { return readFileSync(new URL(relativePath, root), 'utf8'); }

/** Copies cross-realm results for strict assertions. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Extracts production JavaScript embedded in the first Rust raw string. */
function embedded(relativePath) {
  if (relativePath.endsWith('/ToolPkgRegistrationBridge.rs')) return readSdkScript('ToolPkgRegistrationBridge.rs');
  const text = source(relativePath);
  const start = text.indexOf('r#"') + 3;
  return text.slice(start, text.indexOf('"#', start));
}

/** Builds a native store state seeded with the supplied reminder records. */
function storeStateWith(records) {
  return { schema: 1, entries: new Map(records.map(record => [reminderKeyPrefix + record.id, record])) };
}

/** Reads every persisted reminder record from the emulated native store. */
function storedReminders(fixtureInstance) {
  return [...fixtureInstance.storeState.entries.entries()]
    .filter(([key]) => key.startsWith(reminderKeyPrefix))
    .map(([, value]) => plain(value));
}

/** Selects the recorded storage edits that touched reminder records. */
function reminderEdits(fixtureInstance) {
  return fixtureInstance.storageEdits.filter(edit =>
    Object.keys(edit.set).some(key => key.startsWith(reminderKeyPrefix)) || edit.removed.some(key => key.startsWith(reminderKeyPrefix)));
}

/** Tracks one reminder's persisted status sequence across edits. */
function reminderStatusHistory(fixtureInstance, id) {
  const key = reminderKeyPrefix + id;
  const history = [];
  for (const edit of fixtureInstance.storageEdits) {
    if (Object.hasOwn(edit.set, key)) history.push(edit.set[key].status);
    if (edit.removed.includes(key)) history.push('removed');
  }
  return history;
}

/** Loads real plugin modules with strictly mocked Host capabilities behind the native storage channel. */
function fixture({ visitResults = [], notify, storeState = null, failReminderEdits = false } = {}) {
  const requests = [];
  const notifications = [];
  const storageEdits = [];
  const ipcCalls = [];
  const hooks = [];
  const lifecycleHooks = [];
  const handlers = new Map();
  const state = storeState ?? { schema: 0, entries: new Map() };
  let now = startTime;
  let declaredSchema = null;

  /** Uses an explicit fixture clock for instants without changing date parsing semantics. */
  class Clock extends Date {
    /** Creates either the fixture's current instant or the supplied date. */
    constructor(...args) {
      if (args.length === 0) super(now);
      else super(...args);
    }
    /** Returns the exact fixture clock used by scheduled delivery. */
    static now() { return now; }
  }

  function snapshotPreferences(entries) {
    return {
      get(key) { return entries.has(key) ? plain(entries.get(key)) : undefined; },
      contains(key) { return entries.has(key); },
      entries() { return [...entries.entries()].map(([key, value]) => [key, plain(value)]); },
    };
  }

  function mutablePreferences(entries) {
    const changes = { set: new Map(), removed: [] };
    return {
      changes,
      preferences: {
        get(key) { return entries.has(key) ? plain(entries.get(key)) : undefined; },
        contains(key) { return entries.has(key); },
        entries() { return [...entries.entries()].map(([key, value]) => [key, plain(value)]); },
        set(key, value) { entries.set(key, plain(value)); changes.set.set(key, plain(value)); },
        remove(key) { if (entries.delete(key)) changes.removed.push(key); },
        incrementInteger(key, delta) {
          const current = typeof entries.get(key) === 'number' ? entries.get(key) : 0;
          entries.set(key, current + delta);
          changes.set.set(key, current + delta);
        },
        setObjectField(key, field, value) {
          const object = entries.has(key) ? plain(entries.get(key)) : {};
          object[field] = plain(value);
          entries.set(key, object);
          changes.set.set(key, plain(object));
        },
      },
    };
  }

  const storeHandle = {
    /** Applies one declared schema upgrade through a committed edit. */
    async applySchema() {
      const pending = declaredSchema;
      if (pending === null || state.schema >= pending.version) return;
      declaredSchema = null;
      try { await storeHandle.edit(preferences => pending.migrate(state.schema, preferences)); }
      finally { declaredSchema = pending; }
      state.schema = pending.version;
    },
    async data() {
      await this.applySchema();
      return snapshotPreferences(state.entries);
    },
    async edit(transform) {
      await this.applySchema();
      const draft = new Map(state.entries);
      const { changes, preferences } = mutablePreferences(draft);
      await transform(preferences);
      if (failReminderEdits && [...changes.set.keys()].some(key => key.startsWith(reminderKeyPrefix))) {
        throw new Error('disk write denied');
      }
      state.entries = draft;
      storageEdits.push({ path: storePath, set: Object.fromEntries(changes.set), removed: [...changes.removed] });
      return this.data();
    },
    withSchema(version, migrate) { declaredSchema = { version, migrate }; return storeHandle; },
    async schemaVersion() { await this.applySchema(); return state.schema; },
    async close() {},
  };

  const context = vm.createContext({
    Date: Clock, setTimeout, clearTimeout,
    console: { log() {}, error() {} },
    __operitExpose() {},
    toolCall: async (name, params) => {
      assert.equal(name, 'capture_screenshot');
      assert.deepEqual(plain(params), {});
      return '/app/data/temp/screenshot.png';
    },
    Tools: {
      Storage: {
        dataStore: {
          /** Opens the one plugin-owned native preference store. */
          async open(options) {
            assert.deepEqual(plain(options), { path: storePath });
            return storeHandle;
          },
        },
      },
      Net: {
        /** Supplies web-visit Host results without contacting the network. */
        async visit(url) {
          requests.push({ url });
          if (visitResults.length === 0) throw new Error('No configured web-visit Host result');
          const next = visitResults.shift();
          if (next instanceof Error) throw next;
          return next;
        },
        /** Rejects a weather implementation that changes the requested Host capability. */
        async http() { throw new Error('Weather must use the web-visit Host'); },
      },
      System: {
        /** Records the real notification payload at the Host boundary. */
        async sendNotification(message, title) {
          notifications.push({ message, title });
          if (notify !== undefined) return notify(message, title);
          return 'notification accepted';
        },
        /** Returns an explicit device-info Host result. */
        async getDeviceInfo() { return { device: 'fixture-device' }; },
      },
    },
    ToolPkg: {
      /** Supplies the plugin-owned native data directory used by reminder persistence. */
      getSpaceDataDir: () => dataDirectory,
      ipc: {
        /** Registers the one main-runtime reminder owner. */
        on(channel, handler) { handlers.set(channel, handler); },
        /** Forwards subpackage requests only to the registered main owner. */
        async call(channel, request, options) {
          assert.equal(options.targetRuntime, 'main');
          ipcCalls.push({ channel, request: plain(request), options: plain(options) });
          assert.ok(handlers.has(channel));
          return handlers.get(channel)(plain(request));
        },
      },
      /** Captures platform-neutral scheduling and lifecycle Host hooks. */
      registerHostEventHook(definition) { hooks.push(definition); },
      /** Captures the application-start hook used for persisted overdue reminders. */
      registerAppLifecycleHook(definition) { lifecycleHooks.push(definition); },
    },
  });
  const modules = new Map();

  /** Evaluates CommonJS modules from TypeScript in memory without building the application. */
  function load(relativeFile) {
    if (modules.has(relativeFile)) return modules.get(relativeFile).exports;
    const module = { exports: {} };
    modules.set(relativeFile, module);
    const javascript = ts.transpileModule(source(packagePath + relativeFile), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    }).outputText;
    const execute = vm.runInContext(`(function(require, module, exports) { ${javascript}\n})`, context);
    execute((specifier) => {
      assert.match(specifier, /^\.\.?(\/|$)/);
      return load(path.posix.normalize(path.posix.join(path.posix.dirname(relativeFile), `${specifier}.ts`)));
    }, module, module.exports);
    return module.exports;
  }

  const main = load('src/main.ts');
  const tools = load('src/tools.ts');
  const reminders = load('src/reminders.ts');
  return {
    main, tools, reminders, context, requests, notifications, storageEdits, ipcCalls, hooks, lifecycleHooks,
    storeState: state,
    /** Advances the fixture clock without waiting for wall-clock time. */
    setTime(value) { now = value; },
  };
}

/** Produces one explicitly future reminder request. */
function reminderParams(dueDate = '2030-01-01T00:01:00Z') {
  return { title: '喝水', description: '记得喝一杯水', due_date: dueDate };
}

/** Checks packaging preserves existing tool names while advertising only connected functions. */
test('daily_life is a ToolPkg with the unchanged daily_life subpackage and no subagent tools', () => {
  const manifest = JSON.parse(source(packagePath + 'manifest.json'));
  assert.equal(manifest.api_version, '2.0.0');
  assert.equal(manifest.enabled_by_default, true);
  assert.deepEqual(manifest.subpackages, [{ id: 'daily_life', entry: 'dist/tools.js' }]);
  const text = source(packagePath + 'src/tools.ts');
  const metadata = JSON.parse(/\/\* METADATA\s*([\s\S]*?)\*\//.exec(text)[1]);
  const f = fixture();
  assert.deepEqual(metadata.tools.map(tool => tool.name).sort(), [
    'cancel_reminder', 'device_status', 'get_current_date', 'list_reminders', 'search_weather', 'set_reminder', 'take_screenshot',
  ]);
  for (const tool of metadata.tools) assert.equal(typeof f.tools[tool.name], 'function');
  for (const name of ['tools', 'reminders', 'main']) {
    assert.doesNotMatch(source(packagePath + `src/${name}.ts`), /runSubAgent|Tools\.UI|fetch\(|XMLHttpRequest|https?\.request|executeShell|Tools\.Net\.http|open-meteo/);
  }
});

/** Checks the reminder owner no longer persists through the retired PluginConfig file channel. */
test('reminder persistence uses the native preference store instead of PluginConfig', () => {
  const text = source(packagePath + 'src/reminders.ts');
  assert.match(text, /Tools\.Storage\.dataStore\.open/);
  assert.match(text, /ToolPkg\.getSpaceDataDir\(\)/);
  assert.doesNotMatch(text, /PluginConfig\.(use|flush)/);
});

/** Checks existing date, status and screenshot functionality remains directly connected. */
test('basic tools continue using their existing Date and system Host capabilities', async () => {
  const f = fixture();
  assert.equal((await f.tools.get_current_date()).timestamp, startTime);
  assert.deepEqual(plain(await f.tools.device_status()), { device: 'fixture-device' });
  assert.equal(await f.tools.take_screenshot(), '/app/data/temp/screenshot.png');
});

/** Checks weather preserves the web-visit Host response and the original search route. */
test('weather makes one web-visit Host call and returns its result unchanged', async () => {
  const hostResult = {
    url: 'https://www.baidu.com/s?wd=Shanghai%20weather', title: 'Shanghai weather',
    content: 'Weather page extracted by the Host', metadata: {}, links: [], imageLinks: [],
    visitKey: 'host-visit-key', contentSavedTo: '/app/data/temp/visit_web/weather.txt',
  };
  const f = fixture({ visitResults: [hostResult] });
  const result = await f.tools.search_weather({ location: 'Shanghai' });
  assert.equal(result, hostResult);
  assert.deepEqual(f.requests, [{ url: 'https://www.baidu.com/s?wd=Shanghai%20weather' }]);
});

/** Checks user text stays in the encoded search query rather than adding URL parameters. */
test('weather encodes the complete location and weather search text', async () => {
  const hostResult = { content: 'Host weather result' };
  const f = fixture({ visitResults: [hostResult] });
  await f.tools.search_weather({ location: '上海&wd=other' });
  const url = new URL(f.requests[0].url);
  assert.equal(url.hostname, 'www.baidu.com');
  assert.equal(url.pathname, '/s');
  assert.equal(url.searchParams.get('wd'), '上海&wd=other weather');
  assert.equal([...url.searchParams].length, 1);
});

/** Checks web-visit errors are surfaced unchanged without another request or provider. */
test('weather propagates the web-visit Host error without retries', async () => {
  const failure = new Error('visit_web timed out');
  const f = fixture({ visitResults: [failure] });
  await assert.rejects(f.tools.search_weather({ location: 'Shanghai' }), error => error === failure);
  assert.equal(f.requests.length, 1);
});

/** Checks registration only describes Host events and performs no storage or network work. */
test('reminders register existing minute, startup and resume Host hooks', () => {
  const f = fixture();
  assert.equal(f.main.registerToolPkg(), true);
  assert.equal(f.hooks.length, 2);
  assert.equal(f.hooks[0].source, 'interval');
  assert.deepEqual(plain(f.hooks[0].trigger), { kind: 'interval', intervalMs: 60000 });
  assert.equal(f.hooks[0].function, f.main.onClock);
  assert.equal(f.hooks[1].trigger.topic, 'app.lifecycle.resumed');
  assert.equal(f.lifecycleHooks[0].event, 'application_on_create');
  assert.equal(f.lifecycleHooks[0].function, f.main.onOpen);
  assert.equal(f.requests.length, 0);
  assert.equal(f.storageEdits.length, 0);
});

/** Checks actual production registration captures exported callback names durably. */
test('production ToolPkg registration resolves the reminder hooks to main-module exports', () => {
  const f = fixture();
  f.context.__operitGetCallState = () => ({ params: {
    __operit_registration_mode: true, __operit_toolpkg_api_version: '2.0.0',
    toolPkgId: 'com.operit.daily_life',
  } });
  f.context.__operitExpose = (name, value) => { f.context[name] = value; };
  f.context.__operitGetActiveModuleExports = () => f.main;
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs'), f.context);
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs')
    .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), f.context);
  assert.equal(f.main.registerToolPkg(), true);
  const capture = f.context.__operitToolPkgRegistrationCapture;
  const interval = JSON.parse(capture.hostEventHooks[0]);
  const resume = JSON.parse(capture.hostEventHooks[1]);
  const startup = JSON.parse(capture.appLifecycleHooks[0]);
  assert.equal(interval.function, 'onClock');
  assert.equal(interval.trigger.intervalMs, 60000);
  assert.equal(resume.function, 'onResume');
  assert.equal(startup.function, 'onOpen');
  assert.equal(f.storageEdits.length, 0);
  assert.equal(f.requests.length, 0);
});

/** Checks scheduling reports durable acceptance rather than an already delivered notification. */
test('set_reminder persists a pending record in the main owner before returning', async () => {
  const f = fixture();
  const result = plain(await f.tools.set_reminder(reminderParams('2030-01-01T08:01:00+08:00')));
  assert.equal(result.status, 'pending');
  assert.equal(result.due_date, '2030-01-01T00:01:00.000Z');
  assert.equal(result.due_timestamp, startTime + 60000);
  assert.equal(result.delivered_at, null);
  assert.equal(result.error, null);
  assert.equal(f.notifications.length, 0);
  assert.deepEqual(storedReminders(f), [result]);
  assert.deepEqual(f.ipcCalls[0].options, { targetRuntime: 'main' });
});

/** Checks extra user fields cannot change the reminder IPC operation. */
test('set_reminder does not forward an injected action or identity', async () => {
  const f = fixture();
  const reminder = await f.tools.set_reminder({ ...reminderParams(), action: 'list', id: 'forged' });
  assert.equal(reminder.status, 'pending');
  assert.equal(f.ipcCalls[0].request.action, 'create');
  assert.equal(Object.hasOwn(f.ipcCalls[0].request, 'id'), false);
});

/** Checks unsupported and past dates never create a durable reminder. */
test('reminders reject ambiguous, invalid, and past dates', async () => {
  const f = fixture();
  for (const dueDate of [
    '2030-01-01T00:01:00', '2030-02-30T00:00:00Z', '2030-01-01T24:00:00Z',
    '2030-01-01T00:00:00+24:00', '2030-01-01T00:00:00Z', '2029-12-31T00:00:00Z',
  ]) {
    await assert.rejects(f.tools.set_reminder(reminderParams(dueDate)), /due_date/);
  }
  assert.deepEqual(plain(await f.tools.list_reminders()), []);
  assert.equal(f.notifications.length, 0);
  assert.equal(reminderEdits(f).length, 0);
});

/** Checks calendar validation accepts a real leap day but rejects a nonexistent one. */
test('reminder ISO validation respects leap years and explicit offsets', () => {
  const f = fixture();
  assert.equal(f.reminders.parseDueDate('2032-02-29T12:34:56.123Z'), Date.parse('2032-02-29T12:34:56.123Z'));
  assert.throws(() => f.reminders.parseDueDate('2030-02-29T12:34:56Z'), /invalid calendar/);
});

/** Checks persisted claims precede notifications and repeated clock events cannot resend them. */
test('due reminders are claimed durably, notified once, and marked delivered', async () => {
  const f = fixture({ notify(message, title) {
    assert.equal(message, '记得喝一杯水');
    assert.equal(title, '喝水');
    assert.equal(storedReminders(f)[0].status, 'delivering');
    return 'notification accepted';
  } });
  await f.tools.set_reminder(reminderParams());
  await f.main.onClock();
  assert.equal(f.notifications.length, 0);
  f.setTime(startTime + 60000);
  await Promise.all([f.main.onClock(), f.main.onClock(), f.main.onResume()]);
  assert.equal(f.notifications.length, 1);
  const [record] = plain(await f.tools.list_reminders());
  assert.equal(record.status, 'delivered');
  assert.equal(record.delivered_at, '2030-01-01T00:01:00.000Z');
  assert.deepEqual(reminderStatusHistory(f, record.id), ['pending', 'delivering', 'delivered']);
});

/** Checks plugin-owned records survive runtime replacement and startup processes overdue reminders. */
test('pending reminders survive restart and are checked on application startup', async () => {
  const first = fixture();
  const reminder = plain(await first.tools.set_reminder(reminderParams()));
  const next = fixture({ storeState: first.storeState });
  next.setTime(startTime + 120000);
  await next.main.onOpen();
  assert.equal(next.notifications.length, 1);
  const [record] = plain(await next.tools.list_reminders());
  assert.equal(record.id, reminder.id);
  assert.equal(record.status, 'delivered');
});

/** Checks cancellation is durable and cannot be undone by scheduled delivery or a restart. */
test('cancel_reminder prevents delivery across runtime restarts', async () => {
  const f = fixture();
  const reminder = await f.tools.set_reminder(reminderParams());
  assert.equal((await f.tools.cancel_reminder({ id: reminder.id })).status, 'cancelled');
  const next = fixture({ storeState: f.storeState });
  next.setTime(startTime + 60000);
  await next.main.onClock();
  assert.equal(next.notifications.length, 0);
  assert.equal((await next.tools.list_reminders())[0].status, 'cancelled');
  await assert.rejects(next.tools.cancel_reminder({ id: reminder.id }), /cancelled state/);
  await assert.rejects(next.tools.cancel_reminder({ id: 'missing' }), /Reminder not found/);
});

/** Checks a Host notification failure is persisted and never converted into successful delivery. */
test('notification failures remain failed and are not automatically resent', async () => {
  const f = fixture({ notify() { throw new Error('notification permission denied'); } });
  const reminder = await f.tools.set_reminder(reminderParams());
  f.setTime(startTime + 60000);
  await assert.rejects(f.main.onClock(), /notification permission denied/);
  const [record] = plain(await f.tools.list_reminders());
  assert.equal(record.status, 'failed');
  assert.equal(record.delivered_at, null);
  assert.match(record.error, /notification permission denied/);
  await f.main.onClock();
  assert.equal(f.notifications.length, 1);
  await assert.rejects(f.tools.cancel_reminder({ id: reminder.id }), /failed state/);
});

/** Checks one notification error cannot hide another independent reminder's delivery result. */
test('one failed reminder is reported while other due reminders are delivered normally', async () => {
  const f = fixture({ notify(message, title) {
    if (title === '喝水') throw new Error('notification rejected');
    return 'notification accepted';
  } });
  await f.tools.set_reminder(reminderParams());
  await f.tools.set_reminder({ ...reminderParams(), title: '休息' });
  f.setTime(startTime + 60000);
  await assert.rejects(f.main.onClock(), /notification rejected/);
  const records = plain(await f.tools.list_reminders());
  assert.equal(records[0].status, 'failed');
  assert.equal(records[1].status, 'delivered');
  assert.equal(f.notifications.length, 2);
});

/** Checks unfinished persisted deliveries are reported as unknown instead of being sent again. */
test('interrupted delivery is recorded as failed without resending', async () => {
  const first = fixture();
  const reminder = plain(await first.tools.set_reminder(reminderParams()));
  const next = fixture({ storeState: storeStateWith([{ ...reminder, status: 'delivering' }]) });
  next.setTime(startTime + 60000);
  await next.main.onOpen();
  const [record] = plain(await next.tools.list_reminders());
  assert.equal(record.status, 'failed');
  assert.match(record.error, /outcome is unknown/);
  assert.equal(next.notifications.length, 0);
});

/** Checks corrupt records stop the owner rather than being replaced by an empty reminder list. */
test('invalid stored reminders surface a storage-contract error', async () => {
  const f = fixture({ storeState: storeStateWith([{ id: 'invalid' }]) });
  await assert.rejects(f.tools.list_reminders(), /non-empty string/);
  await assert.rejects(f.main.onClock(), /non-empty string/);
  assert.equal(f.notifications.length, 0);
  assert.equal(reminderEdits(f).length, 0);
});

/** Checks failure to persist acceptance stops all delivery of the uncommitted in-memory record. */
test('storage failure rejects scheduling and stops this owner without attempting notification', async () => {
  const f = fixture({ failReminderEdits: true });
  await assert.rejects(f.tools.set_reminder(reminderParams()), /disk write denied/);
  f.setTime(startTime + 60000);
  await assert.rejects(f.main.onClock(), /runtime has stopped delivery/);
  assert.equal(f.notifications.length, 0);
  assert.equal(storedReminders(f).length, 0);
});

/** Checks serialized concurrent creation cannot lose records and returned copies cannot mutate storage. */
test('concurrent reminder requests retain separate persistent records and return detached values', async () => {
  const f = fixture();
  const reminders = await Promise.all([
    f.tools.set_reminder(reminderParams()),
    f.tools.set_reminder({ ...reminderParams(), title: '休息' }),
  ]);
  assert.notEqual(reminders[0].id, reminders[1].id);
  reminders[0].title = 'changed by caller';
  const stored = plain(await f.tools.list_reminders());
  assert.equal(stored.length, 2);
  assert.equal(stored[0].title, '喝水');
  assert.equal(storedReminders(f).length, 2);
});
