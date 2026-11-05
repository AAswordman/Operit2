import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/screens/MarketCreatorScreen.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies the creator center: the GitHub account card, the signed-in
/// review progress, in-place withdrawal and the signed-out invitation.
void main() {
  setUpAll(ClientLogger.initialize);

  testWidgets('signed out, the account card invites a login and loads nothing', (
    tester,
  ) async {
    final bridge = _CreatorBridge(loggedIn: false);
    await _show(tester, bridge);

    expect(find.text('登录 GitHub，开始创作'), findsOneWidget);
    expect(find.text('登录 GitHub'), findsOneWidget);
    expect(find.text('我发布的'), findsNothing);
    expect(bridge.calls, contains('isLoggedIn'));
    expect(bridge.calls, isNot(contains('get_my_entries')));
    expect(bridge.calls, isNot(contains('get_notifications')));
  });

  testWidgets('signed in, the card shows the account and review progress', (
    tester,
  ) async {
    final bridge = _CreatorBridge(
      loggedIn: true,
      entries: <Map<String, Object?>>[
        _entry(id: 'live', title: 'Live Plugin'),
        _entry(
          id: 'fix',
          title: 'Needs Fix',
          stateCode: 'changes_requested',
          reasonCodes: <String>['metadata-incomplete'],
          reviewDetail: 'Please add a description.',
        ),
      ],
    );
    await _show(tester, bridge);

    expect(find.text('Test User'), findsOneWidget);
    expect(find.text('@tester'), findsOneWidget);
    expect(find.text('Live Plugin'), findsOneWidget);
    expect(find.text('Needs Fix'), findsOneWidget);
    expect(find.text('元数据不完整'), findsOneWidget);
    expect(find.text('Please add a description.'), findsOneWidget);
    expect(find.text('修改后提交新版本'), findsOneWidget);
    expect(find.text('Nice plugin'), findsOneWidget);
    expect(
      bridge.calls.where((call) => call == 'get_my_entries'),
      hasLength(1),
    );
    expect(
      bridge.calls.where((call) => call == 'get_notifications'),
      hasLength(1),
    );
  });

  testWidgets('withdrawing explains the consequence before asking the market', (
    tester,
  ) async {
    final bridge = _CreatorBridge(
      loggedIn: true,
      entries: <Map<String, Object?>>[_entry(id: 'live', title: 'Live Plugin')],
    );
    await _show(tester, bridge);

    await tester.tap(find.text('移除'));
    await tester.pumpAndSettle();
    expect(find.textContaining('不会被删除'), findsOneWidget);
    expect(bridge.withdrawn, isEmpty);

    await tester.tap(find.text('确认移除'));
    await tester.pumpAndSettle();
    expect(bridge.withdrawn, <String>['live']);
    expect(find.text('已移除'), findsWidgets);
    expect(find.text('移除'), findsNothing);
  });

  testWidgets('a token the market rejects asks for a new login', (tester) async {
    final bridge = _CreatorBridge(
      loggedIn: true,
      entriesError: 'HTTP 401: {"ok":false}',
    );
    await _show(tester, bridge);

    expect(find.text('GitHub 登录已失效'), findsOneWidget);
    expect(find.text('重新登录'), findsOneWidget);
    expect(find.text('发布记录加载失败'), findsNothing);
  });
}

Future<void> _show(WidgetTester tester, _CreatorBridge bridge) async {
  tester.view.physicalSize = const Size(1000, 2400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final controller = TopBarController();
  addTearDown(controller.dispose);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: TopBarScope(
        controller: controller,
        child: ScaffoldMessenger(
          child: Scaffold(
            body: MarketCreatorScreen(clients: GeneratedCoreProxyClients(bridge)),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Map<String, Object?> _entry({
  required String id,
  required String title,
  String type = 'package',
  String relation = 'owner',
  String stateCode = 'approved',
  List<String> reasonCodes = const <String>[],
  String? reviewDetail,
}) => <String, Object?>{
  'id': id,
  'title': title,
  'type': type,
  'relation': relation,
  'stateCode': stateCode,
  'listingState': null,
  'categoryId': null,
  'updatedAt': '2026-11-02T08:00:00Z',
  'reasonCodes': reasonCodes,
  'reviewDetail': reviewDetail,
  'reviewDetailUpdatedAt': null,
  'revisionAvailableAt': null,
};

class _CreatorBridge extends OperitRuntimeBridge {
  _CreatorBridge({
    required this.loggedIn,
    this.entries = const <Map<String, Object?>>[],
    this.entriesError,
  });

  final bool loggedIn;
  final String? entriesError;
  List<Map<String, Object?>> entries;
  final List<String> calls = <String>[];
  final List<String> withdrawn = <String>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    final Object? result;
    switch (request.methodName) {
      case 'isLoggedIn':
        result = loggedIn;
      case 'getCurrentUserInfo':
        result = loggedIn
            ? <String, Object?>{
                'id': '1',
                'login': 'tester',
                'name': 'Test User',
                'avatar_url': '',
              }
            : null;
      case 'get_my_entries':
        if (entriesError != null) throw StateError(entriesError!);
        result = <String, Object?>{
          'ok': true,
          'entries': entries,
          'generatedAt': null,
        };
      case 'get_notifications':
        result = <String, Object?>{
          'ok': true,
          'items': <Object?>[
            <String, Object?>{
              'id': 'n1',
              'kind': 'comment_new',
              'entryId': 'live',
              'commentId': null,
              'actorId': 'someone',
              'title': 'Nice plugin',
              'body': 'Great work.',
              'createdAt': '2026-11-02T08:00:00Z',
            },
          ],
        };
      case 'withdraw_entry':
        final id = (request.args as Map)['entry_id'] as String;
        withdrawn.add(id);
        entries = <Map<String, Object?>>[
          for (final entry in entries)
            if (entry['id'] == id)
              <String, Object?>{...entry, 'stateCode': 'withdrawn'}
            else
              entry,
        ];
        result = null;
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}
