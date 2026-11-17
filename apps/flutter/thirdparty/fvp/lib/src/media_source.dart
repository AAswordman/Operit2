import 'dart:io';

/// Owns the seekable native input for one player until native teardown finishes.
class MdkMediaSource {
  /// Creates a source with optional ownership of a private temporary directory.
  MdkMediaSource._(this.uri, this._directory);

  final String uri;
  final Directory? _directory;
  Future<void>? _disposal;

  /// Materializes data URLs because MDK cannot open them as seekable media.
  static Future<MdkMediaSource> open(String uri) async {
    if (!uri.startsWith('data:')) return MdkMediaSource._(uri, null);
    final bytes = Uri.parse(uri).data!.contentAsBytes();
    final directory = await Directory.systemTemp.createTemp('fvp-media-');
    try {
      final file = File('${directory.path}/media');
      await file.writeAsBytes(bytes, flush: true);
      return MdkMediaSource._(file.path, directory);
    } catch (_) {
      await directory.delete(recursive: true);
      rethrow;
    }
  }

  /// Removes only this player's temporary input, at most once.
  Future<void> dispose() => _disposal ??= _deleteDirectory();

  /// Leaves externally owned files and network inputs untouched.
  Future<void> _deleteDirectory() async {
    await _directory?.delete(recursive: true);
  }
}
