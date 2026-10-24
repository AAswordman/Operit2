# Extra information injection

The package injects enabled device information into user-message attachments. It
uses ToolPkg API `2.0.0` and declares no package dependencies in `manifest.json`.

Injected items cover time, battery, weather, location, the current screen app,
recent app usage, screen text, and notifications. Character memory recall is a
built-in capability of `com.operit.character_cards` through its own prompt
finalize hook; this package no longer performs any memory query.

Prompt hooks run at `before_process` when injected content persists with the
message, and at `before_send_to_model` when it is transient. The shared
injection deadline bounds every enabled collection item.

Build with the repository plugin synchronizer. A direct type check is:

```powershell
tsc -p plugins/packages/external/message_insert/tsconfig.json --noEmit
```
