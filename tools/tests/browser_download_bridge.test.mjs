import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {webcrypto} from 'node:crypto';
import vm from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/lib/core/host/browser/RuntimeBrowserPageDownloads.dart', import.meta.url), 'utf8');
const script = source.match(/const runtimeBrowserPageDownloadScript = r'''([\s\S]*?)''';/)[1];

/** Runs the production injection against real fetch streams and a native message boundary. */
function createPage(onMessage) {
  const messages = [];
  const listeners = new Map();
  const context = vm.createContext({
    URL, fetch, AbortController, Uint8Array, crypto: webcrypto, setTimeout, clearTimeout,
    btoa: value => Buffer.from(value, 'binary').toString('base64'),
    location: {href: 'https://example.test/'}, frames: [],
    addEventListener: (name, callback) => listeners.set(name, callback),
    OperitBrowserDownloads: {postMessage(encoded) {
      const event = JSON.parse(encoded);
      messages.push(event);
      queueMicrotask(() => {
        const result = onMessage?.(event, context);
        if (result === false) return;
        const sequence = {begin: -1, metadata: -2, chunk: event.sequence}[event.type];
        if (sequence !== undefined) context.__operitPageDownloads.control(event.id, 'ack', sequence);
      });
    }},
  });
  context.parent = context;
  vm.runInContext(script, context);
  return {context, messages, listeners};
}

/** Concatenates the actual binary payload sent across the application channel. */
function received(messages) {
  return Buffer.concat(messages.filter(event => event.type === 'chunk').map(event => Buffer.from(event.data, 'base64')));
}

test('blob downloads preserve binary bytes across multiple bounded chunks', async () => {
  const bytes = Buffer.from(Array.from({length: 200000}, (_, index) => index % 256));
  const url = URL.createObjectURL(new Blob([bytes]));
  const page = createPage();
  try {
    await page.context.__operitPageDownloads.start(url, 'report.bin');
    assert.equal(page.messages[0].type, 'begin');
    assert.equal(page.messages[0].fileName, 'report.bin');
    assert.equal(page.messages.at(-1).type, 'end');
    assert.deepEqual(received(page.messages), bytes);
    const chunks = page.messages.filter(event => event.type === 'chunk');
    assert.ok(chunks.length > 1);
    assert.ok(chunks.every(event => Buffer.from(event.data, 'base64').length <= 65536));
    assert.deepEqual(chunks.map(event => event.sequence), chunks.map((_, index) => index));
  } finally { URL.revokeObjectURL(url); }
});

test('immediate object URL revocation does not lose an already clicked file', async () => {
  const url = URL.createObjectURL(new Blob(['downloaded']));
  const page = createPage();
  const completing = page.context.__operitPageDownloads.start(url, 'file.txt');
  URL.revokeObjectURL(url);
  await completing;
  assert.equal(page.messages.at(-1).type, 'end');
  assert.equal(received(page.messages).toString(), 'downloaded');
});

test('data downloads report the decoded file rather than the encoded URI', async () => {
  const page = createPage();
  await page.context.__operitPageDownloads.start('data:application/octet-stream;base64,AID/', 'file.bin');
  assert.deepEqual(received(page.messages), Buffer.from([0, 128, 255]));
  assert.equal(page.messages.at(-1).receivedBytes, 3);
});

test('unreadable object URLs produce a visible failed record', async () => {
  const page = createPage();
  await page.context.__operitPageDownloads.start('blob:https://example.test/expired', 'expired.txt');
  assert.equal(page.messages[0].type, 'begin');
  assert.equal(page.messages.at(-1).type, 'error');
});

test('cancelling while waiting for a chunk acknowledgement stops the stream', async () => {
  const page = createPage((event, context) => {
    if (event.type !== 'chunk') return;
    context.__operitPageDownloads.control(event.id, 'cancel', -1);
    return false;
  });
  const url = URL.createObjectURL(new Blob([new Uint8Array(200000)]));
  try {
    await page.context.__operitPageDownloads.start(url, 'file.bin');
    assert.equal(page.messages.filter(event => event.type === 'chunk').length, 1);
    assert.equal(page.messages.at(-1).type, 'error');
    assert.match(page.messages.at(-1).error, /cancelled/);
  } finally { URL.revokeObjectURL(url); }
});

test('ordinary navigation stays with the engine and injection is idempotent', () => {
  const page = createPage();
  const installed = page.context.__operitPageDownloads;
  vm.runInContext(script, page.context);
  assert.equal(page.context.__operitPageDownloads, installed);
  const anchor = {tagName: 'A', href: 'https://example.test/archive.zip', hasAttribute: () => true};
  page.listeners.get('click')({composedPath: () => [anchor], preventDefault: () => assert.fail('HTTP navigation must reach the engine')});
  assert.equal(page.messages.length, 0);
});

test('acknowledgements can reach a download in a child frame', () => {
  const page = createPage();
  const commands = [];
  page.context.frames.push({postMessage: (...args) => commands.push(args)});
  page.context.__operitPageDownloads.control('child-file', 'ack', 4);
  assert.equal(commands[0][0].id, 'child-file');
  assert.equal(commands[0][0].sequence, 4);
});
