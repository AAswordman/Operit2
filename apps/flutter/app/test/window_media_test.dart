import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:camera_platform_interface/camera_platform_interface.dart';
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/RuntimeWindowCamera.dart';
import 'package:operit2/core/host/RuntimeWindowCameraFrameStore.dart';
import 'package:operit2/core/host/ComposeDslFilePickerStorage.dart';
import 'package:operit2/ui/window/MediaStage.dart';
import 'package:operit2/ui/window/FloatingWindowFrame.dart';

void main() {
  test(
    'camera files are reusable selections with bounded upload chunks',
    () async {
      final storage = _FrameStorage();
      final bytes = Uint8List(150000);
      final frame = await RuntimeWindowCameraFrameStore(
        storage: storage,
      ).save(bytes, isVisible: () => true);
      expect(frame, {
        'path': storage.path,
        'name': 'camera-frame.jpg',
        'mimeType': 'image/jpeg',
        'size': bytes.length,
      });
      expect(storage.chunks, [65536, 65536, 18928]);
      expect(storage.deleted, isEmpty);
    },
  );

  test('a window hidden during publication discards its camera file', () async {
    var visible = true;
    final storage = _FrameStorage()..afterWrite = () => visible = false;
    final store = RuntimeWindowCameraFrameStore(storage: storage);
    expect(await store.save(Uint8List(10), isVisible: () => visible), isNull);
    expect(storage.deleted, [storage.path]);
  });

  test(
    'failed publication cleans its destination and propagates the failure',
    () async {
      final storage = _FrameStorage()..failWrite = true;
      final store = RuntimeWindowCameraFrameStore(storage: storage);
      await expectLater(
        store.save(Uint8List(10), isVisible: () => true),
        throwsStateError,
      );
      expect(storage.deleted, [storage.path]);
    },
  );

  test(
    'hidden windows and invalid frames allocate no selected files',
    () async {
      final storage = _FrameStorage();
      final store = RuntimeWindowCameraFrameStore(storage: storage);
      expect(await store.save(Uint8List(10), isVisible: () => false), isNull);
      await expectLater(
        store.save(Uint8List(0), isVisible: () => true),
        throwsStateError,
      );
      await expectLater(
        store.save(
          Uint8List(windowCameraFrameLimit + 1),
          isVisible: () => true,
        ),
        throwsStateError,
      );
      expect(storage.created, 0);
    },
  );

  testWidgets('fullscreen commands serialize and propagate platform failures', (
    tester,
  ) async {
    final key = GlobalKey<FloatingWindowFrameState>();
    final updates = <bool>[];
    var fail = false;
    final launch = FloatingWindowLaunch(
      payload: {},
      options: const FloatingWindowOptions(width: 340, height: 460),
      onWindowCommand: (method, arguments) async {
        if (method == 'getBounds') {
          return {'x': 24, 'y': 48, 'width': 340, 'height': 460};
        }
        if (method == 'update') {
          if (fail) throw StateError('Window update failed');
          updates.add((arguments as Map)['fill'] as bool);
          return null;
        }
        throw StateError('Unexpected window command: $method');
      },
    );
    await tester.pumpWidget(
      MaterialApp(
        home: FloatingWindowFrame(
          key: key,
          launch: launch,
          builder: (fullscreen, toolbar, handle) =>
              Column(children: [toolbar, Text('$fullscreen')]),
        ),
      ),
    );
    final enter = key.currentState!.setFullscreen(true);
    final restore = key.currentState!.setFullscreen(false);
    await tester.pumpAndSettle();
    await Future.wait([enter, restore]);
    expect(updates, [true, false]);
    expect(find.text('false'), findsOneWidget);
    fail = true;
    final failed = expectLater(
      key.currentState!.setFullscreen(true),
      throwsStateError,
    );
    await tester.pumpAndSettle();
    await failed;
    expect(find.text('false'), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

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
      final camera = RuntimeWindowCamera(platform: platform);
      addTearDown(camera.dispose);
      const window = ValueKey('call-window');
      final preview = find.byKey(const ValueKey('media-camera-view'));
      final avatar = find.byKey(const ValueKey('media-image-view'));
      final swap = find.byKey(const ValueKey('media-swap-views'));

      await tester.pumpWidget(
        MaterialApp(
          home: MediaStage(
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
      final imageRect = tester.getRect(texture);
      final expectedRatio =
          windowCameraNeedsActivity && size.height > size.width
          ? 480 / 640
          : 640 / 480;
      expect(
        imageRect.width / imageRect.height,
        closeTo(expectedRatio, 0.00001),
      );
      expect(full.inflate(0.001).contains(imageRect.topLeft), isTrue);
      expect(full.inflate(0.001).contains(imageRect.bottomRight), isTrue);
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

  testWidgets('full screen restore and drag resize preserve the live camera', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final platform = _CameraPlatform();
    addTearDown(platform.events.close);
    final camera = RuntimeWindowCamera(platform: platform);
    addTearDown(camera.dispose);
    await tester.pumpWidget(
      FloatingWindowsHost(
        windowBuilder: (launch) => MaterialApp(
          home: FloatingWindowFrame(
            launch: launch,
            builder: (fullscreen, toolbar, resizeHandle) => MediaStage(
              camera: camera,
              fullscreen: fullscreen,
              windowControls: toolbar,
              resizeHandle: resizeHandle,
              controls: const Center(child: Text('Hang up')),
            ),
          ),
        ),
        child: const SizedBox.expand(),
      ),
    );
    final window = await FloatingWindows.open(
      applicationOnly: true,
      payload: {},
      fill: true,
    );
    await tester.pumpAndSettle();
    final toggle = find.byKey(const ValueKey('window-toggle-fullscreen'));
    final handle = find.byKey(const ValueKey('window-resize-handle'));
    final stage = find.byType(MediaStage);
    expect(tester.getSize(stage), const Size(900, 700));
    expect(handle, findsNothing);
    await tester.runAsync(camera.start);
    await tester.pumpAndSettle();
    final texture = find.byKey(const ValueKey('test-camera-texture'));
    final mountedTexture = tester.element(texture);

    await tester.tap(toggle);
    await tester.pumpAndSettle();
    expect(tester.getSize(stage), const Size(340, 460));
    expect(handle, findsOneWidget);
    expect(tester.element(texture), same(mountedTexture));
    final gesture = await tester.startGesture(tester.getCenter(handle));
    await tester.pump();
    await gesture.moveBy(const Offset(90, 70));
    await tester.pump();
    await gesture.up();
    await tester.pumpAndSettle();
    expect(tester.getSize(stage), const Size(430, 530));
    expect((await window.getBounds()).width, 430);
    expect(tester.element(texture), same(mountedTexture));
    expect(platform.previewBuilds, 1);

    await tester.tap(toggle);
    await tester.pumpAndSettle();
    expect(tester.getSize(stage), const Size(900, 700));
    await tester.binding.setSurfaceSize(const Size(760, 620));
    await tester.pumpAndSettle();
    expect(tester.getSize(stage), const Size(760, 620));
    await tester.tap(toggle);
    await tester.pumpAndSettle();
    expect(tester.getSize(stage), const Size(430, 530));
    expect(tester.element(texture), same(mountedTexture));
    expect(platform.releases, isEmpty);
    await tester.runAsync(camera.stop);
    await window.close();
    await tester.pumpAndSettle();
    await tester.pumpWidget(const SizedBox());
  });

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
          home: MediaStage(
            imageUri: avatar,
            imageTitle: 'My character',
            controls: const Text('Call controls'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final portrait = tester.widget<Image>(
        find.byKey(const ValueKey('media-portrait')),
      );
      expect(portrait.image, isA<MemoryImage>());
      expect(
        (portrait.image as MemoryImage).bytes,
        base64Decode(avatar.split(',').last),
      );
      expect(portrait.semanticLabel, 'My character');
      expect(find.byIcon(Icons.smart_toy_outlined), findsNothing);
      final controls = tester.widget<Material>(
        find.byKey(const ValueKey('media-controls-surface')),
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
    final camera = RuntimeWindowCamera(platform: platform);
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

class _FrameStorage extends Fake implements ComposeDslFilePickerStorage {
  final path = '/operitapp/data/selected-files/test/camera-frame.jpg';
  final chunks = <int>[];
  final deleted = <String>[];
  int created = 0;
  bool failWrite = false;
  void Function()? afterWrite;

  @override
  Future<String> createFile(String name) async {
    expect(name, 'camera-frame.jpg');
    created++;
    return path;
  }

  @override
  Future<void> writeFile(String path, int size, Stream<Uint8List> bytes) async {
    expect(path, this.path);
    await for (final chunk in bytes) {
      chunks.add(chunk.length);
    }
    expect(chunks.fold<int>(0, (sum, length) => sum + length), size);
    if (failWrite) throw StateError('write failed');
    afterWrite?.call();
  }

  @override
  Future<void> deleteFile(String path) async {
    deleted.add(path);
  }
}
