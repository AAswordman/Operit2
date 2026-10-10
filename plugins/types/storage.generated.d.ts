// Generated from operit-plugin-sdk Rust declarations.

/**
 * Dispatches typed requests; database object facades retain only authenticated handles.
 */
export namespace Storage {
  /**
   * Performs one parameterized read or atomic bounded write without AI tool dispatch.
   */
  function request(request: StorageRequest): Promise<any>;
}

/**
 * Preserves SQLite integers and binary values without JavaScript number coercion.
 */
export type StorageSqlValue = { kind: "null"; } | { kind: "integer"; value: string; } | { kind: "real"; value: number; } | { kind: "text"; value: string; } | { kind: "blob"; value: number[]; };

/**
 * Executes one parameterized statement inside an explicitly bounded transaction.
 */
export interface StorageStatement {
  sql: string;
  params: StorageSqlValue[];
}

/**
 * Selects a database's permanently recorded data contract.
 */
export type StorageKind = "sqlite" | "objects" | "data_store";

/**
 * Declares a synchronized collection column.
 */
export type StorageColumnType = "text" | "integer" | "real" | "blob";

/**
 * Declares one column without accepting executable schema SQL from a remote peer.
 */
export interface StorageColumn {
  name: string;
  affinity: StorageColumnType;
  nullable: boolean;
}

/**
 * Declares a synchronized table with one text primary key.
 */
export interface StorageTable {
  name: string;
  primaryKey: string;
  columns: StorageColumn[];
}

/**
 * Stages one object or preference-key mutation and an optional exact-version precondition.
 */
export interface StorageMutation {
  collection: string;
  key: string;
  value: unknown;
  deleted: boolean;
  checkVersion: boolean;
  expectedVersion: string | null;
}

/**
 * Dispatches storage operations only through an authenticated engine-owned handle.
 */
export type StorageRequest = { op: "open"; path: string; kind: StorageKind; } | { op: "close"; handle: string; } | { op: "query"; handle: string; statement: StorageStatement; } | { op: "execute"; handle: string; statement: StorageStatement; } | { op: "transaction"; handle: string; statements: StorageStatement[]; } | { op: "define_table"; handle: string; table: StorageTable; } | { op: "get"; handle: string; collection: string; key: string; } | { op: "list"; handle: string; collection: string; after: string | null; limit: number; } | { op: "commit"; handle: string; mutations: StorageMutation[]; } | { op: "changes"; handle: string; after: string; limit: number; };
