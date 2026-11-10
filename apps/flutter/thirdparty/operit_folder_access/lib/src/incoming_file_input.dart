part of '../operit_folder_access.dart';

/// Receives native-owned file inputs without interpreting OS paths in app code.
mixin _IncomingFileAccess on OperitFolderAccessPlatform {
  static const _channel = MethodChannel('operit/incoming_files');

  /// Installs the receiver before atomically draining native startup deliveries.
  @override
  Future<void> receiveIncomingFiles({
    required void Function(List<SelectedFileInput>) onFiles,
    required void Function(Object, StackTrace) onError,
  }) async {
    /// Converts one native share batch into owned inputs for the common uploader.
    void receive(Object? payload) {
      final files = <SelectedFileInput>[];
      try {
        for (final value in payload! as List) {
          final info = Map<String, Object?>.from(value as Map);
          final token = info['token'];
          final name = info['name'];
          final length = info['byteLength'];
          if (token is! String ||
              token.isEmpty ||
              name is! String ||
              name.isEmpty ||
              length is! int ||
              length < 0) {
            throw StateError('Incoming file has invalid metadata');
          }
          files.add(
            SelectedFileInput._native(
              token: token,
              name: name,
              byteLength: length,
              mimeType: info['mimeType'] as String?,
              channel: _channel,
            ),
          );
        }
        if (files.isNotEmpty) onFiles(files);
      } catch (error, stack) {
        for (final file in files) {
          unawaited(
            file.close().catchError((Object error, StackTrace stack) {
              onError(error, stack);
            }),
          );
        }
        onError(error, stack);
      }
    }

    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'files':
          receive(call.arguments);
        case 'error':
          onError(StateError(call.arguments as String), StackTrace.current);
        default:
          throw MissingPluginException(
            'Unknown incoming file event: ${call.method}',
          );
      }
    });
    final batches = await _channel.invokeListMethod<Object?>('ready');
    for (final batch in batches!) {
      receive(batch);
    }
  }
}
