// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../../../util/ChatMarkupRegex.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../attachments/ChatMessageImageAttachments.dart';
import '../bubble/BubbleSurface.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'CursorMessageLayout.dart';

export '../../attachments/ChatMessageImageAttachments.dart'
    show AttachmentData, ChatMessageAttachments, ChatMessageImageAttachments;

class UserMessageComposable extends StatefulWidget {
  const UserMessageComposable({
    super.key,
    required this.message,
  });

  final ChatUiMessage message;

  @override
  State<UserMessageComposable> createState() => _UserMessageComposableState();
}

class _UserMessageComposableState extends State<UserMessageComposable> {
  /// Builds the user message content without a prompt heading.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textColor = colorScheme.onPrimaryContainer;
    final parseResult = parseMessageContent(widget.message.displayText);
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final bubbleColor =
        _optionalColor(themePreferenceSnapshot.cursorUserBubbleColor) ??
        colorScheme.primaryContainer;
    final effectiveTextColor =
        _optionalColor(themePreferenceSnapshot.bubbleUserTextColor) ??
        textColor;
    final contentPadding = const EdgeInsets.fromLTRB(16, 16, 16, 16);
    final bubbleBorderRadius = BorderRadius.circular(
      themePreferenceSnapshot.bubbleUserRoundedCornersEnabled ? 12 : 4,
    );
    final transparentSurface =
        themePreferenceSnapshot.transparentSurfaceEnabled;

    return CursorMessageLayout(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (parseResult.replyInfo != null)
            _ReplyInfoView(replyInfo: parseResult.replyInfo!),
          if (parseResult.attachments.any(
            (attachment) => !attachment.isInlineContext,
          ))
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: ChatMessageAttachments(
                attachments: parseResult.attachments,
                enableDialogs: true,
                isBubbleMode: false,
              ),
            ),
          if (parseResult.processedText.isNotEmpty ||
              parseResult.attachments.isEmpty)
            SizedBox(
              width: double.infinity,
              child: BubbleSurface(
                color: bubbleColor,
                borderRadius: bubbleBorderRadius,
                transparentSurface: transparentSurface,
                child: Padding(
                  padding: contentPadding,
                  child: SelectableText(
                    parseResult.processedText,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: effectiveTextColor,
                    ),
                  ),
                ),
              ),
            ),
          if (parseResult.attachments.any(
            (attachment) => attachment.isInlineContext,
          ))
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: ChatMessageContextMetadata(
                attachments: parseResult.attachments,
                enableDialogs: true,
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

/// Identifies one image pool link parsed from a user message.
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
  final mediaLinkAttachments = <AttachmentData>[
    for (final mediaLink in mediaLinks)
      if (mediaLink.type == 'audio' ||
          mediaLink.type == 'video' ||
          mediaLink.type == 'file')
        AttachmentData(
          id: mediaLink.id,
          filename: mediaLink.fileName ??
              (mediaLink.type == 'audio' ? 'Audio' : 'Video'),
          type: switch (mediaLink.type) {
            'audio' => 'audio/*',
            'video' => 'video/*',
            _ => 'application/octet-stream',
          },
          mediaPoolType: mediaLink.type,
        ),
  ];
  cleanedContent = ChatMarkupRegex.removeMediaLinks(cleanedContent).trim();

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
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: <Widget>[
          Icon(Icons.reply, size: 12, color: theme.colorScheme.primary),
          const SizedBox(width: 4),
          Expanded(
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
