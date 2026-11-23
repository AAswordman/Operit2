// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../../theme/OperitTheme.dart';
import 'ChatAppearancePreview.dart';

/// Shows the overall palette and real background in thumbnails; opens the
/// production chat widgets when a detailed preview is requested.
class ThemeAppearancePreview extends StatelessWidget {
  const ThemeAppearancePreview({
    super.key,
    required this.snapshot,
    this.visual,
    this.assets,
    this.thumbnail = true,
  });

  final ThemePreferenceSnapshot snapshot;
  final PreparedThemeVisual? visual;
  final ThemeAssetResolver? assets;
  final bool thumbnail;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(10),
      child: ExcludeSemantics(
        child: OperitThemePreview(
          snapshot: snapshot,
          visual: visual,
          assets: assets,
          child: thumbnail
              ? const SizedBox(height: 240, child: _ThemeMoodPreview())
              : const SingleChildScrollView(
                  child: ChatAppearancePreview(showInputPreview: true),
                ),
        ),
      ),
    );
  }
}

/// Sparse shapes hint at message surfaces while leaving the background visible.
class _ThemeMoodPreview extends StatelessWidget {
  const _ThemeMoodPreview();

  @override
  Widget build(BuildContext context) {
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final scheme = Theme.of(context).colorScheme;
    final userColor = snapshot.userChatStyle == 'bubble'
        ? snapshot.bubbleUserBubbleColor
        : snapshot.cursorUserBubbleColor;
    final aiColor = snapshot.bubbleAiBubbleColor;
    return OperitThemeBackground(
      themePreferenceSnapshot: snapshot,
      muteVideo: true,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final height = constraints.maxHeight;
          final padding = (width * .08)
              .clamp(0.0, height * .14)
              .clamp(0.0, 20.0);
          final contentWidth = (width - padding * 2).clamp(0.0, width);
          final dotSize = (contentWidth / 6)
              .clamp(0.0, height * .10)
              .clamp(0.0, 10.0);
          return Stack(
            children: [
              Positioned(
                top: padding,
                right: padding,
                width: contentWidth * .46,
                height: (height * .15).clamp(0.0, 14.0),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: userColor == null
                        ? scheme.primaryContainer
                        : Color(userColor),
                    borderRadius: BorderRadius.circular(5),
                  ),
                ),
              ),
              Positioned(
                top: height * .38,
                left: padding,
                width: contentWidth * .64,
                height: (height * .22).clamp(0.0, 22.0),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color:
                        (aiColor == null
                                ? scheme.surfaceContainerHighest
                                : Color(aiColor))
                            .withValues(alpha: .88),
                    borderRadius: BorderRadius.circular(5),
                  ),
                ),
              ),
              Positioned(
                bottom: padding,
                left: padding,
                child: Row(
                  children: [
                    for (final color in [
                      scheme.primary,
                      scheme.secondary,
                      scheme.surface,
                    ])
                      Padding(
                        padding: EdgeInsets.only(right: dotSize * .5),
                        child: Container(
                          width: dotSize,
                          height: dotSize,
                          decoration: BoxDecoration(
                            color: color,
                            shape: BoxShape.circle,
                            border: Border.all(
                              color: scheme.outlineVariant.withValues(
                                alpha: .5,
                              ),
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}
