// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'WorkspaceUserscriptCapabilityRegistry.dart';
import 'WorkspaceUserscriptMatcher.dart';
import 'WorkspaceUserscriptMetadataParser.dart';
import 'WorkspaceUserscriptModels.dart';

class WorkspaceUserscriptStore {
  WorkspaceUserscriptStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
    required Future<String> Function() storagePath,
  }) : _runtimeStorage = runtimeStorage,
       _storagePath = storagePath;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;
  final Future<String> Function() _storagePath;
  final List<WorkspaceUserscriptItem> _items = <WorkspaceUserscriptItem>[];
  final List<WorkspaceUserscriptLogItem> _logs = <WorkspaceUserscriptLogItem>[];
  final List<WorkspaceUserscriptPageRun> _pageRuns =
      <WorkspaceUserscriptPageRun>[];

  List<WorkspaceUserscriptItem> get items =>
      List<WorkspaceUserscriptItem>.unmodifiable(_items);

  List<WorkspaceUserscriptLogItem> get logs =>
      List<WorkspaceUserscriptLogItem>.unmodifiable(_logs);

  List<WorkspaceUserscriptPageRun> get pageRuns =>
      List<WorkspaceUserscriptPageRun>.unmodifiable(_pageRuns);

  Future<void> load() async {
    final content = await _runtimeStorage.readText(path: await _storagePath());
    if (content == null) {
      return;
    }
    final decoded = jsonDecode(content) as List<Object?>;
    _items
      ..clear()
      ..addAll(
        decoded.map(
          (item) =>
              WorkspaceUserscriptItem.fromJson(item as Map<String, Object?>),
        ),
      );
  }

  WorkspaceUserscriptInstallPreview createInstallPreview(
    String source, {
    String? sourceUrl,
  }) {
    final metadata = parseWorkspaceUserscriptMetadata(source);
    final knownGrants = WorkspaceUserscriptCapabilityRegistry.knownGrants(
      metadata.grants,
    );
    final unknownGrants = WorkspaceUserscriptCapabilityRegistry.unknownGrants(
      metadata.grants,
    );
    final warningReasons = WorkspaceUserscriptCapabilityRegistry.warningReasons(
      metadata.grants,
    );
    return WorkspaceUserscriptInstallPreview(
      source: source,
      metadata: metadata,
      knownGrants: knownGrants,
      unknownGrants: unknownGrants,
      blockedReasons: warningReasons,
      sourceUrl: sourceUrl,
    );
  }

  WorkspaceUserscriptItem installFromPreview(
    WorkspaceUserscriptInstallPreview preview,
  ) {
    final metadata = preview.metadata;
    final id = '${metadata.namespace}:${metadata.name}';
    final item = WorkspaceUserscriptItem(
      id: id,
      enabled: true,
      source: preview.source,
      metadata: metadata,
      knownGrants: preview.knownGrants,
      unknownGrants: preview.unknownGrants,
      blockedReasons: preview.blockedReasons,
      sourceUrl: preview.sourceUrl,
    );
    final index = _items.indexWhere((existing) => existing.id == id);
    if (index >= 0) {
      _items[index] = item;
    } else {
      _items.insert(0, item);
    }
    addLog(metadata.name, '已安装脚本');
    _persist();
    return item;
  }

  Future<void> refreshDependencies(String id) async {
    final index = _items.indexWhere((item) => item.id == id);
    if (index < 0) {
      return;
    }
    final item = _items[index];
    final requireSources = <String, String>{};
    for (final url in item.metadata.requires) {
      final response = await http.get(Uri.parse(url));
      requireSources[url] = response.body;
    }
    final resourceTexts = <String, String>{};
    final resourceUrls = <String, String>{};
    for (final resource in item.metadata.resources) {
      final response = await http.get(Uri.parse(resource.url));
      resourceTexts[resource.name] = response.body;
      resourceUrls[resource.name] =
          'data:application/octet-stream;base64,${base64Encode(response.bodyBytes)}';
    }
    _items[index] = item.copyWith(
      requireSources: requireSources,
      resourceTexts: resourceTexts,
      resourceUrls: resourceUrls,
    );
    addLog(item.metadata.name, '依赖和资源已缓存');
    _persist();
  }

  WorkspaceUserscriptItem installFromSource(
    String source, {
    String? sourceUrl,
  }) {
    return installFromPreview(
      createInstallPreview(source, sourceUrl: sourceUrl),
    );
  }

  bool hasUpdateSource(WorkspaceUserscriptItem item) {
    return item.metadata.updateUrl.trim().isNotEmpty ||
        item.metadata.downloadUrl.trim().isNotEmpty ||
        item.sourceUrl?.trim().isNotEmpty == true;
  }

  Future<bool> checkAndInstallUpdate(String id) async {
    final index = _items.indexWhere((item) => item.id == id);
    if (index < 0) {
      return false;
    }
    final item = _items[index];
    final updateUrl = _updateUrlFor(item);
    final sourceUri = Uri.parse(item.sourceUrl ?? updateUrl);
    final updateUri = Uri.parse(updateUrl);
    if (sourceUri.host.isNotEmpty &&
        updateUri.host.isNotEmpty &&
        sourceUri.host != updateUri.host) {
      addLog(item.metadata.name, '更新来源和安装来源不一致');
      return false;
    }
    final response = await http.get(updateUri);
    final preview = createInstallPreview(response.body, sourceUrl: updateUrl);
    if (preview.metadata.version == item.metadata.version) {
      addLog(item.metadata.name, '已经是最新版本');
      return false;
    }
    final updated = installFromPreview(preview);
    await refreshDependencies(updated.id);
    addLog(updated.metadata.name, '已更新到 ${updated.metadata.version}');
    return true;
  }

  void setEnabled(String id, bool enabled) {
    final index = _items.indexWhere((item) => item.id == id);
    if (index < 0) {
      return;
    }
    _items[index] = _items[index].copyWith(enabled: enabled);
    _persist();
  }

  void remove(String id) {
    _items.removeWhere((item) => item.id == id);
    _persist();
  }

  void setValue(String id, String key, Object? value) {
    final index = _items.indexWhere((item) => item.id == id);
    if (index < 0) {
      return;
    }
    final values = Map<String, Object?>.from(_items[index].values);
    values[key] = value;
    _items[index] = _items[index].copyWith(values: values);
    _persist();
  }

  void deleteValue(String id, String key) {
    final index = _items.indexWhere((item) => item.id == id);
    if (index < 0) {
      return;
    }
    final values = Map<String, Object?>.from(_items[index].values);
    values.remove(key);
    _items[index] = _items[index].copyWith(values: values);
    _persist();
  }

  void handleStorageMessage(String rawMessage) {
    final message = jsonDecode(rawMessage) as Map<String, Object?>;
    final id = message['id'] as String;
    final key = message['key'] as String;
    final action = message['action'] as String;
    if (action == 'set') {
      setValue(id, key, message['value']);
      return;
    }
    if (action == 'delete') {
      deleteValue(id, key);
    }
  }

  void handleRuntimeMessage(String rawMessage) {
    final message = jsonDecode(rawMessage) as Map<String, Object?>;
    final scriptId = message['id'] as String;
    final scriptName = message['name'] as String;
    final url = message['url'] as String;
    final status = message['status'] as String;
    final text = message['message'] as String;
    _pageRuns.removeWhere(
      (item) => item.scriptId == scriptId && item.url == url,
    );
    _pageRuns.insert(
      0,
      WorkspaceUserscriptPageRun(
        scriptId: scriptId,
        scriptName: scriptName,
        url: url,
        status: status,
        message: text,
        createdAt: DateTime.now(),
      ),
    );
    if (_pageRuns.length > 120) {
      _pageRuns.removeRange(120, _pageRuns.length);
    }
    addLog(scriptName, text);
  }

  void addLog(String scriptName, String message) {
    _logs.insert(
      0,
      WorkspaceUserscriptLogItem(
        scriptName: scriptName,
        message: message,
        createdAt: DateTime.now(),
      ),
    );
    if (_logs.length > 200) {
      _logs.removeRange(200, _logs.length);
    }
  }

  List<WorkspaceUserscriptItem> scriptsForUrl(String url) {
    return _items
        .where((item) {
          return item.enabled && _matches(item.metadata, url);
        })
        .toList(growable: false);
  }

  void _persist() {
    unawaited(_persistAsync());
  }

  Future<void> _persistAsync() async {
    await _runtimeStorage.writeText(
      path: await _storagePath(),
      content: jsonEncode(
        _items.map((item) => item.toJson()).toList(growable: false),
      ),
    );
  }
}

bool _matches(WorkspaceUserscriptMetadata metadata, String url) {
  return WorkspaceUserscriptMatcher.matches(metadata, url);
}

String _updateUrlFor(WorkspaceUserscriptItem item) {
  final metadata = item.metadata;
  if (metadata.updateUrl.trim().isNotEmpty) {
    return metadata.updateUrl.trim();
  }
  if (metadata.downloadUrl.trim().isNotEmpty) {
    return metadata.downloadUrl.trim();
  }
  return item.sourceUrl?.trim() ?? '';
}
