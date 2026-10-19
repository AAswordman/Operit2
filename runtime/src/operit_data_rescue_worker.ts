export {};
type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
interface SyncFile { getSize(): number; read(buffer: Uint8Array, options: { at: number }): number; write(buffer: Uint8Array, options: { at: number }): number; flush(): void; truncate(length: number): void; close(): void; }
interface FileHandle { kind: 'file'; name: string; createSyncAccessHandle(): Promise<SyncFile>; getFile(): Promise<File>; }
interface Directory { kind: 'directory'; name: string; getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<Directory>; getFileHandle(name: string, options?: { create?: boolean }): Promise<FileHandle>; removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>; entries(): AsyncIterableIterator<[string, Directory | FileHandle]>; }
interface Source { file: FileHandle; handle: SyncFile; }
interface Archive extends Source { name: string; sealed: boolean; expected: number | null; reference?: string; closed?: boolean; }
interface Manifest { files: { path: string; byteLength: number; sha256: string }[]; metadata: unknown; }
interface WasmModule { default(options: { module_or_path: URL }): Promise<{ memory: WebAssembly.Memory }>; operitDataRescue(request: string): string; }
interface Journal { id: string; sequence: number; manifest: Manifest; phase: 'prepared' | 'retained' | 'filesPublished' | 'databasesPublished'; }
interface StoreSchema { name: string; keyPath: string | string[] | null; autoIncrement: boolean; indexes: { name: string; keyPath: string | string[]; unique: boolean; multiEntry: boolean }[]; records: string[]; }
interface DatabaseSchema { name: string; version: number; stores: StoreSchema[]; }
const scope = globalThis as typeof globalThis & { __operitHost?: object; postMessage(value: unknown): void };
const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });
const sources = new Map<string, Source>();
const staged = new Map<string, Source>();
const archives = new Map<string, Archive>();
const removedArchives: string[] = [];
let root: Directory;
let maintenance: Directory;
let snapshot: Directory;
let snapshotName: string | null = null;
let wasm: WasmModule;
let startupLocal: Record<string, string> = {};
let nextArchive: Archive | null = null;
let preparedId: string | null = null;
let publication: Journal | null = null;
let journalStarted = false;
let readingStaged = false;
let queue = Promise.resolve();
let ready: Promise<void> | null = null;

/** Rejects traversal and ambiguous logical resource names. */
function segments(path: string): string[] {
  const parts = path.split('/');
  if (parts.some(part => part.length === 0 || part === '.' || part === '..' || /[\\:\0]/.test(part))) throw new Error(`Invalid rescue path: ${path}`);
  return parts;
}

/** Opens one OPFS resource without interpreting application records. */
async function resource(base: Directory, path: string, create: boolean): Promise<FileHandle> {
  const parts = segments(path);
  let directory = base;
  for (const part of parts.slice(0, -1)) directory = await directory.getDirectoryHandle(part, { create });
  return directory.getFileHandle(parts[parts.length - 1], { create });
}

/** Reads one bounded sync-access range. */
function read(source: Source, offset: number, count: number): Uint8Array {
  if (!Number.isSafeInteger(offset) || offset < 0 || count < 0 || count > 65536) throw new Error('Invalid rescue read range');
  const bytes = new Uint8Array(count);
  return bytes.subarray(0, source.handle.read(bytes, { at: offset }));
}

/** Writes a complete range and rejects partial persistence. */
function write(handle: SyncFile, offset: number, bytes: Uint8Array): void {
  if (handle.write(bytes, { at: offset }) !== bytes.byteLength) throw new Error('Incomplete OPFS maintenance write');
}

/** Copies a source with bounded memory and flushes its destination. */
async function copy(source: Source, target: FileHandle): Promise<void> {
  const output = await target.createSyncAccessHandle();
  try {
    output.truncate(0);
    for (let offset = 0; offset < source.handle.getSize(); offset += 65536) {
      const count = Math.min(65536, source.handle.getSize() - offset);
      const bytes = read(source, offset, count);
      if (bytes.length !== count) throw new Error('Truncated browser publication source');
      write(output, offset, bytes);
    }
    output.flush();
  } finally { output.close(); }
}

/** Enumerates origin files excluding maintenance catalogs. */
async function collect(base: Directory, prefix: string, result: Map<string, Source>): Promise<void> {
  for await (const [name, entry] of base.entries()) {
    if (prefix === '' && name === 'data-rescue') continue;
    const path = prefix === '' ? name : `${prefix}/${name}`;
    if (entry.kind === 'directory') await collect(entry, path, result);
    else result.set(`opfs/${path}`, { file: entry, handle: await entry.createSyncAccessHandle() });
  }
}

/** Converts a database request into an explicit asynchronous result. */
function requested<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
    (request as IDBRequest<T> & { onblocked?: () => void }).onblocked = () => reject(new Error('IndexedDB operation is blocked by another browser context'));
  });
}

/** Waits for a database transaction to complete durably. */
function completed(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => { transaction.oncomplete = () => resolve(); transaction.onabort = () => reject(transaction.error); transaction.onerror = () => reject(transaction.error); });
}

/** Persists synthetic browser metadata as private snapshot files. */
async function snapshotBytes(path: string, bytes: Uint8Array | Blob): Promise<string> {
  const file = await resource(snapshot, path, true);
  const handle = await file.createSyncAccessHandle();
  try {
    handle.truncate(0);
    const size = bytes instanceof Blob ? bytes.size : bytes.byteLength;
    for (let offset = 0; offset < size; offset += 65536) {
      const chunk = bytes instanceof Blob ? new Uint8Array(await bytes.slice(offset, offset + 65536).arrayBuffer()) : bytes.subarray(offset, offset + 65536);
      write(handle, offset, chunk);
    }
    handle.flush();
  } catch (error) { handle.close(); throw error; }
  sources.set(path, { file, handle });
  return path;
}

/** Serializes structured database values with separate binary payloads and explicit type tags. */
async function encode(value: unknown): Promise<Json> {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value as Json;
  if (typeof value === 'number') return Number.isFinite(value) ? value : { type: 'number', value: String(value) };
  if (typeof value === 'undefined') return { type: 'undefined' };
  if (typeof value === 'bigint') return { type: 'bigint', value: String(value) };
  if (value instanceof Date) return { type: 'date', value: value.toISOString() };
  if (value instanceof Blob || value instanceof ArrayBuffer || ArrayBuffer.isView(value)) {
    const path = `browser/binary/${crypto.randomUUID()}`;
    const bytes = value instanceof Blob ? value : value instanceof ArrayBuffer ? new Uint8Array(value) : new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    await snapshotBytes(path, bytes);
    return { type: 'binary', kind: value instanceof File ? 'File' : value instanceof Blob ? 'Blob' : value instanceof ArrayBuffer ? 'ArrayBuffer' : value.constructor.name,
      path, mime: value instanceof Blob ? value.type : '', name: value instanceof File ? value.name : '', lastModified: value instanceof File ? value.lastModified : 0 };
  }
  if (Array.isArray(value)) return { type: 'array', value: await Promise.all(value.map(encode)) };
  if (value instanceof Map) return { type: 'map', value: await Promise.all([...value].map(async ([key, item]) => [await encode(key), await encode(item)])) };
  if (value instanceof Set) return { type: 'set', value: await Promise.all([...value].map(encode)) };
  if (value instanceof RegExp) return { type: 'regexp', source: value.source, flags: value.flags };
  if (typeof value === 'object' && Object.getPrototypeOf(value) === Object.prototype) return { type: 'object', value: await Promise.all(Object.entries(value).map(async ([key, item]) => [key, await encode(item)])) };
  throw new Error('Unsupported database structured value in data rescue');
}

/** Reads one staged structured-value payload for native database insertion. */
function allBytes(path: string): Uint8Array {
  const source = staged.get(path);
  if (source === undefined) throw new Error(`Missing staged browser resource: ${path}`);
  const bytes = new Uint8Array(source.handle.getSize());
  for (let offset = 0; offset < bytes.length; offset += 65536) bytes.set(read(source, offset, Math.min(65536, bytes.length - offset)), offset);
  return bytes;
}

/** Decodes only explicitly supported structured-value tags. */
function decode(value: Json): unknown {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) throw new Error('Invalid structured value envelope');
  switch (value.type) {
    case 'undefined': return undefined;
    case 'bigint': return BigInt(value.value as string);
    case 'number': return Number(value.value as string);
    case 'date': return new Date(value.value as string);
    case 'regexp': return new RegExp(value.source as string, value.flags as string);
    case 'array': return (value.value as Json[]).map(decode);
    case 'set': return new Set((value.value as Json[]).map(decode));
    case 'map': return new Map((value.value as Json[][]).map(pair => [decode(pair[0]), decode(pair[1])]));
    case 'object': return Object.fromEntries((value.value as Json[][]).map(pair => [pair[0] as string, decode(pair[1])]));
    case 'binary': {
      const bytes = allBytes(value.path as string);
      switch (value.kind) {
        case 'Blob': return new Blob([bytes as BlobPart], { type: value.mime as string });
        case 'File': return new File([bytes as BlobPart], value.name as string, { type: value.mime as string, lastModified: value.lastModified as number });
        case 'ArrayBuffer': return bytes.buffer;
        case 'Uint8Array': return bytes;
        case 'Uint8ClampedArray': return new Uint8ClampedArray(bytes.buffer);
        case 'Int8Array': return new Int8Array(bytes.buffer);
        case 'Uint16Array': return new Uint16Array(bytes.buffer);
        case 'Int16Array': return new Int16Array(bytes.buffer);
        case 'Uint32Array': return new Uint32Array(bytes.buffer);
        case 'Int32Array': return new Int32Array(bytes.buffer);
        case 'Float32Array': return new Float32Array(bytes.buffer);
        case 'Float64Array': return new Float64Array(bytes.buffer);
        case 'BigInt64Array': return new BigInt64Array(bytes.buffer);
        case 'BigUint64Array': return new BigUint64Array(bytes.buffer);
        case 'DataView': return new DataView(bytes.buffer);
        default: throw new Error('Unsupported imported binary type');
      }
    }
    default: throw new Error('Unknown structured value tag');
  }
}

/** Checks whether a decoded value can be used as an IndexedDB key. */
function validDatabaseKey(value: unknown): boolean {
  try {
    indexedDB.cmp(value as IDBValidKey, value as IDBValidKey);
    return true;
  } catch (_) {
    return false;
  }
}

/** Validates an IndexedDB key-path shape before any live database is opened. */
function validKeyPath(value: unknown): value is string | string[] | null {
  return value === null || typeof value === 'string' || (Array.isArray(value) && value.every(item => typeof item === 'string'));
}

/** Reads a key-path value from one decoded record without mutating the record. */
function keyPathValue(value: unknown, keyPath: string | string[]): unknown {
  if (Array.isArray(keyPath)) {
    const values = keyPath.map(path => keyPathValue(value, path));
    return values.some(item => item === undefined) ? undefined : values;
  }
  if (keyPath === '') return value;
  const parts = keyPath.split('.');
  let current: unknown = value;
  for (const part of parts) {
    if (current === null || typeof current !== 'object' || !(part in current)) return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return current;
}

/** Writes one generated IndexedDB key into an inline key path without changing other fields. */
function assignKeyPathValue(value: unknown, keyPath: string, key: unknown): void {
  if (keyPath === '' || value === null || typeof value !== 'object' || Array.isArray(value)) throw new Error('Inline IndexedDB key requires an object value');
  const parts = keyPath.split('.');
  let current = value as Record<string, unknown>;
  for (const part of parts.slice(0, -1)) {
    const child = current[part];
    if (child === undefined) {
      current[part] = {};
    } else if (child === null || typeof child !== 'object' || Array.isArray(child)) {
      throw new Error(`Cannot assign IndexedDB key path: ${keyPath}`);
    }
    current = current[part] as Record<string, unknown>;
  }
  const leaf = parts[parts.length - 1];
  const existing = current[leaf];
  if (existing !== undefined && validDatabaseKey(existing) && indexedDB.cmp(existing as IDBValidKey, key as IDBValidKey) !== 0) {
    throw new Error(`IndexedDB key-path value does not match record key: ${keyPath}`);
  }
  current[leaf] = key;
}

/** Finds an exact binary-payload reference in a structured encoded record. */
function collectBinaryReferences(value: unknown, paths: Set<string>): void {
  if (Array.isArray(value)) {
    for (const item of value) collectBinaryReferences(item, paths);
    return;
  }
  if (value === null || typeof value !== 'object') return;
  const record = value as Record<string, unknown>;
  if (record.type === 'binary' && typeof record.path === 'string') paths.add(record.path);
  for (const item of Object.values(record)) collectBinaryReferences(item, paths);
}

/** Validates every staged browser record, schema, and local-storage payload before publication. */
async function validateImport(): Promise<void> {
  if (publication === null) throw new Error('No browser import publication exists');
  const paths = new Set(publication.manifest.files.map(file => file.path));
  if (!paths.has('browser/local-storage.json') || !paths.has('browser/catalog.json')) throw new Error('Browser archive is missing required storage catalogs');
  const local = JSON.parse(decoder.decode(allBytes('browser/local-storage.json'))) as unknown;
  if (local === null || typeof local !== 'object' || Array.isArray(local)) throw new Error('Invalid browser local-storage catalog');
  for (const [key, value] of Object.entries(local as Record<string, unknown>)) if (!key.startsWith('operit2.') || typeof value !== 'string') throw new Error(`Invalid browser local-storage record: ${key}`);
  const catalog = JSON.parse(decoder.decode(allBytes('browser/catalog.json'))) as unknown;
  if (!Array.isArray(catalog)) throw new Error('Invalid IndexedDB catalog');
  const databases = new Set<string>();
  const records = new Set<string>();
  const binaryReferences = new Set<string>();
  for (const item of catalog as DatabaseSchema[]) {
    if (item === null || typeof item !== 'object' || typeof item.name !== 'string' || !item.name.startsWith('operit2.') || databases.has(item.name) || !Number.isSafeInteger(item.version) || item.version < 1 || !Array.isArray(item.stores)) throw new Error('Invalid IndexedDB database schema');
    databases.add(item.name);
    const stores = new Set<string>();
    for (const store of item.stores) {
      if (store === null || typeof store !== 'object' || typeof store.name !== 'string' || stores.has(store.name) || !validKeyPath(store.keyPath) || typeof store.autoIncrement !== 'boolean' || !Array.isArray(store.indexes) || !Array.isArray(store.records)) throw new Error(`Invalid IndexedDB object store: ${item.name}`);
      if (store.autoIncrement && (store.keyPath === '' || Array.isArray(store.keyPath))) throw new Error(`Invalid auto-increment key path: ${item.name}/${store.name}`);
      stores.add(store.name);
      const indexes = new Set<string>();
      for (const index of store.indexes) {
        if (index === null || typeof index !== 'object' || typeof index.name !== 'string' || indexes.has(index.name) || !validKeyPath(index.keyPath) || index.keyPath === null || typeof index.unique !== 'boolean' || typeof index.multiEntry !== 'boolean') throw new Error(`Invalid IndexedDB index: ${item.name}/${store.name}`);
        indexes.add(index.name);
      }
      for (const path of store.records) {
        if (typeof path !== 'string' || !path.startsWith('browser/records/') || records.has(path) || !paths.has(path)) throw new Error(`Invalid IndexedDB record reference: ${path}`);
        records.add(path);
        const record = JSON.parse(decoder.decode(allBytes(path))) as { key?: Json; value?: Json };
        if (!Object.prototype.hasOwnProperty.call(record, 'key') || !Object.prototype.hasOwnProperty.call(record, 'value')) throw new Error(`Invalid IndexedDB record: ${path}`);
        const key = decode(record.key!);
        const value = decode(record.value!);
        collectBinaryReferences(record.value, binaryReferences);
        if (!validDatabaseKey(key)) throw new Error(`Invalid IndexedDB key: ${path}`);
        if (store.keyPath === null) {
          continue;
        } else {
          const embedded = keyPathValue(value, store.keyPath);
          if (embedded === undefined) {
            if (!store.autoIncrement || typeof key !== 'number' || !Number.isFinite(key) || key <= 0) throw new Error(`Missing IndexedDB key-path value: ${path}`);
          } else if (!validDatabaseKey(embedded)) {
            throw new Error(`Invalid IndexedDB key-path value: ${path}`);
          } else if (indexedDB.cmp(key as IDBValidKey, embedded as IDBValidKey) !== 0) {
            throw new Error(`IndexedDB key mismatch: ${path}`);
          }
        }
      }
    }
  }
  for (const path of paths) if (path.startsWith('browser/records/') && !records.has(path)) throw new Error(`Unreferenced IndexedDB record: ${path}`);
  for (const path of binaryReferences) if (!paths.has(path)) throw new Error(`Missing browser binary payload: ${path}`);
  for (const path of paths) if (path.startsWith('browser/binary/') && !binaryReferences.has(path)) throw new Error(`Unreferenced browser binary payload: ${path}`);
}

/** Captures database schemas and records without starting business services. */
async function snapshotDatabases(): Promise<void> {
  const catalog: DatabaseSchema[] = [];
  for (const info of await indexedDB.databases()) {
    if (info.name === undefined || !info.name.startsWith('operit2.')) continue;
    const database = await requested(indexedDB.open(info.name));
    const schema: DatabaseSchema = { name: info.name, version: database.version, stores: [] };
    try {
      for (const name of database.objectStoreNames) {
        const transaction = database.transaction(name, 'readonly');
        const store = transaction.objectStore(name);
        const definition: StoreSchema = { name, keyPath: store.keyPath, autoIncrement: store.autoIncrement, indexes: [], records: [] };
        for (const indexName of store.indexNames) {
          const index = store.index(indexName);
          definition.indexes.push({ name: indexName, keyPath: index.keyPath, unique: index.unique, multiEntry: index.multiEntry });
        }
        let after: IDBValidKey | undefined;
        for (;;) {
          const reading = database.transaction(name, 'readonly');
          const done = completed(reading);
          const cursor = await requested(reading.objectStore(name).openCursor(after === undefined ? undefined : IDBKeyRange.lowerBound(after, true)));
          if (cursor === null) { await done; break; }
          const record = { key: cursor.key, value: cursor.value };
          await done;
          const path = `browser/records/${crypto.randomUUID()}.json`;
          await snapshotBytes(path, encoder.encode(JSON.stringify({ key: await encode(record.key), value: await encode(record.value) })));
          definition.records.push(path);
          after = record.key;
        }
        schema.stores.push(definition);
      }
    } finally { database.close(); }
    catalog.push(schema);
  }
  await snapshotBytes('browser/catalog.json', encoder.encode(JSON.stringify(catalog)));
}

/** Snapshots raw OPFS and browser metadata under exclusive access. */
async function createSnapshot(): Promise<void> {
  await releaseSnapshot();
  snapshotName = `snapshot-${crypto.randomUUID()}`;
  snapshot = await maintenance.getDirectoryHandle(snapshotName, { create: true });
  await collect(root, '', sources);
  await snapshotBytes('browser/local-storage.json', encoder.encode(JSON.stringify(startupLocal)));
  await snapshotDatabases();
}

/** Releases raw source handles and removes the current unencrypted metadata snapshot. */
async function releaseSnapshot(): Promise<void> {
  for (const source of sources.values()) source.handle.close();
  sources.clear();
  if (snapshotName !== null) {
    await maintenance.removeEntry(snapshotName, { recursive: true });
    snapshotName = null;
  }
}

/** Encodes one JSON response at the Rust Host boundary. */
function result(value: unknown = null): string { return JSON.stringify(value); }

/** Reports raw damaged bootstrap bytes without application parsing. */
function startup(): string {
  const bootstrap = startupLocal['operit2.client.runtime.local_storage'];
  let encoded: string | null = null;
  if (bootstrap !== undefined) { let binary = ''; for (const byte of encoder.encode(bootstrap)) binary += String.fromCharCode(byte); encoded = btoa(binary); }
  return result({ defaultRoots: { runtimeRoot: 'browser-storage/runtime', workspaceRoot: 'browser-storage/workspaces' }, rootsEditable: false, bootstrapBase64: encoded, commitPending: publication !== null, layout: 'operit.browser.v1' });
}

/** Requires a preallocated staged file. */
function requireStaged(path: string): Source { const file = staged.get(path); if (file === undefined) throw new Error(`Unprepared import resource: ${path}`); return file; }

/** Assigns a preopened file to the shared engine's archive identifier. */
function assignArchive(id: string, expected: number | null): void {
  if (nextArchive === null || archives.has(id)) throw new Error('Archive resource was not prepared');
  nextArchive.expected = expected; archives.set(id, nextArchive); nextArchive = null;
}

/** Requires a current-session archive identifier. */
function requireArchive(id: string): Archive { const file = archives.get(id); if (file === undefined) throw new Error('Unknown archive identifier'); return file; }

/** Installs only offline storage and archive modules required by the shared engine. */
function installHost(): void {
  scope.__operitHost = {
    dataRescue: {
      startup,
      describe: () => result({ layout: 'operit.browser.v1', sourceFiles: sources.size, commitPending: publication !== null }),
      listFiles: () => {
        if (publication !== null) throw new Error('Complete the interrupted import before exporting');
        return result([...sources].map(([path, source]) => ({ path, byteLength: source.handle.getSize(), sha256: '' })));
      },
      readFile: (path: string, offset: number, length: number) => { const source = (readingStaged ? staged : sources).get(path); if (source === undefined) throw new Error(`Unknown browser resource: ${path}`); return read(source, offset, length); },
      beginRestore: (id: string) => { preparedId = id; return result(); },
      writeRestore: (id: string, path: string, bytes: Uint8Array) => { if (id !== preparedId) throw new Error('Import identifier mismatch'); const source = requireStaged(path); write(source.handle, source.handle.getSize(), bytes); },
      finishRestoreFile: (id: string, path: string) => { if (id !== preparedId) throw new Error('Import identifier mismatch'); requireStaged(path).handle.flush(); return result(); },
      commitRestore: (id: string, manifest: string) => { if (id !== preparedId) throw new Error('Import identifier mismatch'); publication = { id, sequence: 0, manifest: JSON.parse(manifest) as Manifest, phase: 'prepared' }; return result({ preparedImport: id }); },
      resumeRestore: () => { if (publication === null) throw new Error('No browser import is pending'); return result({ preparedImport: publication.id }); },
      discardRestore: (id: string) => { if (id !== preparedId) throw new Error('Import identifier mismatch'); if (journalStarted) throw new Error('A durable browser import journal must be resumed explicitly'); for (const source of staged.values()) source.handle.close(); staged.clear(); preparedId = null; publication = null; return result(); },
    },
    archiveStaging: {
      createArchive: (id: string, length: number) => assignArchive(id, length),
      createExportArchive: (id: string) => assignArchive(id, null),
      writeExportArchive: (id: string, offset: number, bytes: Uint8Array) => { const file = requireArchive(id); if (file.sealed || bytes.length > 65536) throw new Error('Invalid archive write'); write(file.handle, offset, bytes); },
      archiveFileReference: (id: string) => `data-rescue-export:${id}`,
      appendArchive: (id: string, bytes: Uint8Array) => { const file = requireArchive(id); if (file.sealed || file.expected === null || file.handle.getSize() + bytes.length > file.expected) throw new Error('Invalid upload range'); write(file.handle, file.handle.getSize(), bytes); },
      sealArchive: (id: string) => { const file = requireArchive(id); const size = file.handle.getSize(); if (file.expected !== null && file.expected !== size) throw new Error('Archive length mismatch'); file.handle.flush(); file.sealed = true; return size; },
      readArchive: (id: string, offset: number, length: number) => read(requireArchive(id), offset, length),
      removeArchive: (id: string) => { const file = requireArchive(id); if (file.closed !== true) file.handle.close(); if (file.reference !== undefined) URL.revokeObjectURL(file.reference); archives.delete(id); removedArchives.push(file.name); },
    },
  };
}

/** Reads a durable journal without treating corrupt JSON as an absent import. */
async function loadJournal(): Promise<Journal | null> {
  if (!await maintenanceEntryExists('commit.json')) return null;
  const file = await maintenance.getFileHandle('prepared-journal.json');
  const journal = JSON.parse(await (await file.getFile()).text()) as Journal;
  if (journal.phase !== 'prepared' || !Array.isArray(journal.manifest.files)) throw new Error('Invalid immutable browser rescue journal');
  let previousPresent = true;
  for (const phase of ['retained', 'filesPublished', 'databasesPublished'] as const) {
    const present = await maintenanceEntryExists(`commit-${phase}`);
    if (present && !previousPresent) throw new Error('Browser publication phase markers are inconsistent');
    if (present) journal.phase = phase;
    previousPresent = present;
  }
  journalStarted = true;
  return journal;
}

/** Checks exact journal marker presence without interpreting an I/O failure as absence. */
async function maintenanceEntryExists(name: string): Promise<boolean> {
  try { await maintenance.getFileHandle(name); return true; }
  catch (error) { if (error instanceof DOMException && error.name === 'NotFoundError') return false; throw error; }
}

/** Creates an immutable phase marker only after that phase's writes have completed. */
async function markPhase(name: string): Promise<void> {
  const handle = await (await maintenance.getFileHandle(name, { create: true })).createSyncAccessHandle();
  try { handle.flush(); } finally { handle.close(); }
}

/** Publishes a flushed immutable journal, then records progress with append-only markers. */
async function saveJournal(): Promise<void> {
  if (publication === null) throw new Error('No publication journal exists');
  if (!journalStarted) {
    if (publication.phase !== 'prepared') throw new Error('A new publication must start in the prepared phase');
    for (const phase of ['retained', 'filesPublished', 'databasesPublished']) {
      const name = `commit-${phase}`;
      if (await maintenanceEntryExists(name)) await maintenance.removeEntry(name);
    }
    const file = await maintenance.getFileHandle('prepared-journal.json', { create: true });
    const handle = await file.createSyncAccessHandle();
    try { const bytes = encoder.encode(JSON.stringify(publication)); handle.truncate(0); write(handle, 0, bytes); handle.flush(); }
    finally { handle.close(); }
    await markPhase('commit.json');
    journalStarted = true;
  }
  if (publication.phase !== 'prepared') await markPhase(`commit-${publication.phase}`);
}

/** Acquires exclusive access and loads only the standalone WebAssembly export. */
async function initialize(): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    void navigator.locks.request('operit2.data.access', { mode: 'exclusive' }, async lock => {
      if (lock === null) throw new Error('Exclusive maintenance access was not granted');
      resolve();
      await new Promise<void>(() => {});
    }).catch(reject);
  });
  root = await navigator.storage.getDirectory() as unknown as Directory;
  maintenance = await root.getDirectoryHandle('data-rescue', { create: true });
  publication = await loadJournal();
  installHost();
  const dynamicImport = new Function('path', 'return import(path)') as (path: string) => Promise<unknown>;
  wasm = await dynamicImport('./operit_flutter_bridge_worker.js') as WasmModule;
  await wasm.default({ module_or_path: new URL('./operit_flutter_bridge_bg.wasm', import.meta.url) });
}

/** Preallocates isolated files only after the entire archive authenticates. */
async function prepareImport(manifest: Manifest): Promise<void> {
  if (publication !== null) throw new Error('Complete the interrupted import before starting another import');
  for (const source of staged.values()) source.handle.close();
  staged.clear();
  preparedId = null;
  const directory = await maintenance.getDirectoryHandle('prepared', { create: true });
  for (const file of manifest.files) {
    segments(file.path);
    if (!file.path.startsWith('opfs/') && !file.path.startsWith('browser/')) throw new Error('Invalid browser namespace');
    if (file.path.startsWith('opfs/data-rescue/')) throw new Error('Archive addresses maintenance storage');
    const resourceFile = await resource(directory, file.path, true);
    const handle = await resourceFile.createSyncAccessHandle();
    handle.truncate(0);
    staged.set(file.path, { file: resourceFile, handle });
  }
}

/** Removes only an uncommitted browser import and its private staging snapshot. */
async function discardUncommittedImport(): Promise<void> {
  if (journalStarted) throw new Error('A durable browser import journal must be resumed explicitly');
  for (const source of staged.values()) source.handle.close();
  staged.clear();
  preparedId = null;
  publication = null;
  readingStaged = false;
  try { await maintenance.removeEntry('prepared', { recursive: true }); }
  catch (error) { if (!(error instanceof DOMException && error.name === 'NotFoundError')) throw error; }
  for (const name of ['prepared-journal.json', 'commit-retained', 'commit-filesPublished', 'commit-databasesPublished']) {
    try { await maintenance.removeEntry(name); }
    catch (error) { if (!(error instanceof DOMException && error.name === 'NotFoundError')) throw error; }
  }
  await releaseSnapshot();
}

/** Reopens persisted verified payloads when the user explicitly resumes publication. */
async function openPendingStaging(): Promise<void> {
  if (publication === null) throw new Error('No publication journal exists');
  const directory = await maintenance.getDirectoryHandle('prepared');
  for (const source of staged.values()) source.handle.close();
  staged.clear();
  for (const file of publication.manifest.files) {
    segments(file.path);
    if ((!file.path.startsWith('opfs/') && !file.path.startsWith('browser/')) || file.path === 'opfs/data-rescue' || file.path.startsWith('opfs/data-rescue/')) throw new Error('Invalid pending resource namespace');
    const input = await resource(directory, file.path, false);
    staged.set(file.path, { file: input, handle: await input.createSyncAccessHandle() });
  }
  readingStaged = true;
  try {
    const verified = JSON.parse(wasm.operitDataRescue(JSON.stringify({ operation: 'verifyFiles', manifest: publication.manifest }))) as { ok: boolean; error?: string };
    if (verified.ok !== true) throw new Error(verified.error);
  } finally { readingStaged = false; }
}

/** Reconstructs database schemas and records from verified staged resources. */
async function publishDatabases(): Promise<void> {
  const catalog = JSON.parse(decoder.decode(allBytes('browser/catalog.json'))) as DatabaseSchema[];
  for (const item of catalog) {
    if (!item.name.startsWith('operit2.') || !Number.isSafeInteger(item.version) || item.version < 1) throw new Error('Invalid database catalog');
    await requested(indexedDB.deleteDatabase(item.name));
    const opening = indexedDB.open(item.name, item.version);
    opening.onupgradeneeded = () => {
      for (const definition of item.stores) {
        const store = opening.result.createObjectStore(definition.name, { keyPath: definition.keyPath, autoIncrement: definition.autoIncrement });
        for (const index of definition.indexes) store.createIndex(index.name, index.keyPath, { unique: index.unique, multiEntry: index.multiEntry });
      }
    };
    const database = await requested(opening);
    try {
      for (const definition of item.stores) {
        for (const path of definition.records) {
          const record = JSON.parse(decoder.decode(allBytes(path))) as { key: Json; value: Json };
          const key = decode(record.key) as IDBValidKey;
          const value = decode(record.value);
          const transaction = database.transaction(definition.name, 'readwrite');
          const done = completed(transaction);
          const store = transaction.objectStore(definition.name);
          if (definition.keyPath === null) {
            store.put(value, key);
          } else {
            if (keyPathValue(value, definition.keyPath) === undefined) {
              if (!definition.autoIncrement || Array.isArray(definition.keyPath)) throw new Error(`Missing IndexedDB key-path value: ${path}`);
              assignKeyPathValue(value, definition.keyPath, key);
            }
            store.put(value);
          }
          await done;
        }
      }
    } finally { database.close(); }
  }
  const desired = new Set(catalog.map(item => item.name));
  for (const item of await indexedDB.databases()) if (item.name !== undefined && item.name.startsWith('operit2.') && !desired.has(item.name)) await requested(indexedDB.deleteDatabase(item.name));
}

/** Publishes OPFS under a journal and retains original resources before replacing them. */
async function publish(): Promise<Record<string, unknown>> {
  if (publication === null) throw new Error('No prepared publication exists');
  await saveJournal();
  if (publication.phase === 'prepared') {
    const retained = await maintenance.getDirectoryHandle(`retained-${publication.id}`, { create: true });
    for (const [path, source] of sources) await copy(source, await resource(retained, path, true));
    for (const source of sources.values()) source.handle.close();
    sources.clear();
    publication.phase = 'retained';
    await saveJournal();
  }
  if (publication.phase === 'retained') {
    const desired = new Set(publication.manifest.files.filter(file => file.path.startsWith('opfs/')).map(file => file.path));
    for (const [path, source] of staged) if (path.startsWith('opfs/')) await copy(source, await resource(root, path.slice(5), true));
    const current = new Map<string, Source>();
    await collect(root, '', current);
    for (const [path, source] of current) {
      source.handle.close();
      if (!desired.has(path)) {
        const parts = segments(path.slice(5));
        let parent = root;
        for (const part of parts.slice(0, -1)) parent = await parent.getDirectoryHandle(part);
        await parent.removeEntry(parts[parts.length - 1]);
      }
    }
    publication.phase = 'filesPublished';
    await saveJournal();
  }
  if (publication.phase === 'filesPublished') {
    await publishDatabases();
    publication.phase = 'databasesPublished';
    await saveJournal();
  }
  return { requiresLocalStorageCommit: true, localStorage: JSON.parse(decoder.decode(allBytes('browser/local-storage.json'))), retainedData: `OPFS/data-rescue/retained-${publication.id}` };
}

/** Completes one request and applies deferred archive cleanup outside synchronous Rust callbacks. */
async function execute(request: Record<string, unknown>, local: Record<string, string>): Promise<Record<string, unknown>> {
  startupLocal = local;
  if (ready === null) ready = initialize();
  await ready;
  if (request.operation === 'finishPublication') {
    if (publication === null || publication.phase !== 'databasesPublished') throw new Error('Publication is not ready to complete');
    await maintenance.removeEntry('commit.json');
    journalStarted = false;
    const id = publication.id;
    publication = null;
    for (const source of staged.values()) source.handle.close();
    staged.clear();
    await maintenance.removeEntry('prepared', { recursive: true });
    await releaseSnapshot();
    return { ok: true, value: { restored: true, retainedData: `OPFS/data-rescue/retained-${id}` } };
  }
  if (request.operation === 'export') await createSnapshot();
  if (request.operation === 'export' || request.operation === 'beginUpload') {
    const directory = await maintenance.getDirectoryHandle('archives', { create: true });
    const name = crypto.randomUUID();
    const file = await directory.getFileHandle(name, { create: true });
    nextArchive = { file, handle: await file.createSyncAccessHandle(), name, expected: null, sealed: false };
  }
  if (request.operation === 'inspect' || request.operation === 'restore') {
    const archive = requireArchive(request.archiveId as string);
    if (archive.closed === true) {
      archive.handle = await archive.file.createSyncAccessHandle();
      archive.closed = false;
    }
  }
  if (request.operation === 'restore') {
    const preview = JSON.parse(wasm.operitDataRescue(JSON.stringify({ ...request, operation: 'inspect' }))) as { ok: boolean; value: Manifest; error: string };
    if (preview.ok !== true) throw new Error(preview.error);
    await createSnapshot();
    await prepareImport(preview.value);
  }
  if (request.operation === 'resumeRestore') {
    if (publication?.phase === 'prepared') await createSnapshot();
    await openPendingStaging();
  }
  const response = JSON.parse(wasm.operitDataRescue(JSON.stringify(request))) as { ok: boolean; value?: Record<string, unknown>; error?: string };
  if (nextArchive !== null) {
    nextArchive.handle.close();
    removedArchives.push(nextArchive.name);
    nextArchive = null;
  }
  if (response.ok === true && request.operation === 'export') {
    const archive = requireArchive(response.value!.archiveId as string);
    archive.handle.flush();
    archive.handle.close();
    archive.closed = true;
    archive.reference = URL.createObjectURL(await archive.file.getFile());
    response.value!.fileReference = archive.reference;
  }
  if (response.ok === true && (request.operation === 'restore' || request.operation === 'resumeRestore')) {
    await validateImport();
    response.value = await publish();
  }
  if (request.operation === 'close') {
    const hasUncommittedStaging = !journalStarted && (publication !== null || staged.size > 0);
    await releaseSnapshot();
    for (const [id, archive] of archives) {
      if (archive.closed !== true) archive.handle.close();
      if (archive.reference !== undefined) URL.revokeObjectURL(archive.reference);
      archives.delete(id);
      removedArchives.push(archive.name);
    }
    if (hasUncommittedStaging) {
      await discardUncommittedImport();
    } else if (publication === null) {
      try { await maintenance.removeEntry('prepared', { recursive: true }); }
      catch (error) { if (!(error instanceof DOMException && error.name === 'NotFoundError')) throw error; }
    }
  }
  if (removedArchives.length > 0) {
    const directory = await maintenance.getDirectoryHandle('archives');
    for (const name of removedArchives.splice(0)) await directory.removeEntry(name);
  }
  return response as Record<string, unknown>;
}

/** Serializes uploads, verification, and publication without entering a Core operation queue. */
scope.addEventListener('message', (event: MessageEvent<{ id: number; request: Record<string, unknown>; localStorage: Record<string, string> }>) => {
  const message = event.data;
  queue = queue.then(async () => {
    try {
      const value = await execute(message.request, message.localStorage);
      if (message.request.operation === 'export') await releaseSnapshot();
      scope.postMessage({ id: message.id, value });
    }
    catch (error) {
      if (message.request.operation === 'restore' && !journalStarted) {
        try { await discardUncommittedImport(); }
        catch (cleanupError) { error = new Error(`${String(error)}; staging cleanup failed: ${String(cleanupError)}`); }
      }
      scope.postMessage({ id: message.id, error: String(error) });
    }
  });
});
