# File access through VFS

Business file operations use `VisualFileSystem`. Uploads, webpage results, archive imports and package validation must not hand caller-supplied paths directly to `FileSystemHost`.

Construct the VFS with `VisualFileSystem::fromContext(&context)`, or inject a required `VisualFileSystem` into a service constructor. It uses the owning runtime's storage roots and registered mounts. Do not make the mapper optional or fall back to native I/O when VFS setup fails.

Resolve file input with `PathMapper::resolveFileResourcePath(directory, path)`. This preserves VFS locators, resolves relative paths against the current file directory and maps native absolute compatibility paths into the mounted VFS. For tool invocations, obtain `directory` from `ToolExecutionManager::currentToolRuntimeContext()` and `context.fileDirectory(storage)`. A relative path without a file directory remains an explicit error. The `/` VFS directory remains valid for directory operations.

`FileSystemHost` remains the public provider contract because platform crates implement it. Rust cannot make public trait methods private to selected external crates. The repository therefore enforces a module-level native-access boundary during the `operit-tools` build. `build_support/native_filesystem.rs` parses production Rust sources and rejects references to `FileSystemHost` or access through `fileSystemHost` outside the admitted modules. Test fixtures and strings do not grant production access. The check covers Core, the Flutter native bridge and the CLI; platform provider implementations in `hosts/` retain their native role.

Each admitted module has a stated purpose. Native access is reserved for VFS dispatch after mapping, host assembly, installed package/skill/MCP storage, native streaming/staging and files returned by native recording, screenshot or speech APIs. Being admitted does not make it appropriate to read arbitrary caller paths directly. New native users require an explicit module entry and justification; business services should depend on VFS instead.

Package loaders that need a native archive receive a path resolved by VFS. Resource mounts are staged into runtime-owned storage and removed when import completes. The rest of the import pipeline only sees the resolved native file.

## Code search

`grep_code` uses the shared native filesystem search engine on Android native paths, Windows, Linux and Apple hosts. The engine uses ripgrep's matcher, streaming searcher and ignore-aware directory walker. It collects matching lines and their before/after context directly from search callbacks; it never reopens a matching file to read its full contents for context.

`max_results` limits matching lines across the whole request. Ripgrep's `max_matches` setting is per file, so each file receives only the remaining request budget. Once that budget is exhausted, the sink collects only the last admitted match's trailing context and stops. Matches encountered within that trailing context do not add results or extend the context window. No further files are visited. A zero budget performs no search. `totalMatches` and `filesSearched` describe the work actually performed, not a full-tree count beyond the budget.

Android document mounts retain their provider-backed search and existing 8 MiB per-file read limit. Their walker also stops descending and enumerating further directories when the request budget is exhausted. VFS path resolution, host errors and result schemas remain unchanged.

The native search regression suite covers global budgets, early input termination, overlapping and trailing context, CRLF, invalid UTF-8, binary detection and file filters:

```sh
cargo test --manifest-path hosts/common/operit-host-native-filesystem/Cargo.toml --lib grep::tests
```

Run the boundary and tool tests with:

```sh
cd core
cargo test -p operit-tools --lib --test native_filesystem_boundary
```
