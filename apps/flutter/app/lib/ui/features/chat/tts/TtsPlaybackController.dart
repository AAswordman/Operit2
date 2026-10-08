// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
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
  GeneratedCoreProxyClients? _hostSpeechClients;
  _TtsPlaybackRequest? _currentRequest;
  Future<void>? _stopInProgress;
  TtsPlaybackState _state = const TtsPlaybackState.idle();

  /// Returns the latest playback snapshot.
  TtsPlaybackState get state => _state;

  /// Queues the displayed message locator before resolving its persisted execution voice.
  Future<void> speakForMessage({
    required OperitRuntimeBridge bridge,
    required String chatId,
    required int messageTimestamp,
    required int variantIndex,
    required String text,
    bool interrupt = true,
  }) async {
    if (chatId.trim().isEmpty || variantIndex < 0 || text.trim().isEmpty) {
      throw ArgumentError(
        'Message speech requires a chat, selected variant, and text.',
      );
    }
    if (interrupt) {
      await stop();
      _throwStopError();
    }
    await _queueRequest(
      _TtsPlaybackRequest(
        bridge: bridge,
        source: _MessageTtsSource(
          chatId: chatId,
          messageTimestamp: messageTimestamp,
          variantIndex: variantIndex,
        ),
        text: text,
        generation: _generation,
      ),
    );
  }

  /// Starts configured speech and completes after playback actually starts.
  Future<void> speakWithConfig({
    required OperitRuntimeBridge bridge,
    required String ttsConfigId,
    required String text,
    required String title,
    bool interrupt = true,
  }) async {
    if (ttsConfigId.trim().isEmpty || text.trim().isEmpty) {
      throw ArgumentError(
        'Configured speech requires a nonblank voice ID and text.',
      );
    }
    if (interrupt) {
      await stop();
      _throwStopError();
    }
    await _queueRequest(
      _TtsPlaybackRequest(
        bridge: bridge,
        source: _ConfigTtsSource(ttsConfigId: ttsConfigId, title: title),
        text: text,
        generation: _generation,
      ),
    );
  }

  /// Attaches startup completion to the existing ordered and cancellable queue.
  Future<void> _queueRequest(_TtsPlaybackRequest request) async {
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
      final status = await _readHostStatus(
        _hostSpeechClients!.servicesTtsPlaybackService.pauseSpeech(),
      );
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
      final status = await _readHostStatus(
        _hostSpeechClients!.servicesTtsPlaybackService.resumeSpeech(),
      );
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
    final hostSpeechClients = _hostSpeechClients;
    _hostSpeechClients = null;
    Object? stopError;
    try {
      if (hostSpeechBridge != null) {
        await hostSpeechClients!.servicesTtsPlaybackService.stopSpeech();
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
            title: '',
            currentText: request.text,
            currentAudioPath: '',
            queueLength: _queue.length,
            audioIndex: 0,
            audioCount: 0,
            clearError: true,
          ),
        );
        final voice = await _resolveVoice(request);
        if (request.generation != _generation) {
          continue;
        }
        _publish(_state.copyWith(title: voice.title));
        final usesHostSystemSpeech = await _usesHostSystemSpeech(
          request,
          voice,
        );
        if (request.generation != _generation) {
          continue;
        }
        if (usesHostSystemSpeech) {
          final audioPath = _hostSpeechPath(voice.ttsConfigId);
          _hostSpeechBridge = request.bridge;
          _hostSpeechClients = request.clients;
          try {
            final status = await _speakHostSystem(request, voice);
            if (request.generation != _generation) {
              await _stopLateHostSpeech(request.clients);
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
              _hostSpeechClients = null;
            }
          }
          continue;
        }
        final audioSources = await _synthesize(request, voice);
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

  /// Resolves one saved message profile or an explicitly requested settings preview voice.
  Future<_ResolvedTtsVoice> _resolveVoice(_TtsPlaybackRequest request) async {
    final _ResolvedTtsVoice voice;
    switch (request.source) {
      case _MessageTtsSource(
        :final chatId,
        :final messageTimestamp,
        :final variantIndex,
      ):
        final saved = await request.clients.chatRuntimeHolderMain
            .chatConfigurationForMessage(
              chatId: chatId,
              messageTimestamp: messageTimestamp,
              variantIndex: variantIndex,
            );
        voice = _ResolvedTtsVoice(
          ttsConfigId: saved.profile.ttsConfigId,
          title: saved.profile.name,
        );
      case _ConfigTtsSource(:final ttsConfigId, :final title):
        voice = _ResolvedTtsVoice(ttsConfigId: ttsConfigId, title: title);
    }
    if (voice.ttsConfigId.trim().isEmpty) {
      throw StateError(
        'The saved execution profile has no TTS voice configuration.',
      );
    }
    return voice;
  }

  /// Selects the real playback backend using only the resolved configuration and host capability.
  Future<bool> _usesHostSystemSpeech(
    _TtsPlaybackRequest request,
    _ResolvedTtsVoice voice,
  ) async {
    final config = await request.clients.preferencesTtsConfigManager
        .getTtsConfig(id: voice.ttsConfigId);
    if (config.providerType != _systemTtsProviderType) {
      return false;
    }
    final descriptor = await request.clients.servicesRuntimeHostInfoService
        .runtimeHostDescriptor();
    if (!descriptor.systemTtsPlaybackHost) {
      throw UnsupportedError(
        'System TTS playback is not implemented by ${descriptor.displayName}',
      );
    }
    return true;
  }

  /// Synthesizes generated audio sources for one request.
  Future<List<_TtsPlaybackAudioSource>> _synthesize(
    _TtsPlaybackRequest request,
    _ResolvedTtsVoice voice,
  ) async {
    final result = await request.clients.servicesTtsSynthesisService
        .synthesizeWithConfig(
          ttsConfigId: voice.ttsConfigId,
          text: request.text,
        );
    final json = result.toJson();
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
  Future<_TtsHostStatus> _speakHostSystem(
    _TtsPlaybackRequest request,
    _ResolvedTtsVoice voice,
  ) async {
    final result = await request.clients.servicesTtsPlaybackService
        .speakWithConfig(
          ttsConfigId: voice.ttsConfigId,
          text: request.text,
          interrupt: true,
        );
    return _TtsHostStatus.fromJson(result.toJson());
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
      status = await _readHostStatus(
        request.clients.servicesTtsPlaybackService.speechState(),
      );
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
  Future<void> _stopLateHostSpeech(GeneratedCoreProxyClients clients) async {
    try {
      await clients.servicesTtsPlaybackService.stopSpeech();
    } catch (error) {
      _publishPlaybackError(error);
    }
  }

  /// Reads one generated host playback response into the controller status.
  Future<_TtsHostStatus> _readHostStatus(
    Future<core_proxy.TtsHostPlaybackResult> response,
  ) async {
    final result = await response;
    return _TtsHostStatus.fromJson(result.toJson());
  }

  /// Starts one generated audio source through the runtime TTS host.
  Future<void> _playAudioSource(
    _TtsPlaybackRequest request,
    _TtsPlaybackAudioSource audioSource,
  ) async {
    _hostSpeechBridge = request.bridge;
    _hostSpeechClients = request.clients;
    try {
      final result = await request.clients.servicesTtsPlaybackService.playAudio(
        path: audioSource.path,
      );
      final start = _TtsAudioStart.fromJson(result.toJson());
      if (!start.started) {
        throw StateError(
          'TTS host did not start audio playback: ${start.path}',
        );
      }
      if (request.generation != _generation) {
        await _stopLateHostSpeech(request.clients);
        return;
      }
      request.completeStart();
      _publish(_state.copyWith(phase: TtsPlaybackPhase.playing));
      final status = await _readHostStatus(
        request.clients.servicesTtsPlaybackService.speechState(),
      );
      await _waitHostPlayback(request, status);
    } finally {
      if (identical(_hostSpeechBridge, request.bridge)) {
        _hostSpeechBridge = null;
        _hostSpeechClients = null;
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
  /// Captures one explicit voice source within the current cancellation generation.
  _TtsPlaybackRequest({
    required this.bridge,
    required this.source,
    required this.text,
    required this.generation,
  }) : clients = GeneratedCoreProxyClients(bridge),
       _started = Completer<void>();

  final OperitRuntimeBridge bridge;
  final GeneratedCoreProxyClients clients;
  final _TtsPlaybackSource source;
  final String text;
  final int generation;
  final Completer<void> _started;

  /// Returns a future that completes when playback really starts.
  Future<void> get started => _started.future;

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

/// Restricts queued voice sources to a persisted message locator or an explicit preview.
sealed class _TtsPlaybackSource {
  /// Creates a typed voice source without nullable identity placeholders.
  const _TtsPlaybackSource();
}

final class _MessageTtsSource extends _TtsPlaybackSource {
  /// Captures the displayed transcript and selected historical message variant.
  const _MessageTtsSource({
    required this.chatId,
    required this.messageTimestamp,
    required this.variantIndex,
  });

  final String chatId;
  final int messageTimestamp;
  final int variantIndex;
}

final class _ConfigTtsSource extends _TtsPlaybackSource {
  /// Keeps the existing explicit configuration preview separate from message identity.
  const _ConfigTtsSource({required this.ttsConfigId, required this.title});

  final String ttsConfigId;
  final String title;
}

class _ResolvedTtsVoice {
  /// Retains the single authoritative voice resolution used throughout playback.
  const _ResolvedTtsVoice({required this.ttsConfigId, required this.title});

  final String ttsConfigId;
  final String title;
}

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
String _hostSpeechPath(String ttsConfigId) => 'host-tts:$ttsConfigId';
