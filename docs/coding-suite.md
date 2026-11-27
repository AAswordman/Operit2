# Coding environment and project memory

The built-in [Coding Suite](../plugins/packages/buildin/coding_suite/README.md)
owns coding instructions and append-only project memory. Terminal environment
context belongs to `super_admin`; neither capability relies on character cards.

`super_admin` adds current Core platform, current VFS roots, default terminal
profile and available saved profiles immediately before sending a chat request
or estimating its tokens. It queries the Host's terminal information directly,
without executing a shell command or querying unrelated device services. Profile
changes and Core switches are observed on the next request. Only public profile
fields enter the prompt, and each plugin replaces its own prior contribution if
the same prepared request is finalized again.

The native prompt boundary exposes `metadata.outputPath` from the existing
`ChatOutputStore` binding. Plugins must use it for unbound conversations rather
than derive an output folder from a chat ID or date. Workspace and output roots
remain VFS addresses; terminal physical-path mapping continues to belong to the
terminal Host.

## Temporary plugin suspension

A ToolPkg manifest may declare:

```json
{
  "toolpkg_id": "com.operit.coding_suite",
  "suspends": ["com.operit.character_cards"]
}
```

The list contains unique, exact container IDs. Self-suspension and direct required
dependencies are rejected. A requested, dependency-satisfied suspender excludes
its targets from the executable graph. Required dependents of a suspended target
are also blocked. `suspended_by` and `suspended` dependency diagnostics explain
this state in the catalog. Requested enable preferences remain intact, so removal
of the suspension restores exactly the previously requested graph.

Hook discovery, public API admission and package tool admission use this same
effective graph. The mechanism is generic; core runtime code does not recognize
character or coding plugin IDs. Plugin activation currently uses installation
scope and affects all conversations in that scope.

## Verification

Run the focused JavaScript tests without compiling the application:

```text
node --test tools/tests/coding_suite.test.mjs tools/tests/terminal_environment_prompt.test.mjs
node .ci-tools/typescript/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/buildin/coding_suite/tsconfig.json
node .ci-tools/typescript/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/buildin/super_admin/tsconfig.json
```

`environment_suspension_preserves_activation_preferences` in the SDK's Rust tests
covers transitive suspension, hook admission and restoration. Application builds
and live multi-device behavior require separate validation.
