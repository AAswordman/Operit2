# ToolPkg v1 handwritten declarations

These 25 declaration files originate from `assistance/examples/types`.
The v1 handwritten TypeScript contract remains authoritative; it is not generated
from the v2 Rust SDK. Operit2 adds one local declaration in `quickjs-runtime.d.ts`:
`global: typeof globalThis`, matching the alias supplied by the shared engine
bootstrap.
`source.json` records the upstream commit, local extensions, and current file
identities. `LICENSE` is copied from that repository.

`1.0.0` and `1.0.1` share this tree. Their differences remain expressed by the
upstream `@since` annotations. They do not have separate declaration directories.
Annotations document availability; TypeScript does not enforce manifest versions.

## Selecting types

Use this tree instead of `plugins/types` for a v1 plugin. For example, a plugin
located at `plugins/packages/external/my_plugin` can use:

```json
{
  "compilerOptions": {
    "target": "es2020",
    "module": "commonjs",
    "lib": ["es2020"],
    "strict": true,
    "skipLibCheck": true,
    "types": []
  },
  "files": ["../../../types-v1/index.d.ts", "../../../types-v1/quickjs-runtime.d.ts"],
  "include": ["src/**/*.ts"]
}
```

Adjust explicit type-reference paths in plugin sources as well. Do not include
both v1 and v2 global entry points in one plugin compilation. The existing
`plugins/types` directory remains the v2 generated declaration destination.

## Updating the snapshot

From the repository root, with a clean committed v1 type tree:

```powershell
.venv/Scripts/python.exe plugins/tools/sync_v1_types.py import ../assistance
.venv/Scripts/python.exe plugins/tools/sync_v1_types.py check
tsc -p core/crates/plugin/sdk/src/compat/v1/tsconfig.json
node --test tools/tests/toolpkg_api_compatibility.test.mjs tools/tests/toolpkg_v1_adapters.test.mjs
```

Importing reads the committed source bytes, applies the documented `global`
alias extension, and records the resulting hashes. Git preserves the snapshot's
bytes across platforms. Importing updates the declarations and provenance only.
It does not establish runtime support for newly declared capabilities. Review
adapter and fixture changes before committing an upstream update. The verification command does not
need the original repository.

See `plugins/docs/V1_COMPATIBILITY.md` for the implemented runtime boundary and
remaining capability differences.
