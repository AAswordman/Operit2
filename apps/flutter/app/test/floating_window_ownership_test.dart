import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/RuntimeFloatingWindowController.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('plugin-local IDs isolate handles; duplicate opens fail and closing releases registry ownership', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    final messenger = TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    final live = <String>{};
    final invoked = <Map>[];
    var sequence = 0;
    messenger.setMockMethodCallHandler(FloatingWindows.channel, (call) async {
      switch (call.method) {
        case 'capabilities': return {'supported': true, 'permissionGranted': true,
          'positioning': true, 'alwaysOnTop': false, 'clickThrough': true, 'focusable': true};
        case 'create': final id = 'host-${++sequence}'; live.add(id); return id;
        case 'list': return live.toList();
        case 'invoke':
          final args = call.arguments as Map;
          invoked.add(args);
          if (args['method'] == 'ready') return true;
          if (args['method'] == 'getBounds') return {'x': 100, 'y': 120, 'width': 240, 'height': 240};
          if (args['method'] == 'close') live.remove(args['id']);
          return null;
        default: throw StateError('Unexpected host operation: ${call.method}');
      }
    });
    Future<Object?> command(String owner, String operation, [Map<String, Object?> extra = const {}]) =>
      RuntimeFloatingWindowController.command({'packageName': owner, 'operation': operation, ...extra});
    try {
      await command('alpha', 'open', {'id': 'pet', 'route': 'route-alpha', 'plugin': {}});
      await command('beta', 'open', {'id': 'pet', 'route': 'route-beta', 'plugin': {}});
      await expectLater(command('alpha', 'open', {'id': 'pet', 'route': 'route-alpha', 'plugin': {}}), throwsStateError);
      expect(sequence, 2);
      final alpha = await command('alpha', 'list') as List;
      expect(alpha.single['route'], 'route-alpha');
      await command('beta', 'hide', {'id': 'pet'});
      expect(invoked.where((call) => call['method'] == 'hide').last, {'id': 'host-2', 'method': 'hide', 'arguments': null});
      await expectLater(command('gamma', 'close', {'id': 'pet'}), throwsStateError);
      await command('alpha', 'close', {'id': 'pet'});
      expect(await command('alpha', 'list'), isEmpty);
      expect((await command('beta', 'list') as List).single['visible'], false);
      await command('beta', 'close', {'id': 'pet'});
      expect(live, isEmpty);
    } finally {
      // Drain any windows if an assertion fails so the audit timer cannot leak.
      for (final owner in ['alpha', 'beta']) {
        try { await command(owner, 'close', {'id': 'pet'}); } catch (_) {}
      }
      messenger.setMockMethodCallHandler(FloatingWindows.channel, null);
      debugDefaultTargetPlatformOverride = null;
    }
  });
}
