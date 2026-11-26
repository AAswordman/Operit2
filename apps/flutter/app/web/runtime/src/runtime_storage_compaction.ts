/** Synchronous OPFS operations used by runtime storage and its recovery journal. */
export interface StorageFile {
  getSize(): number;
  read(buffer: Uint8Array, options: { at: number }): number;
  write(buffer: Uint8Array, options: { at: number }): number;
  truncate(size: number): void;
  flush(): void;
  close(): void;
}

/** Minimal directory contract shared by the runtime and maintenance workers. */
interface StorageDirectory {
  getFileHandle(name: string, options: { create: boolean }): Promise<{
    createSyncAccessHandle(): Promise<StorageFile>;
  }>;
}

/** Logical file metadata preserved when its physical extent moves. */
interface StorageRecord {
  offset: number;
  byteLength: number;
  modifiedAt: number;
}

export const storageCompactionFiles = new Set([
  "operit_runtime_storage.compact.data",
  "operit_runtime_storage.compact.index",
  "operit_runtime_storage.compact.ready",
]);

/** Copies exact file ranges with bounded memory, including within the same file. */
export function copyStorageRange(
  source: StorageFile, target: StorageFile, sourceOffset: number, targetOffset: number, length: number,
): void {
  const buffer = new Uint8Array(Math.min(length, 65536));
  for (let copied = 0; copied < length; copied += buffer.byteLength) {
    const chunk = buffer.subarray(0, Math.min(buffer.byteLength, length - copied));
    if (source.read(chunk, { at: sourceOffset + copied }) !== chunk.byteLength ||
        target.write(chunk, { at: targetOffset + copied }) !== chunk.byteLength) {
      throw new Error("Incomplete OPFS storage copy");
    }
  }
}

/** Publishes compacted data and its index through a durable, replayable journal. */
export class RuntimeStorageCompaction {
  private closed = false;

  private constructor(
    private readonly data: StorageFile,
    private readonly index: StorageFile,
    private readonly packedData: StorageFile,
    private readonly packedIndex: StorageFile,
    private readonly ready: StorageFile,
  ) {}

  /** Opens every journal handle and completes an interrupted publication before returning. */
  static async open(root: StorageDirectory, data: StorageFile, index: StorageFile): Promise<RuntimeStorageCompaction> {
    const opened: StorageFile[] = [];
    try {
      for (const name of storageCompactionFiles) {
        const file = await root.getFileHandle(name, { create: true });
        opened.push(await file.createSyncAccessHandle());
      }
      const journal = new RuntimeStorageCompaction(data, index, opened[0], opened[1], opened[2]);
      journal.recover();
      return journal;
    } catch (error) {
      for (const file of opened) file.close();
      throw error;
    }
  }

  /** Packs only live extents; the originals remain intact until the journal is committed. */
  compact(records: Map<string, StorageRecord>): Map<string, StorageRecord> {
    const packed = new Map<string, StorageRecord>();
    let offset = 0;
    for (const [key, record] of records) {
      copyStorageRange(this.data, this.packedData, record.offset, offset, record.byteLength);
      packed.set(key, { ...record, offset });
      offset += record.byteLength;
    }
    this.packedData.truncate(offset);
    this.packedData.flush();
    const bytes = new TextEncoder().encode(JSON.stringify({ version: 2, records: Array.from(packed) }));
    if (this.packedIndex.write(bytes, { at: 0 }) !== bytes.byteLength) {
      throw new Error("Incomplete OPFS compaction index");
    }
    this.packedIndex.truncate(bytes.byteLength);
    this.packedIndex.flush();
    if (this.ready.write(Uint8Array.of(1), { at: 0 }) !== 1) {
      throw new Error("Incomplete OPFS compaction commit");
    }
    this.ready.flush();
    this.recover();
    return packed;
  }

  /** Replays a committed copy before clearing its marker; an uncommitted copy is disposable. */
  private recover(): void {
    if (this.ready.getSize() !== 0) {
      const marker = new Uint8Array(1);
      if (this.ready.getSize() !== 1 || this.ready.read(marker, { at: 0 }) !== 1 || marker[0] !== 1) {
        throw new Error("Invalid OPFS compaction commit");
      }
      this.replace(this.packedData, this.data);
      this.replace(this.packedIndex, this.index);
      this.ready.truncate(0);
      this.ready.flush();
    }
    this.packedData.truncate(0);
    this.packedData.flush();
    this.packedIndex.truncate(0);
    this.packedIndex.flush();
  }

  /** Flushes a complete replacement while retaining the journal for interrupted writes. */
  private replace(source: StorageFile, target: StorageFile): void {
    copyStorageRange(source, target, 0, 0, source.getSize());
    target.truncate(source.getSize());
    target.flush();
  }

  /** Releases journal ownership without discarding a pending committed copy. */
  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.ready.close();
    this.packedIndex.close();
    this.packedData.close();
  }
}

/** Completes runtime compaction before maintenance opens raw data and index handles. */
export async function recoverRuntimeStorageCompaction(root: StorageDirectory): Promise<void> {
  const markerFile = await root.getFileHandle("operit_runtime_storage.compact.ready", { create: false });
  const marker = await markerFile.createSyncAccessHandle();
  let committed: boolean;
  try { committed = marker.getSize() !== 0; }
  finally { marker.close(); }
  if (!committed) {
    // An old identity may have been removed by restore; scratch cleanup needs no original files.
    for (const name of ["operit_runtime_storage.compact.data", "operit_runtime_storage.compact.index"]) {
      const file = await (await root.getFileHandle(name, { create: true })).createSyncAccessHandle();
      try { file.truncate(0); file.flush(); }
      finally { file.close(); }
    }
    return;
  }
  const opened: StorageFile[] = [];
  let journal: RuntimeStorageCompaction | null = null;
  try {
    for (const name of ["operit_runtime_storage.data", "operit_runtime_storage.index"]) {
      const file = await root.getFileHandle(name, { create: false });
      opened.push(await file.createSyncAccessHandle());
    }
    journal = await RuntimeStorageCompaction.open(root, opened[0], opened[1]);
  } finally {
    journal?.close();
    for (const file of opened) file.close();
  }
}
