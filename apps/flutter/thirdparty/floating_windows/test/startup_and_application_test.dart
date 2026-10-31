import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:floating_windows/floating_windows.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  const desktop = MethodChannel('mixin.one/desktop_multi_window');
  const wire = MethodChannel('mixin.one/desktop_multi_window/channels');
  var probes = 0;
  var destroyed = false;
  setUp(() {
    probes = 0;
    destroyed = false;
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    messenger.setMockMethodCallHandler(
      FloatingWindows.channel,
      (call) async => {
        'supported': true,
        'permissionGranted': true,
        'alwaysOnTop': true,
      },
    );
    messenger.setMockMethodCallHandler(desktop, (call) async {
      if (call.method == 'createWindow') return 'child';
      if (call.method == 'window_close') {
        destroyed = true;
        return null;
      }
      throw StateError('Unexpected desktop operation ${call.method}');
    });
  });
  tearDown(() {
    for (final channel in [FloatingWindows.channel, desktop, wire]) {
      messenger.setMockMethodCallHandler(channel, null);
    }
    debugDefaultTargetPlatformOverride = null;
  });
  test(
    'desktop startup retries the transport exception while the child installs its handler',
    () async {
      messenger.setMockMethodCallHandler(wire, (call) async {
        expect(call.method, 'invokeMethod');
        final args = call.arguments as Map;
        expect(args['method'], 'ready');
        if (++probes == 1)
          throw PlatformException(code: 'CHANNEL_UNREGISTERED');
        if (probes == 2) throw PlatformException(code: 'CHANNEL_NOT_FOUND');
        return true;
      });
      final window = await FloatingWindows.open(payload: {});
      expect(window.id, 'child');
      expect(probes, 3);
      expect(destroyed, false);
    },
  );
  test(
    'a ready call that never returns times out and destroys the child through its owner',
    () async {
      messenger.setMockMethodCallHandler(
        wire,
        (_) => Completer<Object?>().future,
      );
      await expectLater(
        FloatingWindows.open(
          payload: {},
          startupTimeout: const Duration(milliseconds: 20),
        ),
        throwsA(isA<TimeoutException>()),
      );
      expect(destroyed, true);
    },
  );
  test(
    'existing native engines remain tracked after presentation support is lost',
    () async {
      messenger.setMockMethodCallHandler(
        FloatingWindows.channel,
        (_) async => {'supported': false},
      );
      messenger.setMockMethodCallHandler(desktop, (call) async {
        expect(call.method, 'getAllWindows');
        return [
          {'windowId': 'existing-child', 'windowArgument': ''},
        ];
      });
      expect((await FloatingWindows.capabilities())['supported'], false);
      expect(await FloatingWindows.liveIds(), {'existing-child'});
    },
  );
  for (final unresponsive in [false, true]) {
    test('close reclaims a child through its native owner when its channel '
        '${unresponsive ? "does not reply" : "was unregistered"}', () async {
      messenger.setMockMethodCallHandler(wire, (call) async {
        final method = (call.arguments as Map)['method'];
        if (method == 'ready') return true;
        expect(method, 'close');
        if (unresponsive) return Completer<Object?>().future;
        throw PlatformException(code: 'CHANNEL_UNREGISTERED');
      });
      final window = await FloatingWindows.open(payload: {});
      await window.close(timeout: const Duration(milliseconds: 20));
      expect(destroyed, true);
    });
  }
  testWidgets(
    'application scope keeps hidden state and recovers from a failed per-window close hook',
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      final hooks = <String>[];
      await tester.pumpWidget(
        FloatingWindowsHost(
          windowBuilder: (launch) {
            FloatingWindows.setBeforeClose(launch, () async {
              hooks.add(launch.payload['name'] as String);
              if (launch.payload['name'] == 'alpha')
                throw StateError('drain failed');
            });
            return Text(launch.payload['name'] as String);
          },
          child: const Text('application'),
        ),
      );
      final caps = await FloatingWindows.capabilities();
      expect(caps['scope'], 'application');
      expect(caps['systemOverlay'], false);
      expect(await FloatingWindows.requestPermission(), true);
      final alpha = await FloatingWindows.open(payload: {'name': 'alpha'});
      final beta = await FloatingWindows.open(payload: {'name': 'beta'});
      await tester.pump();
      await alpha.hide();
      await tester.pump();
      expect(find.text('alpha'), findsNothing);
      expect(find.text('beta'), findsOneWidget);
      await alpha.show();
      await alpha.update(
        const FloatingWindowOptions(x: 120, clickThrough: true),
      );
      await tester.pump();
      expect(find.text('alpha'), findsOneWidget);
      await alpha.close();
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      expect(tester.takeException(), isA<StateError>());
      expect(hooks, ['alpha']);
      expect(await FloatingWindows.liveIds(), {beta.id});
      await beta.close();
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      expect(hooks, ['alpha', 'beta']);
      expect(await FloatingWindows.liveIds(), isEmpty);
      await tester.pumpWidget(const SizedBox());
      expect((await FloatingWindows.capabilities())['supported'], false);
      debugDefaultTargetPlatformOverride = null;
    },
  );
}
