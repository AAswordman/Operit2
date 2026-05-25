# operit-host-web

`operit-host-web` provides Web/WASM implementations for every host trait in `operit-host-api`.
Rust calls the same host traits as native builds. The browser side installs `globalThis.__operitHost`
before constructing the web host structs.

## Rust Layout

- `src/lib.rs` exports the web host structs.
- `src/tools/fs/mod.rs` implements `FileSystemHost`.
- `src/tools/browser/mod.rs` implements `WebVisitHost`.
- `src/tools/runtime/mod.rs` implements `ManagedRuntimeHost`.
- `src/tools/storage/mod.rs` implements `RuntimeStorageHost` and `RuntimeSqliteHost`.
- `src/tools/system/mod.rs` implements `SystemOperationHost`.
- `src/common.rs` contains the shared JS bridge and value conversion helpers.

## Browser Bridge Shape

```ts
type OperitWebHost = {
  fileSystem: FileSystemBridge;
  webVisit: WebVisitBridge;
  managedRuntime: ManagedRuntimeBridge;
  managedRuntimeProcess: ManagedRuntimeProcessBridge;
  runtimeStorage: RuntimeStorageBridge;
  sqlite: SqliteBridge;
  systemOperation: SystemOperationBridge;
};
```

The bridge values use the same field names as `operit-host-api`. Binary data is passed as
`Uint8Array`. SQLite integers are passed as strings to preserve 64-bit values across JS.

```ts
type SqliteValue =
  | { kind: "null" }
  | { kind: "integer"; value: string }
  | { kind: "real"; value: number }
  | { kind: "text"; value: string }
  | { kind: "blob"; value: Uint8Array };

type SqliteRow = {
  columns: string[];
  values: SqliteValue[];
};
```
