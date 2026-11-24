# Marketplace update identity

Marketplace versions can be published by different contributors. Installation identity is the market plus the runtime package ID; the version's publisher list is attribution, not an immutable installation owner. Requiring equal publishers caused legitimate updates to fail with `A package with name 'com.deskpet' already exists in available packages`.

On 2026-11-24, the published `com.deskpet` 1.0.0 and 1.6.0 assets were downloaded and their SHA-256 values checked against the market index. Both contain `Operit` / `com.deskpet` provenance, but their publisher lists differ. Version 1.4.1 already uses the same publisher as 1.6.0. These artifacts demonstrate the rejected update condition; they do not establish which version was installed on the reporting device.

The shared provenance comparison now accepts publisher changes for ToolPkg and standalone JS/TS updates while requiring matching markets and runtime package IDs and nonempty publisher metadata. ToolPkg imports still require marketplace authentication. Legacy external installations must authenticate their existing archive before replacement; a plain external manifest with `market_origin` alone does not qualify. Package and subpackage naming conflicts remain checked before replacement.

Disabled legacy ToolPkg installations also decode the main script's structured provenance during catalog reads, just as current marketplace installations do. No main script is executed for this metadata read, and the archive remains classified as external until the authenticated update replaces it.

Updates continue using the existing installation transaction, preserving its source filename, scope, configuration and enabled state. Regression coverage includes contributor changes, rejection of another market, rejection of unauthenticated external packages, failed replacement transactions, JS/TS updates and catalog provenance without registration.

Validation for this change: Rust source parsing through `rustfmt --emit stdout` and `git diff --check` passed. Rust tests were updated but not run; no compilation, application packaging or device update verification was performed.
