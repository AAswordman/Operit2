// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:webview_all/webview_all.dart';

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
  /// Describes one owner download and its actual persisted file.
  const WorkspaceBrowserDownloadItem({
    required this.url,
    String? id,
    this.canRetry = true,
    required this.fileName,
    required this.state,
    required this.createdAt,
    this.progress = 0,
    this.receivedBytes = 0,
    this.totalBytes,
    this.detail,
    this.savedPath,
    this.runtimePath,
  }) : id = id ?? url;

  final String id;
  final bool canRetry;
  final String url;
  final String fileName;
  final WorkspaceBrowserDownloadState state;
  final DateTime createdAt;
  final int progress;
  final int receivedBytes;
  final int? totalBytes;
  final String? detail;
  // The explicit export target, relative to the selected workspace.
  final String? savedPath;
  final String? runtimePath;

  /// Copies transfer metadata without changing this download's identity.
  WorkspaceBrowserDownloadItem copyWith({
    String? fileName,
    WorkspaceBrowserDownloadState? state,
    int? progress,
    int? receivedBytes,
    int? totalBytes,
    String? detail,
    String? savedPath,
    String? runtimePath,
  }) {
    return WorkspaceBrowserDownloadItem(
      url: url,
      id: id,
      canRetry: canRetry,
      fileName: fileName ?? this.fileName,
      state: state ?? this.state,
      createdAt: createdAt,
      progress: progress ?? this.progress,
      receivedBytes: receivedBytes ?? this.receivedBytes,
      totalBytes: totalBytes ?? this.totalBytes,
      detail: detail ?? this.detail,
      savedPath: savedPath ?? this.savedPath,
      runtimePath: runtimePath ?? this.runtimePath,
    );
  }

  /// Reads persisted records, including the original percentage-only schema.
  factory WorkspaceBrowserDownloadItem.fromJson(Map<String, Object?> json) {
    return WorkspaceBrowserDownloadItem(
      url: json['url'] as String,
      id: json['id'] as String?,
      canRetry: json['canRetry'] as bool? ?? true,
      fileName: json['fileName'] as String,
      state: WorkspaceBrowserDownloadState.values[json['stateIndex'] as int],
      createdAt: DateTime.parse(json['createdAt'] as String),
      progress: json['progress'] as int,
      receivedBytes: json['receivedBytes'] as int? ?? 0,
      totalBytes: json['totalBytes'] as int?,
      detail: json['detail'] as String?,
      savedPath: json['savedPath'] as String?,
      runtimePath: json['runtimePath'] as String?,
    );
  }

  /// Encodes one record for persistence and owner event snapshots.
  Map<String, Object?> toJson() => <String, Object?>{
    'url': url,
    'id': id,
    'canRetry': canRetry,
    'fileName': fileName,
    'stateIndex': state.index,
    'createdAt': createdAt.toIso8601String(),
    'progress': progress,
    'receivedBytes': receivedBytes,
    'totalBytes': totalBytes,
    'detail': detail,
    'savedPath': savedPath,
    'runtimePath': runtimePath,
  };
}

class WorkspaceBrowserDownloadStore extends ChangeNotifier {
  /// Creates the authoritative owner store, or an observing controller store.
  WorkspaceBrowserDownloadStore({
    required GeneratedRepositoryRuntimeStorageRepositoryCoreProxy
    runtimeStorage,
    required Future<String> Function() storagePath,
    required Future<String> Function() downloadDirectoryPath,
    this.observer = false,
    http.Client Function()? clientFactory,
  }) : _runtimeStorage = runtimeStorage,
       _storagePath = storagePath,
       _downloadDirectoryPath = downloadDirectoryPath,
       _clientFactory = clientFactory ?? http.Client.new;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;
  final Future<String> Function() _storagePath;
  final Future<String> Function() _downloadDirectoryPath;
  final http.Client Function() _clientFactory;
  final bool observer;
  final List<WorkspaceBrowserDownloadItem> _items = [];
  final Map<String, http.Client> _activeClients = {};
  final Map<String, Future<void> Function()> _engineCancellation = {};
  final Map<String, Object> _engineTokens = {};
  final Set<String> _engineImporting = {};
  final Set<String> _engineDismissed = {};
  bool _disposed = false;
  int _nextFileId = 0;
  final Map<String, Map<String, String>> _requestHeaders = {};
  Future<void> Function(String action, String url)? remoteControl;
  String? _pendingPersistence;
  bool _persisting = false;
  int _revision = 0;
  final String _ownerEpoch = DateTime.now().microsecondsSinceEpoch.toString();
  String? _observedEpoch;

  /// Returns a stable read-only snapshot of the current downloads.
  List<WorkspaceBrowserDownloadItem> get items => List.unmodifiable(_items);

  /// Serializes the authoritative revision with its complete download list.
  String encodeSnapshot() => jsonEncode(<String, Object?>{
    'ownerEpoch': _ownerEpoch,
    'revision': _revision,
    'items': _items.map((item) => item.toJson()).toList(growable: false),
  });

  /// Applies ordered owner snapshots without allowing a late reply to regress UI.
  void applySnapshot(String encoded) {
    final decoded = jsonDecode(encoded) as Map<String, Object?>;
    final revision = decoded['revision'] as int;
    final epoch = decoded['ownerEpoch'] as String;
    if (epoch == _observedEpoch && revision < _revision) {
      return;
    }
    _observedEpoch = epoch;
    _revision = revision;
    _items
      ..clear()
      ..addAll(
        (decoded['items'] as List<Object?>).map(
          (item) => WorkspaceBrowserDownloadItem.fromJson(
            item as Map<String, Object?>,
          ),
        ),
      );
    notifyListeners();
  }

  /// Loads owner records and marks transfers interrupted by process termination.
  Future<void> load() async {
    if (observer) {
      return;
    }
    final content = await _runtimeStorage.readText(path: await _storagePath());
    if (content == null) {
      return;
    }
    final directory = await _downloadDirectoryPath();
    final decoded = jsonDecode(content) as List<Object?>;
    _items.clear();
    for (final value in decoded) {
      var item = WorkspaceBrowserDownloadItem.fromJson(
        value as Map<String, Object?>,
      );
      // Original records stored their files directly under the download directory.
      if (item.runtimePath == null && item.savedPath != null) {
        item = item.copyWith(
          runtimePath: '$directory/${_safeFileName(item.fileName)}',
        );
      }
      if (item.state == WorkspaceBrowserDownloadState.running ||
          item.state == WorkspaceBrowserDownloadState.pending) {
        item = item.copyWith(state: WorkspaceBrowserDownloadState.paused);
      }
      _items.add(item);
    }
    _changed(persist: true);
  }

  /// Downloads on the browser owner and publishes byte progress to every viewer.
  Future<void> startDownload(
    String url, {
    Map<String, String> headers = const {},
    String? contentDisposition,
    String? downloadId,
  }) async {
    if (observer) {
      throw StateError('Only the browser owner may start a download');
    }
    final id =
        downloadId ??
        'http:${DateTime.now().microsecondsSinceEpoch}:${_nextFileId++}';
    _activeClients.remove(id)?.close();
    final client = _clientFactory();
    _activeClients[id] = client;
    _requestHeaders[id] = Map.of(headers);
    final createdAt = DateTime.now();
    _items.removeWhere((item) => item.id == id);
    _items.insert(
      0,
      WorkspaceBrowserDownloadItem(
        url: url,
        id: id,
        fileName: _fileName(url, contentDisposition),
        state: WorkspaceBrowserDownloadState.pending,
        createdAt: createdAt,
      ),
    );
    _changed(persist: true);
    try {
      final request = http.Request('GET', Uri.parse(url))
        ..headers.addAll(headers);
      final response = await client.send(request);
      if (!_owns(id, client)) return;
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw http.ClientException(
          'HTTP ${response.statusCode} ${response.reasonPhrase ?? ''}',
          Uri.parse(url),
        );
      }
      final fileName = _fileName(
        url,
        response.headers['content-disposition'] ?? contentDisposition,
      );
      final storedName =
          '${createdAt.microsecondsSinceEpoch}_${_nextFileId++}_$fileName';
      final directory = await _downloadDirectoryPath();
      if (!_owns(id, client)) return;
      final runtimePath = '$directory/$storedName';
      final encoding = response.headers['content-encoding'];
      final total = encoding == null || encoding == 'identity'
          ? response.contentLength
          : null;
      _replace(
        id,
        (item) => item.copyWith(
          fileName: fileName,
          state: WorkspaceBrowserDownloadState.running,
          totalBytes: total,
          runtimePath: runtimePath,
          savedPath: 'downloads/$storedName',
        ),
      );
      final bytes = BytesBuilder(copy: false);
      final stopwatch = Stopwatch()..start();
      var lastNotification = 0;
      await for (final chunk in response.stream) {
        if (!_owns(id, client)) return;
        bytes.add(chunk);
        if (stopwatch.elapsedMilliseconds - lastNotification >= 100) {
          lastNotification = stopwatch.elapsedMilliseconds;
          _updateProgress(id, bytes.length, total);
        }
      }
      if (!_owns(id, client)) return;
      _updateProgress(id, bytes.length, total);
      if (total != null && total != bytes.length) {
        throw StateError('Incomplete download: ${bytes.length} / $total bytes');
      }
      await _runtimeStorage.writeBase64(
        path: runtimePath,
        base64Content: base64Encode(bytes.takeBytes()),
      );
      if (!_owns(id, client)) return;
      _replace(
        id,
        (item) => item.copyWith(
          state: WorkspaceBrowserDownloadState.completed,
          progress: 100,
          detail: '',
        ),
        persist: true,
      );
    } catch (error) {
      if (_owns(id, client)) {
        _replace(
          id,
          (item) => item.copyWith(
            state: WorkspaceBrowserDownloadState.failed,
            detail: error.toString(),
          ),
          persist: true,
        );
      }
    } finally {
      if (_owns(id, client)) _activeClients.remove(id);
      client.close();
    }
  }

  /// Accepts original engine responses without replaying POST bodies or blob URLs.
  Future<void> acceptEngineDownload(
    String sessionId,
    WebViewDownloadEvent event,
  ) async {
    if (observer) {
      throw StateError('Only the browser owner may accept downloads');
    }
    final id = '$sessionId:${event.id}';
    if (_disposed ||
        _engineImporting.contains(id) ||
        _engineDismissed.contains(id)) {
      return;
    }
    final index = _items.indexWhere((item) => item.id == id);
    if (index >= 0 &&
        _items[index].state != WorkspaceBrowserDownloadState.running) {
      return;
    }
    final token = _engineTokens.putIfAbsent(id, Object.new);
    if (index < 0) {
      _items.insert(
        0,
        WorkspaceBrowserDownloadItem(
          id: id,
          url: event.url,
          fileName: _safeFileName(event.fileName),
          canRetry: false,
          state: WorkspaceBrowserDownloadState.running,
          createdAt: DateTime.now(),
        ),
      );
    }
    if (event.cancel != null) _engineCancellation[id] = event.cancel!;
    _replace(
      id,
      (item) => item.copyWith(
        fileName: _safeFileName(event.fileName),
        receivedBytes: event.receivedBytes,
        totalBytes: event.totalBytes,
        detail: event.error,
        state: switch (event.state) {
          WebViewDownloadState.running || WebViewDownloadState.completed =>
            WorkspaceBrowserDownloadState.running,
          WebViewDownloadState.failed => WorkspaceBrowserDownloadState.failed,
          WebViewDownloadState.cancelled =>
            WorkspaceBrowserDownloadState.cancelled,
        },
        progress: event.totalBytes != null && event.totalBytes! > 0
            ? (event.receivedBytes * 100 / event.totalBytes!)
                  .clamp(0, 99)
                  .floor()
            : 0,
      ),
      persist: index < 0 || event.state != WebViewDownloadState.running,
    );
    if (event.state == WebViewDownloadState.failed ||
        event.state == WebViewDownloadState.cancelled) {
      _engineCancellation.remove(id);
      _engineTokens.remove(id);
      return;
    }
    if (event.state != WebViewDownloadState.completed) return;
    _engineImporting.add(id);
    _engineCancellation.remove(id);
    try {
      final reader = event.readBytes;
      if (reader == null) {
        throw StateError('The engine did not provide the completed file');
      }
      final bytes = await reader();
      if (_disposed || !identical(_engineTokens[id], token)) return;
      final directory = await _downloadDirectoryPath();
      if (_disposed || !identical(_engineTokens[id], token)) return;
      final item = _items.singleWhere((item) => item.id == id);
      final storedName =
          '${item.createdAt.microsecondsSinceEpoch}_${_nextFileId++}_${item.fileName}';
      final runtimePath = '$directory/$storedName';
      await _runtimeStorage.writeBase64(
        path: runtimePath,
        base64Content: base64Encode(bytes),
      );
      if (_disposed || !identical(_engineTokens[id], token)) return;
      _replace(
        id,
        (item) => item.copyWith(
          state: WorkspaceBrowserDownloadState.completed,
          progress: 100,
          receivedBytes: bytes.length,
          runtimePath: runtimePath,
          savedPath: 'downloads/$storedName',
          detail: '',
        ),
        persist: true,
      );
    } catch (error) {
      if (!_disposed && identical(_engineTokens[id], token)) {
        _replace(
          id,
          (item) => item.copyWith(
            state: WorkspaceBrowserDownloadState.failed,
            detail: error.toString(),
          ),
          persist: true,
        );
      }
    } finally {
      _engineImporting.remove(id);
      if (identical(_engineTokens[id], token)) {
        _engineCancellation.remove(id);
        _engineTokens.remove(id);
      }
    }
  }

  /// Marks engine transfers interrupted when their owning browser view closes.
  void closeEngineSession(String sessionId) {
    final prefix = '$sessionId:';
    for (final item in List.of(_items)) {
      if (!item.id.startsWith(prefix) ||
          item.state != WorkspaceBrowserDownloadState.running) {
        continue;
      }
      _engineTokens.remove(item.id);
      _engineCancellation.remove(item.id);
      _replace(
        item.id,
        (item) => item.copyWith(
          state: WorkspaceBrowserDownloadState.failed,
          detail:
              'The originating browser session closed before the download completed',
        ),
        persist: true,
      );
    }
  }

  /// Restarts a transfer explicitly; partial data is never advertised as resumable.
  Future<void> retry(WorkspaceBrowserDownloadItem item) async {
    if (!item.canRetry) {
      throw StateError('Restart this download from its original webpage');
    }
    if (observer) return _control('retry', item.id);
    unawaited(
      startDownload(
        item.url,
        downloadId: item.id,
        headers: _requestHeaders[item.id] ?? const {},
      ),
    );
  }

  /// Cancels the owner request before marking its row as cancelled.
  Future<void> cancel(WorkspaceBrowserDownloadItem item) async {
    if (observer) return _control('cancel', item.id);
    _engineTokens.remove(item.id);
    final cancelEngine = _engineCancellation.remove(item.id);
    if (cancelEngine != null) await cancelEngine();
    _activeClients.remove(item.id)?.close();
    _replace(
      item.id,
      (item) => item.copyWith(
        state: WorkspaceBrowserDownloadState.cancelled,
        detail: '',
      ),
      persist: true,
    );
  }

  /// Removes a record and stops any transfer still associated with it.
  Future<void> remove(String url) async {
    if (observer) return _control('remove', url);
    _engineDismissed.add(url);
    _engineTokens.remove(url);
    final cancelEngine = _engineCancellation.remove(url);
    if (cancelEngine != null) await cancelEngine();
    _activeClients.remove(url)?.close();
    _requestHeaders.remove(url);
    _items.removeWhere((item) => item.id == url);
    _changed(persist: true);
  }

  /// Copies a completed owner file into the explicitly selected workspace.
  Future<void> saveToWorkspace(
    String path,
    Future<void> Function(String path, Uint8List bytes) write,
  ) async {
    final item = _items.singleWhere((item) => item.savedPath == path);
    if (item.state != WorkspaceBrowserDownloadState.completed ||
        item.runtimePath == null) {
      throw StateError('Download has no completed owner file');
    }
    final encoded = await _runtimeStorage.readBase64(path: item.runtimePath!);
    if (encoded == null) throw StateError('Downloaded file no longer exists');
    await write(path, base64Decode(encoded));
  }

  /// Dispatches a controller action through the acknowledged Core command path.
  Future<void> _control(String action, String url) async {
    final control = remoteControl;
    if (control == null) {
      throw StateError('Browser download controls are not attached');
    }
    await control(action, url);
  }

  /// Checks that a late callback still belongs to the current transfer.
  bool _owns(String url, http.Client client) =>
      identical(_activeClients[url], client);

  /// Updates actual transferred bytes even when the server omits Content-Length.
  void _updateProgress(String url, int received, int? total) {
    _replace(
      url,
      (item) => item.copyWith(
        receivedBytes: received,
        progress: total != null && total > 0
            ? (received * 100 / total).clamp(0, 99).floor()
            : 0,
      ),
    );
  }

  /// Replaces one record and publishes a new authoritative revision.
  void _replace(
    String url,
    WorkspaceBrowserDownloadItem Function(WorkspaceBrowserDownloadItem)
    update, {
    bool persist = false,
  }) {
    final index = _items.indexWhere((item) => item.id == url);
    if (index < 0) return;
    _items[index] = update(_items[index]);
    _changed(persist: persist);
  }

  /// Notifies observers and persists lifecycle changes without per-chunk disk writes.
  void _changed({required bool persist}) {
    _revision++;
    if (persist) {
      _pendingPersistence = jsonEncode(
        _items.map((item) => item.toJson()).toList(),
      );
      if (!_persisting) unawaited(_persist());
    }
    notifyListeners();
  }

  /// Serializes state writes so an older snapshot cannot overwrite a newer one.
  Future<void> _persist() async {
    _persisting = true;
    try {
      while (_pendingPersistence != null) {
        final content = _pendingPersistence!;
        _pendingPersistence = null;
        await _runtimeStorage.writeText(
          path: await _storagePath(),
          content: content,
        );
      }
    } catch (error, stack) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stack,
          library: 'browser download persistence',
        ),
      );
    } finally {
      _persisting = false;
    }
  }

  /// Releases all HTTP resources owned by this store.
  @override
  void dispose() {
    _disposed = true;
    _engineTokens.clear();
    for (final cancel in _engineCancellation.values) {
      unawaited(cancel());
    }
    _engineCancellation.clear();
    for (final client in _activeClients.values) {
      client.close();
    }
    _activeClients.clear();
    super.dispose();
  }
}

/// Reads server-provided filenames using RFC 5987 and quoted filename parameters.
String _fileName(String url, String? disposition) {
  if (disposition != null) {
    final extended = RegExp(
      r"filename\*\s*=\s*UTF-8'[^']*'([^;]+)",
      caseSensitive: false,
    ).firstMatch(disposition);
    if (extended != null) {
      return _safeFileName(Uri.decodeComponent(extended[1]!.trim()));
    }
    final quoted = RegExp(
      r'filename\s*=\s*"([^"]+)"',
      caseSensitive: false,
    ).firstMatch(disposition);
    if (quoted != null) return _safeFileName(quoted[1]!);
    final plain = RegExp(
      r'filename\s*=\s*([^;\s]+)',
      caseSensitive: false,
    ).firstMatch(disposition);
    if (plain != null) return _safeFileName(plain[1]!);
  }
  final segments = Uri.parse(url).pathSegments;
  return _safeFileName(segments.isEmpty ? 'download' : segments.last);
}

/// Produces a single safe filename without allowing path traversal or control bytes.
String _safeFileName(String name) {
  final safe = name
      .replaceAll(RegExp(r'[\x00-\x1f\\/:*?"<>|]'), '_')
      .replaceAll(RegExp(r'^[. ]+|[. ]+$'), '');
  return safe.isEmpty ? 'download' : safe;
}
