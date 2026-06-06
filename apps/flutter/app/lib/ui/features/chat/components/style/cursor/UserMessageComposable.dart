// ignore_for_file: file_names

import 'dart:io';

import 'package:flutter/material.dart';

import '../../../../../../util/ChatMarkupRegex.dart';
import '../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../theme/OperitTheme.dart';
import '../../attachments/AttachmentViewerDialog.dart';
import '../bubble/BubbleSurface.dart';
import '../../../viewmodel/ChatViewModel.dart';

class UserMessageComposable extends StatefulWidget {
  const UserMessageComposable({
    super.key,
    required this.message,
    required this.useBubbleStyle,
  });

  final ChatUiMessage message;
  final bool useBubbleStyle;

  @override
  State<UserMessageComposable> createState() => _UserMessageComposableState();
}

class _UserMessageComposableState extends State<UserMessageComposable> {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textColor = colorScheme.onPrimaryContainer;
    final parseResult = parseMessageContent(widget.message.content);
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final showUserName = themePreferenceSnapshot.showUserName;
    final bubbleColor =
        _optionalColor(
          widget.useBubbleStyle
              ? themePreferenceSnapshot.bubbleUserBubbleColor
              : themePreferenceSnapshot.cursorUserBubbleColor,
        ) ??
        colorScheme.primaryContainer;
    final effectiveTextColor =
        _optionalColor(themePreferenceSnapshot.bubbleUserTextColor) ??
        textColor;
    final messageFontFamily = widget.useBubbleStyle
        ? operitMessageFontFamily(themePreferenceSnapshot, isUser: true)
        : null;
    final messageFontFamilyFallback = widget.useBubbleStyle
        ? operitMessageFontFamilyFallback(themePreferenceSnapshot, isUser: true)
        : null;
    final contentPadding = widget.useBubbleStyle
        ? EdgeInsets.fromLTRB(
            themePreferenceSnapshot.bubbleUserContentPaddingLeft,
            16,
            themePreferenceSnapshot.bubbleUserContentPaddingRight,
            16,
          )
        : const EdgeInsets.fromLTRB(16, 16, 16, 16);
    final bubbleBorderRadius = BorderRadius.circular(
      themePreferenceSnapshot.bubbleUserRoundedCornersEnabled ? 12 : 4,
    );
    final transparentSurface =
        themePreferenceSnapshot.transparentSurfaceEnabled;
    final userBubbleImagePath = themePreferenceSnapshot.bubbleUserImageUri;
    final userBubbleImageStyle =
        widget.useBubbleStyle &&
            themePreferenceSnapshot.bubbleUserUseImage &&
            userBubbleImagePath != null &&
            userBubbleImagePath.isNotEmpty
        ? BubbleImageStyle(
            imagePath: userBubbleImagePath,
            cropLeftRatio: themePreferenceSnapshot.bubbleUserImageCropLeft,
            cropTopRatio: themePreferenceSnapshot.bubbleUserImageCropTop,
            cropRightRatio: themePreferenceSnapshot.bubbleUserImageCropRight,
            cropBottomRatio: themePreferenceSnapshot.bubbleUserImageCropBottom,
            repeatXStartRatio:
                themePreferenceSnapshot.bubbleUserImageRepeatStart,
            repeatXEndRatio: themePreferenceSnapshot.bubbleUserImageRepeatEnd,
            repeatYStartRatio:
                themePreferenceSnapshot.bubbleUserImageRepeatYStart,
            repeatYEndRatio: themePreferenceSnapshot.bubbleUserImageRepeatYEnd,
            imageScale: themePreferenceSnapshot.bubbleUserImageScale,
            renderMode: themePreferenceSnapshot.bubbleUserImageRenderMode,
          )
        : null;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (widget.useBubbleStyle &&
              themePreferenceSnapshot.bubbleShowAvatar) ...<Widget>[
            _MessageAvatar(
              imagePath: themePreferenceSnapshot.customUserAvatarUri,
              backgroundColor: bubbleColor,
              foregroundColor: effectiveTextColor,
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
                if (parseResult.replyInfo != null)
                  _ReplyInfoView(replyInfo: parseResult.replyInfo!),
                if (parseResult.trailingAttachments.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: SizedBox(
                      width: double.infinity,
                      child: Wrap(
                        spacing: 4,
                        runSpacing: 4,
                        children: <Widget>[
                          for (final attachment
                              in parseResult.trailingAttachments)
                            AttachmentTag(
                              attachment: attachment,
                              textColor: effectiveTextColor,
                              backgroundColor: bubbleColor,
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
                  ),
                SizedBox(
                  width: double.infinity,
                  child: BubbleSurface(
                    color: bubbleColor,
                    borderRadius: bubbleBorderRadius,
                    imageStyle: userBubbleImageStyle,
                    transparentSurface: transparentSurface,
                    child: Padding(
                      padding: contentPadding,
                      child: DefaultTextStyle.merge(
                        style: TextStyle(
                          fontFamily: messageFontFamily,
                          fontFamilyFallback: messageFontFamilyFallback,
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            if (showUserName) ...<Widget>[
                              Text(
                                parseResult.proxySenderName == null
                                    ? 'Prompt'
                                    : 'Prompt by ${parseResult.proxySenderName}',
                                style: theme.textTheme.labelSmall?.copyWith(
                                  color: effectiveTextColor.withValues(
                                    alpha: 0.7,
                                  ),
                                  fontFamily: messageFontFamily,
                                  fontFamilyFallback: messageFontFamilyFallback,
                                ),
                              ),
                              const SizedBox(height: 8),
                            ],
                            SelectableText(
                              parseResult.processedText,
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: effectiveTextColor,
                                fontFamily: messageFontFamily,
                                fontFamilyFallback: messageFontFamilyFallback,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
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
    required this.backgroundColor,
    required this.foregroundColor,
    required this.imagePath,
    required this.square,
    required this.cornerRadius,
  });

  final Color backgroundColor;
  final Color foregroundColor;
  final String? imagePath;
  final bool square;
  final double cornerRadius;

  @override
  Widget build(BuildContext context) {
    final avatarImagePath = imagePath;
    return Padding(
      padding: const EdgeInsets.only(top: 10),
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
            : Icon(Icons.person, size: 15, color: foregroundColor),
      ),
    );
  }
}

class MessageParseResult {
  const MessageParseResult({
    required this.processedText,
    required this.trailingAttachments,
    this.replyInfo,
    this.proxySenderName,
  });

  final String processedText;
  final List<AttachmentData> trailingAttachments;
  final ReplyInfo? replyInfo;
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
      trailingAttachments: workspaceAttachments,
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
    return MessageParseResult(
      processedText: cleanedContent,
      trailingAttachments: workspaceAttachments,
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
  trailingAttachments.insertAll(0, workspaceAttachments);

  return MessageParseResult(
    processedText: messageText.toString(),
    trailingAttachments: trailingAttachments,
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
      attachment.id.startsWith('media_pool:') ||
      attachment.type.startsWith('image/');
}
