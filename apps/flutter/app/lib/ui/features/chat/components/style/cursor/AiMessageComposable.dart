// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';

import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/StreamMarkdownRendererState.dart';
import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../theme/OperitTheme.dart';
import '../bubble/BubbleSurface.dart';
import '../../part/ThinkToolsXmlNodeGrouper.dart';
import '../../../viewmodel/ChatViewModel.dart';

class AiMessageComposable extends StatefulWidget {
  const AiMessageComposable({
    super.key,
    required this.message,
    required this.isStreaming,
    required this.useBubbleStyle,
  });

  final ChatUiMessage message;
  final bool isStreaming;
  final bool useBubbleStyle;

  @override
  State<AiMessageComposable> createState() => _AiMessageComposableState();
}

class _AiMessageComposableState extends State<AiMessageComposable> {
  late StreamMarkdownRendererState _rendererState;
  late int _messageTimestamp;

  @override
  void initState() {
    super.initState();
    _messageTimestamp = widget.message.timestamp;
    _rendererState = StreamMarkdownRendererState();
  }

  @override
  void didUpdateWidget(covariant AiMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.message.timestamp != _messageTimestamp) {
      _messageTimestamp = widget.message.timestamp;
      _rendererState = StreamMarkdownRendererState();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final detailText = _detailText(widget.message, themePreferenceSnapshot);
    final nodeGrouper = ThinkToolsXmlNodeGrouper(
      showThinkingProcess: themePreferenceSnapshot.showThinkingProcess,
    );
    final useCardStyle = widget.useBubbleStyle;
    final aiBubbleColor =
        _optionalColor(themePreferenceSnapshot.bubbleAiBubbleColor) ??
        colorScheme.surfaceContainerHighest;
    final aiTextColor =
        _optionalColor(themePreferenceSnapshot.bubbleAiTextColor) ??
        colorScheme.onSurface;
    final messageFontFamily = useCardStyle
        ? operitMessageFontFamily(themePreferenceSnapshot, isUser: false)
        : null;
    final messageFontFamilyFallback = useCardStyle
        ? operitMessageFontFamilyFallback(
            themePreferenceSnapshot,
            isUser: false,
          )
        : null;
    final messageTheme = useCardStyle
        ? theme.copyWith(
            textTheme: theme.textTheme.apply(
              fontFamily: messageFontFamily,
              fontFamilyFallback: messageFontFamilyFallback,
            ),
          )
        : theme;
    final contentPadding = EdgeInsets.fromLTRB(
      themePreferenceSnapshot.bubbleAiContentPaddingLeft,
      12,
      themePreferenceSnapshot.bubbleAiContentPaddingRight,
      12,
    );
    final bubbleBorderRadius = BorderRadius.circular(
      themePreferenceSnapshot.bubbleAiRoundedCornersEnabled ? 12 : 4,
    );
    final aiBubbleImagePath = themePreferenceSnapshot.bubbleAiImageUri;
    final aiBubbleImageStyle =
        useCardStyle &&
            themePreferenceSnapshot.bubbleAiUseImage &&
            aiBubbleImagePath != null &&
            aiBubbleImagePath.isNotEmpty
        ? BubbleImageStyle(
            imagePath: aiBubbleImagePath,
            cropLeftRatio: themePreferenceSnapshot.bubbleAiImageCropLeft,
            cropTopRatio: themePreferenceSnapshot.bubbleAiImageCropTop,
            cropRightRatio: themePreferenceSnapshot.bubbleAiImageCropRight,
            cropBottomRatio: themePreferenceSnapshot.bubbleAiImageCropBottom,
            repeatXStartRatio: themePreferenceSnapshot.bubbleAiImageRepeatStart,
            repeatXEndRatio: themePreferenceSnapshot.bubbleAiImageRepeatEnd,
            repeatYStartRatio:
                themePreferenceSnapshot.bubbleAiImageRepeatYStart,
            repeatYEndRatio: themePreferenceSnapshot.bubbleAiImageRepeatYEnd,
            imageScale: themePreferenceSnapshot.bubbleAiImageScale,
            renderMode: themePreferenceSnapshot.bubbleAiImageRenderMode,
          )
        : null;
    final messageBody = Theme(
      data: messageTheme,
      child: DefaultTextStyle.merge(
        style: TextStyle(
          fontFamily: messageFontFamily,
          fontFamilyFallback: messageFontFamilyFallback,
        ),
        child: KeyedSubtree(
          key: ValueKey<int>(widget.message.timestamp),
          child: StreamMarkdownRenderer(
            content: widget.message.content,
            contentStream: widget.message.contentStream,
            isStreaming: widget.isStreaming,
            textColor: aiTextColor,
            backgroundColor: useCardStyle ? aiBubbleColor : colorScheme.surface,
            nodeGrouper: nodeGrouper,
            state: _rendererState,
            showThinkingProcess: themePreferenceSnapshot.showThinkingProcess,
          ),
        ),
      ),
    );

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (widget.useBubbleStyle &&
              themePreferenceSnapshot.bubbleShowAvatar) ...<Widget>[
            _MessageAvatar(
              icon: Icons.auto_awesome,
              imagePath: themePreferenceSnapshot.customAiAvatarUri,
              backgroundColor: aiBubbleColor,
              foregroundColor: aiTextColor,
              square:
                  themePreferenceSnapshot.avatarShape ==
                  UserPreferencesManager.AVATAR_SHAPE_SQUARE,
              cornerRadius: themePreferenceSnapshot.avatarCornerRadius,
            ),
            const SizedBox(width: 8),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
                  child: Row(
                    children: <Widget>[
                      Text(
                        'Response',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colorScheme.onSurface.withValues(alpha: 0.7),
                        ),
                      ),
                      if (detailText.isNotEmpty) ...<Widget>[
                        const Spacer(),
                        Text(
                          detailText,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurface.withValues(alpha: 0.5),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: useCardStyle
                      ? BubbleSurface(
                          color: aiBubbleColor,
                          borderRadius: bubbleBorderRadius,
                          imageStyle: aiBubbleImageStyle,
                          transparentSurface:
                              themePreferenceSnapshot.transparentSurfaceEnabled,
                          child: Padding(
                            padding: contentPadding,
                            child: messageBody,
                          ),
                        )
                      : Padding(
                          padding: EdgeInsets.only(
                            left:
                                (themePreferenceSnapshot
                                            .bubbleAiContentPaddingLeft -
                                        12)
                                    .clamp(0, double.infinity)
                                    .toDouble(),
                            right:
                                (themePreferenceSnapshot
                                            .bubbleAiContentPaddingRight -
                                        12)
                                    .clamp(0, double.infinity)
                                    .toDouble(),
                          ),
                          child: messageBody,
                        ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

Color? _optionalColor(int? value) {
  return value == null ? null : Color(value);
}

class _MessageAvatar extends StatelessWidget {
  const _MessageAvatar({
    required this.icon,
    required this.imagePath,
    required this.backgroundColor,
    required this.foregroundColor,
    required this.square,
    required this.cornerRadius,
  });

  final IconData icon;
  final String? imagePath;
  final Color backgroundColor;
  final Color foregroundColor;
  final bool square;
  final double cornerRadius;

  @override
  Widget build(BuildContext context) {
    final avatarImagePath = imagePath;
    return Padding(
      padding: const EdgeInsets.only(left: 8, top: 18),
      child: Container(
        width: 28,
        height: 28,
        decoration: BoxDecoration(
          color: backgroundColor,
          shape: square ? BoxShape.rectangle : BoxShape.circle,
          borderRadius: square ? BorderRadius.circular(cornerRadius) : null,
        ),
        clipBehavior: Clip.antiAlias,
        child: avatarImagePath != null && avatarImagePath.isNotEmpty
            ? Image.file(File(avatarImagePath), fit: BoxFit.cover)
            : Icon(icon, size: 15, color: foregroundColor),
      ),
    );
  }
}

String _detailText(
  ChatUiMessage message,
  ThemePreferenceSnapshot themePreferenceSnapshot,
) {
  final parts = <String>[];
  if (themePreferenceSnapshot.showRoleName && message.roleName.isNotEmpty) {
    parts.add(message.roleName);
  }
  if (themePreferenceSnapshot.showModelName && message.modelName.isNotEmpty) {
    parts.add(message.modelName);
  }
  if (themePreferenceSnapshot.showModelProvider &&
      message.provider.isNotEmpty) {
    parts.add(message.provider);
  }
  if (themePreferenceSnapshot.showMessageTokenStats) {
    parts.add(
      '${message.inputTokens}+${message.outputTokens}'
      '${message.cachedInputTokens > 0 ? " (${message.cachedInputTokens})" : ""} tokens',
    );
  }
  if (themePreferenceSnapshot.showMessageTimingStats) {
    parts.add(_timingText(message));
  }
  if (themePreferenceSnapshot.showMessageTimestamp) {
    parts.add(_timestampText(message));
  }
  return parts.join(' | ');
}

String _timingText(ChatUiMessage message) {
  final outputSeconds = (message.outputDurationMs / 1000).toStringAsFixed(1);
  final waitSeconds = (message.waitDurationMs / 1000).toStringAsFixed(1);
  return '${waitSeconds}s wait | ${outputSeconds}s output';
}

String _timestampText(ChatUiMessage message) {
  final rawTimestamp = message.completedAt > 0
      ? message.completedAt
      : message.timestamp;
  final dateTime = DateTime.fromMillisecondsSinceEpoch(rawTimestamp);
  String twoDigits(int value) => value.toString().padLeft(2, '0');
  return '${dateTime.year}-${twoDigits(dateTime.month)}-${twoDigits(dateTime.day)} '
      '${twoDigits(dateTime.hour)}:${twoDigits(dateTime.minute)}';
}
