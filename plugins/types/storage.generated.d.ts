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
 * Guards an entire transaction against an already initialized primary-key identity.
 */
export interface StorageRecordIdentity {
  collection: string;
  key: string;
}

/**
 * Selects a database's permanently recorded data contract.
 */
export type StorageKind = "sqlite" | "objects";

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
 * Guards a related record without rewriting it or publishing an extra change.
 */
export interface StorageRecordGuard {
  collection: string;
  key: string;
  expectedVersion: string | null;
}

/**
 * Identifies the last observed record state without exposing journal positions.
 */
export interface StorageRecordWatchState {
  version: string | null;
}

/**
 * Dispatches storage operations only through an authenticated engine-owned handle.
 */
export type StorageRequest = { op: "open"; path: string; kind: StorageKind; merge: StorageMergeDescriptor | null; } | { op: "open_preferences"; path: string; merge: StoragePreferencesMergeDescriptor | null; } | { op: "preferences_data"; handle: string; } | { op: "preferences_begin_edit"; handle: string; } | { op: "preferences_finish_edit"; handle: string; token: string; edits: StoragePreferenceEdit[]; schemaVersion: number; } | { op: "preferences_cancel_edit"; handle: string; token: string; } | { op: "close"; handle: string; } | { op: "query"; handle: string; statement: StorageStatement; } | { op: "execute"; handle: string; statement: StorageStatement; } | { op: "transaction"; handle: string; statements: StorageStatement[]; whenAbsent: StorageRecordIdentity | null; } | { op: "define_table"; handle: string; table: StorageTable; } | { op: "get"; handle: string; collection: string; key: string; } | { op: "get_many"; handle: string; collection: string; keys: string[]; } | { op: "list"; handle: string; collection: string; after: string | null; limit: number; } | { op: "changes"; handle: string; after: string | null; limit: number; } | { op: "commit"; handle: string; mutations: StorageMutation[]; guards: StorageRecordGuard[]; };

/**
 * Identifies one successfully opened database session.
 */
export interface StorageOpenResult {
  handle: string;
  path: string;
}

/**
 * Selects the bounded key encoding used by a declarative space merge contract.
 */
export type StorageMergeKeyEncoding = "length_prefixed_root" | "slash_root";

/**
 * Selects whether an entity is matched by content or by its declared identity.
 */
export type StorageMergeMatchMode = "content" | "identity";

/**
 * Describes a key prefix that points to another declared entity identity.
 */
export interface StorageMergeKeyReference {
  target: StorageMergeTarget;
  prefix: string;
}

/**
 * Selects the representation used around a referenced identifier.
 */
export type StorageMergeReferenceEncoding = "direct" | "uri_component";

/**
 * Selects a core-owned namespace whose identifiers may be rewritten during a space merge.
 */
export type StorageMergeTarget = { kind: "collection"; collection: string; } | { kind: "preference"; path: string; };

/**
 * Declares one explicit scalar or child-row identifier reference.
 */
export interface StorageMergeReference {
  collection: string;
  column: string;
  target: StorageMergeTarget;
  keyPrefix: string;
  keySuffix: string;
  valueEncoding: StorageMergeReferenceEncoding;
}

/**
 * Declares one root entity whose UUID, display name and references are core-mergeable.
 */
export interface StorageMergeEntity {
  collection: string;
  identityColumn: string;
  nameColumn: string | null;
  ignoredColumns: string[];
  keyEncoding: StorageMergeKeyEncoding;
  matchMode: StorageMergeMatchMode;
  keyReferences: StorageMergeKeyReference[];
  childCollections: string[];
  references: StorageMergeReference[];
}

/**
 * Declares a bounded, data-only merge contract verified and executed by Core.
 */
export interface StorageMergeDescriptor {
  version: number;
  maxEntities: number;
  maxReferences: number;
  entities: StorageMergeEntity[];
}

/**
 * Preserves the exact column order and tagged SQLite values of one query row.
 */
export interface StorageSqlRow {
  columns: string[];
  values: StorageSqlValue[];
}

/**
 * Carries the current value (or deletion) for one changed object identity.
 */
export interface StorageObjectChange {
  collection: string;
  key: string;
  entry: StorageValueEntry | null;
}

/**
 * Pages a bounded local change journal, including synchronization and conflict-merge writes.
 */
export interface StorageChangePage {
  cursor: string;
  reset: boolean;
  hasMore: boolean;
  changes: StorageObjectChange[];
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
 * Declares one complete-record catalog using the native preference merge executor.
 */
export interface StoragePreferencesMergeCatalog {
  keyPrefix: string;
  keySuffix: string;
  identityField: string;
  nameField: string;
  ignoredFields: string[];
  retiredKeys: string[];
}

/**
 * Selects an exact preference or complete records with explicit key boundaries.
 */
export type StoragePreferencesMergeKey = { kind: "exact"; key: string; } | { kind: "records"; prefix: string; suffix: string; };

/**
 * Selects nested reference fields without interpreting application-specific JSON.
 */
export type StoragePreferencesMergePathSegment = { kind: "field"; name: string; } | { kind: "object_values"; } | { kind: "array_values"; } | { kind: "variant"; name: string; } | { kind: "identity"; prefix: string; suffix: string; };

/**
 * Declares a native preference reference targeting an explicit catalog file.
 */
export interface StoragePreferencesMergeReference {
  key: StoragePreferencesMergeKey;
  path: StoragePreferencesMergePathSegment[];
  target: string;
}

/**
 * Projects selected entity fields through the native preference merge executor.
 */
export interface StoragePreferencesMergeProjection {
  selectionKey: string;
  sourceField: string;
  fields: Record<string, string>;
  retiredKeys: string[];
}

/**
 * Carries the native merge capabilities as bounded data alongside preference journal entries.
 */
export interface StoragePreferencesMergeDescriptor {
  version: number;
  catalog: StoragePreferencesMergeCatalog | null;
  references: StoragePreferencesMergeReference[];
  projection: StoragePreferencesMergeProjection | null;
}

/**
 * Preserves native preference edit intentions across the callback transport boundary.
 */
export type StoragePreferenceEdit = { op: "set"; key: string; value: unknown; } | { op: "remove"; key: string; } | { op: "increment_integer"; key: string; delta: number; } | { op: "set_object_field"; key: string; field: string; value: unknown; };

/**
 * Carries a complete typed preference snapshot independently of object collection versions.
 */
export interface StoragePreferencesSnapshot {
  values: Record<string, unknown>;
  schemaVersion: number;
}

/**
 * Identifies a single callback invocation whose publication must match its original snapshot.
 */
export interface StoragePreferencesEdit {
  token: string;
  snapshot: StoragePreferencesSnapshot;
}

/**
 * Returns each storage operation's concrete business result.
 * Serialization preserves the database API payload; tool results add their own operation discriminator.
 */
export type StorageResponse = StorageOpenResult | StoragePreferencesSnapshot | StoragePreferencesEdit | StoragePreferencesSnapshot | null | null | StorageSqlRow[] | StorageSqlExecutionResult[] | StorageSqlExecutionResult[] | StorageTableDefinitionResult | StorageValueEntry | null | (StorageValueEntry | null)[] | StorageKeyValueEntry[] | StorageCommitResult | StorageChangePage;
