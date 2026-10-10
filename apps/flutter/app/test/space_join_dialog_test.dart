import 'dart:async';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/AppPeerDialogHost.dart';
import 'package:operit2/ui/common/SpaceJoinWidgets.dart';

Map<String, Object?> joinRequest({
  String status = 'pending',
  bool canApprove = false,
  int version = 1,
}) => {
  'requestId': 'request-1',
  'targetDeviceId': 'ios',
  'applicantDeviceId': 'mac',
  'applicantName': '我的 Mac',
  'spaceName': 'iPhone 的空间',
  'status': status,
  'createdAt': 1,
  'expiresAt': 900001,
  'canApprove': canApprove,
  'reviewerDeviceId': 'ios',
  'reviewerName': '我的 iPhone',
  'reviewerHops': 1,
  'assignmentVersion': version,
  'decisionApprove': null,
};

class JoinBridge extends OperitRuntimeBridge {
  final prompts = StreamController<CoreEvent>.broadcast();
  List<Map<String, Object?>> incoming = [], outgoing = [];
  Map<String, Object?> response = joinRequest();
  final calls = <String>[];
  Map<String, Object?>? decision;
  Completer<Uint8List>? submission, refresh, cancellation;
  Map<String, Object?>? cancellationResponse;
  bool failOutgoing = false;
  bool failSubmit = false, failDecision = false, failCancel = false;
  Object submissionError = StateError('COMMAND_ERROR: not admitted');

  /// Members the local Space projection advertises.
  List<String> members = <String>['ios', 'mac'];
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    switch (request.methodName) {
      case 'requestDeviceSpaceJoin':
        if (failSubmit) throw submissionError;
        if (submission != null) return submission!.future;
        return encodeCoreLink([0, response]);
      case 'refreshDeviceSpaceJoin':
        if (refresh != null) return refresh!.future;
        return encodeCoreLink([0, response]);
      case 'outgoingDeviceSpaceJoins':
        if (failOutgoing) throw StateError('local state unavailable');
        return encodeCoreLink([0, outgoing]);
      case 'incomingDeviceSpaceJoins':
        return encodeCoreLink([0, incoming]);
      case 'decideDeviceSpaceJoin':
        decision = Map<String, Object?>.from(request.args as Map);
        if (failDecision) throw StateError('permission changed');
        incoming = [];
        response = joinRequest(
          status: decision!['approve'] == true ? 'approved' : 'rejected',
        );
        return encodeCoreLink([0, response]);
      case 'cancelDeviceSpaceJoin':
        if (failCancel) throw StateError('local cancellation failed');
        if (cancellation != null) return cancellation!.future;
        response = cancellationResponse ?? joinRequest(status: 'cancelled');
        outgoing = [response];
        return encodeCoreLink([0, response]);
      case 'deviceSpace':
        return encodeCoreLink([
          0,
          {
            'spaceId': 'target-space',
            'spaceName': 'iPhone 的空间',
            'spaceRevision': 3,
            'members': members,
          },
        ]);
      default:
        throw StateError('Unexpected ${request.methodName}');
    }
  }

  void pairing(List<Map<String, Object?>> values) => prompts.add(
    CoreEvent.raw(
      requestId: 'pair-watch',
      target: 'core/server.runtimeRemoteLinkService',
      propertyName: 'pairingPromptsFlow',
      kind: 'Snapshot',
      valueBytes: encodeCoreLink(values),
      decodeValue: decodeCoreLink<Object?>,
    ),
  );
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) => prompts.stream;
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}

void main() {
  Future<void> mount(
    WidgetTester tester,
    JoinBridge bridge, {
    bool host = false,
    bool enabled = true,
  }) async {
    final clients = GeneratedCoreProxyClients(bridge);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Builder(
          builder: (context) {
            final child = Scaffold(
              body: TextButton(
                onPressed: () => showSpaceJoinRequest(
                  context,
                  clients: clients,
                  deviceId: 'ios',
                  deviceName: '我的 iPhone',
                ),
                child: const Text('申请'),
              ),
            );
            return host
                ? AppPeerDialogHost(
                    clients: clients,
                    enabled: enabled,
                    child: child,
                  )
                : child;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> dispose(WidgetTester tester, JoinBridge bridge) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    await bridge.prompts.close();
  }

  testWidgets(
    'applicant opens a real dialog immediately, before network reply',
    (tester) async {
      final bridge = JoinBridge()..submission = Completer<Uint8List>();
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('正在提交申请…'), findsOneWidget);
      bridge.submission!.complete(encodeCoreLink([0, joinRequest()]));
      await tester.pumpAndSettle();
      expect(find.text('审批人：我的 iPhone'), findsOneWidget);
      expect(find.text('等待批准'), findsOneWidget);
      expect(find.textContaining('COMMAND_ERROR'), findsNothing);
      expect(bridge.calls, isNot(contains('joinPairedDeviceSpace')));
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'lost peer reply stops submitting and a late reply cannot overwrite retry state',
    (tester) async {
      final bridge = JoinBridge()..submission = Completer<Uint8List>();
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      expect(find.text('正在提交申请…'), findsOneWidget);
      await tester.pump(const Duration(seconds: 16));
      await tester.pumpAndSettle();
      expect(find.text('正在提交申请…'), findsNothing);
      expect(bridge.calls, contains('outgoingDeviceSpaceJoins'));
      expect(find.byType(FilledButton), findsOneWidget);
      bridge.submission!.complete(encodeCoreLink([0, joinRequest()]));
      await tester.pumpAndSettle();
      expect(find.text('等待批准'), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'submission failure remains in the dialog with retry, not a raw page error',
    (tester) async {
      final bridge = JoinBridge()..failSubmit = true;
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('提交申请'), findsOneWidget);
      expect(find.text('申请暂时未能提交，请检查连接后重试。'), findsOneWidget);
      expect(find.text('后台等待'), findsNothing);
      expect(find.textContaining('关闭窗口后，申请仍会在后台等待'), findsNothing);
      expect(find.textContaining('COMMAND_ERROR'), findsNothing);
      bridge.failSubmit = false;
      await tester.tap(find.text('提交申请'));
      await tester.pumpAndSettle();
      expect(find.text('等待批准'), findsOneWidget);
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'submission errors stay in diagnostics, not the progress status',
    (tester) async {
      final bridge = JoinBridge()
        ..failSubmit = true
        ..submissionError = const CoreLinkError(
          code: 'COMMAND_ERROR',
          message: 'INTERNAL_ERROR: ESP32 NVS capacity exhausted',
        );
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      expect(find.text('申请暂时未能提交，请检查连接后重试。'), findsOneWidget);
      expect(find.textContaining('ESP32 NVS capacity exhausted'), findsNothing);
      expect(find.textContaining('INTERNAL_ERROR'), findsNothing);
      bridge.failSubmit = false;
      await tester.tap(find.text('提交申请'));
      await tester.pumpAndSettle();
      expect(find.text('等待批准'), findsOneWidget);
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'receiver shows approve/reject popup automatically outside settings',
    (tester) async {
      final bridge = JoinBridge()..incoming = [joinRequest(canApprove: true)];
      await mount(tester, bridge, host: true);
      expect(find.text('空间加入申请'), findsOneWidget);
      expect(find.textContaining('我的 Mac 申请加入'), findsOneWidget);
      await tester.tap(find.text('批准'));
      await tester.pumpAndSettle();
      expect(bridge.decision, {
        'requestId': 'request-1',
        'assignmentVersion': 1,
        'approve': true,
      });
      expect(find.byType(AlertDialog), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets('non-assigned/non-authorized member gets no approval popup', (
    tester,
  ) async {
    final bridge = JoinBridge()..incoming = [joinRequest(canApprove: false)];
    await mount(tester, bridge, host: true);
    expect(find.byType(AlertDialog), findsNothing);
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    await dispose(tester, bridge);
  });
  testWidgets(
    'completed pairing code closes automatically and releases approval queue',
    (tester) async {
      final bridge = JoinBridge();
      await mount(tester, bridge, host: true);
      bridge.pairing([
        {
          'pairingId': 'pair-1',
          'peerNodeId': 'mac',
          'displayName': '我的 Mac',
          'confirmationCode': '123456',
        },
      ]);
      await tester.pumpAndSettle();
      expect(find.text('123456'), findsOneWidget);
      bridge.incoming = [joinRequest(canApprove: true)];
      await tester.pump(const Duration(seconds: 3));
      await tester.pumpAndSettle();
      expect(find.text('空间加入申请'), findsNothing); // Still legitimately pairing.
      bridge.pairing([]);
      await tester.pumpAndSettle();
      expect(find.text('123456'), findsNothing);
      expect(find.text('空间加入申请'), findsOneWidget); // No manual OK needed.
      await tester.tap(find.text('拒绝'));
      await tester.pumpAndSettle();
      expect(bridge.decision!['approve'], false);
      await dispose(tester, bridge);
    },
  );
  testWidgets('slow outgoing refresh does not block incoming approval', (
    tester,
  ) async {
    final bridge = JoinBridge()
      ..incoming = [joinRequest(canApprove: true)]
      ..outgoing = [joinRequest()]
      ..refresh = Completer<Uint8List>();
    await mount(tester, bridge, host: true);
    expect(find.text('空间加入申请'), findsOneWidget);
    bridge.refresh!.complete(encodeCoreLink([0, joinRequest()]));
    await tester.pumpAndSettle();
    await dispose(tester, bridge);
  });
  testWidgets(
    'permission/business failure keeps approval dialog open for recovery',
    (tester) async {
      final bridge = JoinBridge()
        ..incoming = [joinRequest(canApprove: true)]
        ..failDecision = true;
      await mount(tester, bridge, host: true);
      await tester.tap(find.text('批准'));
      await tester.pumpAndSettle();
      expect(find.text('空间加入申请'), findsOneWidget);
      expect(find.textContaining('permission changed'), findsOneWidget);
      bridge.failDecision = false;
      await tester.tap(find.text('批准'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets('transferred request dismisses stale reviewer dialog', (
    tester,
  ) async {
    final bridge = JoinBridge()..incoming = [joinRequest(canApprove: true)];
    await mount(tester, bridge, host: true);
    bridge.incoming = [];
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    await dispose(tester, bridge);
  });
  testWidgets('applicant cancellation updates normal status in the dialog', (
    tester,
  ) async {
    final bridge = JoinBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('申请'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消申请'));
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsOneWidget);
    await dispose(tester, bridge);
  });

  /// Verifies user cancellation is dispatched while an older background refresh is pending.
  testWidgets(
    'cancel is enabled during polling and ignores late pending responses',
    (tester) async {
      final bridge = JoinBridge();
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      bridge.refresh = Completer<Uint8List>();
      await tester.pump(const Duration(seconds: 3));
      await tester.pump();
      expect(bridge.calls, contains('refreshDeviceSpaceJoin'));
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(bridge.calls, contains('cancelDeviceSpaceJoin'));
      expect(find.text('已取消'), findsOneWidget);
      bridge.refresh!.complete(encodeCoreLink([0, joinRequest()]));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      expect(find.text('等待批准'), findsNothing);
      await dispose(tester, bridge);
    },
  );

  /// Verifies an unacknowledged local submission is not presented as an offline reviewer.
  testWidgets(
    'failed unassigned submission shows the real failure and remains cancellable',
    (tester) async {
      final pending = joinRequest()
        ..['reviewerDeviceId'] = null
        ..['reviewerName'] = null
        ..['reviewerHops'] = null;
      final bridge = JoinBridge()
        ..failSubmit = true
        ..outgoing = [pending];
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      expect(find.text('等待有审批权限的设备上线'), findsNothing);
      expect(find.text('申请暂时未能提交，请检查连接后重试。'), findsOneWidget);
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      await dispose(tester, bridge);
    },
  );

  testWidgets('cancellation waits for a confirmed Core result', (tester) async {
    final bridge = JoinBridge()..cancellation = Completer<Uint8List>();
    await mount(tester, bridge);
    await tester.tap(find.text('申请'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消申请'));
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsNothing);
    expect(find.text('等待批准'), findsOneWidget);
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, '取消申请'))
          .onPressed,
      isNull,
    );
    bridge.cancellation!.complete(
      encodeCoreLink([0, joinRequest(status: 'cancelled')]),
    );
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsOneWidget);
    final refreshCount = bridge.calls
        .where((method) => method == 'refreshDeviceSpaceJoin')
        .length;
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
    expect(
      bridge.calls.where((method) => method == 'refreshDeviceSpaceJoin').length,
      refreshCount,
    );
    await dispose(tester, bridge);
  });
  testWidgets(
    'failed cancellation preserves pending state, polling and retry',
    (tester) async {
      final bridge = JoinBridge()
        ..failCancel = true
        ..outgoing = [joinRequest()];
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsNothing);
      expect(find.text('等待批准'), findsOneWidget);
      expect(find.text('未能确认申请已取消，请检查连接或稍后重试。'), findsOneWidget);
      expect(bridge.calls, contains('outgoingDeviceSpaceJoins'));
      await tester.pump(const Duration(seconds: 4));
      await tester.pumpAndSettle();
      expect(bridge.calls, contains('refreshDeviceSpaceJoin'));
      bridge.failCancel = false;
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      expect(find.text('未能确认申请已取消，请检查连接或稍后重试。'), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'lost cancellation reply recovers only the same durable request',
    (tester) async {
      final bridge = JoinBridge()
        ..failCancel = true
        ..outgoing = [
          joinRequest(status: 'cancelled'),
          {...joinRequest(), 'requestId': 'another-request'},
        ];
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      expect(find.text('未能确认申请已取消，请检查连接或稍后重试。'), findsNothing);
      await tester.pump(const Duration(seconds: 4));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.where((method) => method == 'refreshDeviceSpaceJoin'),
        isEmpty,
      );
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'failed cancellation and readback never fabricate a terminal state',
    (tester) async {
      final bridge = JoinBridge()
        ..failCancel = true
        ..failOutgoing = true;
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsNothing);
      expect(find.text('等待批准'), findsOneWidget);
      expect(find.text('未能确认申请已取消，请检查连接或稍后重试。'), findsOneWidget);
      expect(
        tester
            .widget<TextButton>(find.widgetWithText(TextButton, '取消申请'))
            .onPressed,
        isNotNull,
      );
      await dispose(tester, bridge);
    },
  );
  testWidgets('timed-out cancellation remains pending until Core confirms it', (
    tester,
  ) async {
    final bridge = JoinBridge()
      ..cancellation = Completer<Uint8List>()
      ..outgoing = [joinRequest()];
    await mount(tester, bridge);
    await tester.tap(find.text('申请'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消申请'));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 16));
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsNothing);
    expect(find.text('等待批准'), findsOneWidget);
    expect(find.text('未能确认申请已取消，请检查连接或稍后重试。'), findsOneWidget);
    bridge.response = joinRequest(status: 'cancelled');
    bridge.outgoing = [bridge.response];
    bridge.cancellation!.complete(encodeCoreLink([0, bridge.response]));
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsNothing);
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.text('已取消'), findsOneWidget);
    await dispose(tester, bridge);
  });
  testWidgets(
    'approval after cancellation timeout is recovered while the request stays pending',
    (tester) async {
      final bridge = JoinBridge()
        ..cancellation = Completer<Uint8List>()
        ..outgoing = [joinRequest()];
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 16));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsNothing);
      expect(find.text('等待批准'), findsOneWidget);
      bridge.response = joinRequest(status: 'joined');
      bridge.outgoing = [bridge.response];
      bridge.cancellation!.complete(
        encodeCoreLink([0, bridge.outgoing.single]),
      );
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      await tester.pump(const Duration(seconds: 3));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(bridge.calls, contains('deviceSpace'));
      expect(bridge.calls, contains('refreshDeviceSpaceJoin'));
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'approval winning cancellation follows the actual joined result',
    (tester) async {
      final bridge = JoinBridge()
        ..cancellationResponse = joinRequest(status: 'joined');
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(bridge.calls, contains('deviceSpace'));
      await dispose(tester, bridge);
    },
  );
  testWidgets('lost cancellation reply can recover a completed approval', (
    tester,
  ) async {
    final bridge = JoinBridge()
      ..failCancel = true
      ..outgoing = [joinRequest(status: 'joined')];
    await mount(tester, bridge);
    await tester.tap(find.text('申请'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消申请'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(bridge.calls, contains('deviceSpace'));
    await dispose(tester, bridge);
  });
  testWidgets('approved applicant refresh closes dialog with adopted space', (
    tester,
  ) async {
    final bridge = JoinBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('申请'));
    await tester.pumpAndSettle();
    bridge.response = joinRequest(status: 'joined');
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(bridge.calls, contains('deviceSpace'));
    await dispose(tester, bridge);
  });
  testWidgets(
    'terminal requests stop querying Core while the dialog stays open',
    (tester) async {
      for (final status in ['cancelled', 'rejected', 'expired']) {
        final bridge = JoinBridge()..response = joinRequest(status: status);
        await mount(tester, bridge);
        await tester.tap(find.text('申请'));
        await tester.pumpAndSettle();
        expect(find.byType(AlertDialog), findsOneWidget);
        final calls = List<String>.of(bridge.calls);
        await tester.pump(const Duration(seconds: 9));
        await tester.pumpAndSettle();
        expect(
          bridge.calls,
          orderedEquals(calls),
          reason: '$status is already confirmed',
        );
        await dispose(tester, bridge);
      }
    },
  );
  testWidgets(
    'confirmed cancellation cannot be resurrected by a later list read',
    (tester) async {
      final bridge = JoinBridge();
      await mount(tester, bridge);
      await tester.tap(find.text('申请'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('取消申请'));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      final calls = List<String>.of(bridge.calls);
      bridge.response = joinRequest(status: 'joined');
      bridge.outgoing = [bridge.response];
      await tester.pump(const Duration(seconds: 9));
      await tester.pumpAndSettle();
      expect(find.text('已取消'), findsOneWidget);
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(bridge.calls, orderedEquals(calls));
      expect(bridge.calls, isNot(contains('deviceSpace')));
      await dispose(tester, bridge);
    },
  );
}
