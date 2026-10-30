// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../data/preferences/UserPreferencesManager.dart';

/// Inherits only the selected chat's workspace while the native tab is selected.
Future<String?> sidebarNewChatWorkspaceId({
  required GeneratedCoreProxyClients clients,
  required List<core_proxy.ChatHistoryListItem> histories,
  required String? currentChatId,
}) async {
  final tabId = await UserPreferencesManager(
    clients: clients,
  ).loadChatSidebarTab();
  if (tabId != UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB) return null;
  for (final history in histories) {
    if (history.id == currentChatId) return history.workspaceId;
  }
  return null;
}
