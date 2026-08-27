# Application

Application crates own composition roots.

This domain is the only place where runtime, proxy, node, and access crates are assembled into one long-lived Core tree. Host surfaces should use handles from this domain and keep UI, command parsing, or platform adapters outside the Core ownership graph.

## Crates

- `core`: the current CoreApplication facade and lifecycle owner.
