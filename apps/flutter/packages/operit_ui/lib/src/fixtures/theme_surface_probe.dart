import 'package:flutter/material.dart';

import '../model/theme_preference_defaults.dart';
import '../model/theme_preference_snapshot.dart';
import '../render/bubble_surface.dart';
import '../render/chat_input_surface.dart';
import '../render/composer_surface.dart';
import '../render/theme_glass_surface.dart';

/// Versioned feasibility fixture, not a substitute for the full chat renderer.
/// The app and standalone preview use these exact production surface widgets.
class ThemeSurfaceProbe extends StatelessWidget {
  const ThemeSurfaceProbe({
    super.key,
    required this.snapshot,
    this.userFontFamily,
    this.aiFontFamily,
  });

  static const fixtureVersion = 'surfaces-v1';
  static const glyphCorpus =
      'Message / 输入消息 Operit2 · Theme surface probe '
      'Shared Material / glass / bubble / input surfaces '
      'Settings / 设置 Hello Operit / 主题预览 '
      'One Dart renderer, no Core runtime. '
      '同源渲染入口；这不是完整聊天场景或可导出的主题。';
  final ThemePreferenceSnapshot snapshot;
  final String? userFontFamily;
  final String? aiFontFamily;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final radius = BorderRadius.circular(20);
        final input = Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: <Widget>[
              const Expanded(child: Text('Message / 输入消息')),
              Icon(Icons.arrow_upward, color: scheme.primary),
            ],
          ),
        );
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            ThemeGlassSurface(
              transparentSurface: snapshot.transparentSurfaceEnabled,
              color: scheme.surfaceContainer,
              borderRadius: BorderRadius.circular(16),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      'Operit2 · Theme surface probe',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      'Shared Material / glass / bubble / input surfaces',
                    ),
                    const SizedBox(height: 12),
                    const TextField(
                      readOnly: true,
                      decoration: InputDecoration(labelText: 'Settings / 设置'),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),
            Align(
              alignment: Alignment.centerRight,
              child: ConstrainedBox(
                constraints: BoxConstraints(maxWidth: width * 0.82),
                child: BubbleSurface(
                  color: snapshot.bubbleUserBubbleColor == null
                      ? scheme.primaryContainer
                      : Color(snapshot.bubbleUserBubbleColor!),
                  borderRadius: snapshot.bubbleUserRoundedCornersEnabled
                      ? BorderRadius.circular(16)
                      : BorderRadius.zero,
                  transparentSurface: snapshot.transparentSurfaceEnabled,
                  imageStyle: snapshot.bubbleUserUseImage
                      ? BubbleImageStyle(
                          imagePath: snapshot.bubbleUserImageUri!,
                          cropLeftRatio: snapshot.bubbleUserImageCropLeft,
                          cropTopRatio: snapshot.bubbleUserImageCropTop,
                          cropRightRatio: snapshot.bubbleUserImageCropRight,
                          cropBottomRatio: snapshot.bubbleUserImageCropBottom,
                          repeatXStartRatio:
                              snapshot.bubbleUserImageRepeatStart,
                          repeatXEndRatio: snapshot.bubbleUserImageRepeatEnd,
                          repeatYStartRatio:
                              snapshot.bubbleUserImageRepeatYStart,
                          repeatYEndRatio: snapshot.bubbleUserImageRepeatYEnd,
                          imageScale: snapshot.bubbleUserImageScale,
                          renderMode: snapshot.bubbleUserImageRenderMode,
                        )
                      : null,
                  child: Padding(
                    padding: EdgeInsets.fromLTRB(
                      snapshot.bubbleUserContentPaddingLeft,
                      12,
                      snapshot.bubbleUserContentPaddingRight,
                      12,
                    ),
                    child: Text(
                      'Hello Operit / 主题预览',
                      style: TextStyle(
                        color: snapshot.bubbleUserTextColor == null
                            ? scheme.onPrimaryContainer
                            : Color(snapshot.bubbleUserTextColor!),
                        fontFamily: userFontFamily,
                        fontFamilyFallback: const <String>[],
                      ),
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 16),
            BubbleSurface(
              color: snapshot.bubbleAiBubbleColor == null
                  ? scheme.surfaceContainerHighest
                  : Color(snapshot.bubbleAiBubbleColor!),
              borderRadius: snapshot.bubbleAiRoundedCornersEnabled
                  ? BorderRadius.circular(16)
                  : BorderRadius.zero,
              transparentSurface: snapshot.transparentSurfaceEnabled,
              imageStyle: snapshot.bubbleAiUseImage
                  ? BubbleImageStyle(
                      imagePath: snapshot.bubbleAiImageUri!,
                      cropLeftRatio: snapshot.bubbleAiImageCropLeft,
                      cropTopRatio: snapshot.bubbleAiImageCropTop,
                      cropRightRatio: snapshot.bubbleAiImageCropRight,
                      cropBottomRatio: snapshot.bubbleAiImageCropBottom,
                      repeatXStartRatio: snapshot.bubbleAiImageRepeatStart,
                      repeatXEndRatio: snapshot.bubbleAiImageRepeatEnd,
                      repeatYStartRatio: snapshot.bubbleAiImageRepeatYStart,
                      repeatYEndRatio: snapshot.bubbleAiImageRepeatYEnd,
                      imageScale: snapshot.bubbleAiImageScale,
                      renderMode: snapshot.bubbleAiImageRenderMode,
                    )
                  : null,
              child: Padding(
                padding: EdgeInsets.fromLTRB(
                  snapshot.bubbleAiContentPaddingLeft,
                  16,
                  snapshot.bubbleAiContentPaddingRight,
                  16,
                ),
                child: Text(
                  'One Dart renderer, no Core runtime.\n'
                  '同源渲染入口；这不是完整聊天场景或可导出的主题。',
                  style: TextStyle(
                    color: snapshot.bubbleAiTextColor == null
                        ? scheme.onSurface
                        : Color(snapshot.bubbleAiTextColor!),
                    fontFamily: aiFontFamily,
                    fontFamilyFallback: const <String>[],
                  ),
                ),
              ),
            ),
            const SizedBox(height: 24),
            if (snapshot.inputStyle ==
                ThemePreferenceDefaults.INPUT_STYLE_AGENT)
              AgentInputSurface(
                floating: snapshot.chatInputFloating,
                color: snapshot.inputBackgroundColor != null
                    ? Color(snapshot.inputBackgroundColor!)
                    : scheme.surfaceContainer,
                shape: RoundedRectangleBorder(
                  borderRadius: snapshot.inputRoundedCornersEnabled
                      ? radius
                      : BorderRadius.zero,
                ),
                borderRadius: snapshot.inputRoundedCornersEnabled
                    ? radius
                    : BorderRadius.zero,
                transparentSurface: snapshot.transparentSurfaceEnabled,
                imageStyle: buildThemeInputImageStyle(snapshot),
                width: width,
                margin: EdgeInsets.zero,
                child: input,
              )
            else
              ClassicInputSurface(
                floating: snapshot.chatInputFloating,
                color: snapshot.inputBackgroundColor != null
                    ? Color(snapshot.inputBackgroundColor!)
                    : scheme.surfaceContainer,
                shape: RoundedRectangleBorder(
                  borderRadius: snapshot.inputRoundedCornersEnabled
                      ? radius
                      : BorderRadius.zero,
                ),
                borderRadius: snapshot.inputRoundedCornersEnabled
                    ? radius
                    : BorderRadius.zero,
                transparentSurface: snapshot.transparentSurfaceEnabled,
                imageStyle: buildThemeInputImageStyle(snapshot),
                width: width,
                margin: EdgeInsets.zero,
                child: input,
              ),
          ],
        );
      },
    );
  }
}
