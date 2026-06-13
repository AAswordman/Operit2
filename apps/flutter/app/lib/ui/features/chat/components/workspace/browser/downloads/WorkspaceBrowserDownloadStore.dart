// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';

enum WorkspaceBrowserDownloadState {
  pending,
  running,
  completed,
  failed,
  paused,
  cancelled,
}

class WorkspaceBrowserDownloadItem {
  const WorkspaceBrowserDownloadItem({
    required this.url,
    required this.fileName,
    required this.state,
    required this.createdAt,
    this.progress = 0,
    this.detail,
    this.savedPath,
  });

  final String url;
  final String fileName;
  final WorkspaceBrowserDownloadState state;
  final DateTime createdAt;
  final int progress;
  final String? detail;
  final String? savedPath;

  WorkspaceBrowserDownloadItem copyWith({
    WorkspaceBrowserDownloadState? state,
    int? progress,
    String? detail,
    String? savedPath,
  }) {
    return WorkspaceBrowserDownloadItem(
      url: url,
      fileName: fileName,
      state: state ?? this.state,
      createdAt: createdAt,
      progress: progress ?? this.progress,
      detail: detail ?? this.detail,
      savedPath: savedPath ?? this.savedPath,
    );
  }

  factory WorkspaceBrowserDownloadItem.fromJson(Map<String, Object?> json) {
    return WorkspaceBrowserDownloadItem(
      url: json['url'] as String,
      fileName: json['fileName'] as String,
      state: WorkspaceBrowserDownloadState.values[json['stateIndex'] as int],
      createdAt: DateTime.parse(json['createdAt'] as String),
      progress: json['progress'] as int,
      detail: json['detail'] as String?,
      savedPath: json['savedPath'] as String?,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'fileName': fileName,
      'stateIndex': state.index,
      'createdAt': createdAt.toIso8601String(),
      'progress': progress,
      'detail': detail,
      'savedPath': savedPath,
    };
  }
}

class WorkspaceBrowserDownloadStore extends ChangeNotifier {
  WorkspaceBrowserDownloadStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
    required Future<String> Function() storagePath,
    required Future<String> Function() downloadDirectoryPath,
  }) : _runtimeStorage = runtimeStorage,
       _storagePath = storagePath,
       _downloadDirectoryPath = downloadDirectoryPath;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;
  final Future<String> Function() _storagePath;
  final Future<String> Function() _downloadDirectoryPath;
  final List<WorkspaceBrowserDownloadItem> _items =
      <WorkspaceBrowserDownloadItem>[];
  final Map<String, http.Client> _activeClients = <String, http.Client>{};
  final Set<String> _cancelledUrls = <String>{};
  Future<void> Function(String path, Uint8List bytes)? _saveToWorkspace;

  List<WorkspaceBrowserDownloadItem> get items =>
      List<WorkspaceBrowserDownloadItem>.unmodifiable(_items);

  void setWorkspaceSaver(
    Future<void> Function(String path, Uint8List bytes) saveToWorkspace,
  ) {
    _saveToWorkspace = saveToWorkspace;
  }

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
          (item) => WorkspaceBrowserDownloadItem.fromJson(
            item as Map<String, Object?>,
          ),
        ),
      );
  }

  Future<void> startDownload(String url) async {
    _items.removeWhere((item) => item.url == url);
    final item = WorkspaceBrowserDownloadItem(
      url: url,
      fileName: _fileNameForUrl(url),
      state: WorkspaceBrowserDownloadState.running,
      createdAt: DateTime.now(),
    );
    _items.insert(0, item);
    _persist();
    notifyListeners();

    try {
      final client = http.Client();
      _activeClients[url] = client;
      final request = http.Request('GET', Uri.parse(url));
      final response = await client.send(request);
      final totalBytes = response.contentLength;
      final chunks = <int>[];
      var receivedBytes = 0;
      await for (final chunk in response.stream) {
        if (_cancelledUrls.contains(url)) {
          return;
        }
        chunks.addAll(chunk);
        receivedBytes += chunk.length;
        if (totalBytes != null && totalBytes > 0) {
          _update(
            url,
            state: WorkspaceBrowserDownloadState.running,
            progress: ((receivedBytes / totalBytes) * 100).clamp(0, 99).round(),
            detail: '',
          );
        }
      }
      final bytes = Uint8List.fromList(chunks);
      final savedPath = 'downloads/${_safeFileName(item.fileName)}';
      final downloadDirectory = await _downloadDirectoryPath();
      await _runtimeStorage.writeBase64(
        path: '$downloadDirectory/${_safeFileName(item.fileName)}',
        base64Content: base64Encode(bytes),
      );
      final saveToWorkspace = _saveToWorkspace;
      if (saveToWorkspace != null) {
        await saveToWorkspace(savedPath, bytes);
      }
      _update(
        url,
        state: WorkspaceBrowserDownloadState.completed,
        progress: 100,
        detail: '',
        savedPath: savedPath,
      );
    } on Object catch (error) {
      if (_cancelledUrls.contains(url)) {
        return;
      }
      _update(
        url,
        state: WorkspaceBrowserDownloadState.failed,
        detail: error.toString(),
      );
    } finally {
      _activeClients.remove(url)?.close();
      _cancelledUrls.remove(url);
    }
  }

  Future<void> retry(WorkspaceBrowserDownloadItem item) {
    return startDownload(item.url);
  }

  void pause(WorkspaceBrowserDownloadItem item) {
    _cancelledUrls.add(item.url);
    _activeClients.remove(item.url)?.close();
    _update(item.url, state: WorkspaceBrowserDownloadState.paused, detail: '');
  }

  Future<void> resume(WorkspaceBrowserDownloadItem item) {
    return startDownload(item.url);
  }

  void cancel(WorkspaceBrowserDownloadItem item) {
    _cancelledUrls.add(item.url);
    _activeClients.remove(item.url)?.close();
    _update(
      item.url,
      state: WorkspaceBrowserDownloadState.cancelled,
      detail: '',
    );
  }

  void remove(String url) {
    _items.removeWhere((item) => item.url == url);
    _persist();
    notifyListeners();
  }

  void _update(
    String url, {
    WorkspaceBrowserDownloadState? state,
    int? progress,
    String? detail,
    String? savedPath,
  }) {
    final index = _items.indexWhere((item) => item.url == url);
    if (index < 0) {
      return;
    }
    _items[index] = _items[index].copyWith(
      state: state,
      progress: progress,
      detail: detail,
      savedPath: savedPath,
    );
    _persist();
    notifyListeners();
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

String _fileNameForUrl(String url) {
  final segments = Uri.tryParse(url)?.pathSegments;
  if (segments == null || segments.isEmpty || segments.last.trim().isEmpty) {
    return 'download';
  }
  return segments.last;
}

String _safeFileName(String fileName) {
  return fileName.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_');
}
