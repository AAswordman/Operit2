// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../viewmodel/ChatViewModel.dart';

const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

const List<String> _pastedImageMimeTypes = <String>[
  'image/png',
  'image/jpeg',
  'image/jpg',
  'image/webp',
  'image/gif',
  'image/bmp',
  'image/heic',
  'image/heif',
  'image/tiff',
  'image/svg+xml',
  'image/x-icon',
];

/// Builds the rich-content insertion config used by chat inputs.
ContentInsertionConfiguration? chatPastedImageContentInsertionConfiguration({
  required ValueChanged<List<PastedImageAttachmentPayload>>? onPasteImages,
}) {
  final handler = onPasteImages;
  if (handler == null) {
    return null;
  }
  return ContentInsertionConfiguration(
    allowedMimeTypes: _pastedImageMimeTypes,
    onContentInserted: (KeyboardInsertedContent content) {
      final bytes = content.data;
      if (bytes == null || bytes.isEmpty) {
        return;
      }
      handler(<PastedImageAttachmentPayload>[
        _pastedImagePayloadFromBytes(
          mimeType: content.mimeType,
          bytes: bytes,
          index: 0,
        ),
      ]);
    },
  );
}

/// Wraps a chat text field with image-aware paste handling.
class ChatPastedImageHandler extends StatefulWidget {
  /// Creates an input wrapper that attaches pasted image payloads.
  const ChatPastedImageHandler({
    super.key,
    required this.focusNode,
    required this.enabled,
    required this.onPasteImages,
    required this.child,
  });

  final FocusNode focusNode;
  final bool enabled;
  final ValueChanged<List<PastedImageAttachmentPayload>>? onPasteImages;
  final Widget child;

  /// Creates mutable state for host clipboard paste handling.
  @override
  State<ChatPastedImageHandler> createState() => _ChatPastedImageHandlerState();
}

class _ChatPastedImageHandlerState extends State<ChatPastedImageHandler> {
  /// Builds the input subtree with a host clipboard image reader.
  @override
  Widget build(BuildContext context) {
    return Actions(
      actions: <Type, Action<Intent>>{
        PasteTextIntent: _ChatPasteTextAction(
          onPasteFromHostClipboard: _pasteFromHostClipboard,
        ),
      },
      child: widget.child,
    );
  }

  /// Reads image payloads from the host clipboard after a paste intent.
  Future<void> _pasteFromHostClipboard() async {
    if (!_acceptsPasteInput()) {
      return;
    }
    try {
      final images = await _readHostClipboardImages();
      if (images.isNotEmpty) {
        widget.onPasteImages?.call(images);
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to read pasted images: $error\n$stackTrace');
    }
  }

  /// Reports whether paste handling belongs to this input instance.
  bool _acceptsPasteInput() {
    return mounted &&
        widget.enabled &&
        widget.focusNode.hasFocus &&
        widget.onPasteImages != null;
  }
}

class _ChatPasteTextAction extends Action<PasteTextIntent> {
  /// Creates a paste action that augments Flutter's text paste.
  _ChatPasteTextAction({required this.onPasteFromHostClipboard});

  final Future<void> Function() onPasteFromHostClipboard;

  /// Invokes the text paste action and schedules image extraction.
  @override
  Object? invoke(PasteTextIntent intent) {
    final result = callingAction?.invoke(intent);
    unawaited(onPasteFromHostClipboard());
    return result;
  }

  /// Mirrors the wrapped text editing action enabled state.
  @override
  bool get isActionEnabled => callingAction?.isActionEnabled ?? true;

  /// Mirrors the wrapped text editing action key handling.
  @override
  bool consumesKey(PasteTextIntent intent) {
    return callingAction?.consumesKey(intent) ?? false;
  }
}

/// Reads image payloads supplied by the native host clipboard.
Future<List<PastedImageAttachmentPayload>> _readHostClipboardImages() async {
  final raw = await _runtimeChannel.invokeMethod<Object?>(
    'readClipboardImages',
  );
  if (raw is! List<Object?>) {
    throw const FormatException('readClipboardImages must return a list');
  }
  final images = <PastedImageAttachmentPayload>[];
  for (var index = 0; index < raw.length; index += 1) {
    final item = raw[index];
    if (item is! Map<Object?, Object?>) {
      throw const FormatException('clipboard image item must be a map');
    }
    images.add(_pastedImagePayloadFromHostItem(item, index));
  }
  return images;
}

/// Builds one pasted image payload from a host clipboard map.
PastedImageAttachmentPayload _pastedImagePayloadFromHostItem(
  Map<Object?, Object?> item,
  int index,
) {
  final mimeType = item['mimeType'];
  final bytes = item['bytes'];
  if (mimeType is! String) {
    throw const FormatException('clipboard image mimeType must be a string');
  }
  if (bytes is! Uint8List || bytes.isEmpty) {
    throw const FormatException('clipboard image bytes must be non-empty');
  }
  return _pastedImagePayloadFromBytes(
    mimeType: mimeType,
    bytes: bytes,
    index: index,
  );
}

/// Builds one runtime attachment payload from image bytes.
PastedImageAttachmentPayload _pastedImagePayloadFromBytes({
  required String mimeType,
  required Uint8List bytes,
  required int index,
}) {
  final normalizedMimeType = mimeType.trim().toLowerCase();
  final extension = _extensionForImageMimeType(normalizedMimeType);
  final timestamp = DateTime.now().millisecondsSinceEpoch;
  final fileName = 'pasted_image_${timestamp}_$index.$extension';
  return PastedImageAttachmentPayload.fromBytes(
    fileName: fileName,
    mimeType: normalizedMimeType,
    bytes: bytes,
  );
}

/// Returns the file extension for a supported pasted-image MIME type.
String _extensionForImageMimeType(String mimeType) {
  return switch (mimeType) {
    'image/png' => 'png',
    'image/jpeg' || 'image/jpg' => 'jpg',
    'image/webp' => 'webp',
    'image/gif' => 'gif',
    'image/bmp' => 'bmp',
    'image/heic' => 'heic',
    'image/heif' => 'heif',
    'image/tiff' => 'tiff',
    'image/svg+xml' => 'svg',
    'image/x-icon' => 'ico',
    _ => throw ArgumentError.value(
      mimeType,
      'mimeType',
      'Unsupported pasted image MIME type',
    ),
  };
}
