import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:fvp/fvp.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/theme/ThemeVideoAssetLoader.dart';
import 'package:video_player/video_player.dart';
import 'package:video_player_platform_interface/video_player_platform_interface.dart';

/// Uses the real video controller and fake native events, including empty stacks.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late VideoPlayerPlatform previousPlatform;
  late _VideoPlatform platform;

  setUp(() {
    previousPlatform = VideoPlayerPlatform.instance;
    platform = _VideoPlatform();
    VideoPlayerPlatform.instance = platform;
  });
  tearDown(() {
    VideoPlayerPlatform.instance = previousPlatform;
    TestWidgetsFlutterBinding.ensureInitialized().handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  ThemeVideoAssetLoader loader({
    Duration timeout = const Duration(seconds: 15),
    Uint8List? bytes,
  }) => ThemeVideoAssetLoader.fromBytes(
    bytes: bytes ?? Uint8List.fromList([1, 2, 3]),
    reference: 'runtime/data/user_assets/theme/test.mp4',
    timeout: timeout,
  );

  test('open failures retain their resource, cause and empty original stack', () async {
    platform.modes.add(_VideoMode.openError);
    final request = loader();
    ThemeAssetFailure? failure;
    try {
      await request.initialize();
    } on ThemeAssetFailure catch (error) {
      failure = error;
    }
    expect(failure, isNotNull);
    expect(failure!.reference, 'runtime/data/user_assets/theme/test.mp4');
    expect(failure.stage, ThemeAssetStage.videoOpen);
    expect(failure.cause, same(platform.openError));
    expect(failure.originalStackTrace.toString(), isEmpty);
    expect(failure.requestStackTrace.toString(), isNotEmpty);
    await request.dispose();
    expect(platform.disposed, [1]);
  });

  test('empty video bytes fail before allocating a native player', () async {
    final request = loader(bytes: Uint8List(0));
    await expectLater(
      request.initialize(),
      throwsA(
        isA<ThemeAssetFailure>()
            .having((failure) => failure.kind, 'kind', ThemeAssetFailureKind.empty)
            .having((failure) => failure.byteLength, 'byteLength', 0),
      ),
    );
    expect(platform.sources, isEmpty);
    await request.dispose();
  });

  test('a video without usable dimensions cannot pass the import probe', () async {
    platform.modes.add(_VideoMode.invalidSize);
    final request = loader();
    await expectLater(request.initialize(), throwsA(isA<ThemeAssetFailure>()));
    await request.dispose();
    expect(platform.disposed, [1]);
  });

  test('cancellation disposes pending initialization without a damage error', () async {
    platform.modes.add(_VideoMode.pending);
    final request = loader();
    final expectation = expectLater(
      request.initialize(),
      throwsA(isA<FvpCreateCancelledException>()),
    );
    await Future<void>.delayed(Duration.zero);
    request.cancel();
    await request.dispose();
    await expectation;
    expect(platform.disposed, [1]);
  });

  test('initialization timeout is classified and observes native cleanup', () async {
    platform.modes.add(_VideoMode.pending);
    final request = loader(timeout: const Duration(milliseconds: 10));
    await expectLater(
      request.initialize(),
      throwsA(
        isA<ThemeAssetFailure>().having(
          (failure) => failure.kind,
          'kind',
          ThemeAssetFailureKind.timeout,
        ),
      ),
    );
    await request.dispose();
    expect(platform.disposed, [1]);
  });

  test('video_player internal unawaited settings failures stay local', () async {
    platform.modes.add(_VideoMode.internalVolumeError);
    final request = loader();
    await expectLater(
      request.initialize(),
      throwsA(
        isA<ThemeAssetFailure>().having(
          (failure) => failure.cause,
          'cause',
          same(platform.volumeError),
        ),
      ),
    );
    await request.dispose();
    expect(platform.disposed, [1]);
  });

  test('a successful probe can require disposal success before activation', () async {
    final request = loader();
    await request.initialize();
    platform.failDisposal = true;
    await expectLater(
      request.dispose(throwOnFailure: true),
      throwsA(
        isA<ThemeAssetFailure>().having(
          (failure) => failure.stage,
          'stage',
          ThemeAssetStage.dispose,
        ),
      ),
    );
    expect(platform.disposed, [1]);
    await request.dispose();
    expect(platform.disposed, [1], reason: 'teardown is attempted only once');
  });

  testWidgets('failed video keeps text, input and the default background', (tester) async {
    platform.modes.add(_VideoMode.openError);
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    final boundary = GlobalKey();
    await tester.pumpWidget(
      RepaintBoundary(key: boundary, child: _scene(resolver, diagnostics, 'bad.mp4')),
    );
    await _pumpRequests(tester);
    expect(find.byType(VideoPlayer), findsNothing);
    expect(find.text('Still usable'), findsOneWidget);
    expect(find.byKey(const ValueKey('draft')), findsOneWidget);
    expect(diagnostics.failures, hasLength(1));
    expect(diagnostics.failures.single.reference, 'bad.mp4');
    expect(platform.disposed, [1]);
    await _expectDefaultBackground(tester, boundary);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('replacement failure retains old video and same-path retry recovers', (tester) async {
    platform.modes.addAll([_VideoMode.good, _VideoMode.openError]);
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'old.mp4'));
    await _pumpRequests(tester);
    final previous = tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller;
    await tester.enterText(find.byKey(const ValueKey('draft')), 'Keep this draft');
    final input = tester.state(find.byType(EditableText));
    await tester.pumpWidget(_scene(resolver, diagnostics, 'new.mp4'));
    await _pumpRequests(tester);
    expect(tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller, same(previous));
    expect(diagnostics.failures.single.retainedPrevious, isTrue);
    expect(platform.disposed, [2]);
    expect(tester.state(find.byType(EditableText)), same(input));
    diagnostics.retry('new.mp4');
    await _pumpRequests(tester);
    expect(platform.sources, hasLength(3));
    expect(resolver.reads, ['old.mp4', 'new.mp4', 'new.mp4']);
    expect(tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller, isNot(same(previous)));
    expect(diagnostics.failures, isEmpty);
    expect(platform.disposed, containsAll([1, 2]));
    expect(tester.state(find.byType(EditableText)), same(input));
    expect(find.text('Keep this draft'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    expect(platform.disposed, containsAll([1, 2, 3]));
    diagnostics.dispose();
  });

  testWidgets('a late native playback error degrades only the video layer', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4'));
    await _pumpRequests(tester);
    expect(find.byType(VideoPlayer), findsOneWidget);
    platform.events[1]!.addError(platform.openError, StackTrace.empty);
    await _pumpRequests(tester);
    expect(find.byType(VideoPlayer), findsNothing);
    expect(find.text('Still usable'), findsOneWidget);
    expect(diagnostics.failures.single.stage, ThemeAssetStage.playback);
    expect(platform.disposed, [1]);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('late creation of an obsolete request cannot replace the new video', (tester) async {
    platform.modes.addAll([_VideoMode.lateCreate, _VideoMode.good]);
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'slow.mp4'));
    await _pumpRequests(tester);
    await tester.pumpWidget(_scene(resolver, diagnostics, 'fast.mp4'));
    await _pumpRequests(tester);
    final current = tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller;
    platform.createGates[1]!.complete();
    await _pumpRequests(tester);
    expect(tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller, same(current));
    expect(diagnostics.failures, isEmpty);
    expect(platform.disposed, [1]);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('moving diagnostics preserves failure and same-path retry eligibility', (tester) async {
    platform.modes.add(_VideoMode.openError);
    final previousDiagnostics = ThemeAssetDiagnostics();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, previousDiagnostics, 'bad.mp4'));
    await _pumpRequests(tester);
    expect(previousDiagnostics.failures, hasLength(1));
    await tester.pumpWidget(_scene(resolver, diagnostics, 'bad.mp4'));
    await _pumpRequests(tester);
    expect(previousDiagnostics.failures, isEmpty);
    expect(diagnostics.failures, hasLength(1));
    expect(platform.sources, hasLength(1), reason: 'moving a notice does not reopen media');
    diagnostics.retry('bad.mp4');
    await _pumpRequests(tester);
    expect(platform.sources, hasLength(2));
    expect(find.byType(VideoPlayer), findsOneWidget);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    previousDiagnostics.dispose();
    diagnostics.dispose();
  });

  testWidgets('a pending pause does not lose resume intent when settings change', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4'));
    await _pumpRequests(tester);
    final gate = Completer<void>();
    platform.pauseGates[1] = gate;
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await _pumpRequests(tester);
    expect(tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller.value.isPlaying, isFalse);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4', muted: false));
    await _pumpRequests(tester);
    gate.complete();
    await _pumpRequests(tester);
    final value = tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller.value;
    expect(value.isPlaying, isTrue);
    expect(value.volume, 1);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('a candidate failing during configuration retains the old video', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'old.mp4'));
    await _pumpRequests(tester);
    final previous = tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller;
    final gate = Completer<void>();
    platform.volumeGates[2] = gate;
    platform.blockVolumeAt[2] = 3;
    await tester.pumpWidget(_scene(resolver, diagnostics, 'candidate.mp4'));
    await _pumpRequests(tester);
    expect(platform.volumes[2], hasLength(3));
    platform.events[2]!.addError(platform.openError, StackTrace.empty);
    await _pumpRequests(tester);
    gate.complete();
    await _pumpRequests(tester);
    expect(tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller, same(previous));
    expect(platform.disposed, [2]);
    expect(diagnostics.failures.single.retainedPrevious, isTrue);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('pending candidates use the latest mute and loop settings', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'old.mp4'));
    await _pumpRequests(tester);
    final gate = Completer<void>();
    platform.volumeGates[2] = gate;
    platform.blockVolumeAt[2] = 3;
    await tester.pumpWidget(_scene(resolver, diagnostics, 'new.mp4', muted: false));
    await _pumpRequests(tester);
    expect(platform.volumes[2], hasLength(3));
    await tester.pumpWidget(_scene(resolver, diagnostics, 'new.mp4', muted: true, loop: false));
    gate.complete();
    await _pumpRequests(tester);
    final current = tester.widget<VideoPlayer>(find.byType(VideoPlayer)).controller;
    expect(current.value.volume, 0);
    expect(current.value.isLooping, isFalse);
    expect(platform.volumes[2]!.last, 0);
    expect(platform.loops[2]!.last, isFalse);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  for (final failOnResume in [false, true]) {
    testWidgets('lifecycle ${failOnResume ? "resume" : "pause"} failures stay local', (tester) async {
      final diagnostics = ThemeAssetDiagnostics();
      final resolver = _VideoAssets();
      await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4'));
      await _pumpRequests(tester);
      if (failOnResume) {
        tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
        await _pumpRequests(tester);
        platform.failPlay = true;
        tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      } else {
        platform.failPause = true;
        tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      }
      await _pumpRequests(tester);
      expect(find.byType(VideoPlayer), findsNothing);
      expect(find.text('Still usable'), findsOneWidget);
      expect(diagnostics.failures.single.stage, ThemeAssetStage.playback);
      expect(platform.disposed, [1]);
      expect(tester.takeException(), isNull);
      await _unmount(tester);
      diagnostics.dispose();
    });
  }

  testWidgets('position timer failures after lifecycle resume remain local', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4'));
    await _pumpRequests(tester);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await _pumpRequests(tester);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await _pumpRequests(tester);
    platform.failPosition = true;
    await tester.pump(const Duration(seconds: 1));
    await _pumpRequests(tester);
    expect(find.byType(VideoPlayer), findsNothing);
    expect(diagnostics.failures.single.stage, ThemeAssetStage.playback);
    expect(diagnostics.failures.single.cause, same(platform.positionError));
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });

  testWidgets('an awaited volume failure cannot become an uncaught zone error', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _VideoAssets();
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4'));
    await _pumpRequests(tester);
    platform.failVolume = true;
    await tester.pumpWidget(_scene(resolver, diagnostics, 'playing.mp4', muted: false));
    await _pumpRequests(tester);
    expect(find.byType(VideoPlayer), findsNothing);
    expect(find.text('Still usable'), findsOneWidget);
    expect(diagnostics.failures.single.stage, ThemeAssetStage.playback);
    expect(platform.disposed, [1]);
    expect(tester.takeException(), isNull);
    await _unmount(tester);
    diagnostics.dispose();
  });
}

Widget _scene(
  ThemeAssetResolver resolver,
  ThemeAssetDiagnostics diagnostics,
  String reference, {
  bool muted = true,
  bool loop = true,
}) => ThemeAssetScope(
  resolver: resolver,
  diagnostics: diagnostics,
  child: MaterialApp(
    home: OperitThemeBackground(
      themePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'useBackgroundImage': true,
        'backgroundImageUri': reference,
        'backgroundMediaType': 'video',
        'videoBackgroundMuted': muted,
        'videoBackgroundLoop': loop,
      }),
      child: const Scaffold(
        backgroundColor: Colors.transparent,
        body: Column(
          children: [Text('Still usable'), TextField(key: ValueKey('draft'))],
        ),
      ),
    ),
  ),
);

Future<void> _pumpRequests(WidgetTester tester) async {
  // Playing media may keep its animation ticker active, so do not settle it.
  for (var frame = 0; frame < 8; frame++) {
    await tester.pump(const Duration(milliseconds: 5));
  }
}

Future<void> _unmount(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox.shrink());
  await _pumpRequests(tester);
}

Future<void> _expectDefaultBackground(
  WidgetTester tester,
  GlobalKey boundary,
) async {
  final surface = Theme.of(
    tester.element(find.text('Still usable')),
  ).colorScheme.surface;
  final rgba = (await tester.runAsync(() async {
    final render = boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await render.toImage(pixelRatio: 1);
    try {
      final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
      final offset = ((image.height - 20) * image.width + image.width ~/ 2) * 4;
      return data.buffer.asUint8List(data.offsetInBytes + offset, 4).toList();
    } finally {
      image.dispose();
    }
  }))!;
  final argb = surface.toARGB32();
  expect(rgba, [(argb >> 16) & 255, (argb >> 8) & 255, argb & 255, 255]);
}

class _VideoAssets implements ThemeAssetResolver {
  final reads = <String>[];

  @override
  Future<Uint8List> readBytes(String reference) async {
    reads.add(reference);
    return Uint8List.fromList([1, 2, 3]);
  }
}

enum _VideoMode { good, openError, invalidSize, pending, lateCreate, internalVolumeError }

class _VideoPlatform extends VideoPlayerPlatform {
  final modes = <_VideoMode>[];
  final sources = <DataSource>[];
  final disposed = <int>[];
  final events = <int, StreamController<VideoEvent>>{};
  final createGates = <int, Completer<void>>{};
  final playerModes = <int, _VideoMode>{};
  final volumeGates = <int, Completer<void>>{};
  final pauseGates = <int, Completer<void>>{};
  final blockVolumeAt = <int, int>{};
  final volumes = <int, List<double>>{};
  final loops = <int, List<bool>>{};
  final openError = PlatformException(
    code: 'media open error',
    message: 'invalid or unsupported media',
  );
  final volumeError = PlatformException(code: 'volume error', message: 'Cannot set volume');
  final positionError = PlatformException(code: 'position error', message: 'Cannot read position');
  bool failDisposal = false;
  bool failVolume = false;
  bool failPause = false;
  bool failPlay = false;
  bool failPosition = false;

  @override
  Future<void> init() async {}

  @override
  Future<int?> createWithOptions(VideoCreationOptions options) async {
    sources.add(options.dataSource);
    final id = sources.length;
    final mode = modes.isEmpty ? _VideoMode.good : modes.removeAt(0);
    playerModes[id] = mode;
    events[id] = StreamController<VideoEvent>();
    if (mode == _VideoMode.lateCreate) {
      final gate = Completer<void>();
      createGates[id] = gate;
      await gate.future;
    }
    return id;
  }

  @override
  Stream<VideoEvent> videoEventsFor(int playerId) {
    final controller = events[playerId]!;
    final mode = playerModes[playerId]!;
    scheduleMicrotask(() {
      if (controller.isClosed) return;
      if (mode == _VideoMode.openError) {
        controller.addError(openError, StackTrace.empty);
      } else if (mode != _VideoMode.pending) {
        controller.add(
          VideoEvent(
            eventType: VideoEventType.initialized,
            duration: const Duration(seconds: 2),
            size: mode == _VideoMode.invalidSize ? Size.zero : const Size(320, 180),
          ),
        );
      }
    });
    return controller.stream;
  }

  @override
  Future<void> dispose(int playerId) async {
    disposed.add(playerId);
    await events[playerId]?.close();
    if (failDisposal) throw PlatformException(code: 'dispose failed', message: 'Native teardown failed');
  }

  @override
  Future<void> setVolume(int playerId, double volume) async {
    final values = volumes.putIfAbsent(playerId, () => []);
    values.add(volume);
    final gate = volumeGates[playerId];
    if (gate != null && values.length >= (blockVolumeAt[playerId] ?? 1)) await gate.future;
    if (failVolume || playerModes[playerId] == _VideoMode.internalVolumeError) {
      Error.throwWithStackTrace(volumeError, StackTrace.empty);
    }
  }

  @override
  Future<void> setLooping(int playerId, bool looping) async {
    loops.putIfAbsent(playerId, () => []).add(looping);
  }
  @override
  Future<void> play(int playerId) async {
    if (failPlay) throw PlatformException(code: 'play error', message: 'Cannot resume');
  }
  @override
  Future<void> pause(int playerId) async {
    final gate = pauseGates[playerId];
    if (gate != null) await gate.future;
    if (failPause) throw PlatformException(code: 'pause error', message: 'Cannot pause');
  }
  @override
  Future<void> seekTo(int playerId, Duration position) async {}
  @override
  Future<void> setPlaybackSpeed(int playerId, double speed) async {}
  @override
  Future<Duration> getPosition(int playerId) async {
    if (failPosition) Error.throwWithStackTrace(positionError, StackTrace.empty);
    return Duration.zero;
  }
  @override
  Widget buildViewWithOptions(VideoViewOptions options) => const ColoredBox(color: Colors.teal);
}
