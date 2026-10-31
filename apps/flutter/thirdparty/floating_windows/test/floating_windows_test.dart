import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:floating_windows/floating_windows.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  final calls = <MethodCall>[];
  var granted = true;
  setUp(() {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    calls.clear();
    granted = true;
    messenger.setMockMethodCallHandler(FloatingWindows.channel, (call) async {
      calls.add(call);
      switch (call.method) {
        case 'capabilities':
          return {
            'supported': true,
            'permissionGranted': granted,
            'positioning': true,
            'alwaysOnTop': false,
            'clickThrough': true,
            'focusable': true,
          };
        case 'create':
          return 'surface-1';
        case 'invoke':
          return (call.arguments as Map)['method'] == 'ready' ? true : null;
        default:
          return null;
      }
    });
  });
  tearDown(() {
    messenger.setMockMethodCallHandler(FloatingWindows.channel, null);
    debugDefaultTargetPlatformOverride = null;
  });

  test(
    'partial geometry preserves input flags and rejects non-finite or invalid sizes',
    () {
      final merged = FloatingWindowOptions.defaults.merge(
        const FloatingWindowOptions(x: -80, width: 64),
      );
      expect(merged.toMap(), {
        'x': -80.0,
        'width': 64.0,
        'height': 240.0,
        'alwaysOnTop': true,
        'clickThrough': false,
        'focusable': false,
        'draggable': false,
      });
      expect(
        () => const FloatingWindowOptions(width: 15).toMap(),
        throwsArgumentError,
      );
      expect(
        () => FloatingWindowOptions(x: double.nan).toMap(),
        throwsArgumentError,
      );
      expect(
        () => FloatingWindowOptions.fromMap({'focusable': 'false'}),
        throwsArgumentError,
      );
      expect(
        () => FloatingWindowOptions.fromMap({'mystery': 1}),
        throwsArgumentError,
      );
    },
  );

  test(
    'permission failure never creates an engine or prompts implicitly',
    () async {
      granted = false;
      await expectLater(FloatingWindows.open(payload: {}), throwsStateError);
      expect(calls.map((call) => call.method), ['capabilities']);
    },
  );

  test(
    'open passes app data and complete defaults; handle controls only its own host identity',
    () async {
      final window = await FloatingWindows.open(
        payload: {'route': 'pet'},
        options: const FloatingWindowOptions(width: 64, height: 64),
      );
      expect(window.id, 'surface-1');
      final arguments =
          calls.singleWhere((call) => call.method == 'create').arguments as Map;
      final launch = FloatingWindows.parseLaunch([
        'floating_windows.window',
        arguments['arguments'] as String,
      ])!;
      expect(launch.payload, {'route': 'pet'});
      expect(launch.options.width, 64);
      await window.update(const FloatingWindowOptions(x: 50));
      await window.hide();
      await window.show();
      await window.close();
      final commands = calls
          .where((call) => call.method == 'invoke')
          .map((call) => call.arguments as Map)
          .toList();
      expect(commands.map((call) => call['method']), [
        'ready',
        'update',
        'hide',
        'show',
        'close',
      ]);
      expect(commands.every((call) => call['id'] == 'surface-1'), isTrue);
      expect(commands[1]['arguments'], {'x': 50.0});
    },
  );

  test('unrelated window envelopes do not select floating startup', () {
    expect(FloatingWindows.parseLaunch([]), isNull);
    expect(
      FloatingWindows.parseLaunch(['multi_window', 'chat', '{"kind":"chat"}']),
      isNull,
    );
    expect(
      FloatingWindows.parseLaunch(['multi_window', 'chat', 'invalid-json']),
      isNull,
    );
  });

  test(
    'unsupported hosts report capabilities without invoking native methods',
    () async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      expect((await FloatingWindows.capabilities())['supported'], false);
      expect(await FloatingWindows.requestPermission(), false);
      expect(calls, isEmpty);
    },
  );
}
