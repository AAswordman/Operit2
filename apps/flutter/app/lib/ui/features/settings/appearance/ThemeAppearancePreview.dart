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
          final padding = constraints.maxWidth < 150 ? 12.0 : 20.0;
          return Padding(
            padding: EdgeInsets.all(padding),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Align(
                  alignment: Alignment.centerRight,
                  child: FractionallySizedBox(
                    widthFactor: .46,
                    child: Container(
                      height: 14,
                      decoration: BoxDecoration(
                        color: userColor == null
                            ? scheme.primaryContainer
                            : Color(userColor),
                        borderRadius: BorderRadius.circular(5),
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 8),
                FractionallySizedBox(
                  widthFactor: .64,
                  child: Container(
                    height: 22,
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
                const Spacer(),
                Row(
                  children: [
                    for (final color in [
                      scheme.primary,
                      scheme.secondary,
                      scheme.surface,
                    ])
                      Padding(
                        padding: const EdgeInsets.only(right: 5),
                        child: Container(
                          width: 10,
                          height: 10,
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
              ],
            ),
          );
        },
      ),
    );
  }
}
