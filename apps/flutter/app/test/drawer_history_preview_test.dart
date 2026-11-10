import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatMessageLookupRequests.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatSelectionTransition.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/ChatHistoryTree.dart';
import 'package:operit2/ui/main/components/NavigationDrawerAppearance.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/main/layout/SidebarDockPreferences.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';

/// Verifies independent previews preserve conversation context and workspace isolation.
void main() {
  setUpAll(ClientLogger.initialize);
  test(
    'conversation tree keeps surviving orphans visible and rejects cycles',
    () {
      final parent = _history('A', 0),
          child = _history('A', 1, parentChatId: 'A-0');
      final orphan = _history('A', 2, parentChatId: 'deleted-parent');
      final tree = ChatHistoryTree([child, parent, orphan]);
      expect(tree.roots.map((chat) => chat.id), ['A-0', 'A-2']);
      expect(tree.children['A-0'], [child]);
      expect(() => ChatHistoryTree([parent, parent]), throwsStateError);
      expect(
        () => ChatHistoryTree([_history('A', 0, parentChatId: 'A-1'), child]),
        throwsStateError,
      );
    },
  );

  testWidgets(
    'workspace conversations stay flat and search finds direct titles',
    (tester) async {
      final bridge = _DrawerBridge();
      final histories = [
        _history('A', 0),
        _history('A', 1, parentChatId: 'A-0'),
        _history('A', 2, parentChatId: 'A-1'),
      ];
      await _pumpDrawer(tester, bridge, histories);
      expect(_visibleIds(tester), ['A-0', 'A-1', 'A-2']);
      await tester.tap(find.byTooltip('搜索对话'));
      await _pumpSidebar(tester);
      await tester.enterText(find.byType(TextField), 'A-2');
      await _pumpSidebar(tester);
      expect(_visibleIds(tester), ['A-2']);
      await tester.enterText(find.byType(TextField), '');
      await _pumpSidebar(tester);
      expect(_visibleIds(tester), ['A-0', 'A-1', 'A-2']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'desktop recents combine projects and unbound chats in activity order',
    (tester) async {
      final bridge = _DrawerBridge();
      final histories = [
        _history('A', 0, pinned: true, updatedAt: '2026-11-04T06:00:00Z'),
        _history(
          'B',
          0,
          groupName: 'Design',
          updatedAt: '2026-11-04T16:30:00+08:00',
        ),
        core.ChatHistoryListItem.fromJson({
          ..._history('unbound', 0, updatedAt: '2026-11-04T07:00:00Z').toJson(),
          'workspaceId': null,
          'workspaceName': null,
        }),
        _history('A', 1, updatedAt: '2026-11-04T08:00:00Z'),
        _history('B', 1, updatedAt: '2026-11-04T08:00:00Z'),
        core.ChatHistoryListItem.fromJson({
          ..._history('missing', 0, updatedAt: 'invalid').toJson(),
          'workspaceName': null,
        }),
      ];
      await _pumpDrawer(
        tester,
        bridge,
        histories,
        currentChatId: 'B-0',
        conversationsOnly: true,
      );
      expect(find.text('最近会话'), findsOneWidget);
      expect(find.text('未关联项目'), findsOneWidget);
      expect(_recentIds(tester), ['B-0', 'A-1', 'B-1', 'unbound-0']);
      final recentRow = tester.widget<ConversationDrawerItem>(
        find.byKey(const ValueKey('recent-chat:B-0')),
      );
      expect(recentRow.selected, isTrue);
      expect(recentRow.history.workspaceId, 'B');
      expect(recentRow.history.group, 'Design');
      expect(recentRow.canAcceptDrop(histories[4]), isFalse);
      recentRow.onMoveTo(histories[4]);
      await _pumpSidebar(tester);
      expect(
        bridge.calls.where((call) => call.methodName == 'updateChatOrder'),
        isEmpty,
      );
      await tester.tap(find.byKey(const ValueKey('recent-history-limit')));
      await _pumpSidebar(tester);
      expect(_recentIds(tester), [
        'B-0',
        'A-1',
        'B-1',
        'unbound-0',
        'A-0',
        'missing-0',
      ]);
      await tester.tap(find.byKey(const ValueKey('recent-history-header')));
      await _pumpSidebar(tester);
      expect(_recentIds(tester), isEmpty);
      expect(_workspaceIds(tester), [
        'unbound-0',
        'A-0',
        'A-1',
        'B-0',
        'B-1',
        'missing-0',
      ]);
      await tester.tap(find.byKey(const ValueKey('recent-history-header')));
      await _pumpSidebar(tester);
      await tester.tap(find.byKey(const ValueKey('recent-history-limit')));
      await _pumpSidebar(tester);
      expect(_recentIds(tester), ['B-0', 'A-1', 'B-1', 'unbound-0']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'desktop recents exist with only bound chats and update with canonical metadata',
    (tester) async {
      final bridge = _DrawerBridge();
      final first = _history('A', 0, updatedAt: '2026-11-04T06:00:00Z');
      final second = _history(
        'B',
        0,
        groupName: 'Design',
        updatedAt: '2026-11-04T08:00:00Z',
      );
      await _pumpDrawer(tester, bridge, [
        first,
        second,
      ], conversationsOnly: true);
      expect(find.text('未关联项目'), findsNothing);
      expect(_recentIds(tester), ['B-0', 'A-0']);
      await tester.tap(find.byKey(const ValueKey('recent-chat:B-0')));
      await _pumpSidebar(tester);
      final activated = bridge.calls.singleWhere(
        (call) => call.methodName == 'switchChat',
      );
      expect((activated.args as Map)['chatId'], 'B-0');
      expect(
        bridge.calls.where((call) => call.methodName == 'updateChatGroups'),
        isEmpty,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'bindChatToWorkspace'),
        isEmpty,
      );
      final changed = core.ChatHistoryListItem.fromJson({
        ...first.toJson(),
        'title': 'Updated A',
        'updatedAt': '2026-11-04T09:00:00Z',
      });
      await _pumpDrawer(
        tester,
        bridge,
        [changed, second],
        currentChatId: 'B-0',
        conversationsOnly: true,
      );
      expect(_recentIds(tester), ['A-0', 'B-0']);
      expect(
        tester
            .widget<ConversationDrawerItem>(
              find.byKey(const ValueKey('recent-chat:A-0')),
            )
            .title,
        'Updated A',
      );
      for (final prefix in ['recent', 'workspace']) {
        final selected = tester.widget<ConversationDrawerItem>(
          find.byKey(ValueKey('$prefix-chat:B-0')),
        );
        expect(selected.selected, isTrue);
        expect(selected.history.workspaceId, 'B');
        expect(selected.history.group, 'Design');
      }
      await _pumpDrawer(tester, bridge, [changed], conversationsOnly: true);
      expect(_recentIds(tester), ['A-0']);
      expect(find.byKey(const ValueKey('recent-chat:B-0')), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('desktop lists every project with compact group hierarchy', (
    tester,
  ) async {
    final unbound = core.ChatHistoryListItem.fromJson({
      ..._history('recent', 0).toJson(),
      'workspaceId': null,
      'workspaceName': null,
    });
    await _pumpDrawer(
      tester,
      _DrawerBridge(),
      [
        unbound,
        _history('A', 0),
        _history('A', 1, groupName: 'Research'),
        _history('B', 0, groupName: 'Design'),
      ],
      currentChatId: 'A-0',
      conversationsOnly: true,
    );
    expect(
      find.byKey(const ValueKey('desktop-workspace-picker')),
      findsNothing,
    );
    expect(_workspaceIds(tester), ['recent-0', 'A-0', 'A-1', 'B-0']);
    expect(find.text('未分组'), findsNWidgets(2));
    expect(find.text('Research'), findsOneWidget);
    expect(find.text('Design'), findsOneWidget);
    final toolbar = find.byKey(const ValueKey('workspace-create-toolbar'));
    final project = find.byKey(const ValueKey('workspace-header:workspace:A'));
    expect(
      tester.getTopLeft(project).dy,
      greaterThan(tester.getBottomLeft(toolbar).dy),
    );
    final projectStart = tester.getTopLeft(find.text('A')).dx;
    final groupStart = tester.getTopLeft(find.text('Research')).dx;
    final conversationStart = tester
        .getTopLeft(
          find.descendant(
            of: find.byKey(const ValueKey('workspace-chat:A-1')),
            matching: find.text('A-1'),
          ),
        )
        .dx;
    expect(groupStart, greaterThan(projectStart));
    expect(conversationStart - groupStart, inExclusiveRange(0, 16));
    await tester.tap(find.text('Research'));
    await _pumpSidebar(tester);
    expect(_workspaceIds(tester), ['recent-0', 'A-0', 'B-0']);
    await tester.tap(find.text('A'));
    await _pumpSidebar(tester);
    expect(_workspaceIds(tester), ['recent-0', 'B-0']);
    await tester.tap(find.text('A'));
    await _pumpSidebar(tester);
    expect(_workspaceIds(tester), ['recent-0', 'A-0', 'B-0']);
    await tester.tap(find.text('Research'));
    await _pumpSidebar(tester);
    expect(_workspaceIds(tester), ['recent-0', 'A-0', 'A-1', 'B-0']);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'desktop creation inherits active chat and project actions stay explicit',
    (tester) async {
      final bridge = _DrawerBridge();
      final unbound = core.ChatHistoryListItem.fromJson({
        ..._history('recent', 0).toJson(),
        'workspaceId': null,
        'workspaceName': null,
      });
      await _pumpDrawer(
        tester,
        bridge,
        [unbound, _history('A', 0), _history('B', 0)],
        currentChatId: 'A-0',
        conversationsOnly: true,
      );
      await tester.tap(find.byKey(const ValueKey('workspace-create-chat')));
      await _pumpSidebar(tester);
      var calls = bridge.calls.where(
        (call) => call.methodName == 'createNewChat',
      );
      expect((calls.last.args as Map)['workspaceId'], 'A');
      for (final entry in [('B', 'B'), ('unbound', null)]) {
        final header = find.byKey(
          ValueKey('workspace-header:workspace:${entry.$1}'),
        );
        await tester.tap(
          find.descendant(of: header, matching: find.byTooltip('项目操作')),
        );
        await _pumpSidebar(tester);
        expect(find.text('新建分组'), findsOneWidget);
        expect(
          find.text('删除工作区'),
          entry.$2 == null ? findsNothing : findsOneWidget,
        );
        await tester.tap(find.text('新建对话').last);
        await _pumpSidebar(tester);
        calls = bridge.calls.where(
          (call) => call.methodName == 'createNewChat',
        );
        expect((calls.last.args as Map)['workspaceId'], entry.$2);
      }
      expect(calls, hasLength(3));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'desktop search reveals collapsed projects and groups without changing folds',
    (tester) async {
      await _pumpDrawer(
        tester,
        _DrawerBridge(),
        [_history('A', 0), _history('B', 0, groupName: 'Design')],
        currentChatId: 'A-0',
        conversationsOnly: true,
      );
      await tester.tap(find.text('Design'));
      await _pumpSidebar(tester);
      await tester.tap(find.text('B'));
      await _pumpSidebar(tester);
      await tester.tap(find.byKey(const ValueKey('recent-history-header')));
      await _pumpSidebar(tester);
      expect(_recentIds(tester), isEmpty);
      await tester.tap(find.byTooltip('搜索对话'));
      await _pumpSidebar(tester);
      await tester.enterText(find.byType(TextField), 'B-0');
      await _pumpSidebar(tester);
      expect(_workspaceIds(tester), ['B-0']);
      expect(_recentIds(tester), ['B-0']);
      expect(find.text('Design'), findsOneWidget);
      await tester.tap(find.byTooltip('收起搜索'));
      await _pumpSidebar(tester);
      expect(_recentIds(tester), isEmpty);
      expect(_workspaceIds(tester), ['A-0']);
      await tester.tap(find.text('B'));
      await _pumpSidebar(tester);
      expect(find.text('Design'), findsOneWidget);
      expect(_workspaceIds(tester), ['A-0']);
      await tester.tap(find.text('Design'));
      await _pumpSidebar(tester);
      expect(_workspaceIds(tester), ['A-0', 'B-0']);
      await tester.tap(find.byTooltip('搜索对话'));
      await _pumpSidebar(tester);
      await tester.enterText(find.byType(TextField), 'no-match');
      await _pumpSidebar(tester);
      expect(find.text('没有匹配的会话'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'desktop starts with all projects open independently of the mobile drawer',
    (tester) async {
      final bridge = _DrawerBridge();
      final histories = [_history('A', 0), _history('B', 0)];
      await _pumpDrawer(tester, bridge, histories);
      await tester.tap(find.text('B'));
      await _pumpSidebar(tester);
      expect(_workspaceIds(tester), ['A-0']);
      await _pumpDrawer(tester, bridge, histories, conversationsOnly: true);
      expect(_workspaceIds(tester), ['A-0', 'B-0']);
      await _pumpDrawer(tester, bridge, histories);
      expect(_workspaceIds(tester), ['A-0']);
      await tester.tap(find.text('B'));
      await _pumpSidebar(tester);
      expect(_workspaceIds(tester), ['A-0', 'B-0']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'desktop keeps group creation and previews scoped to their project',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(
        tester,
        bridge,
        [..._histories('A', 6), _history('B', 0, groupName: 'Design')],
        currentChatId: 'A-0',
        conversationsOnly: true,
      );
      expect(_workspaceIds(tester), ['A-0', 'A-1', 'A-2', 'A-3', 'B-0']);
      await tester.tap(_limitButton('A'));
      await _pumpSidebar(tester);
      expect(_workspaceIds(tester), [
        'A-0',
        'A-1',
        'A-2',
        'A-3',
        'A-4',
        'A-5',
        'B-0',
      ]);
      final group = find.byKey(
        const ValueKey('workspace-group:workspace:B::group:Design'),
      );
      await tester.tap(
        find.descendant(of: group, matching: find.byTooltip('分组操作')),
      );
      await _pumpSidebar(tester);
      expect(find.text('重命名'), findsOneWidget);
      expect(find.text('解散分组'), findsOneWidget);
      await tester.tap(find.text('新建对话').last);
      await _pumpSidebar(tester);
      final created = bridge.calls.lastWhere(
        (call) => call.methodName == 'createNewChat',
      );
      expect((created.args as Map)['workspaceId'], 'B');
      final grouped = bridge.calls.lastWhere(
        (call) => call.methodName == 'updateChatGroups',
      );
      expect((grouped.args as Map)['groupName'], 'Design');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('desktop creation stays fixed while all projects scroll', (
    tester,
  ) async {
    final projects = List.generate(18, (index) => '项目 $index 的长名称用于检查窄栏显示');
    await _pumpDrawer(
      tester,
      _DrawerBridge(),
      [for (final project in projects) _history(project, 0)],
      currentChatId: '${projects.first}-0',
      conversationsOnly: true,
    );
    final toolbar = find.byKey(const ValueKey('workspace-create-toolbar'));
    final before = tester.getTopLeft(toolbar);
    final scroll = find.byKey(
      const PageStorageKey<String>('desktop-workspace-scroll'),
    );
    await tester.drag(scroll, const Offset(0, -500));
    await _pumpSidebar(tester);
    expect(tester.getTopLeft(toolbar), before);
    expect(
      find.byKey(const ValueKey('workspace-create-chat')).hitTestable(),
      findsOneWidget,
    );
    final last = find.byKey(
      ValueKey('workspace-header:workspace:${projects.last}'),
    );
    await tester.scrollUntilVisible(
      last,
      420,
      scrollable: find
          .descendant(of: scroll, matching: find.byType(Scrollable))
          .first,
    );
    await _pumpSidebar(tester);
    expect(last.hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'default and named groups keep controls with shallow indentation',
    (tester) async {
      final bridge = _DrawerBridge();
      final unbound = core.ChatHistoryListItem.fromJson({
        ..._history('unbound', 0).toJson(),
        'workspaceId': null,
        'workspaceName': null,
      });
      await _pumpDrawer(tester, bridge, [
        unbound,
        _history('A', 0),
        _history('A', 1, groupName: 'Research'),
      ]);
      expect(find.text('未分组'), findsNWidgets(2));
      expect(find.text('未绑定项目'), findsNothing);
      expect(find.text('未关联项目'), findsOneWidget);
      expect(find.text('Research'), findsOneWidget);
      expect(_visibleIds(tester), ['unbound-0', 'A-0', 'A-1']);
      for (final row in tester.widgetList<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      )) {
        expect(row.nested, isFalse);
      }
      expect(
        tester.getTopLeft(find.text('A-1')).dx -
            tester.getTopLeft(find.text('A-0')).dx,
        0,
      );
      final defaultGroup = find.byKey(
        const ValueKey('workspace-group:workspace:A'),
      );
      expect(
        find.descendant(of: defaultGroup, matching: find.byTooltip('分组操作')),
        findsOneWidget,
      );
      await tester.tap(
        find.descendant(of: defaultGroup, matching: find.text('未分组')),
      );
      await _pumpSidebar(tester);
      expect(find.text('A-0'), findsNothing);
      expect(find.text('A-1'), findsOneWidget);
      await tester.tap(
        find.descendant(of: defaultGroup, matching: find.text('未分组')),
      );
      await _pumpSidebar(tester);
      expect(find.text('A-0'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'chat activation submits Core work without waiting for another frame',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, [_history('fast', 0)]);
      final row = tester.widget<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      );
      row.onClick();
      expect(
        bridge.calls.where((call) => call.methodName == 'switchChat'),
        hasLength(1),
      );
      ChatSelectionTransition.complete('fast-0');
      await tester.pump();
    },
  );

  for (final entry in <(String, bool)>[('搜索消息', false), ('查看收藏', true)]) {
    testWidgets(
      'sidebar ${entry.$1} activates the selected conversation first',
      (tester) async {
        final bridge = _DrawerBridge();
        await _pumpDrawer(tester, bridge, [
          _history('lookup', 0),
        ], currentChatId: 'another-chat');
        final l10n = AppLocalizations.of(
          tester.element(find.byType(DrawerContent)),
        )!;
        final row = find.byType(ConversationDrawerItem);
        await tester.tap(
          row,
          buttons: kSecondaryMouseButton,
          kind: PointerDeviceKind.mouse,
        );
        await _pumpSidebar(tester);
        await tester.tap(
          find.text(
            entry.$2
                ? l10n.messageLocatorViewFavorites
                : l10n.messageLocatorSearchMessages,
          ),
        );
        await _pumpSidebar(tester);
        final calls = bridge.calls.where(
          (call) => call.methodName == 'switchChat',
        );
        expect(calls, hasLength(1));
        expect((calls.single.args as Map)['chatId'], 'lookup-0');
        final request = ChatMessageLookupRequests.requests.value!;
        expect(request.chatId, 'lookup-0');
        expect(request.favoritesOnly, entry.$2);
        ChatMessageLookupRequests.consume(request);
        ChatSelectionTransition.complete('lookup-0');
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('brand header constrains plugin tabs inside narrow drawers', (
    tester,
  ) async {
    for (final scale in [1.0, 1.4]) {
      for (final (headerWidth, tabWidth) in [(195.0, 98.0), (125.0, 245.0)]) {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: MediaQuery(
                data: MediaQueryData(textScaler: TextScaler.linear(scale)),
                child: Align(
                  alignment: Alignment.topLeft,
                  child: SizedBox(
                    width: headerWidth,
                    child: SidebarInfoCard(
                      brandName: 'Operit',
                      appearance: const NavigationDrawerAppearance(
                        containerColor: Colors.white,
                        titleColor: Colors.black,
                        statusAvailableColor: Colors.blue,
                        itemColor: Colors.black87,
                        buttonContainerColor: Colors.white70,
                        selectedContainerColor: Colors.blue,
                        selectedContentColor: Colors.white,
                        dividerColor: Colors.black12,
                        transparentSurfaceEnabled: false,
                      ),
                      trailing: SizedBox(
                        key: const ValueKey('test-tab-switch'),
                        width: tabWidth,
                        height: 23,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        final tabs = tester.getRect(
          find.byKey(const ValueKey('test-tab-switch')),
        );
        final brand = tester.getRect(find.text('Operit'));
        expect(tabs.right, lessThanOrEqualTo(headerWidth));
        expect(brand.right, lessThanOrEqualTo(tabs.left));
        expect(brand.width, greaterThan(0));
        if (headerWidth == 195) expect(tabs.width, 98);
      }
    }
  });

  testWidgets(
    'workspace retains the legacy compact create bar and scrolling brand header',
    (tester) async {
      await _pumpDrawer(tester, _DrawerBridge(), _histories('A', 6));
      final create = find.byKey(const ValueKey('workspace-create-chat'));
      expect(tester.getSize(create).height, 34);
      expect(
        find.descendant(of: create, matching: find.byIcon(Icons.add_rounded)),
        findsOneWidget,
      );
      expect(
        find.ancestor(of: create, matching: find.byType(FilledButton)),
        findsNothing,
      );
      final scroll = find.byKey(
        const PageStorageKey<String>('drawer-history-scroll'),
      );
      expect(
        find.descendant(of: scroll, matching: find.byType(SidebarInfoCard)),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: scroll,
          matching: find.byKey(const ValueKey('chat-sidebar-segmented-switch')),
        ),
        findsOneWidget,
      );
      expect(find.ancestor(of: create, matching: scroll), findsOneWidget);
      final material = tester.widget<Material>(
        find.ancestor(of: create, matching: find.byType(Material)).first,
      );
      expect(material.shape, isA<StadiumBorder>());
      expect(
        material.color,
        Theme.of(tester.element(create)).colorScheme.primaryContainer,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'conversation paint and hit bounds fill the row without moving title centers',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(
        tester,
        bridge,
        _histories('A', 2),
        currentChatId: 'A-0',
        conversationsOnly: true,
      );
      for (final prefix in ['recent', 'workspace']) {
        final centers = <double>[];
        for (final id in ['A-0', 'A-1']) {
          final row = find.byKey(ValueKey('$prefix-chat:$id'));
          final item = tester.widget<ConversationDrawerItem>(row);
          final rowRect = tester.getRect(row);
          final ink = find.descendant(
            of: row,
            matching: find.byWidgetPredicate(
              (widget) => widget is InkWell && widget.onTap == item.onClick,
            ),
          );
          final inkRect = tester.getRect(ink);
          final title = find.descendant(of: row, matching: find.text(id));
          centers.add(tester.getCenter(title).dy);
          expect(rowRect.height, 30);
          expect(inkRect.top, rowRect.top + 1);
          expect(inkRect.bottom, rowRect.bottom - 1);
          expect(tester.getCenter(title).dy, rowRect.center.dy);
          final material = find
              .ancestor(of: ink, matching: find.byType(Material))
              .first;
          expect(tester.getRect(material), inkRect);
          for (final y in [inkRect.top + 1, inkRect.bottom - 1]) {
            final before = bridge.calls
                .where((call) => call.methodName == 'switchChat')
                .length;
            await tester.tapAt(Offset(inkRect.center.dx, y));
            await _pumpSidebar(tester);
            final switches = bridge.calls
                .where((call) => call.methodName == 'switchChat')
                .toList();
            expect(switches.length, before + 1);
            expect((switches.last.args as Map)['chatId'], id);
          }
        }
        expect(centers[1] - centers[0], 30);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'workspace headers stay compact and conversation rows have no extra rail',
    (tester) async {
      await _pumpDrawer(tester, _DrawerBridge(), _histories('A', 2));
      final header = find.byKey(const ValueKey('workspace-header:workspace:A'));
      final containers = tester.widgetList<Container>(
        find.descendant(of: header, matching: find.byType(Container)),
      );
      expect(
        containers.where(
          (container) =>
              container.constraints?.maxWidth == 3 &&
              container.constraints?.maxHeight == 17,
        ),
        isEmpty,
      );
      for (final item in tester.widgetList<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      )) {
        expect(item.workspaceStyle, isTrue);
        expect(item.nested, isFalse);
      }
      await tester.tap(find.byTooltip('搜索对话'));
      await _pumpSidebar(tester);
      expect(
        find.ancestor(
          of: find.byType(ConversationSearchField),
          matching: find.byType(AnimatedSize),
        ),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('each group has an independent and reversible preview', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    await _pumpDrawer(tester, bridge, <core.ChatHistoryListItem>[
      ..._histories('A', 6),
      ..._histories('B', 6),
    ]);
    expect(_visibleIds(tester), <String>[
      'A-0',
      'A-1',
      'A-2',
      'A-3',
      'B-0',
      'B-1',
      'B-2',
      'B-3',
    ]);
    expect(find.text('展开更多 2'), findsNWidgets(2));

    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    expect(find.text('收起'), findsOneWidget);
    expect(find.text('A-5'), findsOneWidget);
    expect(find.text('B-4'), findsNothing);

    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    expect(find.text('A-4'), findsNothing);
    expect(find.text('B-3'), findsOneWidget);
    expect(find.text('展开更多 2'), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  });

  testWidgets('collapsing a group never changes another group preview', (
    tester,
  ) async {
    await _pumpDrawer(tester, _DrawerBridge(), <core.ChatHistoryListItem>[
      ..._histories('A', 6),
      ..._histories('B', 6),
    ]);
    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    await tester.tap(find.text('A'));
    await _pumpSidebar(tester);
    expect(find.text('A-0'), findsNothing);
    expect(find.text('B-3'), findsOneWidget);
    expect(find.text('B-4'), findsNothing);
    expect(_limitButton('A'), findsNothing);

    await tester.tap(find.text('A'));
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsOneWidget);
    expect(find.text('收起'), findsOneWidget);
    expect(find.text('B-4'), findsNothing);
    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
  });

  testWidgets(
    'previews keep current pinned and streaming conversations visible',
    (tester) async {
      final histories = <core.ChatHistoryListItem>[
        ..._histories('A', 5),
        _history('A', 5, pinned: true),
        _history('A', 6),
        _history('A', 7),
      ];
      await _pumpDrawer(
        tester,
        _DrawerBridge(),
        histories,
        currentChatId: 'A-4',
        streamingIds: <String>{'A-6'},
      );
      expect(_visibleIds(tester), <String>[
        'A-0',
        'A-1',
        'A-2',
        'A-3',
        'A-4',
        'A-5',
        'A-6',
      ]);
      expect(find.text('展开更多 1'), findsOneWidget);
      await tester.tap(_limitButton('A'));
      await _pumpSidebar(tester);
      expect(find.text('A-7'), findsOneWidget);
      await tester.tap(_limitButton('A'));
      await _pumpSidebar(tester);
      expect(find.text('A-7'), findsNothing);
      expect(
        tester
            .widgetList<ConversationDrawerItem>(
              find.byType(ConversationDrawerItem),
            )
            .singleWhere((item) => item.history.id == 'A-4')
            .selected,
        isTrue,
      );
      expect(find.text('A-5'), findsOneWidget);
      expect(find.text('A-6'), findsOneWidget);
    },
  );

  testWidgets(
    'expanded groups include new conversations without another click',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, _histories('A', 6));
      await tester.tap(_limitButton('A'));
      await _pumpSidebar(tester);
      await _pumpDrawer(tester, bridge, _histories('A', 8));
      expect(find.text('A-7'), findsOneWidget);
      expect(find.text('收起'), findsOneWidget);
      expect(find.text('展开更多 2'), findsNothing);
      await tester.tap(_limitButton('A'));
      await _pumpSidebar(tester);
      expect(find.text('展开更多 4'), findsOneWidget);
    },
  );

  testWidgets('search includes hidden rows and preserves preview expansion', (
    tester,
  ) async {
    await _pumpDrawer(tester, _DrawerBridge(), _histories('A', 6));
    await tester.tap(find.byTooltip('搜索对话'));
    await _pumpSidebar(tester);
    await tester.enterText(find.byType(TextField), 'A-5');
    await _pumpSidebar(tester);
    expect(_visibleIds(tester), <String>['A-5']);
    expect(_limitButton('A'), findsNothing);
    await tester.enterText(find.byType(TextField), '');
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsNothing);
    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    await tester.enterText(find.byType(TextField), 'A-5');
    await _pumpSidebar(tester);
    await tester.enterText(find.byType(TextField), '');
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsOneWidget);
    expect(find.text('收起'), findsOneWidget);
  });

  testWidgets(
    'switching the current conversation updates the collapsed preview',
    (tester) async {
      final bridge = _DrawerBridge();
      final histories = _histories('A', 7);
      await _pumpDrawer(tester, bridge, histories, currentChatId: 'A-6');
      expect(find.text('A-6'), findsOneWidget);
      expect(find.text('展开更多 2'), findsOneWidget);
      await _pumpDrawer(tester, bridge, histories, currentChatId: 'A-5');
      expect(find.text('A-6'), findsNothing);
      expect(find.text('A-5'), findsOneWidget);
      expect(find.text('展开更多 2'), findsOneWidget);
    },
  );

  testWidgets('workspace grouping uses the same independent previews', (
    tester,
  ) async {
    await _pumpDrawer(tester, _DrawerBridge(), <core.ChatHistoryListItem>[
      ..._histories('A', 6),
      ..._histories('B', 6),
    ]);
    final button = find.byKey(
      const ValueKey<String>('history-limit:workspace:A'),
    );
    expect(find.text('展开更多 2'), findsNWidgets(2));
    await tester.tap(button);
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsOneWidget);
    expect(find.text('B-4'), findsNothing);
    await tester.tap(button);
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsNothing);
    expect(find.text('展开更多 2'), findsNWidgets(2));
  });

  testWidgets(
    'workspace controls are native and folding performs no backend mutations',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, _histories('A', 6));
      expect(find.text('A'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('workspace-create-group')),
        findsNothing,
      );
      expect(find.byKey(const ValueKey('workspace-bind-folder')), findsNothing);
      expect(find.text('新建对话'), findsOneWidget);
      await tester.tap(find.text('A'));
      await _pumpSidebar(tester);
      expect(find.text('A-0'), findsNothing);
      expect(
        bridge.calls.where(
          (call) =>
              call.methodName == 'updateChatOrderAndGroup' ||
              call.methodName == 'deleteChatHistory' ||
              call.methodName == 'updateChatPinned',
        ),
        isEmpty,
      );
      await tester.tap(find.text('A'));
      await _pumpSidebar(tester);
    },
  );

  testWidgets(
    'only chats without a binding share the unbound workspace header',
    (tester) async {
      final histories = [
        for (final group in [
          'pending-a',
          'pending-b',
          'unbound-a',
          'unbound-b',
        ])
          core.ChatHistoryListItem.fromJson({
            ..._history(group, 0).toJson(),
            'workspaceId': switch (group) {
              'unbound-a' || 'unbound-b' => null,
              _ => group,
            },
            'workspaceName': null,
          }),
      ];
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, histories);
      expect(find.text('未关联项目'), findsOneWidget);
      expect(find.text('项目记录缺失'), findsNWidgets(2));
      expect(_visibleIds(tester), [
        'unbound-a-0',
        'unbound-b-0',
        'pending-a-0',
        'pending-b-0',
      ]);

      final hydrated = [
        for (final history in histories)
          core.ChatHistoryListItem.fromJson({
            ...history.toJson(),
            'workspaceName': switch (history.workspaceId) {
              'pending-a' => 'Project A',
              'pending-b' => 'Project B',
              _ => null,
            },
          }),
      ];
      await _pumpDrawer(tester, bridge, hydrated);
      expect(find.text('未关联项目'), findsOneWidget);
      expect(find.text('项目记录缺失'), findsNothing);
      expect(find.text('Project A'), findsOneWidget);
      expect(find.text('Project B'), findsOneWidget);
      expect(_visibleIds(tester), [
        'unbound-a-0',
        'unbound-b-0',
        'pending-a-0',
        'pending-b-0',
      ]);
    },
  );

  for (final pointerKind in [
    PointerDeviceKind.touch,
    PointerDeviceKind.mouse,
  ]) {
    testWidgets(
      'identical in-row swipes delete valid and missing workspaces with $pointerKind',
      (tester) async {
        final bridge = _DrawerBridge();
        final histories = [
          _history('valid', 0),
          for (final id in ['missing-a', 'missing-b'])
            core.ChatHistoryListItem.fromJson({
              ..._history(id, 0).toJson(),
              'workspaceName': null,
            }),
        ];
        await _pumpDrawer(
          tester,
          bridge,
          histories,
          currentChatId: histories.first.id,
        );
        final l10n = AppLocalizations.of(
          tester.element(find.byType(DrawerContent)),
        )!;
        for (final history in histories) {
          final swipe = find.byKey(
            ValueKey<String>('conversation-${history.id}'),
          );
          final bounds = tester.getRect(swipe);
          final gesture = await tester.startGesture(
            bounds.center,
            kind: pointerKind,
          );
          for (var step = 1; step <= 10; step++) {
            await gesture.moveTo(
              bounds.center - Offset(bounds.width * step / 20, 0),
            );
            await tester.pump(const Duration(milliseconds: 16));
          }
          await gesture.up();
          await _pumpSidebar(tester);
          expect(
            find.text(l10n.deleteConversationTitle),
            findsOneWidget,
            reason: history.id,
          );
          final previous = bridge.calls
              .where((call) => call.methodName == 'deleteChatHistory')
              .length;
          await tester.tap(find.widgetWithText(TextButton, l10n.cancel));
          await _pumpSidebar(tester);
          expect(
            bridge.calls
                .where((call) => call.methodName == 'deleteChatHistory')
                .length,
            previous,
          );
          final row = tester.widget<ConversationDrawerItem>(
            find.byKey(ValueKey('workspace-chat:${history.id}')),
          );
          row.onSwipeDelete();
          await _pumpSidebar(tester);
          await tester.tap(find.widgetWithText(TextButton, l10n.delete));
          await _pumpSidebar(tester);
          final calls = bridge.calls
              .where((call) => call.methodName == 'deleteChatHistory')
              .toList();
          expect(calls, hasLength(previous + 1));
          expect((calls.last.args as Map)['chatId'], history.id);
          expect(tester.takeException(), isNull);
        }
      },
    );
  }

  testWidgets(
    'a missing workspace record does not block direct menu deletion',
    (tester) async {
      final bridge = _DrawerBridge();
      final history = core.ChatHistoryListItem.fromJson({
        ..._history('missing', 0).toJson(),
        'workspaceName': null,
      });
      await _pumpDrawer(tester, bridge, [history], currentChatId: history.id);
      expect(find.text('项目记录缺失'), findsOneWidget);
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DrawerContent)),
      )!;
      await tester.tap(
        find.descendant(
          of: find.byType(ConversationDrawerItem),
          matching: find.byTooltip('更多操作'),
        ),
      );
      await _pumpSidebar(tester);
      await tester.tap(find.text('删除'));
      await _pumpSidebar(tester);
      expect(find.text(l10n.deleteConversationTitle), findsNothing);
      final calls = bridge.calls
          .where((call) => call.methodName == 'deleteChatHistory')
          .toList();
      expect(calls, hasLength(1));
      expect((calls.single.args as Map)['chatId'], history.id);
      expect(
        bridge.calls.where((call) => call.methodName == 'switchChat'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('long-press delete does not ask for a second confirmation', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    final history = _history('valid', 0);
    await _pumpDrawer(tester, bridge, [history]);
    final l10n = AppLocalizations.of(
      tester.element(find.byType(DrawerContent)),
    )!;
    await tester.longPress(find.text(history.title));
    await _pumpSidebar(tester);
    await tester.tap(find.text(l10n.delete));
    await _pumpSidebar(tester);
    expect(find.text(l10n.deleteConversationTitle), findsNothing);
    final calls = bridge.calls
        .where((call) => call.methodName == 'deleteChatHistory')
        .toList();
    expect(calls, hasLength(1));
    expect((calls.single.args as Map)['chatId'], history.id);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'swipe confirmation and direct deletion preserve Core lock refusal',
    (tester) async {
      final bridge = _DrawerBridge(deleteAllowed: false);
      final history = core.ChatHistoryListItem.fromJson({
        ..._history('locked', 0).toJson(),
        'locked': true,
      });
      await _pumpDrawer(tester, bridge, [history]);
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DrawerContent)),
      )!;
      final row = tester.widget<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      );
      row.onSwipeDelete();
      await _pumpSidebar(tester);
      expect(find.text(l10n.deleteConversationTitle), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteChatHistory'),
        isEmpty,
      );
      await tester.tap(find.widgetWithText(TextButton, l10n.delete));
      await _pumpSidebar(tester);
      expect(find.text(l10n.chatLockedCannotDelete), findsOneWidget);
      row.onDelete();
      await _pumpSidebar(tester);
      expect(find.text(l10n.deleteConversationTitle), findsNothing);
      expect(find.text(l10n.chatLockedCannotDelete), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteChatHistory'),
        hasLength(2),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'a missing workspace record can be detached through the explicit chat',
    (tester) async {
      final bridge = _DrawerBridge();
      final history = core.ChatHistoryListItem.fromJson({
        ..._history('missing', 0).toJson(),
        'workspaceName': null,
      });
      await _pumpDrawer(tester, bridge, [history]);
      final row = tester.widget<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      );
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DrawerContent)),
      )!;
      row.onLongPress();
      await _pumpSidebar(tester);
      await tester.tap(find.text(l10n.workspaceUnbindTitle));
      await _pumpSidebar(tester);
      expect(find.text(l10n.workspaceUnbindConfirmation), findsOneWidget);
      await tester.tap(
        find.widgetWithText(FilledButton, l10n.workspaceUnbindTitle),
      );
      await _pumpSidebar(tester);
      final calls = bridge.calls
          .where((call) => call.methodName == 'unbindChatFromWorkspace')
          .toList();
      expect(calls, hasLength(1));
      expect((calls.single.args as Map)['chatId'], 'missing-0');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'workspace deletion confirms before deleting the whole workspace',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, [_history('A', 0), _history('A', 1)]);
      final header = find.byKey(const ValueKey('workspace-header:workspace:A'));
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DrawerContent)),
      )!;
      Future<void> openDelete() async {
        await tester.tap(
          find.descendant(of: header, matching: find.byTooltip('项目操作')),
        );
        await _pumpSidebar(tester);
        await tester.tap(find.text('删除工作区'));
        await _pumpSidebar(tester);
      }

      await openDelete();
      expect(find.textContaining('共享默认区域及其中的文件将被删除'), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteWorkspace'),
        isEmpty,
      );
      await tester.tap(find.widgetWithText(TextButton, l10n.cancel));
      await _pumpSidebar(tester);
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteWorkspace'),
        isEmpty,
      );
      await openDelete();
      await tester.tap(find.widgetWithText(TextButton, l10n.delete));
      await _pumpSidebar(tester);
      final calls = bridge.calls.where(
        (call) => call.methodName == 'deleteWorkspace',
      );
      expect(calls, hasLength(1));
      expect((calls.single.args as Map)['workspaceId'], 'A');
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteChatHistory'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'workspace cleanup errors keep the workspace available for retry',
    (tester) async {
      addTearDown(AppToastController.instance.reset);
      final bridge = _DrawerBridge(workspaceDeleteAllowed: false);
      await _pumpDrawer(tester, bridge, [_history('A', 0)]);
      final header = find.byKey(const ValueKey('workspace-header:workspace:A'));
      await tester.tap(
        find.descendant(of: header, matching: find.byTooltip('项目操作')),
      );
      await _pumpSidebar(tester);
      await tester.tap(find.text('删除工作区'));
      await _pumpSidebar(tester);
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DrawerContent)),
      )!;
      await tester.tap(find.widgetWithText(TextButton, l10n.delete));
      await _pumpSidebar(tester);
      expect(header, findsOneWidget);
      expect(find.textContaining('Workspace cleanup failed'), findsWidgets);
      expect(
        bridge.calls.where((call) => call.methodName == 'removePreference'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
      AppToastController.instance.reset();
      await _pumpSidebar(tester);
    },
  );

  testWidgets('unbound conversations have no workspace deletion action', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    final chat = core.ChatHistoryListItem.fromJson({
      ..._history('unbound', 0).toJson(),
      'workspaceId': null,
      'workspaceName': null,
    });
    await _pumpDrawer(tester, bridge, [chat]);
    await tester.tap(
      find.descendant(
        of: find.byKey(const ValueKey('workspace-header:workspace:unbound')),
        matching: find.byTooltip('项目操作'),
      ),
    );
    await _pumpSidebar(tester);
    expect(find.text('删除工作区'), findsNothing);
  });

  testWidgets('top project and group creation keep explicit ownership', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    await _pumpDrawer(tester, bridge, [
      _history('A', 0, groupName: 'Research'),
    ]);
    await tester.tap(find.byKey(const ValueKey('workspace-create-chat')));
    await _pumpSidebar(tester);
    var createCalls = bridge.calls.where(
      (call) => call.methodName == 'createNewChat',
    );
    expect((createCalls.single.args as Map)['workspaceId'], isNull);
    expect((createCalls.single.args as Map)['sourceChatId'], isNull);

    await tester.tap(
      find.descendant(
        of: find.byKey(const ValueKey('workspace-header:workspace:A')),
        matching: find.byTooltip('项目操作'),
      ),
    );
    await _pumpSidebar(tester);
    await tester.tap(find.text('新建对话').last);
    await _pumpSidebar(tester);
    createCalls = bridge.calls.where(
      (call) => call.methodName == 'createNewChat',
    );
    expect(createCalls, hasLength(2));
    expect((createCalls.last.args as Map)['workspaceId'], 'A');
    expect(
      bridge.calls.where((call) => call.methodName == 'updateChatGroups'),
      isEmpty,
    );

    await tester.tap(
      find.descendant(
        of: find.byKey(
          const ValueKey('workspace-group:workspace:A::group:Research'),
        ),
        matching: find.byTooltip('分组操作'),
      ),
    );
    await _pumpSidebar(tester);
    await tester.tap(find.text('新建对话').last);
    await _pumpSidebar(tester);
    createCalls = bridge.calls.where(
      (call) => call.methodName == 'createNewChat',
    );
    expect(createCalls, hasLength(3));
    expect((createCalls.last.args as Map)['workspaceId'], 'A');
    expect((createCalls.last.args as Map)['sourceChatId'], isNull);
    final groupCall = bridge.calls.singleWhere(
      (call) => call.methodName == 'updateChatGroups',
    );
    expect((groupCall.args as Map)['groupName'], 'Research');
  });

  testWidgets('directory pinning reorders only directories', (tester) async {
    final bridge = _DrawerBridge();
    await _pumpDrawer(tester, bridge, [
      ..._histories('A', 1),
      _history('B', 0, groupName: 'Research'),
    ]);
    await tester.tap(
      find.descendant(
        of: find.byKey(const ValueKey('workspace-header:workspace:B')),
        matching: find.byTooltip('项目操作'),
      ),
    );
    await _pumpSidebar(tester);
    await tester.tap(find.text('置顶').last);
    await _pumpSidebar(tester);
    expect(
      tester
          .getTopLeft(
            find.byKey(const ValueKey('workspace-header:workspace:B')),
          )
          .dy,
      lessThan(
        tester
            .getTopLeft(
              find.byKey(const ValueKey('workspace-header:workspace:A')),
            )
            .dy,
      ),
    );
    expect(
      bridge.directoryPreferences[(const SidebarDirectory.workspace(
        'B',
      )).preferenceKey],
      isTrue,
    );
    await tester.tap(
      find.descendant(
        of: find.byKey(
          const ValueKey('workspace-group:workspace:B::group:Research'),
        ),
        matching: find.byTooltip('分组操作'),
      ),
    );
    await _pumpSidebar(tester);
    await tester.tap(find.text('置顶').last);
    await _pumpSidebar(tester);
    expect(
      bridge.directoryPreferences[(const SidebarDirectory.group(
        workspaceId: 'B',
        groupName: 'Research',
      )).preferenceKey],
      isTrue,
    );
    expect(
      bridge.calls.where((call) => call.methodName == 'updateChatPinned'),
      isEmpty,
    );
  });

  testWidgets(
    'renaming and dissolving a group use all members and keep chats',
    (tester) async {
      final bridge = _DrawerBridge();
      const originalDirectory = SidebarDirectory.group(
        workspaceId: 'A',
        groupName: 'Research',
      );
      const renamedDirectory = SidebarDirectory.group(
        workspaceId: 'A',
        groupName: 'Team',
      );
      bridge.directoryPreferences[originalDirectory.preferenceKey] = true;
      final histories = [
        for (var index = 0; index < 6; index++)
          _history('A', index, groupName: 'Research'),
      ];
      await _pumpDrawer(tester, bridge, histories);
      final menu = find.descendant(
        of: find.byKey(
          const ValueKey('workspace-group:workspace:A::group:Research'),
        ),
        matching: find.byTooltip('分组操作'),
      );
      await tester.tap(menu);
      await _pumpSidebar(tester);
      await tester.tap(find.text('重命名'));
      await _pumpSidebar(tester);
      await tester.enterText(find.byType(TextFormField), 'Team');
      await tester.tap(find.text('确认'));
      await _pumpSidebar(tester);
      var groupCalls = bridge.calls
          .where((call) => call.methodName == 'updateChatGroups')
          .toList();
      expect((groupCalls.single.args as Map)['chatIds'], hasLength(6));
      expect((groupCalls.single.args as Map)['groupName'], 'Team');
      expect(
        bridge.directoryPreferences.containsKey(
          originalDirectory.preferenceKey,
        ),
        isFalse,
      );
      expect(
        bridge.directoryPreferences[renamedDirectory.preferenceKey],
        isTrue,
      );
      final renamed = histories
          .map(
            (history) => core.ChatHistoryListItem.fromJson({
              ...history.toJson(),
              'group': 'Team',
            }),
          )
          .toList();
      await _pumpDrawer(tester, bridge, renamed);

      await tester.tap(
        find.descendant(
          of: find.byKey(
            const ValueKey('workspace-group:workspace:A::group:Team'),
          ),
          matching: find.byTooltip('分组操作'),
        ),
      );
      await _pumpSidebar(tester);
      await tester.tap(find.text('解散分组'));
      await _pumpSidebar(tester);
      groupCalls = bridge.calls
          .where((call) => call.methodName == 'updateChatGroups')
          .toList();
      expect(groupCalls, hasLength(2));
      expect((groupCalls.last.args as Map)['chatIds'], hasLength(6));
      expect((groupCalls.last.args as Map)['groupName'], isNull);
      expect(
        bridge.directoryPreferences.containsKey(renamedDirectory.preferenceKey),
        isFalse,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteChatHistory'),
        isEmpty,
      );
    },
  );

  testWidgets('binding action targets the clicked conversation', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    await _pumpDrawer(tester, bridge, _histories('A', 2), currentChatId: 'A-0');
    await tester.tap(
      find.descendant(
        of: find.byKey(const ValueKey('workspace-chat:A-1')),
        matching: find.byTooltip('更多操作'),
      ),
    );
    await _pumpSidebar(tester);
    await tester.tap(find.text('项目绑定'));
    await _pumpSidebar(tester);
    await tester.tap(find.text('新建空白项目'));
    await _pumpSidebar(tester);
    await tester.enterText(find.byType(TextFormField), 'Bound project');
    await tester.tap(find.text('确认'));
    await _pumpSidebar(tester);
    final binding = bridge.calls.singleWhere(
      (call) => call.methodName == 'createAndBindWorkspace',
    );
    expect((binding.args as Map)['chatId'], 'A-1');
  });

  testWidgets('renamed workspace metadata keeps preview keyed by identity', (
    tester,
  ) async {
    final bridge = _DrawerBridge();
    await _pumpDrawer(tester, bridge, _histories('A', 6));
    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    final renamed = _histories('A', 6)
        .map(
          (history) => core.ChatHistoryListItem.fromJson({
            ...history.toJson(),
            'workspaceName': 'Renamed workspace',
          }),
        )
        .toList();
    await _pumpDrawer(tester, bridge, renamed);
    expect(find.text('Renamed workspace'), findsOneWidget);
    expect(find.text('A-5'), findsOneWidget);
    await tester.tap(_limitButton('A'));
    await _pumpSidebar(tester);
    expect(find.text('A-5'), findsNothing);
  });
}

/// Mounts a tall drawer so all preview rows are materialized in the test viewport.
Future<void> _pumpDrawer(
  WidgetTester tester,
  _DrawerBridge bridge,
  List<core.ChatHistoryListItem> histories, {
  String? currentChatId,
  bool conversationsOnly = false,
  Set<String> streamingIds = const <String>{},
}) async {
  bridge.updateHistories(histories);
  tester.view.physicalSize = conversationsOnly
      ? const Size(280, 800)
      : const Size(500, 2400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: DrawerContent(
          conversationsOnly: conversationsOnly,
          navigationEntries: const [],
          pluginEntries: const [],
          selectedRouteId: ScreenRouteRegistry.routeIdOf(
            ScreenRouteRegistry.aiChat,
          ),
          appearance: const NavigationDrawerAppearance(
            containerColor: Colors.white,
            titleColor: Colors.black,
            statusAvailableColor: Colors.blue,
            itemColor: Colors.black87,
            buttonContainerColor: Colors.white70,
            selectedContainerColor: Colors.blue,
            selectedContentColor: Colors.white,
            dividerColor: Colors.black12,
            transparentSurfaceEnabled: false,
          ),
          histories: histories,
          activeStreamingChatIds: streamingIds,

          currentChatId: currentChatId,
          errorMessage: null,
          loading: false,
          bridge: bridge,
          onNavigationEntrySelected: (_) {},
          onConversationActivated: () {},
        ),
      ),
    ),
  );
  await _pumpSidebar(tester);
}

/// Pumps finite sidebar transitions without waiting for streaming animations to stop.
Future<void> _pumpSidebar(WidgetTester tester) async {
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 300));
}

/// Finds the inline preview control for a named workspace.
Finder _limitButton(String group) =>
    find.byKey(ValueKey<String>('history-limit:workspace:$group'));

/// Reads rendered conversation IDs in their original sidebar order.
List<String> _visibleIds(WidgetTester tester) => tester
    .widgetList<ConversationDrawerItem>(find.byType(ConversationDrawerItem))
    .map((item) => item.history.id)
    .toList();

/// Reads project/group rows separately from the recent conversation shortcuts.
List<String> _workspaceIds(WidgetTester tester) => tester
    .widgetList<ConversationDrawerItem>(find.byType(ConversationDrawerItem))
    .where((item) => item.key == ValueKey('workspace-chat:${item.history.id}'))
    .map((item) => item.history.id)
    .toList();

List<String> _recentIds(WidgetTester tester) => tester
    .widgetList<ConversationDrawerItem>(
      find.descendant(
        of: find.byKey(const ValueKey('recent-history-section')),
        matching: find.byType(ConversationDrawerItem),
      ),
    )
    .map((item) => item.history.id)
    .toList();

/// Creates ordered conversations for one named group.
List<core.ChatHistoryListItem> _histories(String group, int count) =>
    <core.ChatHistoryListItem>[
      for (var index = 0; index < count; index++) _history(group, index),
    ];

/// Creates a history row with explicit grouping and state flags.
core.ChatHistoryListItem _history(
  String group,
  int index, {
  bool pinned = false,
  String? parentChatId,
  String? groupName,
  String updatedAt = '2026-10-06T00:00:00Z',
}) => core.ChatHistoryListItem(
  parentChatId: parentChatId,
  group: groupName,
  id: '$group-$index',
  title: '$group-$index',
  updatedAt: updatedAt,

  displayOrder: index,
  workspaceId: group,
  workspaceName: group,

  locked: false,
  pinned: pinned,
);

class _DrawerBridge extends OperitRuntimeBridge {
  _DrawerBridge({
    this.deleteAllowed = true,
    this.workspaceDeleteAllowed = true,
  });

  static const String _userFile = 'user_preferences.preferences.json';
  static const String _dockFile = 'sidebar_dock.preferences.json';

  final bool deleteAllowed;
  final bool workspaceDeleteAllowed;
  final List<CoreCallRequest> calls = <CoreCallRequest>[];
  final Map<String, Map<String, Object?>> _preferences =
      <String, Map<String, Object?>>{
        _userFile: <String, Object?>{},
        _dockFile: <String, Object?>{},
      };
  final Map<String, CoreWatchRequest> _preferenceRequests = {};
  final Map<String, StreamController<CoreEvent>> _preferenceControllers = {};
  List<core.Workspace> _workspaces = <core.Workspace>[];
  int _createdChatCount = 0;

  Map<String, Object?> get directoryPreferences => _preferences[_dockFile]!;

  void updateHistories(List<core.ChatHistoryListItem> histories) {
    final workspaces = <String, core.Workspace>{};
    for (final history in histories) {
      final id = history.workspaceId;
      final name = history.workspaceName;
      if (id == null || name == null) continue;
      workspaces[id] = core.Workspace(
        id: id,
        name: name,
        folders: const <core.WorkspaceFolder>[],
        createdAt: 0,
        updatedAt: 0,
      );
    }
    _workspaces = workspaces.values.toList(growable: false);
  }

  Map<String, Object?> _preferencesFor(String fileName) {
    final values = _preferences[fileName];
    if (values == null) {
      throw StateError('Unexpected preference file: $fileName');
    }
    return values;
  }

  CoreEvent _event(CoreWatchRequest request, Object? value) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: 'Snapshot',
    valueBytes: encodeCoreLink(value),
    decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
  );

  void _emitPreferences(String fileName) {
    final request = _preferenceRequests[fileName];
    if (request == null) return;
    _preferenceControllers[fileName]?.add(
      _event(request, _preferencesFor(fileName)),
    );
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map;
    switch (request.methodName) {
      case 'getPreferences':
        final values = _preferencesFor(args['fileName'] as String);
        return encodeCoreLink(<Object?>[
          0,
          <String, Object?>{
            for (final key in (args['keys'] as List).cast<String>())
              if (values.containsKey(key)) key: values[key],
          },
        ]);
      case 'getPreference':
        return encodeCoreLink(<Object?>[
          0,
          _preferencesFor(args['fileName'] as String)[args['key']],
        ]);
      case 'setPreference':
        final fileName = args['fileName'] as String;
        _preferencesFor(fileName)[args['key'] as String] = args['value'];
        _emitPreferences(fileName);
        return encodeCoreLink(<Object?>[0, null]);
      case 'setPreferences':
        final fileName = args['fileName'] as String;
        _preferencesFor(
          fileName,
        ).addAll((args['values'] as Map).cast<String, Object?>());
        _emitPreferences(fileName);
        return encodeCoreLink(<Object?>[0, null]);
      case 'removePreference':
        final fileName = args['fileName'] as String;
        _preferencesFor(fileName).remove(args['key']);
        _emitPreferences(fileName);
        return encodeCoreLink(<Object?>[0, null]);
      case 'removePreferences':
        final fileName = args['fileName'] as String;
        for (final key in (args['keys'] as List).cast<String>()) {
          _preferencesFor(fileName).remove(key);
        }
        _emitPreferences(fileName);
        return encodeCoreLink(<Object?>[0, null]);
      case 'listWorkspaces':
        return encodeCoreLink(<Object?>[
          0,
          _workspaces.map((workspace) => workspace.toJson()).toList(),
        ]);
      case 'getToolPkgNavigationEntries' || 'getToolPkgUiRoutes':
        return encodeCoreLink(<Object?>[0, <Object?>[]]);
      case 'createNewChat':
        return encodeCoreLink(<Object?>[
          0,
          'created-chat-${_createdChatCount++}',
        ]);
      case 'createAndBindWorkspace':
        return encodeCoreLink(<Object?>[0, 'created-workspace']);
      case 'updateChatOrderAndGroup' ||
          'updateChatOrder' ||
          'updateChatGroups' ||
          'updateChatPinned' ||
          'updateChatLocked' ||
          'updateChatTitle' ||
          'switchChat' ||
          'moveChatToWorkspace' ||
          'bindChatToWorkspace' ||
          'unbindChatFromWorkspace':
        return encodeCoreLink(<Object?>[0, null]);
      case 'deleteWorkspace':
        if (!workspaceDeleteAllowed) {
          throw StateError('Workspace cleanup failed');
        }
        return encodeCoreLink(<Object?>[0, null]);
      case 'deleteChatHistory':
        return encodeCoreLink(<Object?>[0, deleteAllowed]);
    }
    throw StateError('Unexpected Core call: ${request.methodName}');
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName == 'allWorkspacesFlow') {
      return Stream<CoreEvent>.value(
        _event(
          request,
          _workspaces.map((workspace) => workspace.toJson()).toList(),
        ),
      );
    }
    if (request.propertyName != 'preferencesFlow') {
      throw StateError('Unexpected Core watch: ${request.propertyName}');
    }
    final fileName = (request.args as Map)['fileName'] as String;
    _preferenceRequests[fileName] = request;
    final controller = _preferenceControllers.putIfAbsent(
      fileName,
      () => StreamController<CoreEvent>.broadcast(),
    );
    return (() async* {
      yield _event(request, _preferencesFor(fileName));
      yield* controller.stream;
    })();
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
