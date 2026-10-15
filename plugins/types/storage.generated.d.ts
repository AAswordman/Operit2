// Generated from operit-plugin-sdk Rust declarations.

import type { StorageRequestResultData } from "./results";

/**
 * Dispatches typed requests; database object facades retain only authenticated handles.
 */
export namespace Storage {
  /**
   * Performs one parameterized read or atomic bounded write through the ordinary tool-call pipeline.
   */
  function request(request: StorageRequest): Promise<StorageRequestResultData>;
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
  /**
   * Requires an exact write count inside the transaction for optimistic concurrency.
   */
  expectedAffectedRows: number | null;
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

/**
 * Identifies one successfully opened database session.
 */
export interface StorageOpenResult {
  handle: string;
  path: string;
}

/**
 * Preserves the exact column order and tagged SQLite values of one query row.
 */
export interface StorageSqlRow {
  columns: string[];
  values: StorageSqlValue[];
}

/**
 * Reports one statement's affected rows and exact int64 insertion identity.
 */
export interface StorageSqlExecutionResult {
  affected_rows: number;
  last_insert_id: string;
}

/**
 * Reports whether a declarative table definition changed the schema.
 */
export interface StorageTableDefinitionResult {
  changed: boolean;
}

/**
 * Preserves a stored application JSON value and its exact concurrency version.
 */
export interface StorageValueEntry {
  value: unknown;
  version: string;
}

/**
 * Adds the collection key to a listed value and its concurrency version.
 */
export interface StorageKeyValueEntry {
  key: string;
  value: unknown;
  version: string;
}

/**
 * Reports the actual version committed by an atomic mutation batch.
 */
export interface StorageCommitResult {
  version: string;
}

/**
 * Preserves the durable database identity carried by a change transaction.
 */
export interface StorageManifest {
  path: string;
  owner: string;
  kind: StorageKind;
  sync: boolean;
}

/**
 * Describes one declarative record mutation with an application-owned JSON value.
 */
export interface StorageRecordChange {
  collection: string;
  key: string;
  value: unknown;
  deleted: boolean;
}

/**
 * Preserves all schema and record changes from one atomic storage transaction.
 */
export interface StorageChangeTransaction {
  manifest: StorageManifest;
  tables: StorageTable[];
  changes: StorageRecordChange[];
}

/**
 * Carries one exact int64 cursor and its complete atomic transaction.
 */
export interface StorageChangeEntry {
  cursor: string;
  transaction: StorageChangeTransaction;
}

/**
 * Returns each storage operation's concrete business result.
 * Serialization preserves the database API payload; tool results add their own operation discriminator.
 */
export type StorageResponse = StorageOpenResult | null | StorageSqlRow[] | StorageSqlExecutionResult[] | StorageSqlExecutionResult[] | StorageTableDefinitionResult | StorageValueEntry | null | StorageKeyValueEntry[] | StorageCommitResult | StorageChangeEntry[];
