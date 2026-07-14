// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

const String _systemTtsProviderType = 'SYSTEM_TTS';
const Duration _hostSpeechPollInterval = Duration(milliseconds: 240);

enum TtsPlaybackPhase { idle, preparing, playing, paused, stopped, error }

class TtsPlaybackState {
  /// Creates an immutable TTS playback snapshot.
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

  /// Creates the empty playback snapshot.
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

  /// Creates a new snapshot with selected fields replaced.
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
  /// Creates the process-wide playback controller.
  TtsPlaybackController._();

  static final TtsPlaybackController instance = TtsPlaybackController._();

  final List<_TtsPlaybackRequest> _queue = <_TtsPlaybackRequest>[];
  int _generation = 0;
  bool _draining = false;
  OperitRuntimeBridge? _hostSpeechBridge;
  _TtsPlaybackRequest? _currentRequest;
  Future<void>? _stopInProgress;
  TtsPlaybackState _state = const TtsPlaybackState.idle();

  /// Returns the latest playback snapshot.
  TtsPlaybackState get state => _state;

  /// Starts character speech and completes after playback actually starts.
  Future<void> speakForCharacter({
    required OperitRuntimeBridge bridge,
    required String characterCardId,
    required String text,
    required String title,
    bool interrupt = true,
  }) async {
    if (interrupt) {
      await stop();
      _throwStopError();
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
    await request.started;
  }

  /// Starts configured speech and completes after playback actually starts.
  Future<void> speakWithConfig({
    required OperitRuntimeBridge bridge,
    required String ttsConfigId,
    required String text,
    required String title,
    bool interrupt = true,
  }) async {
    if (interrupt) {
      await stop();
      _throwStopError();
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
    await request.started;
  }

  /// Pauses the active playback using the authoritative host state.
  Future<void> pause() async {
    if (_state.phase != TtsPlaybackPhase.playing) {
      return;
    }
    final generation = _generation;
    try {
      final hostSpeechBridge = _hostSpeechBridge;
      if (hostSpeechBridge == null) {
        throw StateError('active TTS playback is missing its runtime bridge');
      }
      final status = await _callHostSpeech(hostSpeechBridge, 'pauseSpeech');
      if (generation != _generation ||
          _state.phase != TtsPlaybackPhase.playing) {
        return;
      }
      _publish(
        status.active
            ? _state.copyWith(
                phase: status.paused
                    ? TtsPlaybackPhase.paused
                    : TtsPlaybackPhase.playing,
              )
            : const TtsPlaybackState.idle(),
      );
    } catch (error) {
      if (generation == _generation) {
        _publishPlaybackError(error);
      }
    }
  }

  /// Resumes paused playback using the authoritative host state.
  Future<void> resume() async {
    if (_state.phase != TtsPlaybackPhase.paused) {
      return;
    }
    final generation = _generation;
    try {
      final hostSpeechBridge = _hostSpeechBridge;
      if (hostSpeechBridge == null) {
        throw StateError('paused TTS playback is missing its runtime bridge');
      }
      final status = await _callHostSpeech(hostSpeechBridge, 'resumeSpeech');
      if (generation != _generation ||
          _state.phase != TtsPlaybackPhase.paused) {
        return;
      }
      _publish(
        status.active
            ? _state.copyWith(
                phase: status.paused
                    ? TtsPlaybackPhase.paused
                    : TtsPlaybackPhase.playing,
              )
            : const TtsPlaybackState.idle(),
      );
    } catch (error) {
      if (generation == _generation) {
        _publishPlaybackError(error);
      }
    }
  }

  /// Stops queued, preparing, and active playback as one cancellation action.
  Future<void> stop() {
    final activeStop = _stopInProgress;
    if (activeStop != null) {
      return activeStop;
    }
    final stopFuture = _stopPlayback();
    _stopInProgress = stopFuture;
    return stopFuture.whenComplete(() {
      if (identical(_stopInProgress, stopFuture)) {
        _stopInProgress = null;
      }
    });
  }

  /// Performs the stop operation and records every cleanup failure.
  Future<void> _stopPlayback() async {
    _generation += 1;
    final cancelled = TtsPlaybackCancelledException();
    for (final request in _queue) {
      request.failStart(cancelled);
    }
    _queue.clear();
    _currentRequest?.failStart(cancelled);
    final hostSpeechBridge = _hostSpeechBridge;
    _hostSpeechBridge = null;
    Object? stopError;
    try {
      if (hostSpeechBridge != null) {
        await _callHostSpeech(hostSpeechBridge, 'stopSpeech');
      }
    } catch (error) {
      stopError = error;
    }
    if (stopError != null) {
      _publishPlaybackError(stopError);
      return;
    }
    _publish(
      const TtsPlaybackState.idle().copyWith(phase: TtsPlaybackPhase.stopped),
    );
  }

  /// Clears a terminal stopped or error snapshot.
  void clearStopped() {
    if (_state.phase == TtsPlaybackPhase.stopped ||
        _state.phase == TtsPlaybackPhase.error) {
      _publish(const TtsPlaybackState.idle());
    }
  }

  /// Drains queued requests in strict request order.
  Future<void> _drainQueue() async {
    _draining = true;
    _TtsPlaybackRequest? activeRequest;
    try {
      while (_queue.isNotEmpty) {
        final request = _queue.removeAt(0);
        activeRequest = request;
        _currentRequest = request;
        if (request.generation != _generation) {
          request.failStart(TtsPlaybackCancelledException());
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
          _hostSpeechBridge = request.bridge;
          try {
            final status = await _speakHostSystem(request);
            if (request.generation != _generation) {
              await _stopLateHostSpeech(request.bridge);
              continue;
            }
            request.completeStart();
            if (!status.active) {
              continue;
            }
            _publish(
              _state.copyWith(
                phase: status.paused
                    ? TtsPlaybackPhase.paused
                    : TtsPlaybackPhase.playing,
                currentAudioPath: audioPath,
                queueLength: _queue.length,
                audioIndex: 1,
                audioCount: 1,
                clearError: true,
              ),
            );
            await _waitHostPlayback(request, status);
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
        if (audioSources.isEmpty) {
          throw StateError('tts synthesis returned no audio sources');
        }
        for (var index = 0; index < audioSources.length; index += 1) {
          if (request.generation != _generation) {
            break;
          }
          final audioSource = audioSources[index];
          _publish(
            _state.copyWith(
              phase: TtsPlaybackPhase.preparing,
              currentAudioPath: audioSource.path,
              queueLength: _queue.length,
              audioIndex: index + 1,
              audioCount: audioSources.length,
              clearError: true,
            ),
          );
          await _playAudioSource(request, audioSource);
        }
        _currentRequest = null;
        activeRequest = null;
      }
      if (_state.phase != TtsPlaybackPhase.stopped &&
          _state.phase != TtsPlaybackPhase.error) {
        _publish(const TtsPlaybackState.idle());
      }
    } catch (error) {
      activeRequest?.failStart(error);
      if (activeRequest?.generation == _generation) {
        _publishPlaybackError(error);
      }
    } finally {
      if (identical(_currentRequest, activeRequest)) {
        _currentRequest = null;
      }
      _draining = false;
      if (_queue.isNotEmpty) {
        unawaited(_drainQueue());
      }
    }
  }

  /// Resolves whether a request uses live host system speech.
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
    if (config.providerType != _systemTtsProviderType) {
      return false;
    }
    final descriptor = await clients.servicesRuntimeHostInfoService
        .runtimeHostDescriptor();
    if (!descriptor.systemTtsPlaybackHost) {
      throw UnsupportedError(
        'System TTS playback is not implemented by ${descriptor.displayName}',
      );
    }
    return true;
  }

  /// Resolves the effective character TTS configuration.
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

  /// Synthesizes generated audio sources for one request.
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

  /// Starts host system speech and returns its authoritative state.
  Future<_TtsHostStatus> _speakHostSystem(_TtsPlaybackRequest request) async {
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
    return _callHostSpeech(request.bridge, methodName, args: args);
  }

  /// Polls host playback until completion or request cancellation.
  Future<void> _waitHostPlayback(
    _TtsPlaybackRequest request,
    _TtsHostStatus initialStatus,
  ) async {
    var status = initialStatus;
    while (request.generation == _generation && status.active) {
      await Future<void>.delayed(_hostSpeechPollInterval);
      if (request.generation != _generation) {
        return;
      }
      status = await _callHostSpeech(request.bridge, 'speechState');
      if (request.generation == _generation && status.active) {
        final phase = status.paused
            ? TtsPlaybackPhase.paused
            : TtsPlaybackPhase.playing;
        if (_state.phase != phase) {
          _publish(_state.copyWith(phase: phase));
        }
      }
    }
  }

  /// Stops speech that completed startup after its request was cancelled.
  Future<void> _stopLateHostSpeech(OperitRuntimeBridge bridge) async {
    try {
      await _callHostSpeech(bridge, 'stopSpeech');
    } catch (error) {
      _publishPlaybackError(error);
    }
  }

  /// Calls one host speech method and validates the returned status.
  Future<_TtsHostStatus> _callHostSpeech(
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
    return _TtsHostStatus.fromJson(result as Map<String, Object?>);
  }

  /// Starts one generated audio source through the runtime TTS host.
  Future<void> _playAudioSource(
    _TtsPlaybackRequest request,
    _TtsPlaybackAudioSource audioSource,
  ) async {
    _hostSpeechBridge = request.bridge;
    try {
      final result = await request.bridge.call(
        CoreCallRequest(
          requestId: _requestId(),
          targetPath: CoreObjectPath.parse('services.ttsPlaybackService'),
          methodName: 'playAudio',
          args: <String, Object?>{'path': audioSource.path},
        ),
      );
      final start = _TtsAudioStart.fromJson(result as Map<String, Object?>);
      if (!start.started) {
        throw StateError(
          'TTS host did not start audio playback: ${start.path}',
        );
      }
      if (request.generation != _generation) {
        await _stopLateHostSpeech(request.bridge);
        return;
      }
      request.completeStart();
      _publish(_state.copyWith(phase: TtsPlaybackPhase.playing));
      final status = await _callHostSpeech(request.bridge, 'speechState');
      await _waitHostPlayback(request, status);
    } finally {
      if (identical(_hostSpeechBridge, request.bridge)) {
        _hostSpeechBridge = null;
      }
    }
  }

  /// Publishes one immutable playback snapshot.
  void _publish(TtsPlaybackState state) {
    _state = state;
    notifyListeners();
  }

  /// Publishes a terminal playback error.
  void _publishPlaybackError(Object error) {
    _publish(
      _state.copyWith(
        phase: TtsPlaybackPhase.error,
        queueLength: _queue.length,
        error: '$error',
      ),
    );
  }

  /// Throws when the immediately preceding stop operation failed.
  void _throwStopError() {
    final error = _state.error;
    if (_state.phase == TtsPlaybackPhase.error && error != null) {
      throw StateError(error);
    }
  }
}

class _TtsPlaybackRequest {
  /// Creates a character-backed playback request.
  _TtsPlaybackRequest.character({
    required this.bridge,
    required this.characterCardId,
    required this.text,
    required this.title,
    required this.generation,
  }) : source = _TtsPlaybackSource.character,
       ttsConfigId = '',
       _started = Completer<void>();

  /// Creates a config-backed playback request.
  _TtsPlaybackRequest.config({
    required this.bridge,
    required this.ttsConfigId,
    required this.text,
    required this.title,
    required this.generation,
  }) : source = _TtsPlaybackSource.config,
       characterCardId = '',
       _started = Completer<void>();

  final OperitRuntimeBridge bridge;
  final _TtsPlaybackSource source;
  final String characterCardId;
  final String ttsConfigId;
  final String text;
  final String title;
  final int generation;
  final Completer<void> _started;

  /// Returns a future that completes when playback really starts.
  Future<void> get started => _started.future;

  /// Returns the source identifier displayed by the player.
  String get displayId => switch (source) {
    _TtsPlaybackSource.character => characterCardId,
    _TtsPlaybackSource.config => ttsConfigId,
  };

  /// Completes the start future after successful playback startup.
  void completeStart() {
    if (!_started.isCompleted) {
      _started.complete();
    }
  }

  /// Completes the start future with a startup or cancellation error.
  void failStart(Object error) {
    if (!_started.isCompleted) {
      _started.completeError(error);
    }
  }
}

enum _TtsPlaybackSource { character, config }

class _TtsPlaybackAudioSource {
  /// Creates a generated audio source descriptor.
  const _TtsPlaybackAudioSource({
    required this.path,
    required this.storagePath,
  });

  final String path;
  final String storagePath;
}

class _TtsAudioStart {
  /// Creates a validated generated audio start response.
  const _TtsAudioStart({required this.path, required this.started});

  /// Parses a generated audio start response from Core.
  factory _TtsAudioStart.fromJson(Map<String, Object?> json) {
    json['details'] as String;
    return _TtsAudioStart(
      path: json['path'] as String,
      started: json['started'] as bool,
    );
  }

  final String path;
  final bool started;
}

class _TtsHostStatus {
  /// Creates a validated host speech status.
  const _TtsHostStatus({required this.active, required this.paused});

  /// Parses a host speech status without guessing missing fields.
  factory _TtsHostStatus.fromJson(Map<String, Object?> json) {
    json['path'] as String;
    json['details'] as String;
    return _TtsHostStatus(
      active: json['active'] as bool,
      paused: json['paused'] as bool,
    );
  }

  final bool active;
  final bool paused;
}

class TtsPlaybackCancelledException implements Exception {
  /// Creates a cancellation error for playback that never started.
  const TtsPlaybackCancelledException();

  /// Returns the stable cancellation message.
  @override
  String toString() => 'TTS playback was cancelled before startup';
}

/// Reads a required list of strings from a JSON response.
List<String> _jsonStringList(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value is! List<Object?>) {
    throw StateError('tts synthesis result missing $key');
  }
  return value.map((item) => item as String).toList(growable: false);
}

/// Builds the synthetic path used for live host speech.
String _hostSpeechPath(String characterCardId) => 'host-tts:$characterCardId';

/// Creates a unique Flutter-side Core request identifier.
String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';
