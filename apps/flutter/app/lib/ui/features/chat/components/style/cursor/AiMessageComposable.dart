// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/StreamMarkdownRendererState.dart';
import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../theme/OperitTheme.dart';
import '../MessageHeaderMetadata.dart';
import '../../part/StructuredMessagePartRenderer.dart';
import '../../part/ThinkToolsXmlNodeGrouper.dart';
import '../../part/ToolCallResultMergeRender.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'CursorMessageLayout.dart';

class AiMessageComposable extends StatefulWidget {
  const AiMessageComposable({
    super.key,
    required this.message,
    this.avatarImagePath,
    this.onIdentityTap,
    this.splitMarkdownContent,
  });

  final ChatUiMessage message;
  final String? avatarImagePath;
  final VoidCallback? onIdentityTap;
  final MarkdownContentSplitter? splitMarkdownContent;

  @override
  State<AiMessageComposable> createState() => _AiMessageComposableState();
}

class _AiMessageComposableState extends State<AiMessageComposable> {
  static const Duration _touchHoldDuration = Duration(milliseconds: 1500);

  late StreamMarkdownRendererState _rendererState;
  late String _renderIdentity;
  bool _isMessageHovered = false;
  bool _isMessageTouched = false;
  Timer? _touchHoldTimer;

  /// Creates persistent Markdown renderer state for this message widget.
  @override
  void initState() {
    super.initState();
    _renderIdentity = _messageRenderIdentity(widget.message);
    _rendererState = StreamMarkdownRendererState();
  }

  @override
  void dispose() {
    _touchHoldTimer?.cancel();
    super.dispose();
  }

  void _handlePointerEnd() {
    _touchHoldTimer?.cancel();
    _touchHoldTimer = Timer(_touchHoldDuration, () {
      if (!mounted) {
        return;
      }
      setState(() {
        _isMessageTouched = false;
      });
    });
  }

  /// Resets renderer state when the visible message variant changes.
  @override
  void didUpdateWidget(covariant AiMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    final renderIdentity = _messageRenderIdentity(widget.message);
    if (renderIdentity != _renderIdentity) {
      _renderIdentity = renderIdentity;
      _rendererState = StreamMarkdownRendererState();
    }
  }

  /// Builds either the live event renderer or the persisted part renderer.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final colorScheme = Theme.of(context).colorScheme;
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final nodeGrouper = ThinkToolsXmlNodeGrouper(
      showThinkingProcess: themePreferenceSnapshot.showThinkingProcess,
    );
    final aiTextColor =
        _optionalColor(themePreferenceSnapshot.bubbleAiTextColor) ??
        colorScheme.onSurface;
    final renderIdentity = _messageRenderIdentity(widget.message);
    final messageBody = KeyedSubtree(
      key: ValueKey<String>(renderIdentity),
      child: StreamingStructuredMessageRenderer(
        parts: widget.message.parts,
        contentStream: widget.message.contentStream,
        textColor: aiTextColor,
        backgroundColor: colorScheme.surface,
        nodeGrouper: nodeGrouper,
        mergeRender: const ToolCallResultMergeRender(),
        streamState: _rendererState,
        rendererId: 'cursor-ai-$renderIdentity',
        showThinkingProcess: themePreferenceSnapshot.showThinkingProcess,
        splitMarkdownContent: widget.splitMarkdownContent,
      ),
    );

    return MouseRegion(
      onEnter: (_) {
        if (!_isMessageHovered) {
          setState(() {
            _isMessageHovered = true;
          });
        }
      },
      onExit: (_) {
        if (_isMessageHovered) {
          setState(() {
            _isMessageHovered = false;
          });
        }
      },
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (_) {
          _touchHoldTimer?.cancel();
          if (!_isMessageTouched) {
            setState(() {
              _isMessageTouched = true;
            });
          }
        },
        onPointerUp: (_) => _handlePointerEnd(),
        onPointerCancel: (_) => _handlePointerEnd(),
        child: CursorMessageLayout(
          verticalPadding: 2,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Padding(
                padding: EdgeInsets.only(
                  left:
                      (themePreferenceSnapshot.bubbleAiContentPaddingLeft - 12)
                          .clamp(0, double.infinity)
                          .toDouble(),
                  right:
                      (themePreferenceSnapshot.bubbleAiContentPaddingRight - 12)
                          .clamp(0, double.infinity)
                          .toDouble(),
                ),
                child: messageBody,
              ),
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: _CursorAiMessageHeader(
                  message: widget.message,
                  onTap: widget.onIdentityTap,
                  snapshot: themePreferenceSnapshot,
                  l10n: l10n,
                  externalActive: _isMessageHovered || _isMessageTouched,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Builds the renderer identity for one visible AI response variant.
String _messageRenderIdentity(ChatUiMessage message) {
  return '${message.timestamp}-v${message.selectedVariantIndex}';
}

Color? _optionalColor(int? value) {
  return value == null ? null : Color(value);
}

class _CursorAiMessageHeader extends StatelessWidget {
  const _CursorAiMessageHeader({
    required this.message,
    required this.snapshot,
    this.l10n,
    this.externalActive = false,
    this.onTap,
  });

  final ChatUiMessage message;
  final ThemePreferenceSnapshot snapshot;
  final AppLocalizations? l10n;
  final bool externalActive;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final primaryTitle = cursorAiPrimaryTitle(message, snapshot, l10n: l10n);
    final modelLabel = formatModelProviderLabel(message, snapshot);
    final statsText = formatMessageStatsText(message, snapshot, l10n: l10n);
    final tooltipText = formatMessageMetadataTooltip(
      message,
      snapshot,
      l10n: l10n,
    );

    final headerRow = LayoutBuilder(
      builder: (context, constraints) {
        final leftMaxWidth = statsText.isNotEmpty
            ? constraints.maxWidth * 0.48
            : constraints.maxWidth;
        return Row(
          children: <Widget>[
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: leftMaxWidth),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Flexible(
                    child: InkWell(
                      onTap: onTap,
                      child: Text(
                        primaryTitle,
                        softWrap: true,
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                          color: colorScheme.onSurface.withValues(alpha: 0.85),
                        ),
                      ),
                    ),
                  ),
                  if (modelLabel.isNotEmpty) ...<Widget>[
                    const SizedBox(width: 6),
                    Flexible(
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 5,
                          vertical: 1,
                        ),
                        decoration: BoxDecoration(
                          color: colorScheme.onSurface.withValues(alpha: 0.06),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          modelLabel,
                          softWrap: true,
                          style: theme.textTheme.labelSmall?.copyWith(
                            fontSize: 10,
                            color: colorScheme.onSurface.withValues(
                              alpha: 0.72,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
            if (statsText.isNotEmpty) ...<Widget>[
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  statsText,
                  textAlign: TextAlign.end,
                  softWrap: true,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurface.withValues(alpha: 0.75),
                    fontFeatures: const <FontFeature>[
                      FontFeature.tabularFigures(),
                    ],
                  ),
                ),
              ),
            ],
          ],
        );
      },
    );

    return MouseFollowingTooltip(
      message: tooltipText,
      externalActive: externalActive,
      child: headerRow,
    );
  }
}
