// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/components/OperitDialog.dart';

class ChatAttachment {
  const ChatAttachment({
    required this.id,
    required this.filename,
    required this.mimeType,
    this.size = 0,
    this.content = '',
  });

  final String id;
  final String filename;
  final String mimeType;
  final int size;
  final String content;
}

class AttachmentViewerDialog extends StatelessWidget {
  const AttachmentViewerDialog({
    super.key,
    required this.visible,
    required this.attachment,
    required this.onDismiss,
  });

  final bool visible;
  final ChatAttachment? attachment;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final attachment = this.attachment;
    if (!visible || attachment == null) {
      return const SizedBox.shrink();
    }

    final isImage = attachment.mimeType.startsWith('image/');
    final isAudio = attachment.mimeType.startsWith('audio/');
    final isVideo = attachment.mimeType.startsWith('video/');
    final isTextLike = isTextLikeMimeType(attachment.mimeType);

    return OperitDialogScaffold(
      title: attachment.filename,
      icon: Icon(
        _attachmentIcon(isImage: isImage, isAudio: isAudio, isVideo: isVideo),
      ),
      maxWidth: 720,
      maxHeight: 520,
      showCloseButton: true,
      onClose: onDismiss,
      child: SingleChildScrollView(
        child: _AttachmentPreview(
          attachment: attachment,
          isTextLike: isTextLike,
        ),
      ),
    );
  }
}

class _AttachmentPreview extends StatelessWidget {
  const _AttachmentPreview({
    required this.attachment,
    required this.isTextLike,
  });

  final ChatAttachment attachment;
  final bool isTextLike;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (isTextLike || attachment.content.isNotEmpty) {
      return SelectableText(
        attachment.content,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurface,
          fontFamily: 'monospace',
          height: 1.45,
        ),
      );
    }

    return Text(
      '${attachment.mimeType}\n${attachment.size} bytes\n${attachment.id}',
      style: theme.textTheme.bodyMedium?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
        height: 1.45,
      ),
    );
  }
}

bool isTextLikeMimeType(String mimeType) {
  return mimeType.startsWith('text/') ||
      mimeType == 'application/json' ||
      mimeType == 'application/xml' ||
      mimeType == 'application/vnd.workspace-context+xml';
}

IconData _attachmentIcon({
  required bool isImage,
  required bool isAudio,
  required bool isVideo,
}) {
  if (isImage) {
    return Icons.image;
  }
  if (isAudio) {
    return Icons.volume_up;
  }
  if (isVideo) {
    return Icons.play_arrow;
  }
  return Icons.description;
}
