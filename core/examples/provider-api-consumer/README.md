# Provider API Consumer

This example implements a custom `AIService` in an external crate.

Real providers can additionally implement `send_message`, `get_models_list`, and token calculation while keeping provider-specific transport logic outside the Operit runtime.

Run:

```powershell
$env:RUSTFLAGS='-Awarnings'
cargo run
```
