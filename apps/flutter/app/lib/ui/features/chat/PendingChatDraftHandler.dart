// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

class PendingChatDraftHandler {
  const PendingChatDraftHandler._();

  static final ValueNotifier<int> revision = ValueNotifier<int>(0);
  static String? _pendingDraft;

  static void setPendingDraft(String draft) {
    _pendingDraft = draft;
    revision.value += 1;
  }

  static String? takePendingDraft() {
    final draft = _pendingDraft;
    _pendingDraft = null;
    return draft;
  }
}
