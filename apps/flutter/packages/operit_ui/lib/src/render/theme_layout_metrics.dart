/// Shared navigation breakpoint used by the client and the fixed Studio scene.
/// Below this width, the chat owns the viewport and the drawer is overlaid.
const double themeTabletBreakpoint = 600;

/// Shared message geometry for the transcript and the Studio sample.
const double themeBubbleAvatarSize = 32;
const double themeBubbleAvatarGap = 8;
const double themeBubbleNormalWidthFactor = 0.85;

/// Resolves a bubble's limit after its row has reserved any avatar column.
double resolveThemeBubbleMaxWidth(
  double availableWidth, {
  required bool wideLayout,
}) {
  final width = availableWidth.clamp(0.0, double.infinity).toDouble();
  return wideLayout ? width : width * themeBubbleNormalWidthFactor;
}
