// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';

class WorkspaceBrowserBookmarkItem {
  const WorkspaceBrowserBookmarkItem({
    required this.url,
    required this.title,
    required this.createdAt,
  });

  final String url;
  final String title;
  final DateTime createdAt;

  factory WorkspaceBrowserBookmarkItem.fromJson(Map<String, Object?> json) {
    return WorkspaceBrowserBookmarkItem(
      url: json['url'] as String,
      title: json['title'] as String,
      createdAt: DateTime.parse(json['createdAt'] as String),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'createdAt': createdAt.toIso8601String(),
    };
  }
}

class WorkspaceBrowserBookmarkStore {
  WorkspaceBrowserBookmarkStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
    required Future<String> Function() storagePath,
  }) : _runtimeStorage = runtimeStorage,
       _storagePath = storagePath;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;
  final Future<String> Function() _storagePath;
  final List<WorkspaceBrowserBookmarkItem> _items =
      <WorkspaceBrowserBookmarkItem>[];

  List<WorkspaceBrowserBookmarkItem> get items =>
      List<WorkspaceBrowserBookmarkItem>.unmodifiable(_items);

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
          (item) => WorkspaceBrowserBookmarkItem.fromJson(
            item as Map<String, Object?>,
          ),
        ),
      );
  }

  bool contains(String url) {
    return _items.any((item) => item.url == url);
  }

  void toggle({required String url, required String title}) {
    final existingIndex = _items.indexWhere((item) => item.url == url);
    if (existingIndex >= 0) {
      _items.removeAt(existingIndex);
      _persist();
      return;
    }
    _items.insert(
      0,
      WorkspaceBrowserBookmarkItem(
        url: url,
        title: title.trim().isEmpty ? url : title.trim(),
        createdAt: DateTime.now(),
      ),
    );
    _persist();
  }

  void remove(String url) {
    _items.removeWhere((item) => item.url == url);
    _persist();
  }

  void _persist() {
    unawaited(
      _persistAsync(),
    );
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
