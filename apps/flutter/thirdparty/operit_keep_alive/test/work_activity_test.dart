import 'dart:convert';
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
const labels = WorkPresentationLabels(
  open: 'Open',
  review: 'Review',
  stop: 'Stop',
  idle: 'Idle',
  running: 'running',
  waiting: 'Needs review',
  waitingCount: 'awaiting confirmation',
  completed: 'Completed',
  failed: 'Failed',
  cancelled: 'Stopped',
  phases: {
    'processing': 'Preparing',
    'connecting': 'Connecting',
    'receiving': 'Writing',
    'executingTool': 'Running tool',
    'processingToolResult': 'Processing results',
    'summarizing': 'Organizing context',
    'executingPlan': 'Executing plan',
  },
);

/// Builds a versioned record using the production Core wire contract.
Map<String, Object?> work({
  String id = 'work-1',
  String status = 'running',
  int? ended,
  bool canStop = true,
}) => {
  'workId': id,
  'activation': {'type': 'open_chat', 'chatId': 'chat-1'},
  'runId': 4,
  'title': 'Organize documents',
  'status': status,
  'phase': ended == null ? 'executingTool' : 'ended',
  'detail': 'Searching official sources',
  'startedAtMillis': 1000,
  'endedAtMillis': ended,
  'progress': {'value': 60.0, 'scope': 'download'},
  'permissionRequestIds': <String>[],
  'canStop': canStop,
};

/// Builds complete revisions rather than reconstructing state from partial messages.
String snapshot(List<Map<String, Object?>> works, {int revision = 8}) =>
    jsonEncode({'schemaVersion': 1, 'revision': revision, 'works': works});

/// Exercises lifecycle, scoped progress, protocol errors, and action delivery.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('test/work_presentation');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  test(
    'step progress retains its scope and never becomes overall completion',
    () {
      final value = WorkActivitySnapshot.fromJson(snapshot([work()]));
      expect(value.works.single.progress!.scope, 'download');
      expect(value.works.single.progress!.value, 60);
      expect(value.works.single.status, WorkStatus.running);
      expect(labels.describe(value.works.single), contains('download 60%'));
    },
  );

  test('waiting work remains active and exposes review action and counts', () {
    final value = WorkActivitySnapshot.fromJson(
      snapshot([work(status: 'waiting')]),
    );
    final display = value.presentation(labels);
    expect(display['activeCount'], 1);
    expect(display['waitingCount'], 1);
    final item = (display['items']! as List).single as Map;
    expect(item['openLabel'], 'Review');
    expect(item['canStop'], isTrue);
  });

  test('completed work freezes its end and removes the stop action', () {
    final value = WorkActivitySnapshot.fromJson(
      snapshot([work(status: 'completed', ended: 9000, canStop: false)]),
    );
    expect(value.active, isEmpty);
    final item = (value.presentation(labels)['items']! as List).single as Map;
    expect(item['endedAtMillis'], 9000);
    expect(item['canStop'], isFalse);
    expect(item['subtitle'], startsWith('Completed'));
  });

  test('duplicate work identities are rejected', () {
    expect(
      () => WorkActivitySnapshot.fromJson(snapshot([work(), work()])),
      throwsFormatException,
    );
  });

  test('running and terminal lifecycle contradictions are rejected', () {
    expect(
      () =>
          WorkActivitySnapshot.fromJson(snapshot([work(status: 'completed')])),
      throwsFormatException,
    );
    expect(
      () => WorkActivitySnapshot.fromJson(snapshot([work(ended: 8000)])),
      throwsFormatException,
    );
    expect(
      () => WorkActivitySnapshot.fromJson(
        snapshot([work(status: 'completed', ended: 8000)]),
      ),
      throwsFormatException,
    );
  });

  test(
    'out of range step percentages are reported as invalid producer data',
    () {
      final malformed = work();
      malformed['progress'] = {'value': 101, 'scope': 'download'};
      expect(
        () => WorkActivitySnapshot.fromJson(snapshot([malformed])),
        throwsFormatException,
      );
    },
  );

  test('unknown schema and execution phases are rejected', () {
    expect(
      () => WorkActivitySnapshot.fromJson(
        '{"schemaVersion":2,"revision":0,"works":[]}',
      ),
      throwsFormatException,
    );
    final malformed = work();
    malformed['phase'] = 'invented-stage';
    expect(
      () => WorkActivitySnapshot.fromJson(snapshot([malformed])),
      throwsFormatException,
    );
  });

  test('unknown native presentation capability is an explicit error', () {
    expect(
      () => WorkPresentationCapabilities.fromMap({
        'updatesOwner': 'unknown',
        'surface': 'tray',
        'liveUpdates': false,
      }),
      throwsFormatException,
    );
  });

  test('unsupported action names and missing destinations are rejected', () {
    expect(
      () => WorkAction.fromMap({
        'workId': 'w',
        'activation': {'destination': 'c'},
        'action': 'pause',
      }),
      throwsFormatException,
    );
    expect(
      () => WorkAction.fromMap({'workId': 'w', 'action': 'open'}),
      throwsFormatException,
    );
  });

  test(
    'display localization can refresh the same Core snapshot with a new publication revision',
    () async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call);
        if (call.method == 'initialize')
          return {
            'protection': 'continuous',
            'closeToTray': true,
            'activeLeases': 0,
            'detail': 'Test',
          };
        return null;
      });
      final controller = OperitKeepAlive(channel: channel);
      await controller.initialize(options);
      final value = WorkActivitySnapshot.fromJson(snapshot([work()]));
      await controller.publishWork(value, labels);
      await controller.publishWork(value, labels);
      final payloads = calls
          .where((call) => call.method == 'publishWork')
          .map((call) => call.arguments as Map)
          .toList();
      expect(
        payloads[1]['revision'],
        greaterThan(payloads[0]['revision'] as int),
      );
      expect(
        jsonDecode(payloads[1]['json'] as String)['revision'],
        payloads[1]['revision'],
      );
      messenger.setMockMethodCallHandler(channel, null);
      channel.setMethodCallHandler(null);
    },
  );

  test('native actions and errors reach their application receivers', () async {
    messenger.setMockMethodCallHandler(
      channel,
      (call) async => {
        'protection': 'continuous',
        'closeToTray': true,
        'activeLeases': 0,
        'detail': 'Test',
      },
    );
    final controller = OperitKeepAlive(channel: channel);
    await controller.initialize(options);
    final action = controller.workActions.first;
    final error = controller.workErrors.first;
    final codec = const StandardMethodCodec();
    messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('workAction', {
          'workId': 'work-1',
          'activation': {'type': 'open_chat', 'chatId': 'chat-1'},
          'action': 'open',
        }),
      ),
      (_) {},
    );
    messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('workPresentationError', {
          'message': 'Notification rejected',
        }),
      ),
      (_) {},
    );
    expect((await action).activation['chatId'], 'chat-1');
    expect(await error, 'Notification rejected');
    messenger.setMockMethodCallHandler(channel, null);
    channel.setMethodCallHandler(null);
  });
}
