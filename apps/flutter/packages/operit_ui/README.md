# operit_ui

Shared Flutter UI components, appearance models, and asset/font preparation extracted from the Operit2 application. Both the application and the Theme Studio preview use these components. The package has no dependency on Core, Host, filesystem, HTTP, `operit2`, or native build hooks.

## Included Capabilities

- **Model & Contract Projection**:
  - `ThemePreferenceDefaults` and the 103-field `ThemePreferenceSnapshot` (covering Material schemes, background, glass surfaces, user/AI bubbles, 9-slice parameters, input box 9-slice surface, and the 5 customizable input action icons + sizes).
  - `ThemeVisualSpecV1` projects complete template defaults into the ordinary appearance snapshot, including the optional full palette JSON.
- **Asset & Font Preparation**:
  - `ThemeAssetResolver`, `MemoryThemeAssetResolver`, `ThemeAssetScope`, `ResolvedThemeImage`, and `ThemeImageDecoder` (decoding PNG/JPEG and rasterizing validated static SVGs at intrinsic dimensions).
  - `ThemeFontRegistry`, `BuiltinThemeFonts` (pinned `noto-sans-sc` and `cascadia-mono` static TrueType assets with SHA-256 verification), `ThemeFontGlyphs`, and `PreparedThemeVisual`.
- **Shared Client & Studio Surfaces**:
  - `buildOperitThemeData`, `ThemeRenderRoot`, `ThemeBackgroundLayer`, `ThemeGlassSurface`.
  - `BubbleImageStyle`, `NineSliceBubbleSurface`, `BubbleSurface`.
  - `ChatInputSurface`, `ThemeComposerChrome`, `ThemeTopBarChrome`, `ThemeDrawerChrome`, `ThemeHistoryChrome`, `ThemeDesktopSidebarSurface`.
  - `ThemeClientScene` (`client-shell-v7`, shared by client full-scene preview and Web Theme Studio) and `ThemeSurfaceProbe` (`surfaces-v1`).

See [`docs/theme-v1-contract.md`](../../../../docs/theme-v1-contract.md).
