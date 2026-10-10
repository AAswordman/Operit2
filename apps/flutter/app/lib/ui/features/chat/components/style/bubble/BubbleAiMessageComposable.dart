// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../../common/CharacterAvatar.dart';
import '../../../../../common/markdown/MarkdownImageRenderer.dart';
import '../../../../../common/markdown/MarkdownRemoteImage.dart';
import '../../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/StreamMarkdownRendererState.dart';
import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../part/StructuredMessagePartRenderer.dart';
import '../../part/ThinkToolsXmlNodeGrouper.dart';
import '../../part/ToolCallResultMergeRender.dart';
import '../../../viewmodel/ChatViewModel.dart';
import '../MessageHeaderMetadata.dart';
import 'BubbleSurface.dart';

class BubbleAiMessageComposable extends StatefulWidget {
  const BubbleAiMessageComposable({
    super.key,
    required this.message,
    required this.backgroundColor,
    required this.textColor,
    this.transparentSurface = false,
    this.bubbleImageStyle,
    this.bubbleRoundedCornersEnabled = true,
    this.bubbleContentPaddingLeft = 12,
    this.bubbleContentPaddingRight = 12,
    this.avatarImagePath,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.expandThinkToolsGroups = false,
    this.forceShowThinkingProcess = false,
    this.onLinkClick,
    this.isHidden = false,
    this.enableDialogs = true,
    this.onAvatarLongPressMention,
    this.onIdentityTap,
    this.splitMarkdownContent,
  });

  final ChatUiMessage message;
  final Color backgroundColor;
  final Color textColor;
  final bool transparentSurface;
  final BubbleImageStyle? bubbleImageStyle;
  final bool bubbleRoundedCornersEnabled;
  final double bubbleContentPaddingLeft;
  final double bubbleContentPaddingRight;
  final String? avatarImagePath;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final bool expandThinkToolsGroups;
  final bool forceShowThinkingProcess;
  final void Function(String url)? onLinkClick;
  final bool isHidden;
  final bool enableDialogs;
  final void Function(String roleName)? onAvatarLongPressMention;
  final VoidCallback? onIdentityTap;
  final MarkdownContentSplitter? splitMarkdownContent;

  @override
  State<BubbleAiMessageComposable> createState() =>
      _BubbleAiMessageComposableState();
}

class _BubbleAiMessageComposableState extends State<BubbleAiMessageComposable> {
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
  void didUpdateWidget(covariant BubbleAiMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    final renderIdentity = _messageRenderIdentity(widget.message);
    if (renderIdentity != _renderIdentity) {
      _renderIdentity = renderIdentity;
      _rendererState = StreamMarkdownRendererState();
    }
  }

  /// Builds the AI bubble with the active variant renderer identity.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final backgroundColor = widget.backgroundColor;
    final textColor = widget.textColor;
    final showThinkingProcess =
        widget.forceShowThinkingProcess || snapshot.showThinkingProcess;
    final roleNameText =
        snapshot.showRoleName && widget.message.roleName.isNotEmpty
        ? widget.message.roleName
        : '';
    final metadataText = _metadataText(widget.message, snapshot, l10n: l10n);
    final tooltipText = formatMessageMetadataTooltip(
      widget.message,
      snapshot,
      l10n: l10n,
    );
    final avatarImagePath = widget.avatarImagePath;
    final messageFontFamily = operitMessageFontFamily(snapshot, isUser: false);
    final messageFontFamilyFallback = operitMessageFontFamilyFallback(
      snapshot,
      isUser: false,
    );
    final messageTheme = theme.copyWith(
      textTheme: theme.textTheme.apply(
        fontFamily: messageFontFamily,
        fontFamilyFallback: messageFontFamilyFallback,
      ),
    );
    final nodeGrouper = ThinkToolsXmlNodeGrouper(
      showThinkingProcess: showThinkingProcess,
      forceExpandGroups: widget.expandThinkToolsGroups,
    );
    final effectiveBubbleImageStyle = widget.transparentSurface
        ? null
        : widget.bubbleImageStyle;
    final bubbleShape = widget.bubbleRoundedCornersEnabled
        ? const BorderRadius.all(Radius.circular(16))
        : BorderRadius.zero;
    final contentPadding = EdgeInsets.fromLTRB(
      widget.bubbleContentPaddingLeft,
      12,
      widget.bubbleContentPaddingRight,
      12,
    );
    final imageUrl = _singleMarkdownImageUrl(widget.message);
    final renderIdentity = _messageRenderIdentity(widget.message);

    final messageBody = Theme(
      data: messageTheme,
      child: DefaultTextStyle.merge(
        style: TextStyle(
          fontFamily: messageFontFamily,
          fontFamilyFallback: messageFontFamilyFallback,
        ),
        child: KeyedSubtree(
          key: ValueKey<String>(renderIdentity),
          child: StreamingStructuredMessageRenderer(
            parts: widget.message.parts,
            contentStream: widget.message.contentStream,
            textColor: textColor,
            backgroundColor: backgroundColor,
            nodeGrouper: nodeGrouper,
            mergeRender: const ToolCallResultMergeRender(),
            streamState: _rendererState,
            onLinkClick: widget.enableDialogs ? widget.onLinkClick : null,
            rendererId: 'bubble-ai-$renderIdentity',
            showThinkingProcess: showThinkingProcess,
            initialThinkingExpanded: widget.initialThinkingExpanded,
            allowExpandedThinkingFullHeight:
                widget.allowExpandedThinkingFullHeight,
            splitMarkdownContent: widget.splitMarkdownContent,
          ),
        ),
      ),
    );

    final isInteracting = _isMessageHovered || _isMessageTouched;
    final layoutChild = snapshot.bubbleWideLayoutEnabled
        ? _WideAiBubbleLayout(
            bubbleShowAvatar: snapshot.bubbleShowAvatar,
            avatarImagePath: avatarImagePath,
            avatarShape: snapshot.avatarShape,
            avatarCornerRadius: snapshot.avatarCornerRadius,
            onAvatarLongPress: _avatarLongPressCallback(),
            onIdentityTap: widget.enableDialogs ? widget.onIdentityTap : null,
            roleNameText: roleNameText,
            metadataText: metadataText,
            tooltipText: tooltipText,
            isInteracting: isInteracting,
            imageUrl: imageUrl,
            bubbleShape: bubbleShape,
            backgroundColor: backgroundColor,
            textColor: textColor,
            imageStyle: effectiveBubbleImageStyle,
            transparentSurface: widget.transparentSurface,
            contentPadding: contentPadding,
            forceExpandedWidth: _shouldUseExpandedBubbleLayout(_rendererState),
            messageBody: messageBody,
          )
        : _NormalAiBubbleLayout(
            bubbleShowAvatar: snapshot.bubbleShowAvatar,
            avatarImagePath: avatarImagePath,
            avatarShape: snapshot.avatarShape,
            avatarCornerRadius: snapshot.avatarCornerRadius,
            onAvatarLongPress: _avatarLongPressCallback(),
            onIdentityTap: widget.enableDialogs ? widget.onIdentityTap : null,
            displayText: _normalDisplayText(
              widget.message,
              snapshot,
              l10n: l10n,
            ),
            tooltipText: tooltipText,
            isInteracting: isInteracting,
            imageUrl: imageUrl,
            bubbleShape: bubbleShape,
            backgroundColor: backgroundColor,
            textColor: textColor,
            imageStyle: effectiveBubbleImageStyle,
            transparentSurface: widget.transparentSurface,
            contentPadding: contentPadding,
            forceExpandedWidth: _shouldUseExpandedBubbleLayout(_rendererState),
            messageBody: messageBody,
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
        child: _AnimatedAiBubbleVisibility(
          isHidden: widget.isHidden,
          child: layoutChild,
        ),
      ),
    );
  }

  VoidCallback? _avatarLongPressCallback() {
    final roleName = widget.message.roleName.trim();
    if (roleName.isEmpty || widget.onAvatarLongPressMention == null) {
      return null;
    }
    return () => widget.onAvatarLongPressMention!(roleName);
  }
}

/// Builds the renderer identity for one visible AI response variant.
String _messageRenderIdentity(ChatUiMessage message) {
  return '${message.timestamp}-v${message.selectedVariantIndex}';
}

class _AnimatedAiBubbleVisibility extends StatelessWidget {
  const _AnimatedAiBubbleVisibility({
    required this.isHidden,
    required this.child,
  });

  final bool isHidden;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return TweenAnimationBuilder<double>(
      tween: Tween<double>(end: isHidden ? 1 : 0),
      duration: const Duration(milliseconds: 300),
      builder: (context, hiddenValue, child) {
        return Opacity(
          opacity: 1 - hiddenValue,
          child: Transform.translate(
            offset: Offset(0, 100 * hiddenValue),
            child: child,
          ),
        );
      },
      child: child,
    );
  }
}

class _WideAiBubbleLayout extends StatelessWidget {
  const _WideAiBubbleLayout({
    required this.bubbleShowAvatar,
    required this.avatarImagePath,
    required this.avatarShape,
    required this.avatarCornerRadius,
    required this.onAvatarLongPress,
    required this.onIdentityTap,
    required this.roleNameText,
    required this.metadataText,
    required this.tooltipText,
    required this.isInteracting,
    required this.imageUrl,
    required this.bubbleShape,
    required this.backgroundColor,
    required this.textColor,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.forceExpandedWidth,
    required this.messageBody,
  });

  final bool bubbleShowAvatar;
  final String? avatarImagePath;
  final String avatarShape;
  final double avatarCornerRadius;
  final VoidCallback? onAvatarLongPress;
  final VoidCallback? onIdentityTap;
  final String roleNameText;
  final String metadataText;
  final String tooltipText;
  final bool isInteracting;
  final String? imageUrl;
  final BorderRadius bubbleShape;
  final Color backgroundColor;
  final Color textColor;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool forceExpandedWidth;
  final Widget messageBody;

  @override
  Widget build(BuildContext context) {
    final headerVisible = bubbleShowAvatar || roleNameText.isNotEmpty;
    return Padding(
      padding: EdgeInsets.fromLTRB(bubbleShowAvatar ? 0 : 8, 4, 0, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          if (headerVisible) ...<Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                if (bubbleShowAvatar) ...<Widget>[
                  _MessageAvatar(
                    imagePath: avatarImagePath,
                    avatarShape: avatarShape,
                    cornerRadius: avatarCornerRadius,
                    onLongPress: onAvatarLongPress,
                    onTap: onIdentityTap,
                  ),
                  const SizedBox(width: 8),
                ],
                Flexible(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      if (roleNameText.isNotEmpty)
                        InkWell(
                          onTap: onIdentityTap,
                          child: Text(
                            roleNameText,
                            softWrap: true,
                            style: Theme.of(context).textTheme.titleSmall
                                ?.copyWith(
                                  fontWeight: FontWeight.w600,
                                  color: Theme.of(
                                    context,
                                  ).colorScheme.onSurface,
                                ),
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
          ],
          LayoutBuilder(
            builder: (context, constraints) {
              if (imageUrl != null) {
                return _AiImageOnlyBubble(
                  imageUrl: imageUrl!,
                  maxWidth: constraints.maxWidth,
                );
              }
              return ConstrainedBox(
                constraints: BoxConstraints(
                  maxWidth: constraints.maxWidth,
                  minHeight: 44,
                ),
                child: _AiBubbleBody(
                  backgroundColor: backgroundColor,
                  bubbleShape: bubbleShape,
                  imageStyle: imageStyle,
                  transparentSurface: transparentSurface,
                  contentPadding: contentPadding,
                  forceExpandedWidth: forceExpandedWidth,
                  messageBody: messageBody,
                ),
              );
            },
          ),
          if (metadataText.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 4, left: 4),
              child: _BubbleMetadataLabel(
                text: metadataText,
                tooltip: tooltipText,
                externalActive: isInteracting,
              ),
            ),
        ],
      ),
    );
  }
}

class _NormalAiBubbleLayout extends StatelessWidget {
  const _NormalAiBubbleLayout({
    required this.bubbleShowAvatar,
    required this.avatarImagePath,
    required this.avatarShape,
    required this.avatarCornerRadius,
    required this.onAvatarLongPress,
    required this.onIdentityTap,
    required this.displayText,
    required this.tooltipText,
    required this.isInteracting,
    required this.imageUrl,
    required this.bubbleShape,
    required this.backgroundColor,
    required this.textColor,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.forceExpandedWidth,
    required this.messageBody,
  });

  final bool bubbleShowAvatar;
  final String? avatarImagePath;
  final String avatarShape;
  final double avatarCornerRadius;
  final VoidCallback? onAvatarLongPress;
  final VoidCallback? onIdentityTap;
  final String displayText;
  final String tooltipText;
  final bool isInteracting;
  final String? imageUrl;
  final BorderRadius bubbleShape;
  final Color backgroundColor;
  final Color textColor;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool forceExpandedWidth;
  final Widget messageBody;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.start,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (bubbleShowAvatar) ...<Widget>[
            _MessageAvatar(
              imagePath: avatarImagePath,
              avatarShape: avatarShape,
              cornerRadius: avatarCornerRadius,
              onLongPress: onAvatarLongPress,
              onTap: onIdentityTap,
            ),
            const SizedBox(width: 8),
          ],
          Expanded(
            child: Padding(
              padding: EdgeInsets.only(left: bubbleShowAvatar ? 0 : 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  LayoutBuilder(
                    builder: (context, constraints) {
                      final maxBubbleWidth = constraints.maxWidth * 0.85;
                      if (imageUrl != null) {
                        return _AiImageOnlyBubble(
                          imageUrl: imageUrl!,
                          maxWidth: maxBubbleWidth,
                        );
                      }
                      return Align(
                        alignment: Alignment.centerLeft,
                        child: ConstrainedBox(
                          constraints: BoxConstraints(
                            maxWidth: maxBubbleWidth,
                            minHeight: 44,
                          ),
                          child: _AiBubbleBody(
                            backgroundColor: backgroundColor,
                            bubbleShape: bubbleShape,
                            imageStyle: imageStyle,
                            transparentSurface: transparentSurface,
                            contentPadding: contentPadding,
                            forceExpandedWidth: forceExpandedWidth,
                            messageBody: messageBody,
                          ),
                        ),
                      );
                    },
                  ),
                  if (displayText.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(top: 4, left: 4),
                      child: InkWell(
                        onTap: onIdentityTap,
                        child: _BubbleMetadataLabel(
                          text: displayText,
                          tooltip: tooltipText,
                          externalActive: isInteracting,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _AiBubbleBody extends StatelessWidget {
  const _AiBubbleBody({
    required this.backgroundColor,
    required this.bubbleShape,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.forceExpandedWidth,
    required this.messageBody,
  });

  final Color backgroundColor;
  final BorderRadius bubbleShape;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool forceExpandedWidth;
  final Widget messageBody;

  @override
  Widget build(BuildContext context) {
    final body = Padding(
      padding: contentPadding,
      // Kotlin changes the width modifier without replacing the content owner.
      // Keep the same Flutter element path when parsed XML requests full width.
      child: SizedBox(
        width: forceExpandedWidth ? double.infinity : null,
        child: messageBody,
      ),
    );
    return BubbleSurface(
      color: backgroundColor,
      borderRadius: bubbleShape,
      imageStyle: imageStyle,
      transparentSurface: transparentSurface,
      child: body,
    );
  }
}

class _AiImageOnlyBubble extends StatelessWidget {
  const _AiImageOnlyBubble({required this.imageUrl, required this.maxWidth});

  final String imageUrl;
  final double maxWidth;

  @override
  Widget build(BuildContext context) {
    final uri = Uri.tryParse(imageUrl);
    final Widget image;
    if (uri == null) {
      image = const Icon(Icons.broken_image_outlined);
    } else {
      image = switch (uri.scheme) {
        'http' ||
        'https' ||
        'data' => MarkdownRemoteImage(url: imageUrl, fit: BoxFit.contain),
        'file' => Image.file(
          File(uri.toFilePath()),
          fit: BoxFit.contain,
          errorBuilder: (context, error, stackTrace) =>
              const Icon(Icons.broken_image_outlined),
        ),
        _ => const Icon(Icons.broken_image_outlined),
      };
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: maxWidth, maxHeight: 320),
        child: image,
      ),
    );
  }
}

class _MessageAvatar extends StatelessWidget {
  /// Creates an AI message avatar that can render an imported theme asset.
  const _MessageAvatar({
    required this.imagePath,
    required this.avatarShape,
    required this.cornerRadius,
    this.onLongPress,
    this.onTap,
  });

  final String? imagePath;
  final String avatarShape;
  final double cornerRadius;
  final VoidCallback? onLongPress;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final square = avatarShape == UserPreferencesManager.AVATAR_SHAPE_SQUARE;
    return GestureDetector(
      onLongPress: onLongPress,
      onTap: onTap,
      child: Container(
        width: 32,
        height: 32,
        decoration: BoxDecoration(
          shape: square ? BoxShape.rectangle : BoxShape.circle,
          borderRadius: square ? BorderRadius.circular(cornerRadius) : null,
        ),
        clipBehavior: Clip.antiAlias,
        child: CharacterAvatarImage(avatarUri: imagePath, fit: BoxFit.cover),
      ),
    );
  }
}

class _BubbleMetadataLabel extends StatelessWidget {
  const _BubbleMetadataLabel({
    required this.text,
    required this.tooltip,
    this.externalActive = false,
  });

  final String text;
  final String tooltip;
  final bool externalActive;

  @override
  Widget build(BuildContext context) {
    final label = Text(
      text,
      softWrap: true,
      style: Theme.of(context).textTheme.labelSmall?.copyWith(
        color: Theme.of(context).colorScheme.onSurfaceVariant,
        fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
      ),
    );
    return MouseFollowingTooltip(
      message: tooltip,
      externalActive: externalActive,
      child: label,
    );
  }
}

String _metadataText(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot, {
  AppLocalizations? l10n,
}) {
  final parts = <String>[];
  final modelProvider = formatModelProviderLabel(message, snapshot);
  if (modelProvider.isNotEmpty) {
    parts.add(modelProvider);
  }
  final stats = formatMessageStatsText(message, snapshot, l10n: l10n);
  if (stats.isNotEmpty) {
    parts.add(stats);
  }
  return parts.join(' · ');
}

String _normalDisplayText(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot, {
  AppLocalizations? l10n,
}) {
  final parts = <String>[];
  if (snapshot.showRoleName && message.roleName.isNotEmpty) {
    parts.add(message.roleName);
  }
  final modelProvider = formatModelProviderLabel(message, snapshot);
  if (modelProvider.isNotEmpty) {
    parts.add(modelProvider);
  }
  final stats = formatMessageStatsText(message, snapshot, l10n: l10n);
  if (stats.isNotEmpty) {
    parts.add(stats);
  }
  return parts.join(' · ');
}

String? _singleMarkdownImageUrl(ChatUiMessage message) {
  final text = message.displayText.trim();
  if (!isCompleteImageMarkdown(text)) {
    return null;
  }
  final url = extractMarkdownImageUrl(text);
  if (url.isEmpty) {
    return null;
  }
  return url;
}

bool _shouldUseExpandedBubbleLayout(StreamMarkdownRendererState state) {
  return state.renderNodes.any(
    (node) => const <MarkdownNodeType>{
      MarkdownNodeType.codeBlock,
      MarkdownNodeType.table,
      MarkdownNodeType.xmlBlock,
      MarkdownNodeType.image,
      MarkdownNodeType.blockLatex,
    }.contains(node.type),
  );
}
