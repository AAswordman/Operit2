# Chat glass background capture

Transparent bubbles and composers keep their existing liquid shader, tint,
refraction, dispersion, clipping and physical-pixel sampling resolution.

The background repaint boundary owns an `OffsetLayer` that retains the captured
image. Flutter's layer invalidation determines when the source needs capturing;
foreground scrolling and cursor animation reuse the unchanged background image.
Each scene shares one captured image across its lenses. Lens pictures are reused
while their source image, geometry, transform and shader uniforms remain equal.

Source repaint, descendant layer changes, source size, device pixel ratio and
source ownership invalidate the capture. Layers that request composition every
frame continue updating. Unfrozen external `TextureLayer` content also stays live
because native texture pixels can change without a Dart repaint. Video playback
continues requesting compositions through `LiquidGlassSourceAnimation`.

The final lens removal and scene disposal release the retained image. Capture
occurs before foreground lenses are composed, preserving same-frame updates.
No shader parameters, image resolution or visual effects are reduced.

The focused regression checks cover static capture reuse across 60 moving-lens
frames, pixel equality between retained and freshly captured sources, and nested
background repaint. Existing optical tests exercise transforms, corner geometry,
DPR, external composition, playback, offstage restoration and both input styles.
Real-device frame-rate improvement still requires a matching before/after trace.
