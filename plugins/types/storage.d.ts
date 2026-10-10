import type { StorageRequest, StorageTable, StorageSqlValue } from "./storage.generated";
export type { StorageRequest, StorageTable, StorageSqlValue, StorageKind, StorageColumn, StorageColumnType, StorageMutation, StorageStatement } from "./storage.generated";

/** Transactional plugin storage over the registered runtime Host. */
export namespace Storage {
  export type Value = null | boolean | number | string | Value[] | { [key: string]: Value };
  export type SqlValue = null | string | number | bigint | Uint8Array;
  /** Selects an owned database path; registered directory ownership determines synchronization. */
  export interface OpenOptions { path: string; }
  export interface VersionOptions { expectedVersion?: string | null; }
  export interface Receipt { version: string; }
  export interface Entry<T extends Value = Value> { value: T; version: string; }
  export interface ListedEntry<T extends Value = Value> extends Entry<T> { key: string; }
  export interface Change { collection: string; key: string; value: Value | StorageSqlValue[]; deleted: boolean; }
  export interface ChangePageEntry {
    cursor: string;
    transaction: {
      manifest: { path: string; owner: string; kind: "sqlite" | "objects" | "data_store"; sync: boolean; };
      tables: StorageTable[];
      changes: Change[];
    };
  }
  export interface Handle {
    readonly path: string;
    /** Releases the current execution's handle without deleting durable data. */
    close(): Promise<void>;
    /** Pages through atomic changes after an opaque decimal cursor; limit is 1..1000. */
    changes(after?: string, limit?: number): Promise<ChangePageEntry[]>;
  }
  export interface Statement { sql: string; params?: SqlValue[]; }
  export interface SqlReceipt { affectedRows: number; lastInsertId: string; }
  export interface SqliteDatabase extends Handle {
    /** Queries caller-selected columns; all SQLite integers are returned as bigint. */
    query(sql: string, params?: SqlValue[]): Promise<Record<string, SqlValue>[]>;
    /** Commits one parameterized SQL write and its dirty-row journal atomically. */
    execute(sql: string, params?: SqlValue[]): Promise<SqlReceipt>;
    /** Commits 1..1000 statements together and rolls back all writes on any failure. */
    transaction(statements: Statement[]): Promise<SqlReceipt[]>;
    /** Declares a tracked table with one nonnullable TEXT primary key. */
    defineTable(table: StorageTable): Promise<{ changed: boolean }>;
  }
  export interface Editor<T extends Value = Value> {
    readonly value: T;
    /** Atomically saves this modified record with its original exact version. */
    flush(): Promise<Receipt>;
  }
  export interface Collection<T extends Value = Value> {
    /** Returns null for an absent key, and an entry for a stored null value. */
    get(key: string): Promise<Entry<T> | null>;
    /** Returns primary-key-ordered entries without exporting the entire collection. */
    list(after?: string | null, limit?: number): Promise<ListedEntry<T>[]>;
    /** Saves one structured value with an optional exact-version precondition. */
    put(key: string, value: T, options?: VersionOptions): Promise<Receipt>;
    /** Removes one key with an optional exact-version precondition. */
    remove(key: string, options?: VersionOptions): Promise<Receipt>;
    /** Opens an existing object for tracked edits; scalar and root array records reject. */
    edit(key: string): Promise<Editor<T>>;
  }
  export type ObjectMutation =
    | ({ op: "put"; collection: string; key: string; value: Value } & VersionOptions)
    | ({ op: "delete"; collection: string; key: string } & VersionOptions);
  export interface ObjectDatabase extends Handle {
    /** Selects a named collection on this retained database handle. */
    collection<T extends Value = Value>(name: string): Collection<T>;
    /** Atomically commits changes across collections with per-record version checks. */
    commit(mutations: ObjectMutation[]): Promise<Receipt>;
  }
  export interface DataStoreBatch {
    set?: Record<string, Value>;
    remove?: string[];
    expectedVersions?: Record<string, string | null>;
  }
  export interface DataStore extends Handle, Collection {
    /** Atomically applies key writes and removals with explicit per-key versions. */
    commit(batch: DataStoreBatch): Promise<Receipt>;
  }
  export namespace sqlite {
    /** Opens SQLite at an explicit path inside this plugin's config or data root. */
    function open(options: OpenOptions): Promise<SqliteDatabase>;
  }
  export namespace objects {
    /** Opens generic object collections on the same transactional storage engine. */
    function open(options: OpenOptions): Promise<ObjectDatabase>;
  }
  export namespace dataStore {
    /** Opens an atomic key-value store at an explicit plugin-owned path. */
    function open(options: OpenOptions): Promise<DataStore>;
  }
  /** Dispatches the exact typed wire contract through a structured host callback. */
  function request(request: StorageRequest): Promise<unknown>;
}
