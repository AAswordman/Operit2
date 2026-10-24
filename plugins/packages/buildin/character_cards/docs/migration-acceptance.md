# Character storage acceptance

The first plugin-owned storage schema is v1. Configuration files live in the character plugin's persistent Space data folder and use native Preferences DataStore. There is no unpublished v2 format or 1-to-2 compatibility path.

The Rust released-source importer adopts old Core preferences and memory records into their explicit plugin owners. TypeScript's native `withSchema(1)` callback converts released identities and references into UUIDs, including card/group/tag/store identities, chat selections, message author extensions, memory owners and nested memory identities. Existing UUIDs and external model/provider/theme/TTS identifiers remain unchanged.

Owner resources are copied and byte-verified before source retirement. Conversion work is retained in a separate native preference file so interrupted file or message operations resume with the same UUID map. This is initial business-data conversion, not a second synchronization journal. Final schema publication is owned by native Preferences.

Production configuration reads use `data()` and writes use `edit()`. TypeScript declares catalog and reference locations; Rust's existing Preferences merge executor owns content matching, UUID aliases, name collision handling and replay.

Checks cover JSON persistence and reopen, callback rollback, concurrent native edits, schema validation, released-source conversion, interrupted resource/message work and reference rewriting. The Windows restored user dataset must remain untouched by automated checks.

Memory still uses the existing SQLite object engine. Actual ObjectBox integration and a complete restored-Windows-data-to-current-plugin acceptance run remain outstanding. Per-file transactions must not be described as an atomic transaction across all preference owners and memory.
