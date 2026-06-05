// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../../util/ChatMarkupRegex.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../attachments/AttachmentViewerDialog.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'BubbleSurface.dart';

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
        widget.message.displayMode == 'HIDDEN_PLACEHOLDER';
    final parseResult = isHiddenPlaceholder
        ? const MessageParseResult(processedText: '', trailingAttachments: [])
        : parseMessageContent(widget.message.content);
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
        isHiddenPlaceholder || widget.transparentSurface
        ? null
        : widget.bubbleImageStyle;
    final bubbleShape = widget.bubbleRoundedCornersEnabled
        ? const BorderRadius.only(
            topLeft: Radius.circular(20),
            topRight: Radius.circular(4),
            bottomRight: Radius.circular(20),
            bottomLeft: Radius.circular(20),
          )
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
          if (!isHiddenPlaceholder)
            _UserMessagePreBubbleContent(
              parseResult: parseResult,
              textColor: textColor,
              backgroundColor: backgroundColor,
              enableDialogs: widget.enableDialogs,
            ),
          if (snapshot.bubbleWideLayoutEnabled)
            _WideUserBubbleLayout(
              shouldShowAvatar: shouldShowAvatar,
              shouldShowResolvedName: shouldShowResolvedName,
              resolvedDisplayName: resolvedDisplayName,
              isProxySender: isProxySender,
              avatarImagePath: isProxySender
                  ? snapshot.customAiAvatarUri
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
              shouldShowAvatar: shouldShowAvatar,
              shouldShowResolvedName: shouldShowResolvedName,
              resolvedDisplayName: resolvedDisplayName,
              isProxySender: isProxySender,
              avatarImagePath: isProxySender
                  ? snapshot.customAiAvatarUri
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

class _UserMessagePreBubbleContent extends StatelessWidget {
  const _UserMessagePreBubbleContent({
    required this.parseResult,
    required this.textColor,
    required this.backgroundColor,
    required this.enableDialogs,
  });

  final MessageParseResult parseResult;
  final Color textColor;
  final Color backgroundColor;
  final bool enableDialogs;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        if (parseResult.replyInfo != null)
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: <Widget>[
              Flexible(
                child: Padding(
                  padding: const EdgeInsets.only(left: 32, bottom: 4),
                  child: _ReplyInfoView(replyInfo: parseResult.replyInfo!),
                ),
              ),
            ],
          ),
        if (parseResult.imageLinks.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 4),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: <Widget>[
                for (final imageLink in parseResult.imageLinks)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: AttachmentTag(
                      attachment: AttachmentData(
                        id: imageLink.id,
                        filename: 'Image',
                        type: 'image/*',
                      ),
                      textColor: textColor,
                      backgroundColor: backgroundColor,
                      enabled: enableDialogs,
                    ),
                  ),
              ],
            ),
          ),
        if (parseResult.trailingAttachments.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: <Widget>[
                Flexible(
                  child: Wrap(
                    alignment: WrapAlignment.end,
                    spacing: 4,
                    runSpacing: 4,
                    children: <Widget>[
                      for (final attachment in parseResult.trailingAttachments)
                        AttachmentTag(
                          attachment: attachment,
                          textColor: textColor,
                          backgroundColor: backgroundColor,
                          enabled: enableDialogs,
                          onClick: (attachmentData) {
                            final chatAttachment = ChatAttachment(
                              id: attachmentData.id,
                              filename: attachmentData.filename,
                              mimeType: attachmentData.type,
                              size: attachmentData.size,
                              content: attachmentData.content,
                            );
                            showDialog<void>(
                              context: context,
                              builder: (dialogContext) =>
                                  AttachmentViewerDialog(
                                    visible: true,
                                    attachment: chatAttachment,
                                    onDismiss: () {
                                      Navigator.of(dialogContext).pop();
                                    },
                                  ),
                            );
                          },
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _WideUserBubbleLayout extends StatelessWidget {
  const _WideUserBubbleLayout({
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
              return Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: <Widget>[
                  ConstrainedBox(
                    constraints: BoxConstraints(
                      maxWidth: maxBubbleWidth,
                      minHeight: 44,
                    ),
                    child: _UserBubbleBody(
                      backgroundColor: backgroundColor,
                      textColor: textColor,
                      bubbleShape: bubbleShape,
                      imageStyle: imageStyle,
                      transparentSurface: transparentSurface,
                      contentPadding: contentPadding,
                      isHiddenPlaceholder: isHiddenPlaceholder,
                      textContent: textContent,
                    ),
                  ),
                ],
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
                        child: ConstrainedBox(
                          constraints: BoxConstraints(
                            maxWidth: maxBubbleWidth,
                            minHeight: 44,
                          ),
                          child: _UserBubbleBody(
                            backgroundColor: backgroundColor,
                            textColor: textColor,
                            bubbleShape: bubbleShape,
                            imageStyle: imageStyle,
                            transparentSurface: transparentSurface,
                            contentPadding: contentPadding,
                            isHiddenPlaceholder: isHiddenPlaceholder,
                            textContent: textContent,
                          ),
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
    final tint = isProxySender ? colorScheme.secondary : colorScheme.primary;
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        shape: square ? BoxShape.rectangle : BoxShape.circle,
        borderRadius: square ? BorderRadius.circular(cornerRadius) : null,
      ),
      clipBehavior: Clip.antiAlias,
      child: avatarImagePath != null && avatarImagePath.isNotEmpty
          ? Image.file(File(avatarImagePath), fit: BoxFit.cover)
          : Icon(icon, color: tint, size: 22),
    );
  }
}

class MessageParseResult {
  const MessageParseResult({
    required this.processedText,
    required this.trailingAttachments,
    this.replyInfo,
    this.imageLinks = const <ImageLinkData>[],
    this.proxySenderName,
  });

  final String processedText;
  final List<AttachmentData> trailingAttachments;
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

class AttachmentData {
  const AttachmentData({
    required this.id,
    required this.filename,
    required this.type,
    this.size = 0,
    this.content = '',
  });

  final String id;
  final String filename;
  final String type;
  final int size;
  final String content;
}

class _AttachmentMatch {
  const _AttachmentMatch(this.match);

  final RegExpMatch match;
}

MessageParseResult parseMessageContent(String content) {
  var cleanedContent = content.replaceAll(ChatMarkupRegex.memoryTag, '').trim();

  final proxySenderMatch = ChatMarkupRegex.proxySenderTag.firstMatch(
    cleanedContent,
  );
  final proxySenderName = proxySenderMatch?.group(1);
  if (proxySenderMatch != null) {
    cleanedContent = cleanedContent
        .replaceFirst(proxySenderMatch.group(0)!, '')
        .trim();
  }

  final imageLinks = <ImageLinkData>[
    for (final mediaLink in _extractMediaLinkTags(cleanedContent))
      if (mediaLink.type == 'image') ImageLinkData(id: mediaLink.id),
  ];
  cleanedContent = _removeMediaLinks(cleanedContent).trim();
  final mediaLinkAttachments = <AttachmentData>[
    for (final mediaLink in _extractMediaLinkTags(content))
      if (mediaLink.type != 'image')
        AttachmentData(
          id: 'media_pool:${mediaLink.id}',
          filename: mediaLink.type == 'audio' ? 'Audio' : 'Video',
          type: mediaLink.type == 'audio' ? 'audio/*' : 'video/*',
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

  final workspaceAttachments = <AttachmentData>[];
  final workspaceMatch = ChatMarkupRegex.workspaceAttachmentTag.firstMatch(
    cleanedContent,
  );
  if (workspaceMatch != null) {
    final workspaceContent = workspaceMatch.group(0)!;
    workspaceAttachments.add(
      AttachmentData(
        id: 'workspace_context',
        filename: 'Workspace',
        type: 'application/vnd.workspace-context+xml',
        size: workspaceContent.length,
        content: workspaceContent,
      ),
    );
    cleanedContent = cleanedContent.replaceFirst(workspaceContent, '').trim();
  }

  if (!cleanedContent.contains('<attachment')) {
    return MessageParseResult(
      processedText: cleanedContent,
      trailingAttachments: <AttachmentData>[
        ...workspaceAttachments,
        ...mediaLinkAttachments,
      ],
      replyInfo: replyInfo,
      imageLinks: imageLinks,
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
    return MessageParseResult(
      processedText: cleanedContent,
      trailingAttachments: <AttachmentData>[
        ...workspaceAttachments,
        ...mediaLinkAttachments,
      ],
      replyInfo: replyInfo,
      imageLinks: imageLinks,
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
    final id = match.group(1)!;
    final filename = match.group(2)!;
    final type = match.group(3)!;
    final size = _parseLong(match.group(4));
    final attachmentContent = match.group(5) ?? '';
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

  return MessageParseResult(
    processedText: messageText.toString(),
    trailingAttachments: <AttachmentData>[
      ...workspaceAttachments,
      ...mediaLinkAttachments,
      ...trailingAttachments,
    ],
    replyInfo: replyInfo,
    imageLinks: imageLinks,
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

class _MediaLinkTag {
  const _MediaLinkTag({required this.type, required this.id});

  final String type;
  final String id;
}

List<_MediaLinkTag> _extractMediaLinkTags(String message) {
  final tags = <_MediaLinkTag>[];
  final seen = <String>{};
  var cursor = 0;
  while (true) {
    final startRelative = message.indexOf('<link', cursor);
    if (startRelative < 0) {
      break;
    }
    final endRelative = message.indexOf('</link>', startRelative);
    if (endRelative < 0) {
      break;
    }
    final end = endRelative + '</link>'.length;
    final tagText = message.substring(startRelative, end);
    final type = _extractAttr(tagText, 'type');
    final id = _extractAttr(tagText, 'id');
    if (type != null &&
        id != null &&
        id != 'error' &&
        const <String>{'image', 'audio', 'video'}.contains(type)) {
      final key = '$type/$id';
      if (seen.add(key)) {
        tags.add(_MediaLinkTag(type: type, id: id));
      }
    }
    cursor = end;
  }
  return tags;
}

String _removeMediaLinks(String message) {
  final result = StringBuffer();
  var cursor = 0;
  while (true) {
    final startRelative = message.indexOf('<link', cursor);
    if (startRelative < 0) {
      result.write(message.substring(cursor));
      break;
    }
    result.write(message.substring(cursor, startRelative));
    final endRelative = message.indexOf('</link>', startRelative);
    if (endRelative < 0) {
      result.write(message.substring(startRelative));
      break;
    }
    cursor = endRelative + '</link>'.length;
  }
  return result.toString();
}

String? _extractAttr(String source, String attributeName) {
  final attrStart = source.indexOf(attributeName);
  if (attrStart < 0) {
    return null;
  }
  final afterName = source
      .substring(attrStart + attributeName.length)
      .trimLeft();
  if (!afterName.startsWith('=')) {
    return null;
  }
  final afterEquals = afterName.substring(1).trimLeft();
  final afterEscape = afterEquals.startsWith('\\')
      ? afterEquals.substring(1)
      : afterEquals;
  if (afterEscape.isEmpty) {
    return null;
  }
  final quote = afterEscape[0];
  if (quote == '"' || quote == "'") {
    final body = afterEscape.substring(1);
    final end = body.indexOf(quote);
    if (end < 0) {
      return null;
    }
    return body.substring(0, end).replaceFirst(RegExp(r'\\$'), '');
  }
  final end = afterEscape.indexOf(RegExp(r'\s|>'));
  final value = end < 0 ? afterEscape : afterEscape.substring(0, end);
  return value.replaceFirst(RegExp(r'/$'), '').replaceFirst(RegExp(r'\\$'), '');
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
      attachment.id.startsWith('media_pool:') ||
      attachment.type.startsWith('image/');
}
