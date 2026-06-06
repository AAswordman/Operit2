// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

class ChatSwitchRenderRequest {
  const ChatSwitchRenderRequest({
    required this.generation,
    required this.targetChatId,
  });

  final int generation;
  final String targetChatId;
}

class ChatSwitchRenderCoordinator {
  ChatSwitchRenderCoordinator._();

  static final ValueNotifier<ChatSwitchRenderRequest?> _requests =
      ValueNotifier<ChatSwitchRenderRequest?>(null);
  static int _generation = 0;

  static ValueListenable<ChatSwitchRenderRequest?> get requests => _requests;

  static void prepareForChat(String chatId) {
    _generation += 1;
    _requests.value = ChatSwitchRenderRequest(
      generation: _generation,
      targetChatId: chatId,
    );
  }

  static void clear() {
    _requests.value = null;
  }
}
