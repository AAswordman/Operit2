import 'dart:async';
import 'dart:typed_data';

import 'package:operit2/ui/features/chat/voice_input/ChatVoiceInputSession.dart';

/// Creates a voice session that stays idle because nothing drives it.
ChatVoiceInputSession createIdleVoiceInputSession() => ChatVoiceInputSession(
  core: FakeChatVoiceInputCore(),
  selectedConfigId: () async => null,
  commit: (_) {},
);

/// Records what a voice session asks of Core and feeds it scripted transcripts.
class FakeChatVoiceInputCore implements ChatVoiceInputCore {
  final partials = StreamController<String>();
  final started = <String>[];
  final pushedBytes = <int>[];
  final finished = <String>[];
  final cancelled = <String>[];
  String finalTranscript = '';

  @override
  Future<String> startStreamSession({required String configId}) async {
    started.add(configId);
    return 'session-$configId';
  }

  @override
  Stream<String> streamPartialText({required String sessionId}) =>
      partials.stream;

  @override
  Future<void> pushStreamAudio({
    required String sessionId,
    required Uint8List audio,
  }) async {
    pushedBytes.add(audio.length);
  }

  @override
  Future<String> finishStreamSession({required String sessionId}) async {
    finished.add(sessionId);
    return finalTranscript;
  }

  @override
  Future<void> cancelStreamSession({required String sessionId}) async {
    cancelled.add(sessionId);
  }
}
