// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

class ChatSelectionTransitionRequest {
  /// Creates one visual chat-selection transition request.
  const ChatSelectionTransitionRequest({
    required this.generation,
    required this.chatId,
  });

  final int generation;
  final String chatId;
}

class ChatSelectionTransition {
  ChatSelectionTransition._();

  static final ValueNotifier<ChatSelectionTransitionRequest?> _requests =
      ValueNotifier<ChatSelectionTransitionRequest?>(null);
  static int _generation = 0;

  /// Exposes the active chat-selection transition request.
  static ValueListenable<ChatSelectionTransitionRequest?> get requests {
    return _requests;
  }

  /// Starts a visual transition for a selected chat.
  static void begin(String chatId) {
    _generation += 1;
    _requests.value = ChatSelectionTransitionRequest(
      generation: _generation,
      chatId: chatId,
    );
  }

  /// Completes the active visual transition for the selected chat.
  static void complete(String chatId) {
    final request = _requests.value;
    if (request != null && request.chatId == chatId) {
      _requests.value = null;
    }
  }
}
