# Device font access

This plugin enumerates fonts on the **Flutter client's device**, independently of
the connected runtime. It reads only a family from its enumerated catalog and
returns its exact regular face for Flutter registration.

| Host | Catalog and font resource |
| --- | --- |
| Windows | DirectWrite system collection and local font-file loader |
| macOS / iOS | CoreText available families, font URL and PostScript face name |
| Linux | Fontconfig families, file paths and collection indices |
| Android | Accessible OS font directories and OpenType family/style metadata; no storage permission or hidden APIs |
| OpenHarmony | `font.getSystemFontList()` / `font.getFontByName()` |
| Browser | Local Font Access, requested directly from a user gesture |

The browser must implement Local Font Access and grant the requested permission.
Unsupported access and read errors are returned to the caller. Catalogs are not
replaced with hardcoded font lists.

`extractFontFace` turns a selected TTC face into a standalone SFNT. It preserves
the chosen face's tables, removes the collection's digital signature, and
recalculates directory and whole-font checksums. This prevents `FontLoader` from
loading a collection's first face when the user selected another family.

The application registers bytes for preview, then persists those same bytes using
the existing theme asset owner. Appearance stores `fontType: device`, the selected
family name, and the resource reference. Restarting, switching themes, or syncing
the configuration loads this saved face. UI code does not branch on the platform.

Linux builds require the Fontconfig development package (`pkg-config fontconfig`).
This source is covered by the repository's AGPL-3.0 license.
