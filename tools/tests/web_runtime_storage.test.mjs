import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const runtimePath = 'apps/flutter/app/web/runtime/src/';
const dataName = 'operit_runtime_storage.data';
const indexName = 'operit_runtime_storage.index';
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const MiB = 1024 * 1024;

/** Loads the actual storage and journal without starting the browser runtime. */
function implementation() {
  const journal = readFileSync(new URL(`${runtimePath}runtime_storage_compaction.ts`, root), 'utf8');
  const worker = readFileSync(new URL(`${runtimePath}operit_runtime_worker.ts`, root), 'utf8');
  const storage = worker.slice(worker.indexOf('class RuntimeWorkerStorage {'), worker.indexOf('/** Owns temporary append-only archive'));
  const helpers = worker.slice(worker.indexOf('async function openSyncAccessHandle('), worker.indexOf('/** Validates the opaque archive'));
  const code = stripTypeScriptTypes(`${journal}\n${storage}\n${helpers}`, { mode: 'transform' }).replace(/^export /gm, '');
  return new Function(`const textEncoder = new TextEncoder(), textDecoder = new TextDecoder();
    const runtimeStorageDataFileName = '${dataName}', runtimeStorageIndexFileName = '${indexName}';
    let nextId = 0;
    function createRandomUuid() { return String(++nextId); }
    ${code}
    return { RuntimeWorkerStorage, RuntimeStorageCompaction, recoverRuntimeStorageCompaction, storageCompactionFiles };`)();
}

const production = implementation();

/** Models sparse multi-gigabyte OPFS files, exclusive handles, and interrupted mutations. */
class Volume {
  files = new Map();
  operations = [];
  failAt = 0;
  maxRead = 0;
  maxWrite = 0;

  /** Throws after the selected mutation, including partially completed publication. */
  mutated(name, operation) {
    this.operations.push(`${name}:${operation}`);
    if (this.operations.length === this.failAt) throw new Error('simulated interruption');
  }

  /** Implements OPFS file lookup without silently creating missing originals. */
  async getFileHandle(name, { create }) {
    if (!this.files.has(name)) {
      if (!create) throw new Error(`missing file: ${name}`);
      this.files.set(name, { size: 0, pages: new Map(), opened: false });
    }
    const file = this.files.get(name);
    const volume = this;
    return {
      /** Supplies the existing index timestamp used by the legacy migration. */
      async getFile() { return { lastModified: 1234 }; },
      /** Acquires an exclusive access handle with exact short-read behavior. */
      async createSyncAccessHandle() {
        assert.equal(file.opened, false, `duplicate handle: ${name}`);
        file.opened = true;
        let closed = false;
        /** Rejects access through a released handle. */
        function requireOpen() { if (closed) throw new Error('closed handle'); }
        return {
          /** Returns the logical length without allocating sparse holes. */
          getSize() { requireOpen(); return file.size; },
          /** Reads initialized pages and zero-filled holes. */
          read(buffer, { at = 0 }) {
            requireOpen();
            volume.maxRead = Math.max(volume.maxRead, buffer.length);
            const count = Math.max(0, Math.min(buffer.length, file.size - at));
            buffer.fill(0);
            for (let offset = 0; offset < count;) {
              const position = at + offset;
              const pageNumber = Math.floor(position / 65536);
              const pageOffset = position % 65536;
              const length = Math.min(count - offset, 65536 - pageOffset);
              const page = file.pages.get(pageNumber);
              if (page) buffer.set(page.subarray(pageOffset, pageOffset + length), offset);
              offset += length;
            }
            return count;
          },
          /** Persists a byte range while keeping sparse holes unallocated. */
          write(buffer, { at = 0 }) {
            requireOpen();
            volume.maxWrite = Math.max(volume.maxWrite, buffer.length);
            for (let offset = 0; offset < buffer.length;) {
              const position = at + offset;
              const pageNumber = Math.floor(position / 65536);
              const pageOffset = position % 65536;
              const length = Math.min(buffer.length - offset, 65536 - pageOffset);
              let page = file.pages.get(pageNumber);
              if (!page) { page = new Uint8Array(65536); file.pages.set(pageNumber, page); }
              page.set(buffer.subarray(offset, offset + length), pageOffset);
              offset += length;
            }
            file.size = Math.max(file.size, at + buffer.length);
            volume.mutated(name, 'write');
            return buffer.length;
          },
          /** Releases truncated pages and clears the retained partial page. */
          truncate(size) {
            requireOpen();
            for (const [number, page] of file.pages) {
              if (number * 65536 >= size) file.pages.delete(number);
              else if ((number + 1) * 65536 > size) page.fill(0, size % 65536);
            }
            file.size = size;
            volume.mutated(name, 'truncate');
          },
          /** Exposes each durability boundary for crash injection. */
          flush() { requireOpen(); volume.mutated(name, 'flush'); },
          /** Releases ownership without erasing bytes or pending journals. */
          close() { closed = true; file.opened = false; },
        };
      },
    };
  }

  /** Opens a named file for direct journal tests. */
  async open(name) { return (await this.getFileHandle(name, { create: true })).createSyncAccessHandle(); }

  /** Seeds historical storage, including records near the end of a large sparse file. */
  async seed(name, bytes, offset = 0) {
    const file = await this.open(name);
    file.write(bytes, { at: offset });
    file.close();
  }

  /** Reads an isolated small file after releasing all application handles. */
  async bytes(name) {
    const file = await this.open(name);
    const bytes = new Uint8Array(file.getSize());
    file.read(bytes, { at: 0 });
    file.close();
    return bytes;
  }
}

/** Seeds a valid version 2 index with obsolete bytes preceding its live records. */
async function historicalVolume(offset = 20 * 1024 ** 3) {
  const volume = new Volume();
  await volume.seed(dataName, encoder.encode('asset'), 0);
  await volume.seed(dataName, encoder.encode('database'), offset);
  await volume.seed(indexName, encoder.encode(JSON.stringify({ version: 2, records: [
    ['asset', { offset: 0, byteLength: 5, modifiedAt: 1200 }],
    ['database', { offset, byteLength: 8, modifiedAt: 3400 }],
  ] })));
  volume.operations = [];
  volume.maxRead = volume.maxWrite = 0;
  return volume;
}

/** Asserts that journal copies cannot be retained after successful publication. */
function assertJournalEmpty(volume) {
  for (const name of production.storageCompactionFiles) assert.equal(volume.files.get(name).size, 0, name);
}

test('startup shrinks a historical 20 GiB container to its live bytes and preserves metadata', async () => {
  const volume = await historicalVolume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  assert.equal(volume.files.get(dataName).size, 13);
  assert.equal(decoder.decode(storage.read('asset')), 'asset');
  assert.equal(decoder.decode(storage.read('database')), 'database');
  assert.equal(storage.modifiedAt('asset'), 1200);
  assert.equal(storage.modifiedAt('database'), 3400);
  assert.ok(volume.maxRead <= 65536);
  assert.ok(volume.maxWrite <= 65536);
  assertJournalEmpty(volume);
  storage.close();
  const reopened = await production.RuntimeWorkerStorage.open(volume);
  assert.equal(decoder.decode(reopened.read('database')), 'database');
  reopened.close();
});

test('repeated database replacements keep obsolete storage bounded', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  for (let version = 0; version < 50; version++) {
    const bytes = new Uint8Array(MiB).fill(version);
    storage.write('database', bytes);
    assert.ok(volume.files.get(dataName).size < 17 * MiB);
    assert.equal(storage.readRange('database', MiB - 1, 1)[0], version);
  }
  assertJournalEmpty(volume);
  storage.close();
});

test('identical database images do not append bytes and retain normal timestamp semantics', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  const bytes = new Uint8Array(MiB).fill(42);
  storage.write('database', bytes);
  const modified = storage.modifiedAt('database');
  volume.operations = [];
  for (let count = 0; count < 20; count++) storage.write('database', bytes);
  assert.equal(volume.files.get(dataName).size, MiB);
  assert.equal(volume.operations.some(operation => operation === `${dataName}:write`), false);
  assert.ok(storage.modifiedAt('database') > modified);
  storage.close();
});

test('tail appends write only new bytes while non-tail appends preserve both files', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  for (let count = 0; count < 40; count++) storage.append('log', new Uint8Array(65536).fill(count));
  assert.equal(volume.files.get(dataName).size, 40 * 65536);
  storage.write('other', encoder.encode('keep'));
  storage.append('log', Uint8Array.of(255));
  assert.equal(decoder.decode(storage.read('other')), 'keep');
  assert.equal(storage.readRange('log', 40 * 65536, 1)[0], 255);
  assert.equal(storage.readRange('log', 39 * 65536, 1)[0], 39);
  storage.close();
});

test('interleaved streaming writers preserve unpublished bytes and defer compaction', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  storage.write('database', encoder.encode('before'));
  const left = storage.createWriteSession('left');
  const right = storage.createWriteSession('right');
  storage.writeSessionChunk(left, encoder.encode('left-'));
  storage.writeSessionChunk(right, encoder.encode('right-'));
  storage.write('database', new Uint8Array(17 * MiB).fill(12));
  storage.write('database', encoder.encode('after'));
  assert.ok(volume.files.get(dataName).size > 17 * MiB);
  storage.writeSessionChunk(left, encoder.encode('end'));
  storage.writeSessionChunk(right, encoder.encode('end'));
  storage.commitWriteSession(left);
  assert.ok(volume.files.get(dataName).size > 17 * MiB);
  storage.commitWriteSession(right);
  assert.equal(decoder.decode(storage.read('left')), 'left-end');
  assert.equal(decoder.decode(storage.read('right')), 'right-end');
  assert.equal(decoder.decode(storage.read('database')), 'after');
  assert.equal(volume.files.get(dataName).size, 22);
  storage.close();
});

test('discard and recursive deletion reclaim abandoned bytes without removing surviving files', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  storage.write('keep', encoder.encode('safe'));
  storage.write('remove/large', new Uint8Array(17 * MiB));
  storage.delete('remove', true);
  assert.equal(volume.files.get(dataName).size, 4);
  const abandoned = storage.createWriteSession('abandoned');
  storage.writeSessionChunk(abandoned, new Uint8Array(17 * MiB));
  storage.discardWriteSession(abandoned);
  assert.equal(storage.hasFile('abandoned'), false);
  assert.equal(volume.files.get(dataName).size, 4);
  assert.equal(decoder.decode(storage.read('keep')), 'safe');
  storage.delete('keep', false);
  assert.equal(volume.files.get(dataName).size, 0);
  storage.close();
});

/** Runs one journal compaction, optionally interrupting any individual storage mutation. */
async function interruptedCompaction(failAt) {
  const volume = await historicalVolume(100);
  const data = await volume.open(dataName);
  const index = await volume.open(indexName);
  const journal = await production.RuntimeStorageCompaction.open(volume, data, index);
  const records = new Map(JSON.parse(decoder.decode(await indexBytes(index))).records);
  volume.operations = [];
  volume.failAt = failAt;
  try {
    journal.compact(records);
  } catch (error) {
    assert.match(error.message, /simulated interruption/);
  } finally {
    journal.close(); data.close(); index.close();
  }
  const operations = volume.operations.slice();
  volume.failAt = 0;
  await production.recoverRuntimeStorageCompaction(volume);
  const recoveredData = await volume.bytes(dataName);
  const recoveredIndex = JSON.parse(decoder.decode(await volume.bytes(indexName)));
  for (const [key, record] of recoveredIndex.records) {
    assert.equal(decoder.decode(recoveredData.subarray(record.offset, record.offset + record.byteLength)), key === 'asset' ? 'asset' : 'database');
  }
  assertJournalEmpty(volume);
  return operations;
}

/** Reads a small metadata image without releasing the journal's original index handle. */
function indexBytes(index) {
  const bytes = new Uint8Array(index.getSize());
  index.read(bytes, { at: 0 });
  return bytes;
}

test('every interruption boundary recovers a matching data/index pair for runtime and maintenance', async () => {
  const operations = await interruptedCompaction(0);
  assert.ok(operations.length > 15);
  for (let failure = 1; failure <= operations.length; failure++) await interruptedCompaction(failure);
});

test('an interrupted runtime compaction releases handles and refuses subsequent writes', async () => {
  const volume = new Volume();
  const storage = await production.RuntimeWorkerStorage.open(volume);
  storage.write('database', new Uint8Array(17 * MiB).fill(1));
  volume.operations = [];
  // Persisting the small replacement uses five mutations, then compaction starts copying.
  volume.failAt = 6;
  assert.throws(() => storage.write('database', encoder.encode('new')), /simulated interruption/);
  assert.throws(() => storage.write('database', encoder.encode('unsafe')), /closed handle/);
  assert.ok([...volume.files.values()].every(file => !file.opened));
  volume.failAt = 0;
  const reopened = await production.RuntimeWorkerStorage.open(volume);
  assert.equal(decoder.decode(reopened.read('database')), 'new');
  reopened.close();
});

test('maintenance clears uncommitted scratch for an identity removed by snapshot restore', async () => {
  const volume = new Volume();
  await volume.seed('operit_runtime_storage.compact.ready', new Uint8Array());
  await volume.seed('operit_runtime_storage.compact.data', encoder.encode('abandoned copy'));
  await production.recoverRuntimeStorageCompaction(volume);
  assert.equal(volume.files.has(dataName), false);
  assert.equal(volume.files.has(indexName), false);
  assertJournalEmpty(volume);
});
