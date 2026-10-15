import 'package:flutter/material.dart';

import '../model/theme_preference_defaults.dart';
import '../model/theme_preference_snapshot.dart';
import 'bubble_surface.dart';
import 'chat_input_surface.dart';

const double themeChatContentMaxWidth = 700;
const double themeChatWideContentMaxWidth = 1040;

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
            padding: const EdgeInsets.fromLTRB(12, 14, 12, 8),
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
            padding: EdgeInsets.symmetric(
              horizontal: floating ? 14 : 22,
              vertical: floating ? 6 : 8,
            ),
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
