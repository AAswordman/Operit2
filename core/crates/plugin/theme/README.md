# operit-plugin-theme

Theme template definitions and renderer media validation for ordinary ToolPkg contributions.

A ToolPkg may declare scripts, resources, workspace templates and any number of `themes` together. The shared ToolPkg loader and package manager own package transport, installation, scope, synchronization and removal. This crate validates the visual state and media referenced by each theme declaration.

- `normalize_draft` expands documented visual defaults; `parse_theme` validates fully expanded visual state.
- The shared `ToolPkgArchiveParser::readThemeTemplates` resolves ordinary manifests and invokes `validate_declared_theme_templates` for their referenced resources. Packages without declarations produce an empty list.
- `validate_toolpkg_theme_templates` and `validate_toolpkg_theme_template` check Studio's explicit root-manifest output before export.
- `ValidatedThemeTemplate` and `ThemeValidationReceipt` cover only the selected template. They do not authorize installation or attest publisher identity.
- `validate_theme_preview_transfer` validates authoring data using bounded binary resource transfers.
- `export_toolpkg_from_theme` and `export_toolpkg_from_theme_transfer` create an ordinary ToolPkg with one template declaration. Studio preserves the other capabilities when editing an existing package.
- Browser bindings expose `normalizeThemeDraft`, `validateThemePreviewDraft` and `exportToolPkgFromTheme`.

Template resource declarations contain only `path` and `mediaType`. File sizes and cache digests are computed automatically; legacy `bytes` and `sha256` values are ignored.

The package-level optional `license` field references an agreement text inside the ToolPkg. Templates have no platform list, agreement field, API version or application-version requirement.

See [template definitions](../../../../docs/theme-v1-contract.md) and [renderer checks](SECURITY.md).
