// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class DrawerConversationState {
  const DrawerConversationState({
    this.histories = const <core_proxy.ChatHistoryListItem>[],
    this.characterGroupNamesById = const <String, String>{},
    this.currentChatId,
    this.errorMessage,
    this.loading = true,
  });

  final List<core_proxy.ChatHistoryListItem> histories;
  final Map<String, String> characterGroupNamesById;
  final String? currentChatId;
  final String? errorMessage;
  final bool loading;
}
