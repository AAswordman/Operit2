# Third-Party SDK Examples

These crates demonstrate how code outside the Operit workspace can consume the public SDK crates.

## Plugin SDK Consumer

`plugin-sdk-consumer` shows how an embedding application:

- implements `JsExecutionEngine`;
- supplies `ToolPkgExecutionEngineFactory` and `ToolPkgAssetSource`;
- supplies package state through `PackageStateResolver`;
- parses and registers a JavaScript package with `PluginPackageManager`.

Run it with:

```powershell
cd core/examples/plugin-sdk-consumer
$env:RUSTFLAGS='-Awarnings'
cargo run
```

## Provider API Consumer

`provider-api-consumer` shows how a third-party crate implements `AIService` and exposes its own connection behavior.

Run it with:

```powershell
cd core/examples/provider-api-consumer
$env:RUSTFLAGS='-Awarnings'
cargo run
```

Both examples use path dependencies only because they live in this repository. A separately published consumer should replace the path dependency with the published crate version or Git dependency.
