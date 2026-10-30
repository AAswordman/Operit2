// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

@immutable
class ChatMessageLookupRequest {
  const ChatMessageLookupRequest(this.chatId, {required this.favoritesOnly});

  final String chatId;
  final bool favoritesOnly;
}

/// Retains a sidebar request until the selected main chat surface is mounted.
class ChatMessageLookupRequests {
  ChatMessageLookupRequests._();

  static final _requests = ValueNotifier<ChatMessageLookupRequest?>(null);
  static ValueListenable<ChatMessageLookupRequest?> get requests => _requests;

  static void open(String chatId, {bool favoritesOnly = false}) {
    _requests.value = ChatMessageLookupRequest(
      chatId,
      favoritesOnly: favoritesOnly,
    );
  }

  static bool consume(ChatMessageLookupRequest request) {
    if (!identical(_requests.value, request)) return false;
    _requests.value = null;
    return true;
  }
}
