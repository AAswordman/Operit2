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

The liquid material is a Flutter shader port of `FletchMcKee/liquid` **1.1.1**, also used by the Compose application in `prog/assistance`. It preserves the upstream rounded-rectangle SDF, analytical normals, spherical lens profile, cubic RGB dispersion, saturation/contrast operations, and directional edge light. The optical normal-field corner radii cover the refraction band to prevent diagonal creases in small-corner bubbles; the visible shape keeps its configured corner radii. Source samples remain clear; the material does not apply spatial blur. The upstream Apache-2.0 license and attribution ship in `assets/licenses/liquid/`.

`ThemeGlassSurface` retains the original frosted material: clipped backdrop blur and tint, with blur strengths 16/14/10 for panels/cards/controls. `ThemeBackground` supplies a `LiquidGlassBackdrop`; bubbles and both input styles use `LiquidGlassSurface` beneath their foreground content. During scene composition, after all paint work, one source layer image at the view's pixel density is shared by every lens. The image is produced on the raster thread without CPU pixel readback; lenses compose their shader pictures in the same submitted scene, including when foreground paint is retained. There are no post-frame captures. External video textures use `LiquidGlassSourceAnimation(active: playerIsPlaying, child: player)` to request composition frames only during playback while optical surfaces are attached; Flutter `TickerMode` controls visibility. Static sources do not run a ticker. Each lens owns its shader uniforms and maps its local coordinates into the source with its affine transform. Source UVs account for the bottom-up render-target texture used by Impeller GLES through Flutter's shader compiler backend definition; geometry, lens displacement, and dispersion remain in logical coordinates. There are no Dart platform branches. Call `await prepareLiquidGlass()` during application or preview startup to preload the optical shader. A custom scene using `LiquidGlassSurface` must provide `LiquidGlassBackdrop(background: ..., child: ...)`.

The optical checks in `test/liquid_glass_test.dart` compare rendered pixels against the upstream displacement equation, verify independent corner radii, preserve fine background detail, and cover moving/scaled surfaces, view pixel densities, shared dynamic backgrounds, and foreground sharpness. Regression checks cover corner-normal continuity in wide and tall bubbles with square, small and asymmetric corners, post-frame paint invalidation, same-frame lens updates, retained source layers, offstage scenes, playback and ticker lifecycle, and the original frosted surface blur settings. `test/liquid_glass_input_test.dart` also checks the actual Agent and Classic composer sampling at top and bottom positions, including retained movement and device pixel ratios 1 and 3. Running `test/liquid_glass_test.dart` from `apps/flutter/app` also writes `.dart_tool/liquid_glass_comparison.png` with the original frosted blur-and-tint material beside the clear optical material.

See [`docs/theme-v1-contract.md`](../../../../docs/theme-v1-contract.md).
