import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_keep_alive/operit_keep_alive.dart';

const options = KeepAliveOptions(
  applicationName: 'Test',
  androidServiceClassName: 'test.CoreService',
  openLabel: 'Open',
  exitLabel: 'Quit',
  quitMessage: 'Stop active work?',
);

/// Builds an explicit transport response with independently selectable capabilities.
Map<String, Object> report({
  int leases = 0,
  String protection = 'continuous',
}) => {
  'protection': protection,
  'closeToTray': protection == 'continuous',
  'activeLeases': leases,
  'detail': 'Host execution state',
};

/// Verifies protocol ordering, failure visibility and lease lifetimes through the real channel codec.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('test/operit_keep_alive');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  late OperitKeepAlive keepAlive;
  late List<MethodCall> calls;
  late Set<String> leases;

  setUp(() {
    calls = [];
    leases = {};
    keepAlive = OperitKeepAlive(channel: channel);
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      final args = call.arguments as Map<Object?, Object?>;
      if (call.method == 'acquire') leases.add(args['id'] as String);
      if (call.method == 'release') leases.remove(args['id'] as String);
      return report(leases: leases.length);
    });
  });

  tearDown(() {
    messenger.setMockMethodCallHandler(channel, null);
    channel.setMethodCallHandler(null);
  });

  test('concurrent initialization sends exactly one native request', () async {
    final first = keepAlive.initialize(options);
    final second = keepAlive.initialize(options);
    expect(identical(first, second), isTrue);
    await Future.wait([first, second]);
    expect(calls.map((call) => call.method), ['initialize']);
  });

  test('work cannot acquire a lease before initialization', () async {
    await expectLater(keepAlive.acquire(reason: 'work'), throwsStateError);
    expect(calls, isEmpty);
  });

  test('acquisition waits for host initialization acknowledgement', () async {
    final ready = Completer<Map<String, Object>>();
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (call.method == 'initialize') return ready.future;
      return report(leases: 1);
    });
    final initialization = keepAlive.initialize(options);
    final acquiring = keepAlive.acquire(reason: 'workflow');
    await Future<void>.delayed(Duration.zero);
    expect(calls.map((call) => call.method), ['initialize']);
    ready.complete(report());
    await initialization;
    final lease = await acquiring;
    expect(lease.report.activeLeases, 1);
    expect(calls.map((call) => call.method), ['initialize', 'acquire']);
  });

  test(
    'native service errors reach the caller without creating a lease',
    () async {
      await keepAlive.initialize(options);
      messenger.setMockMethodCallHandler(channel, (call) async {
        throw PlatformException(
          code: 'SERVICE_START_DENIED',
          message: 'Not foreground',
        );
      });
      await expectLater(
        keepAlive.acquire(reason: 'workflow'),
        throwsA(
          isA<PlatformException>().having(
            (e) => e.code,
            'code',
            'SERVICE_START_DENIED',
          ),
        ),
      );
      expect(keepAlive.report.value!.activeLeases, 0);
    },
  );

  test('finite work releases its lease even when work throws', () async {
    await keepAlive.initialize(options);
    final error = StateError('workflow failed');
    await expectLater(
      keepAlive.run<void>(reason: 'workflow', work: () async => throw error),
      throwsA(same(error)),
    );
    expect(calls.map((call) => call.method), [
      'initialize',
      'acquire',
      'release',
    ]);
    expect(leases, isEmpty);
    expect(keepAlive.report.value!.activeLeases, 0);
  });

  test('concurrent release is acknowledged by one native operation', () async {
    await keepAlive.initialize(options);
    final lease = await keepAlive.acquire(reason: 'workflow');
    final first = lease.release();
    final second = lease.release();
    expect(identical(first, second), isTrue);
    await Future.wait([first, second]);
    await lease.release();
    expect(calls.where((call) => call.method == 'release').length, 1);
  });

  test('different controllers generate distinct lease identifiers', () async {
    final other = OperitKeepAlive(channel: channel);
    await keepAlive.initialize(options);
    await other.initialize(options);
    final first = await keepAlive.acquire(reason: 'first engine');
    final second = await other.acquire(reason: 'second engine');
    expect(first.id, isNot(second.id));
    await first.release();
    await second.release();
  });

  test('blank work descriptions never reach the native host', () async {
    await keepAlive.initialize(options);
    await expectLater(keepAlive.acquire(reason: '  '), throwsArgumentError);
    expect(calls.length, 1);
  });

  test('malformed and missing host reports remain explicit errors', () async {
    for (final response in [
      null,
      {...report(), 'activeLeases': -1},
      {...report(), 'protection': 'unknown'},
    ]) {
      final controller = OperitKeepAlive(channel: channel);
      messenger.setMockMethodCallHandler(channel, (call) async => response);
      await expectLater(
        controller.initialize(options),
        throwsA(anyOf(isA<StateError>(), isA<FormatException>())),
      );
      expect(controller.report.value, isNull);
    }
  });

  test(
    'unavailable browser protection is preserved with its actual lease count',
    () async {
      messenger.setMockMethodCallHandler(
        channel,
        (call) async => report(leases: 1, protection: 'unavailable'),
      );
      await keepAlive.initialize(options);
      final lease = await keepAlive.acquire(reason: 'page runtime');
      expect(lease.report.protection, KeepAliveProtection.unavailable);
      expect(lease.report.closeToTray, isFalse);
    },
  );

  test(
    'native expiration updates observed protection and keeps active work count',
    () async {
      await keepAlive.initialize(options);
      await keepAlive.acquire(reason: 'finite mobile task');
      final response = Completer<void>();
      await messenger.handlePlatformMessage(
        channel.name,
        const StandardMethodCodec().encodeMethodCall(
          MethodCall(
            'protectionChanged',
            report(leases: 1, protection: 'unavailable'),
          ),
        ),
        (_) => response.complete(),
      );
      await response.future;
      expect(
        keepAlive.report.value!.protection,
        KeepAliveProtection.unavailable,
      );
      expect(keepAlive.report.value!.activeLeases, 1);
    },
  );
}
