// ignore_for_file: file_names

import 'dart:typed_data';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'ChatVoiceInputSession.dart';

/// Streams microphone audio into the Core speech recognition session.
class CoreChatVoiceInput implements ChatVoiceInputCore {
  /// Creates the Core bridge over the generated STT proxy client.
  const CoreChatVoiceInput(this._service);

  final GeneratedServicesSttRecognitionServiceCoreProxy _service;

  @override
  Future<String> startStreamSession({required String configId}) =>
      _service.startStreamSession(configId: configId, language: null);

  @override
  Stream<String> streamPartialText({required String sessionId}) =>
      _service.streamPartialText(sessionId: sessionId);

  @override
  Future<void> pushStreamAudio({
    required String sessionId,
    required Uint8List audio,
  }) => _service.pushStreamAudio(sessionId: sessionId, audio: audio);

  @override
  Future<String> finishStreamSession({required String sessionId}) =>
      _service.finishStreamSession(sessionId: sessionId);

  @override
  Future<void> cancelStreamSession({required String sessionId}) =>
      _service.cancelStreamSession(sessionId: sessionId);
}
