import 'dart:async';
import 'dart:io';

import 'package:camera_platform_interface/camera_platform_interface.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/VoiceCallCamera.dart';

void main() {
  test('turning off the camera discards an in-flight frame', () async {
    final directory = await Directory.systemTemp.createTemp('call-camera-');
    addTearDown(() => directory.delete(recursive: true));
    final file = await File('${directory.path}/frame.jpg').writeAsBytes([1, 2]);
    final platform = _CameraPlatform();
    addTearDown(platform.events.close);
    final camera = VoiceCallCamera(platform: platform);
    addTearDown(camera.dispose);

    expect(await camera.capture(), isNull);
    expect(platform.captureStarted.isCompleted, isFalse);
    await camera.start();
    expect(camera.preview?.cameraId, 1);
    final frame = camera.capture();
    await platform.captureStarted.future;
    final stopping = camera.stop();
    expect(camera.preview, isNull);
    platform.picture.complete(XFile(file.path));

    expect(await frame, isNull);
    await stopping;
    expect(await file.exists(), isFalse);
    expect(platform.releases, [1]);
  });
}

class _CameraPlatform extends CameraPlatform {
  final events = StreamController<CameraInitializedEvent>.broadcast();
  final captureStarted = Completer<void>();
  final picture = Completer<XFile>();
  final releases = <int>[];

  @override
  Future<List<CameraDescription>> availableCameras() async => const [
    CameraDescription(
      name: 'Test camera',
      lensDirection: CameraLensDirection.front,
      sensorOrientation: 0,
    ),
  ];

  @override
  Future<int> createCameraWithSettings(
    CameraDescription cameraDescription,
    MediaSettings? mediaSettings,
  ) async {
    expect(mediaSettings?.enableAudio, isFalse);
    return 1;
  }

  @override
  Future<void> initializeCamera(
    int cameraId, {
    ImageFormatGroup imageFormatGroup = ImageFormatGroup.unknown,
  }) async {
    events.add(
      CameraInitializedEvent(
        cameraId,
        640,
        480,
        ExposureMode.auto,
        false,
        FocusMode.auto,
        false,
      ),
    );
  }

  @override
  Stream<CameraInitializedEvent> onCameraInitialized(int cameraId) =>
      events.stream;

  @override
  Stream<CameraErrorEvent> onCameraError(int cameraId) => const Stream.empty();

  @override
  Future<XFile> takePicture(int cameraId) {
    captureStarted.complete();
    return picture.future;
  }

  @override
  Future<void> dispose(int cameraId) async => releases.add(cameraId);
}
