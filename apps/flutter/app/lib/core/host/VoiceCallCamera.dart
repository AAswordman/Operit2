// ignore_for_file: file_names
import 'dart:async';
import 'dart:io';

import 'package:camera_platform_interface/camera_platform_interface.dart';
import 'package:flutter/foundation.dart';

const voiceCallPackage = 'com.operit.voice_call';
const voiceCallRoute = 'toolpkg:$voiceCallPackage:ui:call';
const voiceCallOverlayRoute = 'toolpkg:$voiceCallPackage:ui:voice-overlay';
const voiceCallCameraCapture = 'voiceCallCameraCapture';
const voiceCallCameraStop = 'voiceCallCameraStop';
const voiceCallFrameLimit = 2 * 1024 * 1024;

bool get voiceCallCameraSupported =>
    !kIsWeb &&
    (Platform.isAndroid ||
        Platform.isIOS ||
        Platform.isMacOS ||
        Platform.isWindows ||
        Platform.isLinux);

bool get voiceCallCameraNeedsActivity => !kIsWeb && Platform.isAndroid;

/// Owns the camera in the same engine that displays its native texture.
class VoiceCallCamera extends ChangeNotifier {
  VoiceCallCamera({CameraPlatform? platform})
    : platform = platform ?? CameraPlatform.instance;

  final CameraPlatform platform;
  Future<void> _tail = Future<void>.value();
  bool _requested = false;
  bool _disposed = false;
  int _generation = 0;
  int? _cameraId;
  CameraInitializedEvent? _preview;
  Object? _error;
  StreamSubscription<CameraErrorEvent>? _errors;

  CameraInitializedEvent? get preview => _preview;
  Object? get error => _error;
  bool get enabled => _requested;

  Future<T> _enqueue<T>(Future<T> Function() operation) {
    final future = _tail.then((_) => operation());
    _tail = future.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return future;
  }

  Future<void> start() {
    if (_disposed || _requested) return Future<void>.value();
    _requested = true;
    _error = null;
    final generation = ++_generation;
    notifyListeners();
    return _enqueue(() async {
      if (generation != _generation) return;
      try {
        final cameras = await platform.availableCameras();
        if (generation != _generation) return;
        if (cameras.isEmpty) throw CameraException('noCamera', null);
        final camera =
            cameras
                .where(
                  (device) => device.lensDirection == CameraLensDirection.front,
                )
                .firstOrNull ??
            cameras.first;
        final id = await platform.createCameraWithSettings(
          camera,
          const MediaSettings(
            resolutionPreset: ResolutionPreset.medium,
            enableAudio: false,
          ),
        );
        _cameraId = id;
        if (generation != _generation) {
          await _release();
          return;
        }
        _errors = platform.onCameraError(id).listen((event) {
          if (generation != _generation) return;
          _error = CameraException('cameraError', event.description);
          unawaited(stop());
        });
        final initialized = Completer<CameraInitializedEvent>();
        final subscription = platform.onCameraInitialized(id).listen((event) {
          if (!initialized.isCompleted) initialized.complete(event);
        });
        final CameraInitializedEvent event;
        try {
          await platform
              .initializeCamera(id)
              .timeout(const Duration(seconds: 10));
          event = await initialized.future.timeout(const Duration(seconds: 3));
        } finally {
          await subscription.cancel();
        }
        if (generation != _generation) {
          await _release();
          return;
        }
        if (event.previewWidth <= 0 || event.previewHeight <= 0) {
          throw CameraException('previewUnavailable', null);
        }
        _preview = event;
      } catch (error) {
        if (generation == _generation) {
          _requested = false;
          _error = error;
          _preview = null;
        }
        await _release();
      }
      if (!_disposed && generation == _generation) notifyListeners();
    });
  }

  Future<Uint8List?> capture() {
    final generation = _generation;
    if (!_requested || _preview == null) return Future.value(null);
    return _enqueue(() async {
      final id = _cameraId;
      if (generation != _generation || id == null) return null;
      final picture = await platform.takePicture(id);
      try {
        if (generation != _generation) return null;
        final length = await picture.length();
        if (length <= 0 || length > voiceCallFrameLimit) {
          throw StateError('Camera frame exceeds the supported size');
        }
        final bytes = await picture.readAsBytes();
        return generation == _generation ? bytes : null;
      } finally {
        // The attachment store keeps the submitted frame independently.
        await File(picture.path).delete();
      }
    });
  }

  Future<void> stop() {
    if (!_requested && _cameraId == null) return Future<void>.value();
    _generation++;
    final wasRequested = _requested;
    _requested = false;
    _preview = null;
    if (!_disposed && wasRequested) notifyListeners();
    return _enqueue(_release);
  }

  Future<void> _release() async {
    final id = _cameraId;
    _cameraId = null;
    await _errors?.cancel();
    _errors = null;
    if (id == null) return;
    try {
      await platform.dispose(id);
    } catch (error) {
      debugPrint('Could not release call camera: $error');
    }
  }

  @override
  void dispose() {
    _disposed = true;
    unawaited(stop());
    super.dispose();
  }
}
