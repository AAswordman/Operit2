// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

/// Renders a chat attachment image from client-readable attachment bytes.
class ChatAttachmentImagePreview extends StatefulWidget {
  /// Creates an image preview for one attachment path.
  const ChatAttachmentImagePreview({
    super.key,
    required this.attachmentPath,
    required this.fileName,
    this.fit = BoxFit.cover,
  });

  final String attachmentPath;
  final String fileName;
  final BoxFit fit;

  /// Creates mutable state for the attachment byte load.
  @override
  State<ChatAttachmentImagePreview> createState() =>
      _ChatAttachmentImagePreviewState();
}

class _ChatAttachmentImagePreviewState
    extends State<ChatAttachmentImagePreview> {
  late Future<Uint8List> _imageBytesFuture;

  /// Initializes the image byte request for the current attachment.
  @override
  void initState() {
    super.initState();
    _imageBytesFuture = _readAttachmentImageBytes(widget.attachmentPath);
  }

  /// Restarts the byte request when the attachment path changes.
  @override
  void didUpdateWidget(covariant ChatAttachmentImagePreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.attachmentPath != widget.attachmentPath) {
      _imageBytesFuture = _readAttachmentImageBytes(widget.attachmentPath);
    }
  }

  /// Builds the image preview for loaded attachment bytes.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return FutureBuilder<Uint8List>(
      future: _imageBytesFuture,
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          return Image.memory(
            snapshot.requireData,
            fit: widget.fit,
            gaplessPlayback: true,
            semanticLabel: widget.fileName,
          );
        }
        if (snapshot.hasError) {
          return ColoredBox(
            color: colorScheme.surfaceContainerHighest,
            child: Center(
              child: Icon(
                Icons.broken_image_outlined,
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          );
        }
        return Center(
          child: SizedBox.square(
            dimension: 18,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: colorScheme.primary,
            ),
          ),
        );
      },
    );
  }
}

/// Reads attachment image bytes through Flutter's cross-platform file wrapper.
Future<Uint8List> _readAttachmentImageBytes(String attachmentPath) {
  return XFile(_localPathForAttachment(attachmentPath)).readAsBytes();
}

/// Converts a file URI attachment id into the local path used by XFile.
String _localPathForAttachment(String attachmentPath) {
  final uri = Uri.tryParse(attachmentPath);
  if (uri != null && uri.scheme == 'file') {
    return uri.toFilePath();
  }
  return attachmentPath;
}
