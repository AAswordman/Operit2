// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Defines the finite generic text and committed-file presentation results.
sealed class ChatAttachmentResult {
  /// Creates a result without inferring domain-specific attachment ownership.
  const ChatAttachmentResult();

  /// Rejects unknown types, fields, or malformed data instead of coercing them.
  factory ChatAttachmentResult.fromPresentationValue(Object? value) {
    final data = _attachmentObject(value, 'attachment result');
    switch (data['type']) {
      case 'text':
        _attachmentFields(data, const <String>{
          'type',
          'name',
          'content',
          'mediaType',
        }, 'text result');
        if (data['mediaType'] != 'text/plain' || data['content'] is! String) {
          throw const FormatException(
            'Text completion requires plain text content.',
          );
        }
        return ChatUiTextAttachment(
          name: _attachmentName(data['name']),
          content: data['content'] as String,
        );
      case 'file':
        _attachmentFields(data, const <String>{
          'type',
          'attachment',
        }, 'file result');
        final file = _attachmentObject(data['attachment'], 'file metadata');
        _attachmentFields(file, const <String>{
          'filePath',
          'nodeId',
          'fileName',
          'mimeType',
          'fileSize',
          'content',
        }, 'file metadata');
        final nodeId = file['nodeId'];
        final size = file['fileSize'];
        if ((nodeId != null && (nodeId is! String || nodeId.trim().isEmpty)) ||
            size is! int ||
            size < 0 ||
            file['content'] != '') {
          throw const FormatException(
            'File completion requires valid committed-file metadata.',
          );
        }
        return ChatUiFileAttachment(
          attachment: core_proxy.AttachmentInfo(
            filePath: _attachmentText(file['filePath'], 'filePath'),
            nodeId: nodeId as String?,
            fileName: _attachmentName(file['fileName']),
            mimeType: _attachmentText(file['mimeType'], 'mimeType'),
            fileSize: size,
            content: '',
          ),
        );
      default:
        throw const FormatException(
          'Attachment completion type must be text or file.',
        );
    }
  }
}

/// Preserves the name and content supplied by a plain-text attachment picker.
class ChatUiTextAttachment extends ChatAttachmentResult {
  /// Creates a validated plain-text result for the existing streamed upload path.
  const ChatUiTextAttachment({required this.name, required this.content});

  final String name;
  final String content;

  /// Identifies the media type locked by the plain-text completion contract.
  String get mediaType => 'text/plain';
}

/// Carries an existing Host upload that still requires canonical runtime validation.
class ChatUiFileAttachment extends ChatAttachmentResult {
  /// Creates a file result without copying bytes or inventing trusted metadata.
  const ChatUiFileAttachment({required this.attachment});

  final core_proxy.AttachmentInfo attachment;
}

/// Requires an object with exact JSON keys before reading completion fields.
Map<String, Object?> _attachmentObject(Object? value, String path) {
  if (value is! Map || value.keys.any((key) => key is! String)) {
    throw FormatException('$path must be an object.');
  }
  return value.map((key, value) => MapEntry(key as String, value));
}

/// Enforces the declared result shape without recognizing aliases or domain data.
void _attachmentFields(
  Map<String, Object?> value,
  Set<String> keys,
  String path,
) {
  if (value.length != keys.length || !keys.every(value.containsKey)) {
    throw FormatException('$path has invalid fields.');
  }
}

/// Requires nonblank file metadata without changing its persisted spelling.
String _attachmentText(Object? value, String path) {
  if (value is! String || value.trim().isEmpty) {
    throw FormatException('$path must be nonblank text.');
  }
  return value;
}

/// Rejects path-like or unnamed values before allocating an upload session.
String _attachmentName(Object? value) {
  final name = _attachmentText(value, 'attachment name');
  if (name == '.' || name == '..' || RegExp(r'[/\\\x00]').hasMatch(name)) {
    throw const FormatException('Attachment name must be a file name.');
  }
  return name;
}

/// Captures a composer selection so returning to the same chat cannot revive it.
class ChatAttachmentRequest {
  /// Records the chat and generation before its plugin presentation is opened.
  const ChatAttachmentRequest({
    required this.chatId,
    required this.chatSelectionGeneration,
  });

  final String? chatId;
  final int chatSelectionGeneration;

  /// Requires the same completed selection rather than matching only a chat ID.
  bool isCurrent({
    required String? currentChatId,
    required String? requestedChatId,
    required int currentSelectionGeneration,
    required bool switching,
  }) =>
      !switching &&
      currentChatId == chatId &&
      requestedChatId == chatId &&
      currentSelectionGeneration == chatSelectionGeneration;
}
