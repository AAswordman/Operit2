// ignore_for_file: file_names

import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Shares immutable data-URI bytes so Flutter can reuse the same decoded image.
/// The LRU bounds both entry count and retained source/byte memory. Mutable VFS
/// and OS files keep their ordinary resolution and freshness behavior.
class ComposeDslDataImageCache {
  ComposeDslDataImageCache({
    this.maximumEntries = 64,
    this.maximumBytes = 8 * 1024 * 1024,
  });

  final int maximumEntries;
  final int maximumBytes;
  final _providers = <String, MemoryImage>{};
  int _retainedBytes = 0;

  MemoryImage resolve(String source) {
    final existing = _providers.remove(source);
    if (existing != null) {
      _providers[source] = existing;
      return existing;
    }
    final provider = MemoryImage(UriData.parse(source).contentAsBytes());
    final size = source.length * 2 + provider.bytes.length;
    if (maximumEntries <= 0 || size > maximumBytes) return provider;
    while (_providers.isNotEmpty &&
        (_providers.length >= maximumEntries ||
            _retainedBytes + size > maximumBytes)) {
      final oldest = _providers.keys.first;
      final removed = _providers.remove(oldest)!;
      _retainedBytes -= oldest.length * 2 + removed.bytes.length;
    }
    _providers[source] = provider;
    _retainedBytes += size;
    return provider;
  }
}

final _dataImages = ComposeDslDataImageCache();

/// Uses Flutter's bounded image cache to coalesce VFS reads and image decoding.
/// The owner separates independent sessions/readers; changing a mutable image
/// at the same path requires evict(), just like FileImage and NetworkImage.
@immutable
class ComposeDslVfsImage extends ImageProvider<ComposeDslVfsImage> {
  const ComposeDslVfsImage({
    required this.path,
    required this.owner,
    required this.readBytes,
  });

  final String path;
  final Object owner;
  final Future<Uint8List> Function(String path) readBytes;

  @override
  Future<ComposeDslVfsImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(
    ComposeDslVfsImage key,
    ImageDecoderCallback decode,
  ) => MultiFrameImageStreamCompleter(
    codec: _load(decode),
    scale: 1,
    debugLabel: path,
  );

  Future<ui.Codec> _load(ImageDecoderCallback decode) async {
    try {
      final bytes = await readBytes(path);
      if (bytes.isEmpty) throw StateError('DSL VFS image is empty: $path');
      return await decode(await ui.ImmutableBuffer.fromUint8List(bytes));
    } catch (_) {
      PaintingBinding.instance.imageCache.evict(this);
      rethrow;
    }
  }

  @override
  bool operator ==(Object other) =>
      other is ComposeDslVfsImage && other.path == path && other.owner == owner;

  @override
  int get hashCode => Object.hash(path, owner);
}

/// Reads virtual files through the actual Core file service, never dart:io.
Future<Uint8List> readComposeDslVfsBytes(
  GeneratedCoreProxyClients clients,
  String path,
) async {
  final result = await clients.application.aiToolHandler().executeTool(
    tool: core_proxy.CoreOperitToolsToolExecutionManagerAiTool(
      name: 'read_file_binary',
      parameters: [
        core_proxy.CoreOperitToolsToolExecutionManagerToolParameter(
          name: 'path',
          value: path,
        ),
      ],
    ),
  );
  if (!result.success) {
    throw StateError('DSL VFS image $path: ${result.error}');
  }
  final data = result.result.value;
  if (data is! core_proxy.BinaryFileContentData) {
    throw StateError('DSL VFS image did not return binary file content');
  }
  return base64Decode(data.contentBase64);
}

/// Gives known VFS roots precedence even when wrapped in a file URI.
String? composeDslVfsImagePath(String source) {
  final uri = Uri.tryParse(source);
  if (uri?.scheme == 'vfs' || uri?.scheme == 'operit-vfs') {
    final host = uri!.host;
    return host.isEmpty || host == 'localhost' ? uri.path : '/$host${uri.path}';
  }
  final path = uri?.scheme == 'file' ? uri!.toFilePath() : source;
  if (path == '/app' ||
      path.startsWith('/app/') ||
      path == '/mnt' ||
      path.startsWith('/mnt/') ||
      path.startsWith('runtime/')) {
    return path;
  }
  return null;
}

/// Resolves VFS before native files. A VFS failure must not fall back to the OS.
Future<ImageProvider> resolveComposeDslImage(
  String source, {
  required Future<Uint8List> Function(String path) readVfsBytes,
  Object? vfsOwner,
}) async {
  // Data URIs cannot name a VFS file. Check their fixed scheme before the
  // generic URI parser scans a multi-megabyte avatar twice on every cache hit.
  if (source.startsWith('data:')) return _dataImages.resolve(source);
  final vfs = composeDslVfsImagePath(source);
  if (vfs != null) {
    return ComposeDslVfsImage(
      path: vfs,
      owner: vfsOwner ?? readVfsBytes,
      readBytes: readVfsBytes,
    );
  }
  final uri = Uri.tryParse(source);
  return switch (uri?.scheme) {
    'data' => _dataImages.resolve(source),
    'file' => FileImage(File.fromUri(uri!)),
    _ =>
      source.startsWith('/') ? FileImage(File(source)) : NetworkImage(source),
  };
}
