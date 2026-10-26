import 'dart:typed_data';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
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
    'child conversations stay collapsed while running and search reveals their ancestors',
    (tester) async {
      final bridge = _DrawerBridge();
      final histories = [
        _history('A', 0),
        _history('A', 1, parentChatId: 'A-0'),
        _history('A', 2, parentChatId: 'A-1'),
      ];
      await _pumpDrawer(tester, bridge, histories, streamingIds: {'A-1'});
      expect(_visibleIds(tester), ['A-0']);
      await tester.tap(find.byKey(const ValueKey('workspace-children:A-0')));
      await _pumpSidebar(tester);
      expect(_visibleIds(tester), ['A-0', 'A-1']);
      final parent = tester.getTopLeft(
        find.byKey(const ValueKey('workspace-chat:A-0')),
      );
      final child = tester.getTopLeft(
        find.byKey(const ValueKey('workspace-chat:A-1')),
      );
      expect(child.dx - parent.dx, 16);
      await tester.tap(find.byKey(const ValueKey('workspace-children:A-0')));
      await _pumpSidebar(tester);
      await _pumpDrawer(
        tester,
        bridge,
        histories,
        streamingIds: {'A-1', 'A-2'},
      );
      expect(_visibleIds(tester), ['A-0']);
      await tester.tap(find.byTooltip('搜索对话'));
      await _pumpSidebar(tester);
      await tester.enterText(find.byType(TextField), 'A-2');
      await _pumpSidebar(tester);
      expect(_visibleIds(tester), ['A-0', 'A-1', 'A-2']);
      await tester.enterText(find.byType(TextField), '');
      await _pumpSidebar(tester);
      expect(_visibleIds(tester), ['A-0']);
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

  testWidgets('brand header fits the narrow drawer beside the tab switch', (
    tester,
  ) async {
    for (final scale in [1.0, 1.4]) {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MediaQuery(
              data: MediaQueryData(textScaler: TextScaler.linear(scale)),
              child: Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 195,
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
                    trailing: const SizedBox(
                      key: ValueKey('test-tab-switch'),
                      width: 98,
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
      expect(
        tester.getSize(find.byKey(const ValueKey('test-tab-switch'))).width,
        98,
      );
      expect(tester.getSize(find.text('Operit')).width, lessThanOrEqualTo(63));
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
    'workspace headers restore the legacy accent marker and row style',
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
        hasLength(1),
      );
      for (final item in tester.widgetList<ConversationDrawerItem>(
        find.byType(ConversationDrawerItem),
      )) {
        expect(item.workspaceStyle, isTrue);
        expect(item.nested, isTrue);
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
    'workspace group controls are native and folding performs no backend mutations',
    (tester) async {
      final bridge = _DrawerBridge();
      await _pumpDrawer(tester, bridge, _histories('A', 6));
      expect(find.text('A'), findsOneWidget);
      expect(find.byTooltip('分组操作'), findsOneWidget);
      expect(find.text('未分组'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('workspace-create-group')),
        findsOneWidget,
      );
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
      expect(find.text('未绑定工作区'), findsOneWidget);
      expect(find.text('工作区缺失'), findsNWidgets(2));
      expect(_visibleIds(tester), [
        'pending-a-0',
        'pending-b-0',
        'unbound-a-0',
        'unbound-b-0',
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
      expect(find.text('未绑定工作区'), findsOneWidget);
      expect(find.text('工作区缺失'), findsNothing);
      expect(find.text('Project A'), findsOneWidget);
      expect(find.text('Project B'), findsOneWidget);
      expect(_visibleIds(tester), [
        'pending-a-0',
        'pending-b-0',
        'unbound-a-0',
        'unbound-b-0',
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
      expect(find.text('工作区缺失'), findsOneWidget);
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
  Set<String> streamingIds = const <String>{},
}) async {
  tester.view.physicalSize = const Size(500, 2400);
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
}) => core.ChatHistoryListItem(
  parentChatId: parentChatId,
  group: null,
  id: '$group-$index',
  title: '$group-$index',
  updatedAt: '2026-10-06T00:00:00Z',

  displayOrder: index,
  workspaceId: group,
  workspaceName: group,

  locked: false,
  pinned: pinned,
);

class _DrawerBridge extends OperitRuntimeBridge {
  /// Creates a workspace sidebar bridge without a selected plugin tab.
  _DrawerBridge({this.deleteAllowed = true});

  final bool deleteAllowed;

  /// Exposes the committed preference snapshot used by reads and watches.
  Map<String, String> get preferences => <String, String>{};
  final List<CoreCallRequest> calls = <CoreCallRequest>[];

  /// Implements only preference reads and the group mutations exercised here.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final value = switch (request.methodName) {
      'getPreferences' => preferences,
      'getToolPkgNavigationEntries' || 'getToolPkgUiRoutes' => <Object?>[],
      'updateChatOrderAndGroup' ||
      'updateChatPinned' ||
      'switchChat' ||
      'unbindChatFromWorkspace' => null,
      'deleteChatHistory' => deleteAllowed,
      _ => throw StateError('Unexpected Core call: ${request.methodName}'),
    };
    return encodeCoreLink(<Object?>[0, value]);
  }

  /// Emits the initial preference snapshot required by the grouping observer.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'preferencesFlow') {
      throw StateError('Unexpected Core watch: ${request.propertyName}');
    }
    return Stream<CoreEvent>.value(
      CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(preferences),
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      ),
    );
  }

  /// Rejects pushes outside the sidebar test contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshot reads outside the sidebar test contract.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
