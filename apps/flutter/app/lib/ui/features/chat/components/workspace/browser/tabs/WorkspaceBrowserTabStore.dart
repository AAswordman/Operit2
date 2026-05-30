// ignore_for_file: file_names

import 'dart:convert';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';

class WorkspaceBrowserSavedTabs {
  const WorkspaceBrowserSavedTabs({
    required this.selectedIndex,
    required this.tabs,
  });

  final int selectedIndex;
  final List<WorkspaceBrowserSavedTab> tabs;

  factory WorkspaceBrowserSavedTabs.fromJson(Map<String, Object?> json) {
    final tabsJson = json['tabs'] as List<Object?>;
    return WorkspaceBrowserSavedTabs(
      selectedIndex: json['selectedIndex'] as int,
      tabs: tabsJson
          .map(
            (item) =>
                WorkspaceBrowserSavedTab.fromJson(item as Map<String, Object?>),
          )
          .toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'selectedIndex': selectedIndex,
      'tabs': tabs.map((tab) => tab.toJson()).toList(growable: false),
    };
  }
}

class WorkspaceBrowserSavedTab {
  const WorkspaceBrowserSavedTab({
    required this.url,
    required this.title,
    this.localFilePath,
  });

  final String url;
  final String title;
  final String? localFilePath;

  factory WorkspaceBrowserSavedTab.fromJson(Map<String, Object?> json) {
    return WorkspaceBrowserSavedTab(
      url: json['url'] as String,
      title: json['title'] as String,
      localFilePath: json['localFilePath'] as String?,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'localFilePath': localFilePath,
    };
  }
}

class WorkspaceBrowserTabStore {
  WorkspaceBrowserTabStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
  }) : _runtimeStorage = runtimeStorage;

  static const String _storagePath = 'workspace_browser/tabs.json';

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;

  Future<WorkspaceBrowserSavedTabs?> load() async {
    final content = await _runtimeStorage.readText(path: _storagePath);
    if (content == null) {
      return null;
    }
    return WorkspaceBrowserSavedTabs.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }

  Future<void> save(WorkspaceBrowserSavedTabs tabs) {
    return _runtimeStorage.writeText(
      path: _storagePath,
      content: jsonEncode(tabs.toJson()),
    );
  }
}
