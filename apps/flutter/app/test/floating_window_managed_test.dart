import 'dart:async';
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/RuntimeFloatingWindowController.dart';

/// Exercises the production owner controller against a controllable native channel.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  final live = <String>{};
  final calls = <MethodCall>[];
  var sequence = 0;
  var granted = true;
  var supported = true;
  var failUpdate = false;
  Completer<void>? createGate;
  Completer<void>? createStarted;

  /// Sends a desired window state through the actual serialized host controller.
  Future<Object?> set(String owner, Map<String, Object?>? content) =>
      RuntimeFloatingWindowController.command({
        'packageName': owner,
        'operation': 'set',
        ...?content,
        'id': 'approval',
        'visible': content != null,
      });

  /// Builds content with the same immutable route and layout used by the approval plugin.
  Map<String, Object?> content(String request) => {
    'route': 'approval-route',
    'fill': true,
    'plugin': <String, Object?>{},
    'state': {'request': request},
  };

  setUp(() {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    live.clear();
    calls.clear();
    sequence = 0;
    granted = true;
    supported = true;
    failUpdate = false;
    createGate = null;
    createStarted = null;
    messenger.setMockMethodCallHandler(FloatingWindows.channel, (call) async {
      calls.add(call);
      switch (call.method) {
        case 'capabilities':
          return {
            'supported': supported,
            'permissionGranted': granted,
            'positioning': true,
            'alwaysOnTop': false,
            'clickThrough': true,
            'focusable': true,
          };
        case 'create':
          createStarted?.complete();
          await createGate?.future;
          final id = 'host-${++sequence}';
          live.add(id);
          return id;
        case 'list':
          return live.toList();
        case 'invoke':
          final args = call.arguments as Map;
          if (args['method'] == 'ready') return true;
          if (args['method'] == 'getBounds') {
            return {'x': 0, 'y': 0, 'width': 390, 'height': 844};
          }
          if (args['method'] == 'update' && failUpdate) {
            throw PlatformException(code: 'update-failed');
          }
          if (args['method'] == 'close') live.remove(args['id']);
          return null;
        default:
          throw StateError('Unexpected native operation: ${call.method}');
      }
    });
  });

  tearDown(() async {
    if (createGate != null && !createGate!.isCompleted) createGate!.complete();
    await set('alpha', null);
    await set('beta', null);
    messenger.setMockMethodCallHandler(FloatingWindows.channel, null);
    debugDefaultTargetPlatformOverride = null;
  });

  test(
    'set creates once, preserves the UI session, restores visibility and closes idempotently',
    () async {
      expect(await set('alpha', content('first')), 'shown');
      expect(await set('alpha', content('second')), 'shown');
      expect(sequence, 1);
      final update = calls
          .where((call) => call.method == 'invoke')
          .map((call) => call.arguments as Map)
          .singleWhere((args) => args['method'] == 'update');
      expect(update['arguments'], {
        'payload': {
          'state': {'request': 'second'},
        },
      });
      await RuntimeFloatingWindowController.command({
        'packageName': 'alpha',
        'operation': 'hide',
        'id': 'approval',
      });
      expect(await set('alpha', content('third')), 'shown');
      expect(
        calls
            .where((call) => call.method == 'invoke')
            .map((call) => (call.arguments as Map)['method'])
            .last,
        'show',
      );
      expect(await set('beta', content('other-owner')), 'shown');
      expect(sequence, 2);
      expect(await set('alpha', null), 'closed');
      expect(live, {'host-2'});
      expect(await set('alpha', null), 'closed');
      expect(live, {'host-2'});
    },
  );

  test(
    'a close arriving during creation waits for that engine and removes it',
    () async {
      createGate = Completer<void>();
      createStarted = Completer<void>();
      final opening = set('alpha', content('first'));
      await createStarted!.future;
      final updating = set('alpha', content('second'));
      final closing = set('alpha', null);
      createGate!.complete();
      expect(await opening, 'shown');
      expect(await updating, 'shown');
      expect(await closing, 'closed');
      expect(sequence, 1);
      expect(live, isEmpty);
    },
  );

  test(
    'availability is explicit, never prompts, and revoked permission removes a live window',
    () async {
      granted = false;
      expect(await set('alpha', content('first')), 'permission_required');
      expect(sequence, 0);
      granted = true;
      expect(await set('alpha', content('first')), 'shown');
      granted = false;
      expect(await set('alpha', content('second')), 'permission_required');
      expect(live, isEmpty);
      supported = false;
      expect(await set('alpha', content('second')), 'unsupported');
      expect(calls.any((call) => call.method == 'requestPermission'), false);
    },
  );

  test(
    'real failures propagate and changing immutable layout requires an explicit close',
    () async {
      await set('alpha', content('first'));
      await expectLater(
        set('alpha', {...content('second'), 'route': 'other-route'}),
        throwsStateError,
      );
      await expectLater(
        set('alpha', {...content('second'), 'fill': false}),
        throwsStateError,
      );
      failUpdate = true;
      await expectLater(
        set('alpha', content('second')),
        throwsA(isA<PlatformException>()),
      );
      failUpdate = false;
      expect(await set('alpha', content('third')), 'shown');
      expect(sequence, 1);
    },
  );
  test(
    'any plugin can control its own media and fullscreen without chat state',
    () async {
      await set('alpha', content('media'));
      Future<Object?> command(
        String owner,
        String operation, [
        Map<String, Object?> extra = const {},
      ]) => RuntimeFloatingWindowController.command({
        'packageName': owner,
        'id': 'approval',
        'operation': operation,
        ...extra,
      });
      await command('alpha', 'setFullscreen', {'fullscreen': true});
      await command('alpha', 'setFullscreen', {'fullscreen': false});
      expect(await command('alpha', 'captureCameraFrame'), isNull);
      final media = calls
          .where((call) => call.method == 'invoke')
          .map((call) => call.arguments as Map)
          .where((args) => args['method'] == 'contentCommand')
          .map((args) => args['arguments'])
          .toList();
      expect(media, [
        'window.fullscreen.enter',
        'window.fullscreen.exit',
        'window.camera.capture',
      ]);
      await expectLater(
        command('beta', 'captureCameraFrame'),
        throwsStateError,
      );
      await expectLater(
        command('beta', 'setFullscreen', {'fullscreen': true}),
        throwsStateError,
      );
      await expectLater(
        command('alpha', 'setFullscreen', {'fullscreen': 'true'}),
        throwsArgumentError,
      );
      await command('alpha', 'hide');
      expect(await command('alpha', 'captureCameraFrame'), isNull);
      final commands = calls
          .where((call) => call.method == 'invoke')
          .map((call) => call.arguments as Map)
          .where((args) => args['method'] == 'contentCommand')
          .map((args) => args['arguments'])
          .toList();
      expect(commands.last, 'window.camera.stop');
      expect(
        commands.where((name) => name == 'window.camera.capture'),
        hasLength(1),
      );
    },
  );

  test(
    'window scope is explicit and cannot change on an existing window',
    () async {
      await set('alpha', content('first'));
      await expectLater(
        set('alpha', {...content('changed'), 'applicationOnly': true}),
        throwsStateError,
      );
      await expectLater(
        set('beta', {...content('invalid'), 'applicationOnly': 'true'}),
        throwsArgumentError,
      );
    },
  );
}
