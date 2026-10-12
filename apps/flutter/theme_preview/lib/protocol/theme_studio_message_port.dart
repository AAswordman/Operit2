import 'dart:async';
import 'dart:js_interop';
import 'dart:typed_data';

import 'package:web/web.dart' as web;

@JS('operitThemePostCapture')
external void _postCapture(
  web.MessagePort port,
  JSAny? header,
  JSUint8Array pngBytes,
);

/// Versioned iframe protocol. The parent must provide an exact origin and a
/// per-tab nonce in the preview URL; wildcard origins and nonce-less messages
/// are rejected. `package:web` keeps this entrypoint compatible with Flutter's
/// WebAssembly dry-run/build path; the shared renderer remains platform-neutral.
class ThemeStudioMessagePort {
  ThemeStudioMessagePort({
    required this.expectedOrigin,
    required this.expectedNonce,
    required this.rendererReleaseId,
  }) {
    final origin = Uri.parse(expectedOrigin);
    if ((origin.scheme != 'https' && origin.scheme != 'http') ||
        origin.host.isEmpty ||
        origin.origin != expectedOrigin) {
      throw ArgumentError('Studio must use an exact HTTP(S) origin');
    }
    if (!RegExp(r'^[A-Za-z0-9_-]{16,128}$').hasMatch(expectedNonce) ||
        !RegExp(r'^[A-Za-z0-9._-]{1,128}$').hasMatch(rendererReleaseId)) {
      throw ArgumentError(
        'A nonce and pinned renderer release ID are required',
      );
    }
  }

  final String expectedOrigin;
  final String expectedNonce;
  final String rendererReleaseId;

  StreamSubscription<web.MessageEvent>? _windowSubscription;
  StreamSubscription<web.MessageEvent>? _portSubscription;
  web.MessagePort? _port;
  bool _attached = false;

  bool get isConnected => _port != null;

  static ThemeStudioMessagePort? fromLocation() {
    final location = Uri.parse(web.window.location.href);
    final origin = location.queryParameters['studioOrigin'];
    final nonce = location.queryParameters['nonce'];
    final requestedRelease = location.queryParameters['rendererReleaseId'];
    if (origin == null && nonce == null && requestedRelease == null) {
      return null;
    }
    const compiledRelease = String.fromEnvironment(
      'OPERIT_THEME_RENDERER_RELEASE_ID',
    );
    if (origin == null ||
        nonce == null ||
        compiledRelease.isEmpty ||
        requestedRelease != compiledRelease) {
      throw ArgumentError(
        'Studio parameters must match the compiled renderer release',
      );
    }
    return ThemeStudioMessagePort(
      expectedOrigin: origin,
      expectedNonce: nonce,
      rendererReleaseId: compiledRelease,
    );
  }

  void attach({
    required FutureOr<void> Function(Map<String, dynamic>) onRequest,
    required int Function() nextRevision,
  }) {
    if (_attached) throw StateError('Preview protocol is already attached');
    _attached = true;
    _windowSubscription = web.window.onMessage.listen((event) {
      final data = _asMap(event.data);
      final ports = event.ports.toDart;
      if (event.origin != expectedOrigin ||
          event.source != web.window.parent ||
          data == null) {
        return;
      }
      if (data['type'] != 'operit.theme.studio.connect.v1' ||
          data['nonce'] != expectedNonce ||
          data['protocolVersion'] != 1 ||
          data['rendererReleaseId'] != rendererReleaseId ||
          ports.length != 1 ||
          _port != null) {
        for (final port in ports) {
          port.close();
        }
        return;
      }
      _port = ports.first;
      _portSubscription = web.EventStreamProviders.messageEvent
          .forTarget(_port!)
          .listen((event) async {
            final request = _asMap(event.data);
            if (request == null ||
                request['nonce'] != expectedNonce ||
                request['protocolVersion'] != 1 ||
                request['rendererReleaseId'] != rendererReleaseId) {
              return;
            }
            try {
              await onRequest(request);
            } catch (error) {
              sendError(
                code: 'preview-request-invalid',
                message: error.toString(),
                revision: request['revision'] is int
                    ? request['revision'] as int
                    : null,
              );
            }
          });
      // addEventListener-style MessagePort listeners do not receive queued
      // messages until start() is called. Sending ready without this left the
      // Studio waiting forever for load/receipt responses.
      _port!.start();
      _send(<String, dynamic>{
        'type': 'operit.theme.preview.ready.v1',
        'protocolVersion': 1,
        'nonce': expectedNonce,
        'rendererReleaseId': rendererReleaseId,
        'nextRevision': nextRevision(),
      });
    });
    web.window.parent?.postMessage(
      <String, dynamic>{
        'type': 'operit.theme.preview.available.v1',
        'protocolVersion': 1,
        'nonce': expectedNonce,
        'rendererReleaseId': rendererReleaseId,
      }.jsify(),
      expectedOrigin.toJS,
    );
  }

  void sendRendered({
    required int revision,
    required Map<String, dynamic> receipt,
    required String fixtureVersion,
  }) {
    _send(<String, dynamic>{
      'type': 'operit.theme.preview.rendered.v1',
      'protocolVersion': 1,
      'nonce': expectedNonce,
      'rendererReleaseId': rendererReleaseId,
      'fixtureVersion': fixtureVersion,
      'revision': revision,
      'receipt': receipt,
    });
  }

  void sendSelection({
    required int revision,
    required bool enabled,
    String? targetId,
  }) {
    _send(<String, dynamic>{
      'type': 'operit.theme.preview.selection.v1',
      'protocolVersion': 1,
      'nonce': expectedNonce,
      'rendererReleaseId': rendererReleaseId,
      'revision': revision,
      'enabled': enabled,
      'targetId': targetId,
    });
  }

  void sendLoaded({required int revision, required String themeSha256}) {
    _send(<String, dynamic>{
      'type': 'operit.theme.preview.loaded.v1',
      'protocolVersion': 1,
      'nonce': expectedNonce,
      'rendererReleaseId': rendererReleaseId,
      'revision': revision,
      'themeSha256': themeSha256,
    });
  }

  void sendProgress({required int revision, required String stage}) {
    _send(<String, dynamic>{
      'type': 'operit.theme.preview.progress.v1',
      'protocolVersion': 1,
      'nonce': expectedNonce,
      'rendererReleaseId': rendererReleaseId,
      'revision': revision,
      'stage': stage,
    });
  }

  void sendCaptured({
    required int revision,
    required String themeSha256,
    required String fixtureVersion,
    required bool showConversation,
    required int viewportWidth,
    required int viewportHeight,
    required String pngSha256,
    required Uint8List pngBytes,
  }) {
    final port = _port;
    if (port == null) throw StateError('Studio capture port is not connected');
    _postCapture(
      port,
      <String, Object?>{
        'type': 'operit.theme.preview.captured.v1',
        'protocolVersion': 1,
        'nonce': expectedNonce,
        'rendererReleaseId': rendererReleaseId,
        'revision': revision,
        'themeSha256': themeSha256,
        'fixtureVersion': fixtureVersion,
        'showConversation': showConversation,
        'viewport': <String, int>{
          'width': viewportWidth,
          'height': viewportHeight,
        },
        'pngSha256': pngSha256,
      }.jsify(),
      pngBytes.toJS,
    );
  }

  void sendError({
    required String code,
    required String message,
    int? revision,
  }) {
    _send(<String, dynamic>{
      'type': 'operit.theme.preview.error.v1',
      'protocolVersion': 1,
      'nonce': expectedNonce,
      'rendererReleaseId': rendererReleaseId,
      'code': code,
      'message': message,
      ...?revision == null ? null : <String, dynamic>{'revision': revision},
    });
  }

  void reportStartupError({required String code, required String message}) {
    web.window.parent?.postMessage(
      <String, dynamic>{
        'type': 'operit.theme.preview.error.v1',
        'protocolVersion': 1,
        'nonce': expectedNonce,
        'rendererReleaseId': rendererReleaseId,
        'code': code,
        'message': message,
      }.jsify(),
      expectedOrigin.toJS,
    );
  }

  void _send(Map<String, dynamic> message) {
    final port = _port;
    if (port == null) return;
    port.postMessage(message.jsify());
  }

  void dispose() {
    _windowSubscription?.cancel();
    _portSubscription?.cancel();
    _port?.close();
    _port = null;
  }
}

Map<String, dynamic>? _asMap(JSAny? value) {
  final dartValue = value?.dartify();
  if (dartValue is! Map) return null;
  return Map<String, dynamic>.from(dartValue);
}
