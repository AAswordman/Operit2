// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

import '../../../../common/components/OperitDialog.dart';
import '../style/input/common/ChatAttachmentImagePreview.dart';
import '../workspace/file_preview/WorkspaceMediaPreviewWidgets.dart';

const GeneratedCoreProxyClients _attachmentMediaClients =
    GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
    );

class ChatAttachment {
  const ChatAttachment({
    required this.id,
    required this.filename,
    required this.mimeType,
    this.size = 0,
    this.content = '',
    this.mediaPoolType,
  });

  final String id;
  final String filename;
  final String mimeType;
  final int size;
  final String content;
  final String? mediaPoolType;
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
    if (attachment.mimeType.startsWith('image/')) {
      return ClipRRect(
        borderRadius: BorderRadius.circular(8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 420),
          child: ChatAttachmentImagePreview(
            attachmentPath: attachment.id,
            fileName: attachment.filename,
            mediaPoolType: attachment.mediaPoolType,
            fit: BoxFit.contain,
          ),
        ),
      );
    }

    if (attachment.mimeType.startsWith('audio/') ||
        attachment.mimeType.startsWith('video/')) {
      return _MediaPoolAttachmentPreview(attachment: attachment);
    }

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

/// Loads audio or video bytes from a media pool id or an ordinary attachment path.
class _MediaPoolAttachmentPreview extends StatelessWidget {
  const _MediaPoolAttachmentPreview({required this.attachment});

  final ChatAttachment attachment;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Uint8List>(
      future: _readAttachmentMediaBytes(attachment),
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Text(
            snapshot.error.toString(),
            style: Theme.of(context).textTheme.bodySmall,
          );
        }
        if (!snapshot.hasData) {
          return const Center(child: CircularProgressIndicator());
        }
        final bytes = snapshot.requireData;
        if (attachment.mimeType.startsWith('audio/')) {
          return WorkspaceAudioPreview(
            bytes: bytes,
            title: attachment.filename,
          );
        }
        return WorkspaceVideoPreview(
          bytes: bytes,
          fileName: attachment.filename,
        );
      },
    );
  }
}

/// Reads attachment media bytes through the runtime pool bridge or file host.
Future<Uint8List> _readAttachmentMediaBytes(ChatAttachment attachment) async {
  if (attachment.mediaPoolType != null) {
    final data = await _attachmentMediaClients
        .servicesRuntimeHostInteractionService
        .getMediaPoolData(
          mediaType: attachment.mediaPoolType!,
          id: attachment.id,
        );
    return base64Decode(data.base64);
  }
  return readChatAttachmentFileBytes(attachment.id);
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
