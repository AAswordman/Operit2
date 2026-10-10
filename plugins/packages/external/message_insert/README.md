# Extra information injection

The package injects enabled information into user-message attachments. It uses
ToolPkg API `2.0.0` and requires `com.operit.character_cards >= 0.1.0`, as declared
in `manifest.json`. Both packages must be installed and enabled.

Memory injection imports the character package's copied public client at
`src/dependencies/character_cards/api.ts`. That file is an exact copy of the
provider's `public_api` (`character_cards/src/api.ts`); update the copy explicitly
when adopting a changed contract. It is self-contained and imports no provider
implementation files. See `plugins/docs/PUBLIC_API.md` for the dependency protocol.

Prompt hooks forward the generic `executionContext.participantId` to
`characterCards.memory.query`. The provider resolves that participant's persisted
memory binding and returns typed matches, including document snippets. It retains
owner-scoped query snapshots in its main runtime. Disabling repeated searches
uses the full chat ID as the snapshot key; enabling them requests a fresh query.
The consumer formats those matches into attachments. Missing participants and
provider failures remain visible through the existing injection error reporting.

Build with the repository plugin synchronizer. A direct type check is:

```powershell
tsc -p plugins/packages/external/message_insert/tsconfig.json --noEmit
```
