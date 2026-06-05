// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';

import '../../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/StreamMarkdownRendererState.dart';
import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../part/ThinkToolsXmlNodeGrouper.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'BubbleSurface.dart';

class BubbleAiMessageComposable extends StatefulWidget {
  const BubbleAiMessageComposable({
    super.key,
    required this.message,
    required this.isStreaming,
    required this.backgroundColor,
    required this.textColor,
    this.transparentSurface = false,
    this.bubbleImageStyle,
    this.bubbleRoundedCornersEnabled = true,
    this.bubbleContentPaddingLeft = 12,
    this.bubbleContentPaddingRight = 12,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.expandThinkToolsGroups = false,
    this.forceShowThinkingProcess = false,
    this.onLinkClick,
    this.isHidden = false,
    this.enableDialogs = true,
    this.onAvatarLongPressMention,
  });

  final ChatUiMessage message;
  final bool isStreaming;
  final Color backgroundColor;
  final Color textColor;
  final bool transparentSurface;
  final BubbleImageStyle? bubbleImageStyle;
  final bool bubbleRoundedCornersEnabled;
  final double bubbleContentPaddingLeft;
  final double bubbleContentPaddingRight;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final bool expandThinkToolsGroups;
  final bool forceShowThinkingProcess;
  final void Function(String url)? onLinkClick;
  final bool isHidden;
  final bool enableDialogs;
  final void Function(String roleName)? onAvatarLongPressMention;

  @override
  State<BubbleAiMessageComposable> createState() =>
      _BubbleAiMessageComposableState();
}

class _BubbleAiMessageComposableState extends State<BubbleAiMessageComposable> {
  late StreamMarkdownRendererState _rendererState;
  late int _messageTimestamp;

  @override
  void initState() {
    super.initState();
    _messageTimestamp = widget.message.timestamp;
    _rendererState = StreamMarkdownRendererState();
  }

  @override
  void didUpdateWidget(covariant BubbleAiMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.message.timestamp != _messageTimestamp) {
      _messageTimestamp = widget.message.timestamp;
      _rendererState = StreamMarkdownRendererState();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final backgroundColor = widget.backgroundColor;
    final textColor = widget.textColor;
    final showThinkingProcess =
        widget.forceShowThinkingProcess || snapshot.showThinkingProcess;
    final roleNameText =
        snapshot.showRoleName && widget.message.roleName.isNotEmpty
        ? widget.message.roleName
        : '';
    final metadataText = _metadataText(widget.message, snapshot);
    final avatarImagePath = snapshot.customAiAvatarUri;
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
        ? const BorderRadius.only(
            topLeft: Radius.circular(4),
            topRight: Radius.circular(20),
            bottomRight: Radius.circular(20),
            bottomLeft: Radius.circular(20),
          )
        : BorderRadius.zero;
    final contentPadding = EdgeInsets.fromLTRB(
      widget.bubbleContentPaddingLeft,
      12,
      widget.bubbleContentPaddingRight,
      12,
    );
    final imageUrl = _singleMarkdownImageUrl(widget.message);

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
            textColor: textColor,
            backgroundColor: backgroundColor,
            nodeGrouper: nodeGrouper,
            state: _rendererState,
            onLinkClick: widget.enableDialogs ? widget.onLinkClick : null,
            rendererId: 'bubble-ai-${widget.message.timestamp}',
            showThinkingProcess: showThinkingProcess,
            initialThinkingExpanded: widget.initialThinkingExpanded,
            allowExpandedThinkingFullHeight:
                widget.allowExpandedThinkingFullHeight,
          ),
        ),
      ),
    );

    if (snapshot.bubbleWideLayoutEnabled) {
      return _AnimatedAiBubbleVisibility(
        isHidden: widget.isHidden,
        child: _WideAiBubbleLayout(
          bubbleShowAvatar: snapshot.bubbleShowAvatar,
          avatarImagePath: avatarImagePath,
          avatarShape: snapshot.avatarShape,
          avatarCornerRadius: snapshot.avatarCornerRadius,
          onAvatarLongPress: _avatarLongPressCallback(),
          roleNameText: roleNameText,
          metadataText: metadataText,
          imageUrl: imageUrl,
          bubbleShape: bubbleShape,
          backgroundColor: backgroundColor,
          textColor: textColor,
          imageStyle: effectiveBubbleImageStyle,
          transparentSurface: widget.transparentSurface,
          contentPadding: contentPadding,
          forceExpandedWidth: _shouldUseExpandedBubbleLayout(_rendererState),
          messageBody: messageBody,
        ),
      );
    }

    return _AnimatedAiBubbleVisibility(
      isHidden: widget.isHidden,
      child: _NormalAiBubbleLayout(
        bubbleShowAvatar: snapshot.bubbleShowAvatar,
        avatarImagePath: avatarImagePath,
        avatarShape: snapshot.avatarShape,
        avatarCornerRadius: snapshot.avatarCornerRadius,
        onAvatarLongPress: _avatarLongPressCallback(),
        displayText: _normalDisplayText(widget.message, snapshot),
        imageUrl: imageUrl,
        bubbleShape: bubbleShape,
        backgroundColor: backgroundColor,
        textColor: textColor,
        imageStyle: effectiveBubbleImageStyle,
        transparentSurface: widget.transparentSurface,
        contentPadding: contentPadding,
        forceExpandedWidth: _shouldUseExpandedBubbleLayout(_rendererState),
        messageBody: messageBody,
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
    required this.roleNameText,
    required this.metadataText,
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
  final String roleNameText;
  final String metadataText;
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
    final headerVisible =
        bubbleShowAvatar || roleNameText.isNotEmpty || metadataText.isNotEmpty;
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
                  ),
                  const SizedBox(width: 8),
                ],
                Flexible(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      if (roleNameText.isNotEmpty)
                        Text(
                          roleNameText,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.titleSmall
                              ?.copyWith(
                                fontWeight: FontWeight.w600,
                                color: Theme.of(context).colorScheme.onSurface,
                              ),
                        ),
                      if (metadataText.isNotEmpty)
                        Text(
                          metadataText,
                          style: Theme.of(context).textTheme.labelSmall
                              ?.copyWith(
                                color: Theme.of(
                                  context,
                                ).colorScheme.onSurfaceVariant,
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
    required this.displayText,
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
  final String displayText;
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
            ),
            const SizedBox(width: 8),
          ],
          Expanded(
            child: Padding(
              padding: EdgeInsets.only(left: bubbleShowAvatar ? 0 : 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  if (displayText.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 4, left: 4),
                      child: Text(
                        displayText,
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
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
      child: forceExpandedWidth
          ? SizedBox(width: double.infinity, child: messageBody)
          : messageBody,
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
    final uri = Uri.parse(imageUrl);
    final image = switch (uri.scheme) {
      'http' || 'https' => Image.network(imageUrl, fit: BoxFit.contain),
      'file' => Image.file(File(uri.toFilePath()), fit: BoxFit.contain),
      _ => Image.file(File(imageUrl), fit: BoxFit.contain),
    };
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: maxWidth, maxHeight: 80),
        child: image,
      ),
    );
  }
}

class _MessageAvatar extends StatelessWidget {
  const _MessageAvatar({
    required this.imagePath,
    required this.avatarShape,
    required this.cornerRadius,
    this.onLongPress,
  });

  final String? imagePath;
  final String avatarShape;
  final double cornerRadius;
  final VoidCallback? onLongPress;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final square = avatarShape == UserPreferencesManager.AVATAR_SHAPE_SQUARE;
    final avatarImagePath = imagePath;
    return GestureDetector(
      onLongPress: onLongPress,
      child: Container(
        width: 32,
        height: 32,
        decoration: BoxDecoration(
          shape: square ? BoxShape.rectangle : BoxShape.circle,
          borderRadius: square ? BorderRadius.circular(cornerRadius) : null,
        ),
        clipBehavior: Clip.antiAlias,
        child: avatarImagePath != null && avatarImagePath.isNotEmpty
            ? Image.file(File(avatarImagePath), fit: BoxFit.cover)
            : Icon(Icons.assistant, color: colorScheme.secondary, size: 22),
      ),
    );
  }
}

String _metadataText(ChatUiMessage message, ThemePreferenceSnapshot snapshot) {
  final buffer = StringBuffer();
  if (snapshot.showModelName && message.modelName.isNotEmpty) {
    buffer.write(message.modelName);
  }
  if (snapshot.showModelProvider && message.provider.isNotEmpty) {
    if (snapshot.showModelName && message.modelName.isNotEmpty) {
      buffer.write(' by ');
    } else if (buffer.isNotEmpty) {
      buffer.write(' | ');
    }
    buffer.write(message.provider);
  }
  return buffer.toString();
}

String _normalDisplayText(
  ChatUiMessage message,
  ThemePreferenceSnapshot snapshot,
) {
  final buffer = StringBuffer();
  if (snapshot.showRoleName && message.roleName.isNotEmpty) {
    buffer.write(message.roleName);
  }
  if (snapshot.showModelName && message.modelName.isNotEmpty) {
    if (buffer.isNotEmpty) {
      buffer.write(' | ');
    }
    buffer.write(message.modelName);
  }
  if (snapshot.showModelProvider && message.provider.isNotEmpty) {
    if (snapshot.showModelName && message.modelName.isNotEmpty) {
      buffer.write(' by ');
    } else if (buffer.isNotEmpty) {
      buffer.write(' | ');
    }
    buffer.write(message.provider);
  }
  return buffer.toString();
}

String? _singleMarkdownImageUrl(ChatUiMessage message) {
  if (message.contentStream != null) {
    return null;
  }
  return RegExp(
    r'^\s*!\[[^\]]*\]\(([^)]+)\)\s*$',
  ).firstMatch(message.content)?.group(1);
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
