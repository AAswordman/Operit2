import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/application/AppVisibilityService.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  const channel = AppVisibilityService.channel;

  Future<void> nativeVisibility(bool visible) async {
    final completed = Completer<void>();
    messenger.handlePlatformMessage(
      channel.name,
      const StandardMethodCodec().encodeMethodCall(
        MethodCall('visibilityChanged', visible),
      ),
      (_) => completed.complete(),
    );
    await completed.future;
  }

  tearDown(() {
    messenger.setMockMethodCallHandler(channel, null);
    debugDefaultTargetPlatformOverride = null;
  });

  for (final platform in [
    TargetPlatform.macOS,
    TargetPlatform.windows,
    TargetPlatform.linux,
  ]) {
    test(
      '$platform uses main window state and ignores Flutter focus/occlusion',
      () async {
        debugDefaultTargetPlatformOverride = platform;
        messenger.setMockMethodCallHandler(channel, (_) async => true);
        final service = AppVisibilityService();
        addTearDown(service.dispose);
        final changes = <AppVisibility>[];
        service.addListener(() => changes.add(service.value));
        await service.initialize();
        for (final state in AppLifecycleState.values) {
          service.didChangeAppLifecycleState(state);
        }
        expect(changes, [AppVisibility.visible]);
        await nativeVisibility(false);
        await nativeVisibility(false);
        expect(changes, [AppVisibility.visible, AppVisibility.hidden]);
        // Auxiliary windows can repeatedly resume/hide the Flutter application
        // while the real main window remains minimized.
        for (var cycle = 0; cycle < 5; cycle++) {
          service.didChangeAppLifecycleState(AppLifecycleState.inactive);
          service.didChangeAppLifecycleState(AppLifecycleState.resumed);
          service.didChangeAppLifecycleState(AppLifecycleState.inactive);
          service.didChangeAppLifecycleState(AppLifecycleState.hidden);
        }
        expect(service.value, AppVisibility.hidden);
        expect(changes, [AppVisibility.visible, AppVisibility.hidden]);
        await nativeVisibility(true);
        expect(changes, [
          AppVisibility.visible,
          AppVisibility.hidden,
          AppVisibility.visible,
        ]);
      },
    );
  }

  test(
    'a delayed native snapshot cannot overwrite a newer window event',
    () async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      final snapshot = Completer<bool>();
      messenger.setMockMethodCallHandler(channel, (_) => snapshot.future);
      final service = AppVisibilityService();
      addTearDown(service.dispose);
      final initialized = service.initialize();
      await nativeVisibility(false);
      snapshot.complete(true);
      await initialized;
      expect(service.value, AppVisibility.hidden);
    },
  );

  test(
    'missing desktop adapters retain unknown instead of guessing from focus',
    () async {
      debugDefaultTargetPlatformOverride = TargetPlatform.linux;
      final service = AppVisibilityService();
      addTearDown(service.dispose);
      await service.initialize();
      service.didChangeAppLifecycleState(AppLifecycleState.hidden);
      expect(service.value, AppVisibility.unknown);
    },
  );

  for (final platform in [TargetPlatform.android, TargetPlatform.iOS]) {
    test(
      '$platform maps scene visibility without treating inactive as background',
      () async {
        debugDefaultTargetPlatformOverride = platform;
        final service = AppVisibilityService();
        addTearDown(service.dispose);
        await service.initialize();
        final changes = <AppVisibility>[];
        service.addListener(() => changes.add(service.value));
        service.didChangeAppLifecycleState(AppLifecycleState.resumed);
        service.didChangeAppLifecycleState(AppLifecycleState.inactive);
        expect(service.value, AppVisibility.visible);
        service.didChangeAppLifecycleState(AppLifecycleState.hidden);
        service.didChangeAppLifecycleState(AppLifecycleState.paused);
        expect(service.value, AppVisibility.hidden);
        expect(
          changes.where((state) => state == AppVisibility.hidden).length,
          1,
        );
        service.didChangeAppLifecycleState(AppLifecycleState.inactive);
        expect(service.value, AppVisibility.visible);
      },
    );
  }

  test('disposing during initial query prevents late notifications', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    final snapshot = Completer<bool>();
    messenger.setMockMethodCallHandler(channel, (_) => snapshot.future);
    final service = AppVisibilityService();
    final initialized = service.initialize();
    service.dispose();
    snapshot.complete(true);
    await initialized;
    expect(service.value, AppVisibility.unknown);
  });
}
