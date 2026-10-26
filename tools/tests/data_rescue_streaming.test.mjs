import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_data_rescue_worker.ts', import.meta.url), 'utf8');

/** Extracts the production worker functions while retaining their real chunk loops. */
function implementation(start, end, dependencies, result) {
  const first = source.indexOf(start);
  const last = source.indexOf(end, first);
  assert.ok(first >= 0 && last > first);
  const code = stripTypeScriptTypes(source.slice(first, last), { mode: 'transform' });
  return new Function(...Object.keys(dependencies), `${code}; return ${result};`)(...Object.values(dependencies));
}

/** Copies a virtual multi-gigabyte resource without creating a resource-sized allocation. */
test('browser rescue publication copies a 5 GiB source in bounded ranges with actual progress', async () => {
  const size = 5 * 1024 ** 3 + 17;
  const scratch = new Uint8Array(65536);
  let readOffset = 0;
  let writeOffset = 0;
  let flushed = false;
  let closed = false;
  const progress = { phase: 'publish', completed: 0, total: size, unit: 'bytes' };
  const output = {
    truncate(length) { assert.equal(length, 0); },
    flush() { flushed = true; },
    close() { closed = true; },
  };
  const copy = implementation('async function copy(', '/** Enumerates origin files', {
    read(_source, offset, length) {
      assert.equal(offset, readOffset);
      assert.ok(length > 0 && length <= 65536);
      readOffset += length;
      return scratch.subarray(0, length);
    },
    write(handle, offset, bytes) {
      assert.equal(handle, output);
      assert.equal(offset, writeOffset);
      writeOffset += bytes.length;
    },
    advance(counter, amount) {
      assert.ok(amount <= 65536);
      assert.equal(counter.completed + amount, writeOffset);
      counter.completed += amount;
    },
  }, 'copy');
  await copy({ handle: { getSize() { return size; } } }, { async createSyncAccessHandle() { return output; } }, progress);
  assert.equal(readOffset, size);
  assert.equal(writeOffset, size);
  assert.equal(progress.completed, size);
  assert.equal(flushed, true);
  assert.equal(closed, true);
});

/** Fails a partial source before advancing its write progress or reporting a completed copy. */
test('browser rescue publication rejects truncated chunks and closes its destination', async () => {
  let closed = false;
  let advanced = false;
  const copy = implementation('async function copy(', '/** Enumerates origin files', {
    read() { return new Uint8Array(3); },
    write() { assert.fail('A truncated source must not be written'); },
    advance() { advanced = true; },
  }, 'copy');
  const output = { truncate() {}, flush() { assert.fail('Truncated output must not be flushed'); }, close() { closed = true; } };
  await assert.rejects(copy({ handle: { getSize() { return 65536; } } }, { async createSyncAccessHandle() { return output; } }, {}), /Truncated/);
  assert.equal(closed, true);
  assert.equal(advanced, false);
});

/** Reconstructs large Blob/File values with file references and zero payload reads. */
test('browser rescue keeps a 5 GiB Blob on disk while decoding nested database records', async () => {
  const size = 5 * 1024 ** 3;
  let payloadReads = 0;
  let closed = 0;
  let reopened = 0;
  const underlyingFile = { size };
  const staged = {
    handle: { close() { closed += 1; } },
    file: {
      async getFile() { assert.equal(closed, 1); return underlyingFile; },
      async createSyncAccessHandle() { reopened += 1; return { close() {} }; },
    },
  };
  class BlobReference {
    constructor(parts, options) { this.parts = parts; this.type = options.type; }
  }
  class FileReference extends BlobReference {
    constructor(parts, name, options) { super(parts, options); this.name = name; }
  }
  const decode = implementation('async function stagedBlob(', '/** Checks whether a decoded value', {
    requireStaged(path) { assert.equal(path, 'browser/binary/large'); return staged; },
    read() { payloadReads += 1; assert.fail('Blob payload must remain on disk'); },
    reportProgress() {}, advance() {},
    Blob: BlobReference, File: FileReference,
  }, 'decode');
  const record = await decode({ type: 'object', value: [['attachment', { type: 'binary', kind: 'Blob', path: 'browser/binary/large', mime: 'application/octet-stream' }]] });
  assert.equal(record.attachment.parts[0], underlyingFile);
  assert.equal(record.attachment.type, 'application/octet-stream');
  assert.equal(payloadReads, 0);
  assert.equal(closed, 1);
  assert.equal(reopened, 1);
});

/** Rejects a metadata-sized allocation before reading an oversized catalog or index. */
test('browser rescue enforces a metadata bound before allocating', () => {
  let reads = 0;
  const metadataBytes = implementation('function metadataBytes(', '/** Opens a file-backed binary', {
    metadataLimit: 16 * 1024 * 1024,
    read() { reads += 1; assert.fail('Oversized metadata must not be read'); },
    requireStaged() { assert.fail('Direct metadata source was supplied'); },
  }, 'metadataBytes');
  assert.throws(() => metadataBytes({ handle: { getSize() { return 5 * 1024 ** 3; } } }), /exceeds 16 MiB/);
  assert.equal(reads, 0);
});

/** Verifies preference serialization handles a large record as bounded I/O rather than a large Host read. */
test('browser rescue converts preference records larger than one chunk using bounded reads', async () => {
  const size = 196625;
  const preferenceKey = 'operit2.runtime.runtime/config/preferences/settings.json';
  const indexPath = 'opfs/runtime/identities/identity-test/operit_runtime_storage.index';
  const dataPath = 'opfs/runtime/identities/identity-test/operit_runtime_storage.data';
  const index = { records: [[preferenceKey, { offset: 0, byteLength: size }]] };
  const sources = new Map([[indexPath, { handle: { close() {} } }], [dataPath, { handle: { getSize() { return size; }, close() {} } }]]);
  const reads = [];
  const output = { truncate() {}, close() {}, flush() {} };
  const convert = implementation('async function snapshotPortableRuntime(', '/** Captures database schemas', {
    sources, snapshot: {}, decoder: new TextDecoder(), encoder: new TextEncoder(), metadataLimit: 16 * 1024 * 1024,
    metadataBytes() { return new TextEncoder().encode(JSON.stringify(index)); },
    async resource() { return { async createSyncAccessHandle() { return output; } }; },
    read(_source, offset, count) {
      assert.ok(count <= 65536);
      reads.push([offset, count]);
      return new Uint8Array(count);
    },
    preferenceStoragePath() { return 'runtime/config/preferences/settings.json'; },
    portablePreferences(_path, bytes) { assert.equal(bytes.length, size); return new TextEncoder().encode('{}'); },
    write(_output, _offset, bytes) { assert.equal(bytes.length, 2); },
    async snapshotBytes() {}, reportProgress() {}, advance(progress, amount) { progress.completed += amount; },
  }, 'snapshotPortableRuntime');
  await convert();
  assert.deepEqual(reads.map(([, count]) => count), [65536, 65536, 65536, 17]);
});

/** Refuses an oversized exported catalog before creating a snapshot that its importer cannot read. */
test('browser rescue rejects oversized export metadata before opening a destination', async () => {
  class VirtualBlob { constructor() { this.size = 32 * 1024 * 1024; } }
  const snapshotBytes = implementation('async function snapshotBytes(', '/** Serializes structured database values', {
    Blob: VirtualBlob, metadataLimit: 16 * 1024 * 1024,
    resource() { assert.fail('Oversized metadata must fail before opening its output'); },
  }, 'snapshotBytes');
  await assert.rejects(snapshotBytes('browser/catalog.json', new VirtualBlob()), /exceeds 16 MiB/);
});

/** Excludes only the real browser archive namespace before any payload handle is opened. */
test('browser rescue excludes identity staging and saved export copies from inventory', async () => {
  const opened = [];
  /** Creates a source file that records accidental reads of temporary payloads. */
  function file(path) {
    return { kind: 'file', async createSyncAccessHandle() { opened.push(path); return {}; } };
  }
  /** Exposes a deterministic OPFS directory tree to the production enumerator. */
  function directory(entries) {
    return { kind: 'directory', async *entries() { yield* entries; } };
  }
  const identity = directory([
    ['operit_archive_staging.data', file('temporary archive container')],
    ['snapshot_exports', directory([['old.zip', file('temporary export copy')]])],
    ['operit_runtime_storage.data', file('runtime data')],
    ['data', directory([['snapshot_exports', directory([['user.zip', file('user export')]])]])],
  ]);
  const root = directory([
    ['runtime', directory([['identities', directory([['identity-test', identity]])]])],
    ['workspaces', directory([['snapshot_exports', directory([['project.zip', file('workspace export')]])]])],
    ['data-rescue', directory([['archive.zip', file('maintenance archive')]])],
  ]);
  const collect = implementation('async function collect(', '/** Converts a database request', {
    reportProgress() {},
  }, 'collect');
  const inventory = new Map();
  await collect(root, '', inventory);
  assert.deepEqual(opened, ['runtime data', 'user export', 'workspace export']);
  assert.deepEqual([...inventory.keys()], [
    'opfs/runtime/identities/identity-test/operit_runtime_storage.data',
    'opfs/runtime/identities/identity-test/data/snapshot_exports/user.zip',
    'opfs/workspaces/snapshot_exports/project.zip',
  ]);
});
