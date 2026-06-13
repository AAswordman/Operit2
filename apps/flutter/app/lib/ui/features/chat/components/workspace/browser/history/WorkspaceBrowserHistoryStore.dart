// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';

class WorkspaceBrowserHistoryItem {
  const WorkspaceBrowserHistoryItem({
    required this.url,
    required this.title,
    required this.visitedAt,
  });

  final String url;
  final String title;
  final DateTime visitedAt;

  factory WorkspaceBrowserHistoryItem.fromJson(Map<String, Object?> json) {
    return WorkspaceBrowserHistoryItem(
      url: json['url'] as String,
      title: json['title'] as String,
      visitedAt: DateTime.parse(json['visitedAt'] as String),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'visitedAt': visitedAt.toIso8601String(),
    };
  }
}

class WorkspaceBrowserHistoryStore {
  WorkspaceBrowserHistoryStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
    required Future<String> Function() storagePath,
  }) : _runtimeStorage = runtimeStorage,
       _storagePath = storagePath;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;
  final Future<String> Function() _storagePath;
  final List<WorkspaceBrowserHistoryItem> _items =
      <WorkspaceBrowserHistoryItem>[];

  List<WorkspaceBrowserHistoryItem> get items =>
      List<WorkspaceBrowserHistoryItem>.unmodifiable(_items);

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
          (item) => WorkspaceBrowserHistoryItem.fromJson(
            item as Map<String, Object?>,
          ),
        ),
      );
  }

  void add({required String url, required String title}) {
    final normalizedTitle = title.trim().isEmpty ? url : title.trim();
    _items.removeWhere((item) => item.url == url);
    _items.insert(
      0,
      WorkspaceBrowserHistoryItem(
        url: url,
        title: normalizedTitle,
        visitedAt: DateTime.now(),
      ),
    );
    if (_items.length > 300) {
      _items.removeRange(300, _items.length);
    }
    _persist();
  }

  List<WorkspaceBrowserHistoryItem> search(String query) {
    final needle = query.trim().toLowerCase();
    if (needle.isEmpty) {
      return items;
    }
    return _items
        .where((item) {
          return item.title.toLowerCase().contains(needle) ||
              item.url.toLowerCase().contains(needle);
        })
        .toList(growable: false);
  }

  void clear() {
    _items.clear();
    _persist();
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
