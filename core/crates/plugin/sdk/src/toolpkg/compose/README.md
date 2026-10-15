# ToolPkg Compose runtime

The Rust SDK packages these JavaScript modules at build time. `sources.json` is the ordered
source manifest shared by `build.rs` and `tools/runtime/compose_scripts.mjs`. Runtime engines
receive the prepared context and execution scripts through the existing Rust builders;
page engines never read this directory or assemble a second runtime.

| Module | Responsibility |
| --- | --- |
| `Compiler.js`, `Parser.js` | Produce and recognize portable artifacts; lazily initialize Acorn for legacy source. |
| `Generation.js` | Cache construction expressions, retain current closures, and observe node mutations. |
| `Actions.js` | Own callback closures and action IDs, bind mounted occurrences, and release detached registrations. |
| `Retained.js` | Reconcile mounted nodes and emit ordered flat revisions. |
| `Reactive.js` | Track state dependencies and schedule only affected expressions. |
| `Nodes.js` | Normalize public UI properties, child slots, modifiers, and callback references. |
| `State.js` | Own keyed hooks and state-change subscriptions. |
| `Theme.js` | Validate host snapshots and deliver theme changes. |
| `WebView.js` | Own controller requests and accepted JavaScript-interface methods. |
| `Host.js` | Adapt required native environment, tool, package, navigation, and picker bindings. |
| `Context.js` | Assemble a single SDK context and preserve its stores for its session. |
| `Session.js` | Resolve entries and create or refresh the exact owning session. |
| `Dispatch.js` | Run actions, drain intermediate work, and retire listeners when ownership ends. |
| `Responses.js`, `Exports.js` | Deliver each revision through the owning response sink and publish host entry points. |

## Ownership rules

A context has one action registry. Generated nodes hold weak references to callback ownership,
so cached nodes can mount again after their old transport IDs have expired. A callback from
another context cannot be rebound just because its string ID happens to match. Generation
owns expression caching and mutation observation; it does not keep a second callback registry.

A screen must keep the original SDK context. Local modal drafts use distinct hook keys,
not cloned context objects. Generation rejects a copied context at the boundary.

A command owns its response sink and final revision. Intermediate delivery must finish before
the final response is built. Synchronous callbacks render once in their final response;
asynchronous callbacks can publish intermediate revisions. Node transport IDs are compact
session-owned handles; keyed occurrence matching and ancestor paths stay inside the runtime.
Detached asynchronous work may keep its listener only while the
host still owns its pending references. Render and state-flush failures propagate to that
command; detached failures report through its host sink. They do not become successful empty
responses.

## Validation

`tools/tests/compose_dsl*.test.mjs` exercise the assembled runtime and compiled artifacts.
Character-card native tests execute the packaged screen modules against the same runtime.
`operit-js-bridge` Compose tests run the Rust-assembled sources in QuickJS. Performance probes
also consume the manifest, so they measure the same module order as production.
