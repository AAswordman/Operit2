import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:camera_platform_interface/camera_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/VoiceCallCamera.dart';
import 'package:operit2/ui/window/VoiceCallStage.dart';

void main() {
  for (final size in [
    const Size(340, 460),
    const Size(260, 360),
    const Size(620, 380),
  ]) {
    testWidgets('swaps one live preview and restores voice layout at $size', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = size;
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final platform = _CameraPlatform();
      addTearDown(platform.events.close);
      final camera = VoiceCallCamera(platform: platform);
      addTearDown(camera.dispose);
      const window = ValueKey('call-window');
      final preview = find.byKey(const ValueKey('call-camera-view'));
      final avatar = find.byKey(const ValueKey('call-avatar-view'));
      final swap = find.byKey(const ValueKey('call-swap-views'));

      await tester.pumpWidget(
        MaterialApp(
          home: VoiceCallStage(
            key: window,
            camera: camera,
            controls: const Center(child: Text('Hang up')),
          ),
        ),
      );
      final full = tester.getRect(find.byKey(window));
      final portrait = find.text('O');
      expect(tester.getCenter(portrait), full.center);
      expect(swap, findsNothing);
      expect(preview, findsNothing);
      expect(find.text('Hang up').hitTestable(), findsOneWidget);

      await tester.runAsync(camera.start);
      expect(camera.error, isNull);
      expect(camera.preview, isNotNull);
      await tester.pumpAndSettle();
      expect(tester.getRect(preview), full);
      final inset = tester.getRect(avatar);
      expect(inset.center.dx, greaterThan(full.center.dx));
      expect(inset.center.dy, lessThan(full.center.dy));
      expect(tester.getRect(swap), inset);
      expect(find.text('Hang up').hitTestable(), findsOneWidget);
      final texture = find.byKey(const ValueKey('test-camera-texture'));
      expect(platform.previewBuilds, 1);
      expect(texture, findsOneWidget);
      final mountedTexture = tester.element(texture);

      await tester.tap(swap);
      await tester.pumpAndSettle();
      expect(tester.getRect(preview), inset);
      expect(tester.getCenter(portrait), full.center);
      expect(tester.element(texture), same(mountedTexture));
      expect(platform.previewBuilds, 1);
      expect(platform.releases, isEmpty);

      await tester.tap(swap);
      await tester.pumpAndSettle();
      expect(tester.getRect(preview), full);
      await tester.tap(swap);
      await tester.pumpAndSettle();
      await tester.runAsync(camera.stop);
      await tester.pumpAndSettle();
      expect(preview, findsNothing);
      expect(swap, findsNothing);
      expect(tester.getCenter(portrait), full.center);
      expect(platform.releases, [1]);

      await tester.runAsync(camera.start);
      expect(camera.error, isNull);
      expect(camera.preview, isNotNull);
      await tester.pumpAndSettle();
      expect(tester.getRect(preview), full);
      await tester.runAsync(camera.stop);
      await tester.pumpAndSettle();
    });
  }

  testWidgets(
    'uses role artwork and translucent controls with the selected theme',
    (tester) async {
      const avatar =
          'data:image/png;base64,'
          'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==';
      final scheme = ColorScheme.fromSeed(seedColor: Colors.green).copyWith(
        primary: const Color(0xff108640),
        surface: const Color(0xffe1f3e7),
      );
      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData(colorScheme: scheme),
          home: VoiceCallStage(
            avatarUri: avatar,
            avatarTitle: 'My character',
            controls: const Text('Call controls'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final portrait = tester.widget<Image>(
        find.byKey(const ValueKey('call-role-avatar')),
      );
      expect(portrait.image, isA<MemoryImage>());
      expect(
        (portrait.image as MemoryImage).bytes,
        base64Decode(avatar.split(',').last),
      );
      expect(portrait.semanticLabel, 'My character');
      expect(find.byIcon(Icons.smart_toy_outlined), findsNothing);
      final controls = tester.widget<Material>(
        find.byKey(const ValueKey('call-controls-surface')),
      );
      expect(controls.color, scheme.surface.withValues(alpha: 0.28));
      expect(
        Theme.of(
          tester.element(find.text('Call controls')),
        ).colorScheme.primary,
        scheme.primary,
      );
      expect(tester.takeException(), isNull);
    },
  );

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
  int previewBuilds = 0;

  @override
  Widget buildPreview(int cameraId) {
    previewBuilds++;
    return const ColoredBox(
      key: ValueKey('test-camera-texture'),
      color: Colors.red,
    );
  }

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
