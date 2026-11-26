import 'package:flutter/material.dart';

import '../model/theme_preference_defaults.dart';
import '../model/theme_preference_snapshot.dart';
import 'bubble_surface.dart';
import 'chat_input_surface.dart';

const double themeChatContentMaxWidth = 700;
const double themeChatWideContentMaxWidth = 1040;

/// Space between the Agent composer surface and its content.
const EdgeInsets themeAgentComposerInsets = EdgeInsets.fromLTRB(12, 14, 12, 8);

/// Space between the Classic composer surface and its content.
EdgeInsets themeClassicComposerInsets({required bool floating}) =>
    EdgeInsets.symmetric(
      horizontal: floating ? 14 : 22,
      vertical: floating ? 6 : 8,
    );

/// Space between a composer's content and the bottom edge of its surface,
/// counting the system inset that a surface keeps clear below it when it is
/// not floating.
double themeComposerBottomInset({
  required EdgeInsets insets,
  required bool floating,
  required double systemBottomInset,
}) => insets.bottom + (floating ? 0 : systemBottomInset);

/// Extracts the nine-slice image style for the chat input box when configured.
BubbleImageStyle? buildThemeInputImageStyle(ThemePreferenceSnapshot snapshot) {
  final imagePath = snapshot.inputImageUri;
  if (!snapshot.inputUseImage || imagePath == null || imagePath.isEmpty) {
    return null;
  }
  return BubbleImageStyle(
    imagePath: imagePath,
    cropLeftRatio: snapshot.inputImageCropLeft,
    cropTopRatio: snapshot.inputImageCropTop,
    cropRightRatio: snapshot.inputImageCropRight,
    cropBottomRatio: snapshot.inputImageCropBottom,
    repeatXStartRatio: snapshot.inputImageRepeatStart,
    repeatXEndRatio: snapshot.inputImageRepeatEnd,
    repeatYStartRatio: snapshot.inputImageRepeatYStart,
    repeatYEndRatio: snapshot.inputImageRepeatYEnd,
    imageScale: snapshot.inputImageScale,
    renderMode: snapshot.inputImageRenderMode,
  );
}

/// Shared input-card configuration, including the distinct Classic geometry.
class ThemeComposerCard extends StatelessWidget {
  const ThemeComposerCard({
    super.key,
    required this.inputStyle,
    required this.floating,
    required this.transparentSurface,
    required this.child,
    this.backgroundColor,
    this.roundedCorners = true,
    this.imageStyle,
  });
  final String inputStyle;
  final bool floating;
  final bool transparentSurface;
  final Widget child;
  final Color? backgroundColor;
  final bool roundedCorners;
  final BubbleImageStyle? imageStyle;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    switch (inputStyle) {
      case ThemePreferenceDefaults.INPUT_STYLE_AGENT:
        final radius = !roundedCorners
            ? BorderRadius.zero
            : floating
            ? BorderRadius.circular(22)
            : const BorderRadius.vertical(top: Radius.circular(20));
        return AgentInputSurface(
          floating: floating,
          color: backgroundColor ?? scheme.surfaceContainer,
          shape: RoundedRectangleBorder(borderRadius: radius),
          borderRadius: radius,
          transparentSurface: transparentSurface,
          imageStyle: imageStyle,
          width: double.infinity,
          margin: floating
              ? const EdgeInsets.fromLTRB(8, 4, 8, 6)
              : const EdgeInsets.only(top: 4),
          child: Padding(
            padding: themeAgentComposerInsets,
            child: child,
          ),
        );
      case ThemePreferenceDefaults.INPUT_STYLE_CLASSIC:
        final radius = !roundedCorners
            ? BorderRadius.zero
            : floating
            ? BorderRadius.circular(22)
            : BorderRadius.zero;
        return ClassicInputSurface(
          floating: floating,
          color: backgroundColor ?? scheme.surface,
          shape: RoundedRectangleBorder(borderRadius: radius),
          borderRadius: radius,
          transparentSurface: transparentSurface,
          imageStyle: imageStyle,
          width: double.infinity,
          margin: floating
              ? const EdgeInsets.fromLTRB(8, 0, 8, 6)
              : EdgeInsets.zero,
          child: Padding(
            padding: themeClassicComposerInsets(floating: floating),
            child: child,
          ),
        );
      default:
        throw FormatException('Unsupported theme input style: $inputStyle');
    }
  }
}

/// Resolves the custom action button asset for the current composer state.
String? resolveThemeInputActionAsset(
  ThemePreferenceSnapshot snapshot, {
  required bool isStop,
  required bool isSend,
}) {
  if (isStop) return snapshot.inputStopAsset;
  if (isSend) return snapshot.inputSendAsset;
  return snapshot.inputMicAsset;
}

/// Resolves the custom action button asset size for the current composer state.
double resolveThemeInputActionAssetSize(
  ThemePreferenceSnapshot snapshot, {
  required bool isStop,
  required bool isSend,
}) {
  if (isStop) return snapshot.inputStopAssetSize;
  if (isSend) return snapshot.inputSendAssetSize;
  return snapshot.inputMicAssetSize;
}
