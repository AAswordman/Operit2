// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../../util/ChatMarkupRegex.dart';
import '../../../../../common/ChatAvatarImage.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../../../../theme/OperitThemeAssets.dart';
import '../../attachments/ChatMessageImageAttachments.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'BubbleSurface.dart';

export '../../attachments/ChatMessageImageAttachments.dart'
    show AttachmentData, ChatMessageAttachments, ChatMessageImageAttachments;

class BubbleUserMessageComposable extends StatefulWidget {
  const BubbleUserMessageComposable({
    super.key,
    required this.message,
    required this.backgroundColor,
    required this.textColor,
    this.transparentSurface = false,
    this.bubbleImageStyle,
    this.bubbleRoundedCornersEnabled = true,
    this.bubbleContentPaddingLeft = 12,
    this.bubbleContentPaddingRight = 12,
    this.proxyAvatarImagePath,
    this.enableDialogs = true,
  });

  final ChatUiMessage message;
  final Color backgroundColor;
  final Color textColor;
  final bool transparentSurface;
  final BubbleImageStyle? bubbleImageStyle;
  final bool bubbleRoundedCornersEnabled;
  final double bubbleContentPaddingLeft;
  final double bubbleContentPaddingRight;
  final String? proxyAvatarImagePath;
  final bool enableDialogs;

  @override
  State<BubbleUserMessageComposable> createState() =>
      _BubbleUserMessageComposableState();
}

class _BubbleUserMessageComposableState
    extends State<BubbleUserMessageComposable> {
  @override
  Widget build(BuildContext context) {
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final isHiddenPlaceholder =
        widget.message.sender == 'user' &&
        widget.message.displayMode.value == 'HIDDEN_PLACEHOLDER';
    final parseResult = isHiddenPlaceholder
        ? const MessageParseResult(processedText: '', trailingAttachments: [])
        : parseMessageContent(widget.message.displayText);
    final isProxySender =
        parseResult.proxySenderName != null &&
        parseResult.proxySenderName!.isNotEmpty;
    final backgroundColor = isHiddenPlaceholder
        ? Colors.transparent
        : widget.backgroundColor;
    final textColor = widget.textColor;
    final shouldShowAvatar = !isHiddenPlaceholder && snapshot.bubbleShowAvatar;
    final resolvedDisplayName = isProxySender
        ? parseResult.proxySenderName
        : null;
    final shouldShowResolvedName =
        !isHiddenPlaceholder &&
        (isProxySender || snapshot.showUserName) &&
        resolvedDisplayName != null &&
        resolvedDisplayName.isNotEmpty;
    final messageFontFamily = operitMessageFontFamily(snapshot, isUser: true);
    final messageFontFamilyFallback = operitMessageFontFamilyFallback(
      snapshot,
      isUser: true,
    );
    final effectiveBubbleImageStyle =
        isHiddenPlaceholder ? null : widget.bubbleImageStyle;
    final bubbleShape = widget.bubbleRoundedCornersEnabled
        ? const BorderRadius.all(Radius.circular(16))
        : BorderRadius.zero;
    final contentPadding = EdgeInsets.fromLTRB(
      widget.bubbleContentPaddingLeft,
      isHiddenPlaceholder ? 0 : 12,
      widget.bubbleContentPaddingRight,
      isHiddenPlaceholder ? 0 : 12,
    );

    return DefaultTextStyle.merge(
      style: TextStyle(
        fontFamily: messageFontFamily,
        fontFamilyFallback: messageFontFamilyFallback,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          if (snapshot.bubbleWideLayoutEnabled)
            _WideUserBubbleLayout(
              parseResult: parseResult,
              enableDialogs: widget.enableDialogs,
              shouldShowAvatar: shouldShowAvatar,
              shouldShowResolvedName: shouldShowResolvedName,
              resolvedDisplayName: resolvedDisplayName,
              isProxySender: isProxySender,
              avatarImagePath: isProxySender
                  ? widget.proxyAvatarImagePath
                  : snapshot.customUserAvatarUri,
              avatarShape: snapshot.avatarShape,
              avatarCornerRadius: snapshot.avatarCornerRadius,
              bubbleShape: bubbleShape,
              backgroundColor: backgroundColor,
              textColor: textColor,
              imageStyle: effectiveBubbleImageStyle,
              transparentSurface:
                  !isHiddenPlaceholder && widget.transparentSurface,
              contentPadding: contentPadding,
              isHiddenPlaceholder: isHiddenPlaceholder,
              textContent: parseResult.processedText,
            )
          else
            _NormalUserBubbleLayout(
              parseResult: parseResult,
              enableDialogs: widget.enableDialogs,
              shouldShowAvatar: shouldShowAvatar,
              shouldShowResolvedName: shouldShowResolvedName,
              resolvedDisplayName: resolvedDisplayName,
              isProxySender: isProxySender,
              avatarImagePath: isProxySender
                  ? widget.proxyAvatarImagePath
                  : snapshot.customUserAvatarUri,
              avatarShape: snapshot.avatarShape,
              avatarCornerRadius: snapshot.avatarCornerRadius,
              bubbleShape: bubbleShape,
              backgroundColor: backgroundColor,
              textColor: textColor,
              imageStyle: effectiveBubbleImageStyle,
              transparentSurface:
                  !isHiddenPlaceholder && widget.transparentSurface,
              contentPadding: contentPadding,
              isHiddenPlaceholder: isHiddenPlaceholder,
              textContent: parseResult.processedText,
            ),
        ],
      ),
    );
  }
}

class _UserBubbleWithAttachments extends StatefulWidget {
  const _UserBubbleWithAttachments({
    required this.parseResult,
    required this.enableDialogs,
    required this.maxBubbleWidth,
    required this.backgroundColor,
    required this.textColor,
    required this.bubbleShape,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.isHiddenPlaceholder,
    required this.textContent,
  });

  final MessageParseResult parseResult;
  final bool enableDialogs;
  final double maxBubbleWidth;
  final Color backgroundColor;
  final Color textColor;
  final BorderRadius bubbleShape;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool isHiddenPlaceholder;
  final String textContent;

  @override
  State<_UserBubbleWithAttachments> createState() =>
      _UserBubbleWithAttachmentsState();
}

class _UserBubbleWithAttachmentsState
    extends State<_UserBubbleWithAttachments> {
  final GlobalKey _bubbleKey = GlobalKey();
  double? _measuredBubbleWidth;

  @override
  void initState() {
    super.initState();
    _scheduleMeasure();
  }

  @override
  void didUpdateWidget(covariant _UserBubbleWithAttachments oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.textContent != widget.textContent ||
        oldWidget.maxBubbleWidth != widget.maxBubbleWidth ||
        oldWidget.parseResult != widget.parseResult) {
      _scheduleMeasure();
    }
  }

  void _scheduleMeasure() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final renderBox =
          _bubbleKey.currentContext?.findRenderObject() as RenderBox?;
      if (renderBox != null && renderBox.hasSize) {
        final width = renderBox.size.width;
        if (_measuredBubbleWidth == null ||
            (_measuredBubbleWidth! - width).abs() > 0.5) {
          setState(() {
            _measuredBubbleWidth = width;
          });
        }
      }
    });
  }

  double? _calculateInitialBubbleWidth(BuildContext context) {
    if (widget.isHiddenPlaceholder) {
      return 220.0;
    }
    if (widget.textContent.isEmpty) {
      return 44.0;
    }
    final textPainter = TextPainter(
      text: TextSpan(
        text: widget.textContent,
        style: Theme.of(context)
            .textTheme
            .bodyMedium
            ?.copyWith(color: widget.textColor),
      ),
      textDirection: Directionality.of(context),
    )..layout(
        maxWidth: (widget.maxBubbleWidth - widget.contentPadding.horizontal)
            .clamp(0.0, double.infinity),
      );
    return (textPainter.width + widget.contentPadding.horizontal)
        .clamp(44.0, widget.maxBubbleWidth);
  }

  @override
  Widget build(BuildContext context) {
    final effectiveBubbleWidth =
        _measuredBubbleWidth ?? _calculateInitialBubbleWidth(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (widget.parseResult.replyInfo != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: _ReplyInfoView(replyInfo: widget.parseResult.replyInfo!),
          ),
        if (widget.parseResult.attachments.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 6),
            child: ChatMessageAttachments(
              attachments: widget.parseResult.attachments,
              enableDialogs: widget.enableDialogs,
              bubbleWidth: effectiveBubbleWidth,
              isBubbleMode: true,
            ),
          ),
        ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: widget.maxBubbleWidth,
            minHeight: 44,
          ),
          child: Container(
            key: _bubbleKey,
            child: _UserBubbleBody(
              backgroundColor: widget.backgroundColor,
              textColor: widget.textColor,
              bubbleShape: widget.bubbleShape,
              imageStyle: widget.imageStyle,
              transparentSurface: widget.transparentSurface,
              contentPadding: widget.contentPadding,
              isHiddenPlaceholder: widget.isHiddenPlaceholder,
              textContent: widget.textContent,
            ),
          ),
        ),
      ],
    );
  }
}

class _WideUserBubbleLayout extends StatelessWidget {
  const _WideUserBubbleLayout({
    required this.parseResult,
    required this.enableDialogs,
    required this.shouldShowAvatar,
    required this.shouldShowResolvedName,
    required this.resolvedDisplayName,
    required this.isProxySender,
    required this.avatarImagePath,
    required this.avatarShape,
    required this.avatarCornerRadius,
    required this.bubbleShape,
    required this.backgroundColor,
    required this.textColor,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.isHiddenPlaceholder,
    required this.textContent,
  });

  final MessageParseResult parseResult;
  final bool enableDialogs;
  final bool shouldShowAvatar;
  final bool shouldShowResolvedName;
  final String? resolvedDisplayName;
  final bool isProxySender;
  final String? avatarImagePath;
  final String avatarShape;
  final double avatarCornerRadius;
  final BorderRadius bubbleShape;
  final Color backgroundColor;
  final Color textColor;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool isHiddenPlaceholder;
  final String textContent;

  @override
  Widget build(BuildContext context) {
    final headerVisible = shouldShowAvatar || shouldShowResolvedName;
    return Padding(
      padding: EdgeInsets.symmetric(vertical: isHiddenPlaceholder ? 0 : 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          if (headerVisible) ...<Widget>[
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                if (shouldShowResolvedName)
                  Flexible(
                    child: Text(
                      resolvedDisplayName!,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                        color: isProxySender
                            ? Theme.of(context).colorScheme.primary
                            : Theme.of(context).colorScheme.onSurface,
                      ),
                    ),
                  ),
                if (shouldShowAvatar && shouldShowResolvedName)
                  const SizedBox(width: 8),
                if (shouldShowAvatar)
                  _MessageAvatar(
                    imagePath: avatarImagePath,
                    isProxySender: isProxySender,
                    avatarShape: avatarShape,
                    cornerRadius: avatarCornerRadius,
                  ),
              ],
            ),
            const SizedBox(height: 6),
          ],
          LayoutBuilder(
            builder: (context, constraints) {
              final maxBubbleWidth = isHiddenPlaceholder
                  ? constraints.maxWidth.clamp(0, 320).toDouble()
                  : constraints.maxWidth;
              return Align(
                alignment: Alignment.centerRight,
                child: _UserBubbleWithAttachments(
                  parseResult: parseResult,
                  enableDialogs: enableDialogs,
                  maxBubbleWidth: maxBubbleWidth,
                  backgroundColor: backgroundColor,
                  textColor: textColor,
                  bubbleShape: bubbleShape,
                  imageStyle: imageStyle,
                  transparentSurface: transparentSurface,
                  contentPadding: contentPadding,
                  isHiddenPlaceholder: isHiddenPlaceholder,
                  textContent: textContent,
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _NormalUserBubbleLayout extends StatelessWidget {
  const _NormalUserBubbleLayout({
    required this.parseResult,
    required this.enableDialogs,
    required this.shouldShowAvatar,
    required this.shouldShowResolvedName,
    required this.resolvedDisplayName,
    required this.isProxySender,
    required this.avatarImagePath,
    required this.avatarShape,
    required this.avatarCornerRadius,
    required this.bubbleShape,
    required this.backgroundColor,
    required this.textColor,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.isHiddenPlaceholder,
    required this.textContent,
  });

  final MessageParseResult parseResult;
  final bool enableDialogs;
  final bool shouldShowAvatar;
  final bool shouldShowResolvedName;
  final String? resolvedDisplayName;
  final bool isProxySender;
  final String? avatarImagePath;
  final String avatarShape;
  final double avatarCornerRadius;
  final BorderRadius bubbleShape;
  final Color backgroundColor;
  final Color textColor;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool isHiddenPlaceholder;
  final String textContent;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.symmetric(vertical: isHiddenPlaceholder ? 0 : 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Expanded(
            child: Padding(
              padding: EdgeInsets.only(
                left: 32,
                right: shouldShowAvatar ? 0 : 8,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: <Widget>[
                  if (shouldShowResolvedName)
                    Padding(
                      padding: const EdgeInsets.only(right: 4, bottom: 4),
                      child: Text(
                        resolvedDisplayName!,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: isProxySender
                              ? Theme.of(context).colorScheme.primary
                              : Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  LayoutBuilder(
                    builder: (context, constraints) {
                      final maxBubbleWidth = isHiddenPlaceholder
                          ? (constraints.maxWidth * 0.85)
                                .clamp(0, 320)
                                .toDouble()
                          : constraints.maxWidth * 0.85;
                      return Align(
                        alignment: Alignment.centerRight,
                        child: _UserBubbleWithAttachments(
                          parseResult: parseResult,
                          enableDialogs: enableDialogs,
                          maxBubbleWidth: maxBubbleWidth,
                          backgroundColor: backgroundColor,
                          textColor: textColor,
                          bubbleShape: bubbleShape,
                          imageStyle: imageStyle,
                          transparentSurface: transparentSurface,
                          contentPadding: contentPadding,
                          isHiddenPlaceholder: isHiddenPlaceholder,
                          textContent: textContent,
                        ),
                      );
                    },
                  ),
                ],
              ),
            ),
          ),
          if (shouldShowAvatar) ...<Widget>[
            const SizedBox(width: 8),
            _MessageAvatar(
              imagePath: avatarImagePath,
              isProxySender: isProxySender,
              avatarShape: avatarShape,
              cornerRadius: avatarCornerRadius,
            ),
          ],
        ],
      ),
    );
  }
}

class _UserBubbleBody extends StatelessWidget {
  const _UserBubbleBody({
    required this.backgroundColor,
    required this.textColor,
    required this.bubbleShape,
    required this.imageStyle,
    required this.transparentSurface,
    required this.contentPadding,
    required this.isHiddenPlaceholder,
    required this.textContent,
  });

  final Color backgroundColor;
  final Color textColor;
  final BorderRadius bubbleShape;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;
  final EdgeInsets contentPadding;
  final bool isHiddenPlaceholder;
  final String textContent;

  @override
  Widget build(BuildContext context) {
    return BubbleSurface(
      color: backgroundColor,
      borderRadius: bubbleShape,
      imageStyle: imageStyle,
      transparentSurface: transparentSurface,
      child: Padding(
        padding: contentPadding,
        child: isHiddenPlaceholder
            ? _HiddenUserMessagePlaceholderContent(textColor: textColor)
            : SelectableText(
                textContent,
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: textColor),
              ),
      ),
    );
  }
}

class _HiddenUserMessagePlaceholderContent extends StatelessWidget {
  const _HiddenUserMessagePlaceholderContent({required this.textColor});

  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 220),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Expanded(
            child: Divider(
              color: Theme.of(
                context,
              ).colorScheme.primary.withValues(alpha: 0.28),
              thickness: 1,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            l10n?.hiddenUserMessage ?? 'Hidden user message',
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: textColor.withValues(alpha: 0.72),
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Divider(
              color: Theme.of(
                context,
              ).colorScheme.primary.withValues(alpha: 0.28),
              thickness: 1,
            ),
          ),
        ],
      ),
    );
  }
}

class _MessageAvatar extends StatelessWidget {
  /// Creates a message avatar that can render an imported theme asset.
  const _MessageAvatar({
    required this.imagePath,
    required this.isProxySender,
    required this.avatarShape,
    required this.cornerRadius,
  });

  final String? imagePath;
  final bool isProxySender;
  final String avatarShape;
  final double cornerRadius;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final square = avatarShape == UserPreferencesManager.AVATAR_SHAPE_SQUARE;
    final avatarImagePath = imagePath;
    final icon = isProxySender ? Icons.assistant : Icons.person;
    final backgroundColor = isProxySender
        ? colorScheme.secondaryContainer
        : colorScheme.primaryContainer;
    final tint = isProxySender
        ? colorScheme.onSecondaryContainer
        : colorScheme.onPrimaryContainer;
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        color: backgroundColor,
        shape: square ? BoxShape.rectangle : BoxShape.circle,
        borderRadius: square ? BorderRadius.circular(cornerRadius) : null,
      ),
      clipBehavior: Clip.antiAlias,
      child: isProxySender
          ? ChatAvatarImage(avatarUri: avatarImagePath, fit: BoxFit.cover)
          : avatarImagePath != null && avatarImagePath.isNotEmpty
          ? ThemeAssetImage(storagePath: avatarImagePath, fit: BoxFit.cover)
          : Icon(icon, color: tint, size: 22),
    );
  }
}

class MessageParseResult {
  const MessageParseResult({
    required this.processedText,
    required this.trailingAttachments,
    this.attachments = const <AttachmentData>[],
    this.imageAttachments = const <AttachmentData>[],
    this.nonImageAttachments = const <AttachmentData>[],
    this.replyInfo,
    this.imageLinks = const <ImageLinkData>[],
    this.proxySenderName,
  });

  final String processedText;
  final List<AttachmentData> trailingAttachments;
  final List<AttachmentData> attachments;
  final List<AttachmentData> imageAttachments;
  final List<AttachmentData> nonImageAttachments;
  final ReplyInfo? replyInfo;
  final List<ImageLinkData> imageLinks;
  final String? proxySenderName;
}

class ReplyInfo {
  const ReplyInfo({
    required this.sender,
    required this.timestamp,
    required this.content,
  });

  final String sender;
  final int timestamp;
  final String content;
}

class ImageLinkData {
  const ImageLinkData({required this.id});

  final String id;
}

class _AttachmentMatch {
  const _AttachmentMatch(this.match);

  final RegExpMatch match;
}

MessageParseResult _buildMessageParseResult({
  required String text,
  required List<AttachmentData> rawAttachments,
  required List<ImageLinkData> imageLinks,
  required ReplyInfo? replyInfo,
  required String? proxySenderName,
}) {
  final rawImageAttachments = <AttachmentData>[];
  final nonImageAttachments = <AttachmentData>[];
  for (final att in rawAttachments) {
    if (att.type == 'application/vnd.workspace-context+xml' ||
        att.id == 'workspace_context') {
      continue;
    }
    if (att.isImage) {
      rawImageAttachments.add(att);
    } else {
      nonImageAttachments.add(att);
    }
  }

  final imageAttachments = <AttachmentData>[];
  final linkCount = imageLinks.length;
  final rawCount = rawImageAttachments.length;
  final maxCount = math.max(linkCount, rawCount);

  for (var i = 0; i < maxCount; i++) {
    final link = i < linkCount ? imageLinks[i] : null;
    final raw = i < rawCount ? rawImageAttachments[i] : null;

    if (link != null && raw != null) {
      imageAttachments.add(
        AttachmentData(
          id: link.id,
          filename: raw.filename,
          type: raw.type,
          size: raw.size,
          content: raw.content,
          mediaPoolType: 'image',
        ),
      );
    } else if (link != null) {
      imageAttachments.add(
        AttachmentData(
          id: link.id,
          filename: 'Image',
          type: 'image/*',
          mediaPoolType: 'image',
        ),
      );
    } else if (raw != null) {
      imageAttachments.add(raw);
    }
  }

  final allAttachments = <AttachmentData>[
    ...imageAttachments,
    ...nonImageAttachments,
  ];

  return MessageParseResult(
    processedText: text,
    trailingAttachments: nonImageAttachments,
    attachments: allAttachments,
    imageAttachments: imageAttachments,
    nonImageAttachments: nonImageAttachments,
    replyInfo: replyInfo,
    imageLinks: imageLinks,
    proxySenderName: proxySenderName,
  );
}

/// Extracts message decorations and trims the remaining display text.
MessageParseResult parseMessageContent(String content) {
  var cleanedContent = content.trim();

  final proxySenderMatch = ChatMarkupRegex.proxySenderTag.firstMatch(
    cleanedContent,
  );
  final proxySenderName = proxySenderMatch?.group(1);
  if (proxySenderMatch != null) {
    cleanedContent = cleanedContent
        .replaceFirst(proxySenderMatch.group(0)!, '')
        .trim();
  }

  final mediaLinks = ChatMarkupRegex.extractMediaLinks(cleanedContent);
  final imageLinks = <ImageLinkData>[
    for (final mediaLink in mediaLinks)
      if (mediaLink.type == 'image') ImageLinkData(id: mediaLink.id),
  ];
  cleanedContent = ChatMarkupRegex.removeMediaLinks(cleanedContent).trim();
  final mediaLinkAttachments = <AttachmentData>[
    for (final mediaLink in mediaLinks)
      if (mediaLink.type == 'audio' ||
          mediaLink.type == 'video' ||
          mediaLink.type == 'file')
        AttachmentData(
          id: mediaLink.id,
          filename:
              mediaLink.fileName ??
              (mediaLink.type == 'audio' ? 'Audio' : 'Video'),
          type: switch (mediaLink.type) {
            'audio' => 'audio/*',
            'video' => 'video/*',
            _ => 'application/octet-stream',
          },
          mediaPoolType: mediaLink.type,
        ),
  ];

  final replyMatch = ChatMarkupRegex.replyToTag.firstMatch(cleanedContent);
  final replyInfo = replyMatch == null
      ? null
      : ReplyInfo(
          sender: replyMatch.group(1)!,
          timestamp: int.tryParse(replyMatch.group(2)!) ?? 0,
          content: replyMatch.group(3)!.trim().replaceAll(RegExp(r'^"|"$'), ''),
        );
  if (replyMatch != null) {
    cleanedContent = cleanedContent
        .replaceFirst(replyMatch.group(0)!, '')
        .trim();
  }

  final workspaceMatch = ChatMarkupRegex.workspaceAttachmentTag.firstMatch(
    cleanedContent,
  );
  if (workspaceMatch != null) {
    final workspaceContent = workspaceMatch.group(0)!;
    cleanedContent = cleanedContent.replaceFirst(workspaceContent, '').trim();
  }

  if (!cleanedContent.contains('<attachment')) {
    return _buildMessageParseResult(
      text: cleanedContent,
      rawAttachments: mediaLinkAttachments,
      imageLinks: imageLinks,
      replyInfo: replyInfo,
      proxySenderName: proxySenderName,
    );
  }

  final pairedMatches = ChatMarkupRegex.attachmentDataTag
      .allMatches(cleanedContent)
      .map(_AttachmentMatch.new);
  final selfClosingMatches = ChatMarkupRegex.attachmentDataSelfClosingTag
      .allMatches(cleanedContent)
      .map(_AttachmentMatch.new);
  final allMatches = <_AttachmentMatch>[...pairedMatches, ...selfClosingMatches]
    ..sort((a, b) => a.match.start.compareTo(b.match.start));

  final matches = <_AttachmentMatch>[];
  var lastEnd = -1;
  for (final attachmentMatch in allMatches) {
    if (attachmentMatch.match.start > lastEnd) {
      matches.add(attachmentMatch);
      lastEnd = attachmentMatch.match.end - 1;
    }
  }

  if (matches.isEmpty) {
    return _buildMessageParseResult(
      text: cleanedContent,
      rawAttachments: mediaLinkAttachments,
      imageLinks: imageLinks,
      replyInfo: replyInfo,
      proxySenderName: proxySenderName,
    );
  }

  final trailingAttachmentIndices = <int>{};
  final contentAfterLast = cleanedContent.substring(matches.last.match.end);
  if (contentAfterLast.trim().isEmpty) {
    trailingAttachmentIndices.add(matches.length - 1);
    for (var i = matches.length - 2; i >= 0; i--) {
      final textBetween = cleanedContent.substring(
        matches[i].match.end,
        matches[i + 1].match.start,
      );
      if (textBetween.trim().isEmpty) {
        trailingAttachmentIndices.add(i);
      } else {
        break;
      }
    }
  }

  final trailingAttachments = <AttachmentData>[];
  final messageText = StringBuffer();
  var lastIndex = 0;
  for (var index = 0; index < matches.length; index++) {
    final match = matches[index].match;
    final startIndex = match.start;
    final id = decodeChatXmlText(match.group(1)!);
    final filename = decodeChatXmlText(match.group(2)!);
    final type = decodeChatXmlText(match.group(3)!);
    final size = _parseLong(match.group(4));
    final attachmentContent = decodeChatXmlText(match.group(5) ?? '');
    final attachment = AttachmentData(
      id: id,
      filename: filename,
      type: type,
      size: size,
      content: attachmentContent,
    );
    final isTrailingAttachment = trailingAttachmentIndices.contains(index);
    final isScreenContent =
        type == 'text/json' && filename == 'screen_content.json';
    final shouldBeTrailing = isTrailingAttachment || isScreenContent;

    if (startIndex > lastIndex) {
      final textBefore = cleanedContent.substring(lastIndex, startIndex);
      if (!shouldBeTrailing ||
          (trailingAttachmentIndices.isNotEmpty &&
              index ==
                  trailingAttachmentIndices.reduce((a, b) => a < b ? a : b))) {
        messageText.write(textBefore);
      }
    }

    if (shouldBeTrailing) {
      trailingAttachments.add(attachment);
    } else {
      messageText.write('@$filename');
    }

    lastIndex = match.end;
  }

  if (lastIndex < cleanedContent.length) {
    messageText.write(cleanedContent.substring(lastIndex));
  }

  return _buildMessageParseResult(
    text: messageText.toString().trim(),
    rawAttachments: <AttachmentData>[
      ...mediaLinkAttachments,
      ...trailingAttachments,
    ],
    imageLinks: imageLinks,
    replyInfo: replyInfo,
    proxySenderName: proxySenderName,
  );
}

int _parseLong(String? value) {
  if (value == null || value.isEmpty) {
    return 0;
  }
  final parsed = int.tryParse(value);
  if (parsed == null) {
    return 0;
  }
  return parsed;
}

class _ReplyInfoView extends StatelessWidget {
  const _ReplyInfoView({required this.replyInfo});

  final ReplyInfo replyInfo;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: const BorderRadius.only(
          topLeft: Radius.circular(8),
          topRight: Radius.circular(8),
          bottomRight: Radius.circular(2),
          bottomLeft: Radius.circular(8),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Row(
          children: <Widget>[
            Icon(Icons.reply, size: 12, color: theme.colorScheme.primary),
            const SizedBox(width: 4),
            Flexible(
              child: Text(
                '${replyInfo.sender}: ${replyInfo.content}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class AttachmentTag extends StatelessWidget {
  const AttachmentTag({
    super.key,
    required this.attachment,
    required this.textColor,
    required this.backgroundColor,
    this.enabled = true,
    this.onClick,
  });

  final AttachmentData attachment;
  final Color textColor;
  final Color backgroundColor;
  final bool enabled;
  final ValueChanged<AttachmentData>? onClick;

  @override
  Widget build(BuildContext context) {
    final icon = _attachmentIcon(attachment);
    final displayLabel = _attachmentDisplayLabel(attachment);
    return Material(
      color: backgroundColor.withValues(alpha: 0.5),
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: enabled && _attachmentClickable(attachment) && onClick != null
            ? () => onClick!(attachment)
            : null,
        child: Container(
          height: 24,
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 12, color: textColor.withValues(alpha: 0.8)),
              const SizedBox(width: 4),
              ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 120),
                child: Text(
                  displayLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: textColor),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

IconData _attachmentIcon(AttachmentData attachment) {
  if (attachment.type.startsWith('image/')) {
    return Icons.image;
  }
  if (attachment.type.startsWith('audio/')) {
    return Icons.volume_up;
  }
  if (attachment.type.startsWith('video/')) {
    return Icons.play_arrow;
  }
  if (attachment.type == 'text/json' &&
      attachment.filename == 'screen_content.json') {
    return Icons.screenshot_monitor;
  }
  if (attachment.type == 'application/vnd.workspace-context+xml') {
    return Icons.code;
  }
  return Icons.description;
}

String _attachmentDisplayLabel(AttachmentData attachment) {
  if (attachment.type == 'text/json' &&
      attachment.filename == 'screen_content.json') {
    return 'Screen content';
  }
  if (attachment.type == 'application/vnd.workspace-context+xml') {
    return 'Workspace';
  }
  return attachment.filename;
}

bool _attachmentClickable(AttachmentData attachment) {
  return attachment.content.isNotEmpty ||
      attachment.id.startsWith('/') ||
      attachment.id.startsWith('content://') ||
      attachment.id.startsWith('file://') ||
      attachment.mediaPoolType != null ||
      attachment.type.startsWith('image/');
}
