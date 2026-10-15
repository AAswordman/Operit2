// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class DrawerConversationState {
  /// Carries only authoritative generic conversation state for sidebar surfaces.
  const DrawerConversationState({
    this.histories = const <core_proxy.ChatHistoryListItem>[],
    this.activeStreamingChatIds = const <String>{},
    this.currentChatId,
    this.errorMessage,
    this.loading = true,
  });

  final List<core_proxy.ChatHistoryListItem> histories;
  final Set<String> activeStreamingChatIds;
  final String? currentChatId;
  final String? errorMessage;
  final bool loading;
}
