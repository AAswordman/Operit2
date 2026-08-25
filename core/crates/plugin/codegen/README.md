# operit-plugin-sdk-codegen

`operit-plugin-sdk-codegen` generates SDK binding artifacts from Rust and JavaScript source declarations.

It is a build-time helper for plugin SDK and tool integration code. It does not own runtime plugin state, JavaScript engine instances, or tool execution.

## Boundary

- Owns AST parsing and generated binding text.
- Serves build scripts in plugin and tool crates.
- Keeps SDK code generation separate from proxy and rslink code generation.
