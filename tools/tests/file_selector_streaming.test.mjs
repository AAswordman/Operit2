import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const prefix = 'apps/flutter/thirdparty/file_selector_ohos/ohos/src/main/ets/file_selector/';
const source = path => readFileSync(new URL(path, root), 'utf8');
const chunkBytes = 1024 * 1024;

/** Runs the actual native selector implementation against asynchronous URI I/O. */
function fixture() {
  const text = source(`${prefix}FileSelectorApiImpl.ets`);
  const js = stripTypeScriptTypes(text.slice(text.indexOf('const TAG =')).replace('export class', 'class'), { mode: 'transform' });
  const calls = [];
  const handles = new Set();
  let nextFd = 0;
  let failRead = false;
  let readGate = null;
  let shortWrite = false;
  const fs = {
    OpenMode: { READ_ONLY: 0, READ_WRITE: 2, CREATE: 4, TRUNC: 8 },
    async open(path) {
      calls.push(['open', path]);
      const file = { fd: ++nextFd };
      handles.add(file.fd);
      return file;
    },
    async stat(fd) {
      assert.ok(handles.has(fd));
      calls.push(['stat', fd]);
      return { size: 143388128 };
    },
    async read(fd, buffer, { offset, length }) {
      assert.ok(handles.has(fd));
      assert.ok(buffer.byteLength <= chunkBytes);
      assert.ok(length <= buffer.byteLength);
      calls.push(['read', fd, offset, length]);
      await readGate;
      if (failRead) throw new Error('READ_FAILED');
      const count = Math.max(0, Math.min(length, 143388128 - offset));
      const bytes = new Uint8Array(buffer);
      for (let i = 0; i < count; i++) bytes[i] = (offset + i) % 251;
      return count;
    },
    /** Writes only one bounded source chunk and can expose an incomplete destination. */
    async write(fd, buffer, { offset, length }) {
      assert.ok(handles.has(fd));
      assert.ok(buffer.byteLength <= 65536);
      assert.ok(length <= buffer.byteLength);
      calls.push(['write', fd, offset, length]);
      return shortWrite ? length - 1 : length;
    },
    /** Requires an open destination when the copy requests durability. */
    async fsync(fd) { assert.ok(handles.has(fd)); calls.push(['fsync', fd]); },
    async close(file) {
      assert.ok(handles.delete(file.fd), 'Descriptor must be closed exactly once');
      calls.push(['close', file.fd]);
    },
  };
  const channels = new Map();
  class Channel {
    constructor(_messenger, name) { channels.set(name, this); }
    setMethodCallHandler(handler) { this.handler = handler; }
    setMessageHandler(_handler) {}
    /** Records platform progress events without allocating any file-sized payload. */
    invokeMethod(method, values) { calls.push([method, new Map(values)]); }
  }
  class FileResponse {
    constructor(path, mimeType, name, size) { Object.assign(this, { path, mimeType, name, size }); }
  }
  const factory = new Function('fs', 'Log', 'MethodChannel', 'BasicMessageChannel', 'FileSelectorApiCodec', 'FileResponse', `${js}; return FileSelectorApiImpl;`);
  const Api = factory(fs, { e() {}, d() {}, i() {} }, Channel, Channel, { INSTANCE: {} }, FileResponse);
  const api = new Api({});
  api.setup({}, {});
  const input = channels.get('dev.flutter.packages.file_selector_ohos/input');
  const call = (method, args) => new Promise((resolve, reject) => {
    input.handler.onMethodCall({ method, arguments: new Map(Object.entries(args)) }, {
      success: resolve,
      error: (_code, message) => reject(new Error(message)),
      notImplemented: () => reject(new Error('NOT_IMPLEMENTED')),
    });
  });
  return { Api, api, call, calls, handles, channels,
    /** Calls the real save-channel handler with a URI destination. */
    save() { return new Promise((resolve, reject) => channels.get('dev.flutter.packages.file_selector_ohos/save').handler.onMethodCall(
      { method: 'copyFileFromPathWithProgress', arguments: new Map([['sourcePath', '/host/archive'], ['destinationUri', 'file://selected/archive']]) },
      { success: resolve, error: (_code, message) => reject(new Error(message)), notImplemented: () => reject(new Error('NOT_IMPLEMENTED')) },
    )); },
    shortWrites() { shortWrite = true; },
    failReads() { failRead = true; },
    blockRead() { return new Promise(resolve => { readGate = new Promise(release => resolve(release)); }); },
  };
}

test('OHOS picker queries metadata only and releases descriptors, including multi-select', async () => {
  const f = fixture();
  const files = await f.Api.toFileListResponse(['file://large.apk', 'file://other.zip'], '*/*');
  assert.equal(files.length, 2);
  assert.deepEqual({ ...files[0] }, { path: 'file://large.apk', mimeType: '*/*', name: 'large.apk', size: 143388128 });
  assert.deepEqual(f.calls.map(call => call[0]), ['open', 'stat', 'close', 'open', 'stat', 'close']);
  assert.equal(f.handles.size, 0);
});

test('OHOS native stream reads 143 MB with bounded buffers and exact offsets', async () => {
  const f = fixture();
  const token = await f.call('open', { path: 'file://large.apk', start: 0 });
  let offset = 0;
  while (offset < 143388128) {
    const bytes = await f.call('readChunk', { token, maxBytes: chunkBytes });
    assert.ok(bytes.byteLength <= chunkBytes);
    assert.equal(bytes[0], offset % 251);
    assert.equal(bytes.at(-1), (offset + bytes.byteLength - 1) % 251);
    offset += bytes.byteLength;
  }
  assert.equal(offset, 143388128);
  assert.equal((await f.call('readChunk', { token, maxBytes: chunkBytes })).length, 0);
  await f.call('close', { token });
  await f.call('close', { token });
  assert.equal(f.handles.size, 0);
});

test('OHOS rejects oversized native read requests before allocating or reading', async () => {
  const f = fixture();
  const token = await f.call('open', { path: 'file://large.apk', start: 10 });
  for (const maxBytes of [0, -1, chunkBytes + 1, 1.5, NaN]) {
    await assert.rejects(f.call('readChunk', { token, maxBytes }), /Invalid document input read/);
  }
  assert.equal(f.calls.filter(call => call[0] === 'read').length, 0);
  const bytes = await f.call('readChunk', { token, maxBytes: 6 });
  assert.deepEqual([...bytes], [10, 11, 12, 13, 14, 15]);
  await f.call('close', { token });
});

test('OHOS read failure does not poison the close queue', async () => {
  const f = fixture();
  const token = await f.call('open', { path: 'file://large.apk', start: 0 });
  f.failReads();
  await assert.rejects(f.call('readChunk', { token, maxBytes: 8 }), /READ_FAILED/);
  await f.call('close', { token });
  assert.equal(f.handles.size, 0);
});

test('OHOS detach waits for in-flight reads and then closes every descriptor', async () => {
  const f = fixture();
  const token = await f.call('open', { path: 'file://large.apk', start: 0 });
  await f.call('open', { path: 'file://other.apk', start: 5 });
  const release = await f.blockRead();
  const reading = f.call('readChunk', { token, maxBytes: 8 });
  await new Promise(resolve => setImmediate(resolve));
  f.api.closeChannels();
  assert.equal(f.handles.size, 2);
  release();
  await reading;
  await f.api.inputOperations;
  assert.equal(f.handles.size, 0);
  assert.equal(f.channels.get('dev.flutter.packages.file_selector_ohos/input').handler, null);
});

test('OHOS selection protocol contains no full-file content field in Dart or native codecs', () => {
  const dart = source('apps/flutter/thirdparty/file_selector_ohos/lib/src/file_selector_api.g.dart');
  const native = source(`${prefix}GeneratedFileSelectorApi.ets`);
  assert.doesNotMatch(dart, /Uint8List bytes|this\.bytes|result\[4\]/);
  assert.doesNotMatch(native, /getBytes|setBytes|this\.bytes|list\[4\]/);
  assert.doesNotMatch(source(`${prefix}FileSelectorApiImpl.ets`), /new ArrayBuffer\(size\)|getUrisForPaths|fdopenStreamSync/);
});

/** Verifies real URI saving uses bounded writes and completes progress only after flushing. */
test('OHOS progress saving copies 143 MB in 64 KiB writes without a whole-file allocation', async () => {
  const f = fixture();
  await f.save();
  const writes = f.calls.filter(call => call[0] === 'write');
  assert.ok(writes.length > 2000);
  let completed = 0;
  for (const [, , offset, length] of writes) { assert.equal(offset, completed); assert.ok(length <= 65536); completed += length; }
  assert.equal(completed, 143388128);
  const progress = f.calls.filter(call => call[0] === 'saveProgress');
  assert.equal(progress[0][1].get('completed'), 0);
  assert.equal(progress.at(-1)[1].get('completed'), completed);
  assert.equal(progress.at(-1)[1].get('total'), completed);
  assert.ok(f.calls.findIndex(call => call[0] === 'fsync') < f.calls.findLastIndex(call => call[0] === 'saveProgress'));
  assert.equal(f.handles.size, 0);
});

/** Propagates a partial write before reporting any completed copy. */
test('OHOS progress saving rejects partial destination writes and closes both descriptors', async () => {
  const f = fixture();
  f.shortWrites();
  await assert.rejects(f.save(), /Incomplete document destination write/);
  assert.equal(f.calls.filter(call => call[0] === 'saveProgress').at(-1)[1].get('completed'), 0);
  assert.equal(f.calls.some(call => call[0] === 'fsync'), false);
  assert.equal(f.handles.size, 0);
});
