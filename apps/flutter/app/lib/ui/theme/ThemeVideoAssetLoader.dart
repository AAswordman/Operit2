// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:fvp/fvp.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:video_player/video_player.dart';

import '../../core/logging/ClientLogger.dart';
import 'OperitThemeAssets.dart';

/// Owns one video request, including cancellation before native teardown.
class ThemeVideoAssetLoader with WidgetsBindingObserver {
  ThemeVideoAssetLoader({
    required ThemeAssetResolver resolver,
    required this.reference,
    this.diagnostics,
    this.onPlaybackFailure,
    this.timeout = const Duration(seconds: 15),
  }) : _resolver = resolver,
       _sourceBytes = null;

  /// Probes selected bytes before a new resource is written or activated.
  ThemeVideoAssetLoader.fromBytes({
    required Uint8List bytes,
    required this.reference,
    this.diagnostics,
    this.onPlaybackFailure,
    this.timeout = const Duration(seconds: 15),
  }) : _resolver = null,
       _sourceBytes = bytes;

  final String reference;
  final ThemeAssetDiagnostics? diagnostics;
  final ValueChanged<ThemeAssetFailure>? onPlaybackFailure;
  final Duration timeout;
  final ThemeAssetResolver? _resolver;
  final Uint8List? _sourceBytes;
  final FvpCreateCancellation _creationCancellation = FvpCreateCancellation();
  final Completer<void> _cancelSignal = Completer<void>();
  final StackTrace _requestStack = StackTrace.current;
  VideoPlayerController? _controller;
  Future<VideoPlayerController>? _initialization;
  Future<ThemeAssetFailure?>? _disposal;
  ThemeAssetFailure? _asynchronousFailure;
  ThemeAssetStage _stage = ThemeAssetStage.read;
  int? _byteLength;
  bool _cancelledByOwner = false;
  bool _disposed = false;
  bool _observingLifecycle = false;
  bool _foreground = true;
  bool _autoplay = false;
  bool _resumePlayback = false;
  int _lifecyclePending = 0;
  Future<void> _lifecycleUpdate = Future<void>.value();

  VideoPlayerController? get controller => _controller;
  bool get isCancelled => _cancelledByOwner;

  /// Initializes, verifies dimensions, then configures before any playback.
  Future<VideoPlayerController> initialize({
    bool muted = true,
    bool loop = true,
    bool autoplay = false,
  }) => _initialization ??= _guardResourceCallbacks(
    () => _initialize(muted: muted, loop: loop, autoplay: autoplay),
  );

  Future<VideoPlayerController> _initialize({
    required bool muted,
    required bool loop,
    required bool autoplay,
  }) async {
    try {
      _checkCurrent();
      final Uint8List bytes = _sourceBytes ??
          await _bounded<Uint8List>(_resolver!.readBytes(reference));
      _checkCurrent();
      _byteLength = bytes.length;
      if (bytes.isEmpty) {
        throw _failure(
          StateError('Theme video is empty'),
          StackTrace.current,
          kind: ThemeAssetFailureKind.empty,
        );
      }
      _stage = ThemeAssetStage.videoOpen;
      final uri = Uri.dataFromBytes(
        bytes,
        mimeType: themeAssetVideoMimeType(reference),
      );
      final controller = VideoPlayerController.networkUrl(
        uri,
        // The library observer starts unguarded pause/play futures. Our own
        // observer below restores that behavior inside the resource error zone.
        videoPlayerOptions: VideoPlayerOptions(allowBackgroundPlayback: true),
      );
      _controller = controller;
      final lifecycle = WidgetsBinding.instance.lifecycleState;
      _foreground = lifecycle == null || lifecycle == AppLifecycleState.resumed;
      WidgetsBinding.instance.addObserver(this);
      _observingLifecycle = true;
      await _bounded(_creationCancellation.run(controller.initialize));
      _checkCurrent();
      _checkReady();
      await _configure(muted: muted, loop: loop, autoplay: autoplay);
      _checkCurrent();
      return controller;
    } catch (error, stack) {
      final failure = _asynchronousFailure ??
          (error is FvpCreateCancelledException
              ? error
              : _failure(error, stack));
      await dispose();
      Error.throwWithStackTrace(failure, stack);
    }
  }

  /// Keeps player callbacks/timers in the same locally guarded resource zone.
  Future<void> configure({
    required bool muted,
    required bool loop,
    bool? autoplay,
  }) => _guardResourceCallbacks(
    () => _configure(
      muted: muted,
      loop: loop,
      autoplay: autoplay ?? (_foreground && _lifecyclePending == 0
          ? _controller?.value.isPlaying == true
          : _resumePlayback),
    ),
  );

  Future<void> _configure({
    required bool muted,
    required bool loop,
    required bool autoplay,
  }) async {
    _stage = ThemeAssetStage.playback;
    _autoplay = autoplay;
    _resumePlayback = autoplay;
    try {
      _checkCurrent();
      final controller = _controller!;
      await _bounded(controller.setLooping(loop));
      _checkCurrent();
      await _bounded(controller.setVolume(muted ? 0 : 1));
      _checkCurrent();
      await _bounded(_foreground && _autoplay ? controller.play() : controller.pause());
      _checkCurrent();
      _checkReady();
    } catch (error, stack) {
      if (error is FvpCreateCancelledException) rethrow;
      Error.throwWithStackTrace(_failure(error, stack), stack);
    }
  }

  /// Public controller value is the only available evidence for late events.
  void _checkReady() {
    final value = _controller!.value;
    if (value.hasError) {
      throw _failure(
        StateError('Video player reported: ${value.errorDescription}'),
        StackTrace.empty,
      );
    }
    if (!value.isInitialized ||
        !value.size.width.isFinite || !value.size.height.isFinite ||
        value.size.width <= 0 || value.size.height <= 0) {
      throw StateError('Theme video has no usable dimensions');
    }
  }

  /// Synchronous final check for callers publishing after an await boundary.
  void validateReady() {
    try {
      _checkCurrent();
      _checkReady();
    } catch (error, stack) {
      if (error is FvpCreateCancelledException) rethrow;
      Error.throwWithStackTrace(_failure(error, stack), stack);
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final foreground = state == AppLifecycleState.resumed;
    if (_foreground && !foreground && _lifecyclePending == 0) {
      _resumePlayback = _controller?.value.isPlaying == true;
    }
    _foreground = foreground;
    if (_disposed || _controller?.value.isInitialized != true) return;
    _lifecyclePending++;
    _lifecycleUpdate = _lifecycleUpdate.then((_) => _updateLifecyclePlayback());
  }

  Future<void> _updateLifecyclePlayback() async {
    try {
      await _guardResourceCallbacks(() async {
        _stage = ThemeAssetStage.playback;
        _checkCurrent();
        final controller = _controller!;
        await _bounded(_foreground && _resumePlayback ? controller.play() : controller.pause());
        _checkCurrent();
        _checkReady();
      });
    } catch (error, stack) {
      if (_disposed || _cancelledByOwner) return;
      final failure = _failure(error, stack);
      _asynchronousFailure ??= failure;
      _cancelNativePreparation();
      onPlaybackFailure?.call(failure);
      if (onPlaybackFailure == null) {
        diagnostics?.report(failure, owner: this);
        await dispose();
      }
    } finally {
      _lifecyclePending--;
    }
  }

  /// video_player also launches internal settings/timer futures without awaiting
  /// them. Consume those failures locally without changing the global error hook.
  /// The outer completer is created before entering the error zone, so an error
  /// future never has to cross an error-zone boundary to reach its caller.
  Future<T> _guardResourceCallbacks<T>(Future<T> Function() action) {
    final result = Completer<T>();
    runZonedGuarded<void>(
      () {
        Future<T>.sync(action).then<void>(
          (value) {
            if (!result.isCompleted) result.complete(value);
          },
          onError: (Object error, StackTrace stack) {
            if (!result.isCompleted) result.completeError(error, stack);
          },
        );
      },
      (Object error, StackTrace stack) {
        if (error is FvpCreateCancelledException &&
            (_cancelledByOwner || _disposed)) {
          return;
        }
        final failure = _failure(error, stack);
        if (_disposed) {
          _logCleanupFailure(failure);
          return;
        }
        _asynchronousFailure ??= failure;
        _cancelNativePreparation();
        if (!result.isCompleted) {
          result.completeError(failure, stack);
        } else {
          if (onPlaybackFailure != null) {
            onPlaybackFailure!(failure);
          } else {
            diagnostics?.report(failure, owner: this);
            unawaited(dispose());
          }
        }
      },
    );
    return result.future;
  }

  Future<T> _bounded<T>(Future<T> operation) => Future.any<T>([
    operation,
    _cancelSignal.future.then<T>((_) => throw FvpCreateCancelledException()),
  ]).timeout(
    timeout,
    onTimeout: () {
      _creationCancellation.cancel();
      throw TimeoutException('Theme video operation timed out', timeout);
    },
  );

  void _checkCurrent() {
    final failure = _asynchronousFailure;
    if (failure != null) throw failure;
    if (_cancelledByOwner || _disposed) throw FvpCreateCancelledException();
  }

  void _cancelNativePreparation() {
    _creationCancellation.cancel();
    if (!_cancelSignal.isCompleted) _cancelSignal.complete();
  }

  /// Invalidates a request without calling it a damaged or unsupported file.
  void cancel() {
    _cancelledByOwner = true;
    _cancelNativePreparation();
  }

  /// Cancels before waiting for native disposal. Cleanup failures are logged
  /// separately; a probe can require cleanup success before saving its URI.
  Future<void> dispose({bool throwOnFailure = false}) async {
    cancel();
    _disposed = true;
    if (_observingLifecycle) {
      WidgetsBinding.instance.removeObserver(this);
      _observingLifecycle = false;
    }
    final failure = await (_disposal ??= _disposeController());
    final primary = _asynchronousFailure ?? failure;
    if (primary != null && throwOnFailure) {
      Error.throwWithStackTrace(primary, primary.originalStackTrace);
    }
  }

  Future<ThemeAssetFailure?> _disposeController() async {
    final controller = _controller;
    if (controller == null) return null;
    try {
      // timeout observes the still-running teardown future, including a late
      // error; it does not claim that native resources were already released.
      await controller.dispose().timeout(timeout);
      return null;
    } catch (error, stack) {
      final failure = _failure(
        error,
        stack,
        stage: ThemeAssetStage.dispose,
      );
      _logCleanupFailure(failure);
      return failure;
    }
  }

  ThemeAssetFailure _failure(
    Object cause,
    StackTrace stack, {
    ThemeAssetStage? stage,
    ThemeAssetFailureKind? kind,
  }) => ThemeAssetFailure.capture(
    reference: reference,
    purpose: ThemeAssetPurpose.background,
    stage: stage ?? _stage,
    kind: kind ??
        (cause is TimeoutException
            ? ThemeAssetFailureKind.timeout
            : _stage == ThemeAssetStage.read
            ? ThemeAssetFailureKind.read
            : ThemeAssetFailureKind.media),
    byteLength: _byteLength,
    cause: cause,
    originalStackTrace: stack,
    requestStackTrace: _requestStack,
  ).withContext(themeRevision: diagnostics?.themeRevision);

  void _logCleanupFailure(ThemeAssetFailure failure) {
    ClientLogger.w(
      failure.details,
      tag: 'ThemeVideoCleanup',
      error: failure,
      stackTrace: failure.originalStackTrace,
    );
  }
}
