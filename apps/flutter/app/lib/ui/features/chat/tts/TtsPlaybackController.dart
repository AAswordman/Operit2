// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/foundation.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../core/runtime/RuntimeConnectionManager.dart';

const String _systemTtsProviderType = 'SYSTEM_TTS';
const Duration _hostSpeechPollInterval = Duration(milliseconds: 240);

enum TtsPlaybackPhase { idle, preparing, playing, paused, stopped, error }

class TtsPlaybackState {
  const TtsPlaybackState({
    required this.phase,
    required this.title,
    required this.currentText,
    required this.currentAudioPath,
    required this.queueLength,
    required this.audioIndex,
    required this.audioCount,
    required this.error,
  });

  const TtsPlaybackState.idle()
    : phase = TtsPlaybackPhase.idle,
      title = '',
      currentText = '',
      currentAudioPath = '',
      queueLength = 0,
      audioIndex = 0,
      audioCount = 0,
      error = null;

  final TtsPlaybackPhase phase;
  final String title;
  final String currentText;
  final String currentAudioPath;
  final int queueLength;
  final int audioIndex;
  final int audioCount;
  final String? error;

  TtsPlaybackState copyWith({
    TtsPlaybackPhase? phase,
    String? title,
    String? currentText,
    String? currentAudioPath,
    int? queueLength,
    int? audioIndex,
    int? audioCount,
    String? error,
    bool clearError = false,
  }) {
    return TtsPlaybackState(
      phase: phase ?? this.phase,
      title: title ?? this.title,
      currentText: currentText ?? this.currentText,
      currentAudioPath: currentAudioPath ?? this.currentAudioPath,
      queueLength: queueLength ?? this.queueLength,
      audioIndex: audioIndex ?? this.audioIndex,
      audioCount: audioCount ?? this.audioCount,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class TtsPlaybackController extends ChangeNotifier {
  TtsPlaybackController._() {
    unawaited(_player.setReleaseMode(ReleaseMode.stop));
  }

  static final TtsPlaybackController instance = TtsPlaybackController._();

  final AudioPlayer _player = AudioPlayer(playerId: 'operit_tts_playback');
  final List<_TtsPlaybackRequest> _queue = <_TtsPlaybackRequest>[];
  int _generation = 0;
  bool _draining = false;
  OperitRuntimeBridge? _hostSpeechBridge;
  Completer<void>? _currentPlaybackCompletion;
  TtsPlaybackState _state = const TtsPlaybackState.idle();

  TtsPlaybackState get state => _state;

  Future<void> speakForCharacter({
    required OperitRuntimeBridge bridge,
    required String characterCardId,
    required String text,
    required String title,
    bool interrupt = true,
  }) async {
    if (interrupt) {
      await stop();
    }
    final request = _TtsPlaybackRequest.character(
      bridge: bridge,
      characterCardId: characterCardId,
      text: text,
      title: title,
      generation: _generation,
    );
    _queue.add(request);
    _publish(
      _state.copyWith(
        phase: _state.phase == TtsPlaybackPhase.idle
            ? TtsPlaybackPhase.preparing
            : _state.phase,
        queueLength: _queue.length,
        clearError: true,
      ),
    );
    if (!_draining) {
      unawaited(_drainQueue());
    }
  }

  Future<void> speakWithConfig({
    required OperitRuntimeBridge bridge,
    required String ttsConfigId,
    required String text,
    required String title,
    bool interrupt = true,
  }) async {
    if (interrupt) {
      await stop();
    }
    final request = _TtsPlaybackRequest.config(
      bridge: bridge,
      ttsConfigId: ttsConfigId,
      text: text,
      title: title,
      generation: _generation,
    );
    _queue.add(request);
    _publish(
      _state.copyWith(
        phase: _state.phase == TtsPlaybackPhase.idle
            ? TtsPlaybackPhase.preparing
            : _state.phase,
        queueLength: _queue.length,
        clearError: true,
      ),
    );
    if (!_draining) {
      unawaited(_drainQueue());
    }
  }

  Future<void> pause() async {
    if (_state.phase != TtsPlaybackPhase.playing) {
      return;
    }
    final hostSpeechBridge = _hostSpeechBridge;
    if (hostSpeechBridge != null) {
      await _callHostSpeech(hostSpeechBridge, 'pauseSpeech');
    } else {
      await _player.pause();
    }
    _publish(_state.copyWith(phase: TtsPlaybackPhase.paused));
  }

  Future<void> resume() async {
    if (_state.phase != TtsPlaybackPhase.paused) {
      return;
    }
    final hostSpeechBridge = _hostSpeechBridge;
    if (hostSpeechBridge != null) {
      await _callHostSpeech(hostSpeechBridge, 'resumeSpeech');
    } else {
      await _player.resume();
    }
    _publish(_state.copyWith(phase: TtsPlaybackPhase.playing));
  }

  Future<void> stop() async {
    _generation += 1;
    _queue.clear();
    _currentPlaybackCompletion?.complete();
    _currentPlaybackCompletion = null;
    final hostSpeechBridge = _hostSpeechBridge;
    _hostSpeechBridge = null;
    if (hostSpeechBridge != null) {
      await _callHostSpeech(hostSpeechBridge, 'stopSpeech');
    }
    await _player.stop();
    _publish(
      const TtsPlaybackState.idle().copyWith(phase: TtsPlaybackPhase.stopped),
    );
  }

  void clearStopped() {
    if (_state.phase == TtsPlaybackPhase.stopped ||
        _state.phase == TtsPlaybackPhase.error) {
      _publish(const TtsPlaybackState.idle());
    }
  }

  Future<void> _drainQueue() async {
    _draining = true;
    try {
      while (_queue.isNotEmpty) {
        final request = _queue.removeAt(0);
        if (request.generation != _generation) {
          continue;
        }
        _publish(
          _state.copyWith(
            phase: TtsPlaybackPhase.preparing,
            title: request.title,
            currentText: request.text,
            currentAudioPath: '',
            queueLength: _queue.length,
            audioIndex: 0,
            audioCount: 0,
            clearError: true,
          ),
        );
        final usesHostSystemSpeech = await _usesHostSystemSpeech(request);
        if (request.generation != _generation) {
          continue;
        }
        if (usesHostSystemSpeech) {
          final audioPath = _hostSpeechPath(request.displayId);
          _publish(
            _state.copyWith(
              phase: TtsPlaybackPhase.playing,
              currentAudioPath: audioPath,
              queueLength: _queue.length,
              audioIndex: 1,
              audioCount: 1,
              clearError: true,
            ),
          );
          _hostSpeechBridge = request.bridge;
          try {
            await _speakHostSystem(request);
            await _waitHostSystemSpeech(request);
          } finally {
            if (identical(_hostSpeechBridge, request.bridge)) {
              _hostSpeechBridge = null;
            }
          }
          continue;
        }
        final audioSources = await _synthesize(request);
        if (request.generation != _generation) {
          continue;
        }
        for (var index = 0; index < audioSources.length; index += 1) {
          if (request.generation != _generation) {
            break;
          }
          final audioSource = audioSources[index];
          _publish(
            _state.copyWith(
              phase: TtsPlaybackPhase.playing,
              currentAudioPath: audioSource.path,
              queueLength: _queue.length,
              audioIndex: index + 1,
              audioCount: audioSources.length,
              clearError: true,
            ),
          );
          await _playAudioSource(request, audioSource);
        }
      }
      if (_state.phase != TtsPlaybackPhase.stopped &&
          _state.phase != TtsPlaybackPhase.error) {
        _publish(const TtsPlaybackState.idle());
      }
    } catch (error) {
      _publish(
        _state.copyWith(
          phase: TtsPlaybackPhase.error,
          queueLength: _queue.length,
          error: '$error',
        ),
      );
    } finally {
      _draining = false;
      if (_queue.isNotEmpty) {
        unawaited(_drainQueue());
      }
    }
  }

  Future<bool> _usesHostSystemSpeech(_TtsPlaybackRequest request) async {
    final clients = GeneratedCoreProxyClients(request.bridge);
    final config = switch (request.source) {
      _TtsPlaybackSource.character => await _resolvedCharacterTtsConfig(
        clients,
        request.characterCardId,
      ),
      _TtsPlaybackSource.config =>
        await clients.preferencesTtsConfigManager.getTtsConfig(
          id: request.ttsConfigId,
        ),
    };
    return config.providerType == _systemTtsProviderType;
  }

  Future<core_proxy.TtsConfig> _resolvedCharacterTtsConfig(
    GeneratedCoreProxyClients clients,
    String characterCardId,
  ) async {
    final card = await clients.preferencesCharacterCardManager.getCharacterCard(
      id: characterCardId,
    );
    final ttsConfigId = card.ttsConfigId?.trim();
    return ttsConfigId == null || ttsConfigId.isEmpty
        ? await clients.preferencesTtsConfigManager.getCurrentTtsConfig()
        : await clients.preferencesTtsConfigManager.getTtsConfig(
            id: ttsConfigId,
          );
  }

  Future<List<_TtsPlaybackAudioSource>> _synthesize(
    _TtsPlaybackRequest request,
  ) async {
    final methodName = switch (request.source) {
      _TtsPlaybackSource.character => 'synthesizeForCharacter',
      _TtsPlaybackSource.config => 'synthesizeWithConfig',
    };
    final args = switch (request.source) {
      _TtsPlaybackSource.character => <String, Object?>{
        'characterCardId': request.characterCardId,
        'text': request.text,
      },
      _TtsPlaybackSource.config => <String, Object?>{
        'ttsConfigId': request.ttsConfigId,
        'text': request.text,
      },
    };
    final result = await request.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('services.ttsSynthesisService'),
        methodName: methodName,
        args: args,
      ),
    );
    final json = result as Map<String, Object?>;
    final audioPaths = _jsonStringList(json, 'audioPaths');
    final audioStoragePaths = _jsonStringList(json, 'audioStoragePaths');
    if (audioPaths.length != audioStoragePaths.length) {
      throw StateError('tts audio path count mismatch');
    }
    return <_TtsPlaybackAudioSource>[
      for (var index = 0; index < audioPaths.length; index += 1)
        _TtsPlaybackAudioSource(
          path: audioPaths[index],
          storagePath: audioStoragePaths[index],
        ),
    ];
  }

  Future<void> _speakHostSystem(_TtsPlaybackRequest request) async {
    final methodName = switch (request.source) {
      _TtsPlaybackSource.character => 'speakForCharacter',
      _TtsPlaybackSource.config => 'speakWithConfig',
    };
    final args = switch (request.source) {
      _TtsPlaybackSource.character => <String, Object?>{
        'characterCardId': request.characterCardId,
        'text': request.text,
        'interrupt': true,
      },
      _TtsPlaybackSource.config => <String, Object?>{
        'ttsConfigId': request.ttsConfigId,
        'text': request.text,
        'interrupt': true,
      },
    };
    await _callHostSpeech(
      request.bridge,
      methodName,
      args: args,
    );
  }

  Future<void> _waitHostSystemSpeech(_TtsPlaybackRequest request) async {
    while (request.generation == _generation) {
      final state = await _callHostSpeech(request.bridge, 'speechState');
      if (state['active'] != true) {
        return;
      }
      await Future<void>.delayed(_hostSpeechPollInterval);
    }
  }

  Future<Map<String, Object?>> _callHostSpeech(
    OperitRuntimeBridge bridge,
    String methodName, {
    Map<String, Object?> args = const <String, Object?>{},
  }) async {
    final result = await bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('services.ttsPlaybackService'),
        methodName: methodName,
        args: args,
      ),
    );
    return result as Map<String, Object?>;
  }

  Future<void> _playAudioSource(
    _TtsPlaybackRequest request,
    _TtsPlaybackAudioSource audioSource,
  ) async {
    if (kIsWeb ||
        RuntimeConnectionManager.instance.config.mode ==
            RuntimeConnectionMode.remote) {
      final base64Content = await GeneratedCoreProxyClients(request.bridge)
          .repositoryRuntimeStorageRepository
          .readBase64(path: audioSource.storagePath);
      if (base64Content == null) {
        throw StateError('tts audio resource not found: ${audioSource.storagePath}');
      }
      final Uint8List bytes = base64Decode(base64Content);
      await _playSource(
        BytesSource(bytes, mimeType: _audioMimeType(audioSource.storagePath)),
      );
      return;
    }
    await _playSource(DeviceFileSource(audioSource.path));
  }

  Future<void> _playSource(Source source) async {
    final completion = Completer<void>();
    _currentPlaybackCompletion = completion;
    late final StreamSubscription<void> subscription;
    subscription = _player.onPlayerComplete.listen((_) {
      if (!completion.isCompleted) {
        completion.complete();
      }
    });
    try {
      await _player.play(source);
      await completion.future;
    } finally {
      await subscription.cancel();
      if (identical(_currentPlaybackCompletion, completion)) {
        _currentPlaybackCompletion = null;
      }
    }
  }

  void _publish(TtsPlaybackState state) {
    _state = state;
    notifyListeners();
  }
}

class _TtsPlaybackRequest {
  const _TtsPlaybackRequest.character({
    required this.bridge,
    required this.characterCardId,
    required this.text,
    required this.title,
    required this.generation,
  }) : source = _TtsPlaybackSource.character,
       ttsConfigId = '';

  const _TtsPlaybackRequest.config({
    required this.bridge,
    required this.ttsConfigId,
    required this.text,
    required this.title,
    required this.generation,
  }) : source = _TtsPlaybackSource.config,
       characterCardId = '';

  final OperitRuntimeBridge bridge;
  final _TtsPlaybackSource source;
  final String characterCardId;
  final String ttsConfigId;
  final String text;
  final String title;
  final int generation;

  String get displayId => switch (source) {
    _TtsPlaybackSource.character => characterCardId,
    _TtsPlaybackSource.config => ttsConfigId,
  };
}

enum _TtsPlaybackSource { character, config }

class _TtsPlaybackAudioSource {
  const _TtsPlaybackAudioSource({
    required this.path,
    required this.storagePath,
  });

  final String path;
  final String storagePath;
}

List<String> _jsonStringList(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value is! List<Object?>) {
    throw StateError('tts synthesis result missing $key');
  }
  return value.map((item) => item as String).toList(growable: false);
}

String _hostSpeechPath(String characterCardId) => 'host-tts:$characterCardId';

String? _audioMimeType(String path) {
  final dotIndex = path.lastIndexOf('.');
  if (dotIndex < 0 || dotIndex == path.length - 1) {
    return null;
  }
  final extension = path.substring(dotIndex + 1).toLowerCase();
  return switch (extension) {
    'aac' => 'audio/aac',
    'flac' => 'audio/flac',
    'm4a' => 'audio/mp4',
    'mp3' => 'audio/mpeg',
    'mp4' => 'audio/mp4',
    'mpeg' => 'audio/mpeg',
    'oga' => 'audio/ogg',
    'ogg' => 'audio/ogg',
    'opus' => 'audio/ogg',
    'wav' => 'audio/wav',
    'webm' => 'audio/webm',
    _ => null,
  };
}

String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';
