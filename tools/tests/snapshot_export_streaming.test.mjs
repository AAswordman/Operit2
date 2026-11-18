import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads one production source file without evaluating its application bootstrap. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Extracts one exact documented source section for isolated production-code execution. */
function section(text, start, end) {
  const from = text.indexOf(start);
  assert.notEqual(from, -1, `Missing section: ${start}`);
  const to = text.indexOf(end, from + start.length);
  assert.notEqual(to, -1, `Missing section end: ${end}`);
  return text.slice(from, to);
}

/** Creates the actual worker staging class over a deterministic bounded-write storage handle. */
function stagingFixture() {
  const worker = source('apps/flutter/app/web/runtime/src/operit_runtime_worker.ts');
  const classSource = section(worker, 'class RuntimeWorkerArchiveStaging {', '/** Opens one pre-created OPFS file');
  const helpers = worker.slice(worker.indexOf('function readRecord('));
  const js = stripTypeScriptTypes(classSource + '\n' + helpers, { mode: 'transform' });
  const calls = [];
  const factory = new Function('callMainHost', 'runtimeIdentityId', `${js}; return data => new RuntimeWorkerArchiveStaging(data);`);
  const data = new TestSyncAccessHandle();
  const staging = factory((module, method, args) => {
    calls.push({ module, method, args });
    return method === 'openExportFile' ? 'blob:snapshot-export-test' : null;
  }, 'identity-snapshot-test')(data);
  return { staging, data, calls };
}

/** Implements the OPFS handle contract while recording the largest individual write. */
class TestSyncAccessHandle {
  bytes = new Uint8Array(2 * 1024 * 1024);
  size = 0;
  largestWrite = 0;
  flushes = 0;

  /** Returns the currently persisted logical length. */
  getSize() { return this.size; }

  /** Writes exactly one range without changing unrelated extents. */
  write(content, { at = 0 } = {}) {
    this.largestWrite = Math.max(this.largestWrite, content.byteLength);
    assert.ok(at + content.byteLength <= this.bytes.byteLength);
    this.bytes.set(content, at);
    this.size = Math.max(this.size, at + content.byteLength);
    return content.byteLength;
  }

  /** Reads exactly the requested persisted range. */
  read(buffer, { at = 0 } = {}) {
    const length = Math.min(buffer.byteLength, this.size - at);
    buffer.set(this.bytes.subarray(at, at + length));
    return length;
  }

  /** Changes the logical length of the test container. */
  truncate(size) { this.size = size; }

  /** Records the explicit host sealing flush. */
  flush() { this.flushes += 1; }

  /** Implements resource release for the production handle contract. */
  close() {}
}

/** Verifies growable output, ZIP header updates, and descriptor-only export results. */
test('snapshot staging grows in bounded chunks and preserves header updates', () => {
  const { staging, data, calls } = stagingFixture();
  staging.createExport('snapshot');
  const chunk = new Uint8Array(65536).fill(42);
  for (let index = 0; index < 16; index += 1) {
    staging.writeExport('snapshot', index * chunk.byteLength, chunk);
  }
  staging.writeExport('snapshot', 0, Uint8Array.of(80, 75));
  assert.equal(staging.seal('snapshot'), 16 * chunk.byteLength);
  assert.equal(data.largestWrite, 65536);
  assert.deepEqual([...staging.read('snapshot', 0, 4)], [80, 75, 42, 42]);
  assert.equal(staging.fileReference('snapshot'), 'blob:snapshot-export-test');
  assert.deepEqual(calls, [{ module: 'archiveStaging', method: 'openExportFile', args: ['identity-snapshot-test', 0, 1048576] }]);
  staging.remove('snapshot');
  assert.equal(data.getSize(), 0);
  assert.equal(calls.at(-1).method, 'releaseExportFile');
});

/** Rejects malformed output ranges rather than accepting oversized buffering or sparse writes. */
test('snapshot staging rejects invalid ranges, oversized chunks, and sealed writes', () => {
  const { staging } = stagingFixture();
  staging.createExport('snapshot');
  assert.throws(() => staging.fileReference('snapshot'), /must be sealed/);
  assert.throws(() => staging.writeExport('snapshot', 1, Uint8Array.of(1)), /write range/);
  assert.throws(() => staging.writeExport('snapshot', -1, Uint8Array.of(1)), /write range/);
  assert.throws(() => staging.writeExport('snapshot', 0, new Uint8Array(65537)), /write range/);
  assert.throws(() => staging.append('snapshot', Uint8Array.of(1)), /ranged writes/);
  staging.writeExport('snapshot', 0, Uint8Array.of(1, 2));
  staging.seal('snapshot');
  assert.throws(() => staging.writeExport('snapshot', 0, Uint8Array.of(3)), /active snapshot export/);
});

/** Keeps exact-length uploads distinct from growable snapshot generation. */
test('streamed snapshot support preserves exact-length archive upload validation', () => {
  const { staging } = stagingFixture();
  staging.create('upload', 4);
  assert.throws(() => staging.writeExport('upload', 0, Uint8Array.of(1)), /active snapshot export/);
  staging.append('upload', Uint8Array.of(1, 2));
  assert.throws(() => staging.seal('upload'), /declared byte length/);
  assert.throws(() => staging.append('upload', Uint8Array.of(3, 4, 5)), /declared byte length/);
  staging.append('upload', Uint8Array.of(3, 4));
  assert.equal(staging.seal('upload'), 4);
  assert.deepEqual([...staging.read('upload', 0, 4)], [1, 2, 3, 4]);
});

/** Makes extent collisions explicit instead of overwriting an unrelated archive. */
test('snapshot export cannot grow into a separately reserved upload', () => {
  const { staging } = stagingFixture();
  staging.createExport('snapshot');
  staging.writeExport('snapshot', 0, Uint8Array.of(1, 2));
  staging.create('upload', 4);
  assert.throws(() => staging.writeExport('snapshot', 2, Uint8Array.of(3)), /another archive extent/);
  staging.writeExport('snapshot', 0, Uint8Array.of(80, 75));
  assert.equal(staging.seal('snapshot'), 2);
  staging.append('upload', Uint8Array.of(3, 4, 5, 6));
  staging.seal('upload');
  assert.deepEqual([...staging.read('upload', 0, 4)], [3, 4, 5, 6]);
});

/** Prevents the raw snapshot path from reintroducing full ZIP payloads in Rust or Flutter. */
test('snapshot export uses host files throughout generation and saving', () => {
  const backup = source('core/crates/runtime/application/src/data/backup/RawSnapshotBackupManager.rs');
  const exportCode = section(backup, 'pub(crate) fn exportSnapshot', '/// Replaces runtime storage contents');
  const archive = source('core/crates/persistence/rescue/src/archive.rs');
  const writer = section(archive, 'pub fn write_archive', 'fn archive_names');
  assert.match(exportCode, /archive::write_archive\(\s*output,/);
  assert.match(writer, /ZipWriter::new\(output\)/);
  assert.match(writer, /large_file\(true\)/);
  assert.doesNotMatch(exportCode, /Cursor|into_inner|to_string_pretty/);
  const ui = section(source('apps/flutter/app/lib/ui/features/settings/data/DataSettingsPanel.dart'), 'Future<void> _exportRawSnapshot()', 'Future<void> _importRawSnapshot()');
  assert.match(ui, /saveGeneratedFile/);
  assert.match(ui, /discardArchiveUpload/);
  assert.doesNotMatch(ui, /saveBytes|Uint8List|readAsBytes/);
  const web = section(source('apps/flutter/thirdparty/file_selector_web/lib/file_selector_web.dart'), 'Future<FileSaveLocation?> saveGeneratedFile', 'Future<XFile?> openFile');
  assert.match(web, /showSaveFilePicker/);
  assert.match(web, /pipeTo\(output\)/);
  assert.doesNotMatch(web, /readAsBytes|arrayBuffer|toList/);
});


/** Keeps upload writes buffered until one successful, idempotent sealing flush. */
test('snapshot upload flushes once at sealing rather than once per chunk', () => {
  const { staging, data } = stagingFixture();
  const chunk = new Uint8Array(64 * 1024).fill(42);
  staging.create('upload', 16 * chunk.byteLength);
  const initialFlushes = data.flushes;
  for (let index = 0; index < 16; index += 1) {
    staging.append('upload', chunk);
  }
  assert.equal(data.flushes, initialFlushes);
  assert.throws(() => staging.read('upload', 0, 4), /not sealed/);
  assert.equal(staging.seal('upload'), 16 * chunk.byteLength);
  assert.equal(data.flushes, initialFlushes + 1);
  assert.equal(staging.seal('upload'), 16 * chunk.byteLength);
  assert.equal(data.flushes, initialFlushes + 1);
  assert.deepEqual([...staging.read('upload', chunk.byteLength - 2, 4)], [42, 42, 42, 42]);
});

/** Leaves failed flushes unsealed and rejects reads until persistence succeeds. */
test('snapshot upload remains unsealed when the sealing flush fails', () => {
  const { staging, data } = stagingFixture();
  staging.create('upload', 4);
  staging.append('upload', Uint8Array.of(1, 2, 3, 4));
  const flush = data.flush.bind(data);
  data.flush = () => { throw new Error('test persistence failure'); };
  assert.throws(() => staging.seal('upload'), /persistence failure/);
  assert.throws(() => staging.read('upload', 0, 4), /not sealed/);
  data.flush = flush;
  assert.equal(staging.seal('upload'), 4);
  assert.deepEqual([...staging.read('upload', 0, 4)], [1, 2, 3, 4]);
});

/** Checks that every selected-file input and the runtime upload accept the same bounded chunks. */
test('snapshot upload uses 1 MiB chunks while exports keep their existing bound', () => {
  const dart = source('apps/flutter/thirdparty/operit_folder_access/lib/src/selected_file_input.dart');
  const runtime = source('core/crates/runtime/application/src/services/ArchiveTransferManager.rs');
  const android = source('apps/flutter/app/android/app/src/main/kotlin/app/operit/DocumentInputChannel.kt');
  const apple = source('apps/flutter/app/ios/Runner/AppleSnapshotImportInputChannel.swift');
  assert.match(dart, /selectedFileChunkBytes = 1024 \* 1024/);
  assert.match(runtime, /ARCHIVE_UPLOAD_MAX_CHUNK_BYTES: usize = 1024 \* 1024/);
  assert.match(runtime, /ARCHIVE_TRANSFER_MAX_CHUNK_BYTES: usize = 64 \* 1024/);
  assert.match(android, /MAX_CHUNK_SIZE = 1024 \* 1024/);
  assert.match(apple, /min\(maxBytes, 1024 \* 1024\)/);
});
