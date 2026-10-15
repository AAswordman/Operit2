# operit-route-macros

`operit-route-macros` contains the procedural macros for annotated Rust route functions.

It expands route wrappers and route metadata for Rust-internal route calls. The wrapper path uses the shared link protocol and node route runtime; it is separate from core-app proxy generation.

## Boundary

- Owns annotation parsing and macro expansion.
- Does not own runtime construction, proxy scanning, peer transport, or route dispatch state.
- Keeps route code generation distinct from root-registered proxy code generation.

Routes may declare only `binding = argument`; authenticated Space membership and Binding routing still apply.
An optional `permission = "scope:capability"` adds a specific capability check.
`create_binding = "runtime.execute"` selects a node that can host Core execution, without requiring the caller to host it.
