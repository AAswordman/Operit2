# ToolPkg Compose runtime

`sources.json` is the ordered source manifest used by the Rust SDK build and
`tools/runtime/compose_scripts.mjs`. Page runtimes receive the prepared SDK
scripts through the host. They do not read this directory themselves.

## Execution model

The compiler emits ABI 4 artifacts containing immutable layout programs. A
program contains instruction operands for constants, dynamic parameters,
objects, arrays, local instruction addresses, and array concatenation. Directly
nested UI calls share one program. Arbitrary helper calls, object accessors,
spreads, evaluation order and reactive lexical ownership retain ordinary
JavaScript behavior. Helper boundaries can therefore produce multiple instances
for a single logical row.

Calling a compiled UI expression declares a source instance and its parameter
vector. Construction normalizes the dynamic values at that boundary, binds
instance-owned events, and records direct source references. It does not expand
the program into JavaScript nodes. The normal commit visits source membership
and emits a transaction containing programs, instances, explicit patches, and
removed source IDs. Each program is announced once per session.

Public node handles expose lazy instruction views for plugins that explicitly
read `props`, `children`, `slots` or `type`. Assignments produce local field
patches. Local mutation validation is atomic. Parameters hidden by explicit
edits do not retain removed source addresses in transport snapshots. The cached
constructor values remain available to the owning JavaScript instance.

Flutter installs and validates each program once. An instruction cursor contains
only a source and a local instruction address. Source instances own parameters
and change notifications; individual instructions do not own retained mutable
node records. Flutter owns occurrence keys, mount lifetime, event bindings and
resources. A source may be mounted more than once without duplicating its
JavaScript event slots. CLI and desktop consumers use the same program store
and transient instruction views.

## Modules

| Module | Responsibility |
| --- | --- |
| `Compiler.js`, `Parser.js`, `Artifact.js` | Compile source into shared programs and enforce artifact ABI 4. |
| `Buffer.js` | Write indexed parameter storage during construction and seal the single transferable arena. |
| `Composition.js` | Own source parameters, reference membership, explicit edits and atomic revisions. |
| `Actions.js` | Own callback closures, context ownership and transport event addresses. |
| `Nodes.js` | Expose logical public handles, mutation observation and dynamic UI constructors. |
| `Reactive.js` | Track lexical dependencies, source slots and scheduled expression execution. |
| `State.js`, `Lazy.js` | Own hooks, memo execution and bounded list windows. |
| `Theme.js`, `WebView.js`, `Host.js` | Bind host snapshots, controller operations and host capabilities. |
| `Context.js` | Assemble the original owning context and its stores. |
| `Session.js`, `Dispatch.js` | Resolve screen entries and execute commands and lifecycle events. |
| `Responses.js`, `Exports.js` | Publish command-owned transactions and expose host entry points. |

Generation, Records and Retained are removed from production execution. Earlier
executable artifact ABIs must be rebuilt; there is no old-protocol adapter.

## Ownership and delivery

A context owns one event registry. Callback references preserve their context
and logical event ownership through explicit public property views. Detached
sources retain their cached closures, but their live transport addresses expire.
Remounting creates fresh addresses. Removing an event from a live source also
expires its registration. A foreign context cannot bind a callback merely
because its string ID happens to match.

A command owns its response sink and completion revision. Intermediate updates
complete before the final response. Unchanged intermediate results allocate no
revision. Successful final responses preserve contiguous revisions. Render,
validation and action failures propagate to the owning command.

Host input updates refresh the current root and execute `onInputsChanged` in the
same command. Memo and lazy item execution retain their lexical hook and
subscription ownership. A shared layout program does not merge independent
reactive executions or their effects. List windowing remains a separate API;
the program protocol also applies to fully materialized lists.

## Binary ownership

`Buffer.js` writes final indexed storage while construction normalizes each
parameter. Commit assembles address tables; it does not construct another
transaction object tree. The 32-byte CPB1 header stores version, used length,
phase, transaction address, optional action-result address and navigation
address. Values use aligned 16-byte descriptors, little-endian fields and
UTF-16 string storage. Transaction records are
`[reset, revision, rootInstance, rootNode, programs, instances, removed]`;
instances are `[id, programId, values, patches]` and patches are
`[node, field, value]`. Programs retain their instruction object schema.

The existing synchronous Host registration accepts either ordinary structured
arguments or a top-level owned byte buffer. Compose sends only a Uint8Array.
Native V8 detaches its ArrayBuffer and transfers the backing-store owner without
reading the layout payload. Rust validates the header and reads application
results and navigation independently; updates retain the original buffer through
CoreBuffer and CoreLink attachments. Flutter projects indexed containers from
that buffer rather than decoding a second parameter tree. Finite CLI and desktop
snapshots explicitly materialize their views using the same binary format.

Web uses the same Host registration and Compose format. Its Host transfers the
JS allocation before copying the bytes into Wasm memory. Network transports
also pay their byte-transfer cost. Constant work for the native submission step
does not make construction, arbitrary action results, list membership or Flutter
layout constant-time.

## Validation

`tools/tests/compose_dsl_*.test.mjs` exercises compilation, execution, ownership,
mutation and transaction behavior. Assertion-only host projections live in test
support files and run outside benchmark timers. Flutter layout and lifecycle
tests consume program transactions. `tools/benchmarks/compose_sidebar.mjs`
compares full SDK versions against the same raw sidebar source; Node timings
must not be reported as native engine timings.
