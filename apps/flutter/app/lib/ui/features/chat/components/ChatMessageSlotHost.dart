// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../common/contributions/EmbeddedPluginSurfaceCatalog.dart';
import '../../../common/contributions/EmbeddedPluginSurfaceHost.dart';
import '../viewmodel/ChatViewModel.dart';

/// Serializes semantic message parts without deriving any plugin-specific feature model.
Map<String, Object?> pluginMessageState(ChatUiMessage message) => {
  'timestamp': message.timestamp,
  'variantIndex': message.selectedVariantIndex,
  'variantCount': message.variantCount,
  'sender': message.sender,
  'parts': [for (final part in message.parts) part.toJson()],
};

/// Hosts plugin-owned content after a message without interpreting tool results.
class ChatMessageSlotHost extends StatelessWidget {
  /// Binds visible plugin instances to one exact message and answer revision.
  const ChatMessageSlotHost({
    super.key,
    required this.catalog,
    required this.chatId,
    required this.message,
    required this.isStreaming,
    required this.onActionResult,
  });

  final EmbeddedPluginSurfaceCatalog catalog;
  final String chatId;
  final ChatUiMessage message;
  final bool isStreaming;
  final void Function(EmbeddedPluginSurfaceEntry contribution, Object? result)
  onActionResult;

  /// Passes semantic parts, not a host-computed domain-specific presentation.
  @override
  Widget build(BuildContext context) => EmbeddedPluginSurfaceHost(
    catalog: catalog,
    surface: 'chat_message_after',
    instanceIdentity: (chatId, message.timestamp, message.selectedVariantIndex),
    hostState: {
      'chatId': chatId,
      'isStreaming': isStreaming,
      'message': pluginMessageState(message),
    },
    onActionResult: onActionResult,
  );
}
