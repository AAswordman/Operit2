import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const html = readFileSync(new URL('apps/flutter/app/web/index.html', root), 'utf8');
const bridge = readFileSync(new URL(
  'apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', root,
), 'utf8');

/** Loads one complete production function without initializing unrelated browser hosts. */
function productionFunction(name, next) {
  const start = bridge.indexOf(`  function ${name}(`);
  const end = bridge.indexOf(next, start);
  assert.ok(start >= 0 && end > start);
  return stripTypeScriptTypes(bridge.slice(start, end), { mode: 'transform' });
}

/** Runs the actual error surface against a small DOM and clipboard adapter. */
function errorSurface() {
  const listeners = new Map();
  const elements = new Map();
  let copied;
  const context = vm.createContext({
    Error, console, HTMLScriptElement: class {}, HTMLLinkElement: class {}, HTMLImageElement: class {},
    navigator: { clipboard: { async writeText(value) { copied = value; } } },
    window: { addEventListener(type, callback) { listeners.set(type, callback); } },
    document: {
      body: { appendChild(element) { elements.set(element.id, element); } },
      getElementById(id) { return elements.get(id); },
      createElement() {
        return {
          children: [], events: new Map(),
          append(...children) { this.children.push(...children); },
          addEventListener(type, callback) { this.events.set(type, callback); },
          remove() { elements.delete(this.id); },
        };
      },
    },
  });
  vm.runInContext(html.match(/<script>\s*([\s\S]*?)<\/script>/)[1], context);
  return { context, listeners, elements, copied: () => copied };
}

test('Safari trap details retain the original Rust panic, message and source location', async () => {
  const surface = errorSurface();
  surface.context.window.__operitShowWebError('Rust panic', 'panicked at core/example.rs:42:9\noriginal cause');
  const trap = new Error('Unreachable code should not be executed');
  // Safari can return a stack that contains only frames, without the error message.
  trap.stack = 'wasm_bindgen__convert__closures_____invoke@bridge.js:913:27';
  surface.listeners.get('error')({ error: trap, filename: 'bridge.js', lineno: 913, colno: 27 });
  const card = surface.elements.get('operit-web-error-surface').children[0];
  const details = card.children[1].textContent;
  assert.match(details, /panicked at core\/example.rs:42:9\noriginal cause/);
  assert.match(details, /Error: Unreachable code should not be executed/);
  assert.match(details, /bridge\.js:913:27/);
  await card.children[2].children[1].events.get('click')();
  assert.equal(surface.copied(), details);
  card.children[2].children[0].events.get('click')();
  assert.equal(surface.elements.size, 0);
});

test('repeated terminal errors keep a bounded history and preserve the first failure', () => {
  const surface = errorSurface();
  for (let index = 0; index < 12; index++) {
    surface.context.window.__operitShowWebError('Runtime worker error', `failure ${index}\n`);
  }
  const details = surface.elements.get('operit-web-error-surface').children[0].children[1].textContent;
  assert.match(details, /failure 0\n/);
  assert.doesNotMatch(details, /failure 1\n/);
  assert.match(details, /failure 11\n/);
  assert.equal(details.split('\n\n---\n\n').length, 8);
});

test('Rust panic reporter transports the original panic before the generic browser trap', () => {
  const source = productionFunction('reportRuntimePanic', '\n  async function bridge(');
  for (const kind of ['runtime', 'model', 'page']) {
    const messages = [];
    const context = vm.createContext({
      runtimeGlobal: {
        postMessage(message) { messages.push(message); },
        __operitShowWebError(source, message) { messages.push({ source, message }); },
      },
      isRuntimeWorker: () => kind === 'runtime',
      isModelInstallWorker: () => kind === 'model',
    });
    vm.runInContext(`${source}\nreportRuntimePanic('panicked at task.rs:7:3: original cause');`, context);
    assert.equal(messages.length, 1);
    assert.match(messages[0].message, /^panicked at task\.rs:7:3: original cause/);
    if (kind === 'runtime') assert.equal(messages[0].type, 'runtimeFatalError');
    if (kind === 'model') assert.equal(messages[0].type, 'error');
    if (kind === 'page') assert.equal(messages[0].source, 'Rust panic');
  }
});

/** Creates two browser page adapters around the real origin coordinator implementation. */
function runtimePages() {
  const channels = [];
  const workers = [];
  let owner = false;
  class Channel {
    /** Registers one page's origin channel adapter. */
    constructor() { channels.push(this); }
    /** Records the production coordinator's message listener. */
    addEventListener(type, listener) { this.listener = listener; }
    /** Delivers origin messages to every other page. */
    postMessage(data) { for (const peer of channels) if (peer !== this) peer.listener({ data }); }
  }
  class Worker {
    /** Records the sole runtime owner's worker instance. */
    constructor() { this.listeners = new Map(); this.messages = []; workers.push(this); }
    /** Records production worker listeners for controlled failure delivery. */
    addEventListener(type, listener) { this.listeners.set(type, listener); }
    /** Tracks Core requests without implementing the Wasm runtime. */
    postMessage(message) { this.messages.push(message); }
  }
  const source = productionFunction('installRuntimeWorkerProxy', '\n  /** Executes one synchronous host method');
  const pages = [];
  /** Joins another page to the same simulated browser origin. */
  function addPage() {
    const index = pages.length;
    const errors = [];
    const runtimeGlobal = {
      addEventListener() {},
      __operitShowWebError(source, message) { errors.push({ source, message }); },
    };
    const context = vm.createContext({
      runtimeGlobal, BroadcastChannel: Channel, Worker, Uint8Array, TextDecoder,
      navigator: { locks: { request(name, options, callback) {
        if (owner) return new Promise(() => {});
        owner = true;
        return callback();
      } } },
      createRandomUuid: () => `page-${index}`,
      runtimeCoordinatorName: 'test-coordinator', runtimeOwnerLockName: 'test-owner', runtimeIdentityId: 'test',
      runtimeIdentityWorkerUrl: path => path, notifyTerminalFilesystemWorker: null,
    });
    vm.runInContext(`${source}\ninstallRuntimeWorkerProxy();`, context);
    const page = { runtime: runtimeGlobal.__operitRuntime, errors };
    pages.push(page);
    return page;
  }
  addPage();
  addPage();
  return { pages, workers, addPage };
}

test('a terminal Wasm panic rejects pending and future calls on every connected page', { timeout: 1000 }, async () => {
  const { pages, workers } = runtimePages();
  const pending = pages.map(({ runtime }) => assert.rejects(runtime.call(new Uint8Array([1])), /original cause/));
  workers[0].listeners.get('message')({ data: {
    type: 'runtimeFatalError', source: 'Rust panic', message: 'task.rs:7:3: original cause',
  } });
  await Promise.all(pending);
  const sent = workers[0].messages.length;
  for (const { runtime, errors } of pages) {
    assert.deepEqual(errors, [{ source: 'Rust panic', message: 'task.rs:7:3: original cause' }]);
    await assert.rejects(runtime.call(new Uint8Array([2])), /original cause/);
  }
  assert.equal(workers.length, 1);
  assert.equal(workers[0].messages.length, sent);
});

test('uncaught worker errors retain their filename and line in every page', { timeout: 1000 }, async () => {
  const { pages, workers } = runtimePages();
  const pending = assert.rejects(pages[0].runtime.call(new Uint8Array([1])), /worker\.js:42:9/);
  workers[0].listeners.get('error')({ message: 'unreachable', filename: 'worker.js', lineno: 42, colno: 9 });
  await pending;
  for (const { errors } of pages) {
    assert.deepEqual(errors, [{ source: 'Runtime worker error', message: 'unreachable\nworker.js:42:9' }]);
  }
});

test('pages opened after a panic also reject calls instead of waiting on the failed owner', { timeout: 1000 }, async () => {
  const { workers, addPage } = runtimePages();
  workers[0].listeners.get('message')({ data: {
    type: 'runtimeFatalError', source: 'Rust panic', message: 'original cause',
  } });
  const sent = workers[0].messages.length;
  const page = addPage();
  await assert.rejects(page.runtime.call(new Uint8Array([1])), /original cause/);
  assert.match(page.errors[0].message, /original cause/);
  assert.equal(workers[0].messages.length, sent);
});

test('healthy owner replies still complete Core requests on both pages', { timeout: 1000 }, async () => {
  const { pages, workers } = runtimePages();
  const pending = pages.map(({ runtime }) => runtime.call(new Uint8Array([1])));
  const requests = workers[0].messages.filter(message => message.type === 'coreRequest');
  assert.equal(requests.length, 2);
  for (const request of requests) {
    workers[0].listeners.get('message')({ data: {
      type: 'coreResult', id: request.id, clientId: request.clientId, response: new Uint8Array([9]),
    } });
  }
  const responses = await Promise.all(pending);
  assert.deepEqual(responses, [new Uint8Array([9]), new Uint8Array([9])]);
  assert.ok(pages.every(page => page.errors.length === 0));
});
