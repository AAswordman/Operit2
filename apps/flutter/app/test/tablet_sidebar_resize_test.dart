import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/l10n/generated/app_localizations_en.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/NavigationLayoutMetrics.dart';
import 'package:operit2/ui/main/layout/TabletLayout.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/native_sidebar_bridge.dart';

const _panelKey = ValueKey('desktop-conversation-panel');
const _handleKey = ValueKey('desktop-conversation-resize-handle');
const _contentKey = ValueKey('resize-main-content');

void main() {
  for (final zoom in <double>[1, 1.25]) {
    testWidgets('mouse resize follows local movement at zoom $zoom', (
      tester,
    ) async {
      final controller = TextEditingController(text: 'Unsent draft');
      addTearDown(controller.dispose);
      await _pumpLayout(
        tester,
        zoom: zoom,
        content: TextField(controller: controller),
      );
      controller.selection = const TextSelection(
        baseOffset: 1,
        extentOffset: 6,
      );
      final draftState = tester.state(find.byType(TextField));
      final drawerState = tester.state(find.byType(DrawerContent));
      final gesture = await tester.startGesture(
        tester.getCenter(find.byKey(_handleKey)),
        kind: PointerDeviceKind.mouse,
      );
      await gesture.moveBy(Offset(40 * zoom, 0));
      await tester.pump();

      expect(_panelWidth(tester), closeTo(320, 0.001));
      expect(tester.state(find.byType(TextField)), same(draftState));
      expect(tester.state(find.byType(DrawerContent)), same(drawerState));
      expect(controller.text, 'Unsent draft');
      expect(
        controller.selection,
        const TextSelection(baseOffset: 1, extentOffset: 6),
      );
      expect(tester.takeException(), isNull);
      await gesture.up();
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('drag limits protect both panes and allow immediate reversal', (
    tester,
  ) async {
    await _pumpLayout(tester);
    final gesture = await tester.startGesture(
      tester.getCenter(find.byKey(_handleKey)),
      kind: PointerDeviceKind.mouse,
    );
    await gesture.moveBy(const Offset(-1000, 0));
    await tester.pump();
    expect(_panelWidth(tester), navigationSidebarMinWidth);

    await gesture.moveBy(const Offset(30, 0));
    await tester.pump();
    expect(_panelWidth(tester), navigationSidebarMinWidth + 30);

    await gesture.moveBy(const Offset(2000, 0));
    await tester.pump();
    expect(_panelWidth(tester), navigationSidebarMaxWidth);
    expect(
      tester.getSize(find.byKey(_contentKey)).width,
      greaterThanOrEqualTo(navigationMinContentWidth),
    );

    await gesture.moveBy(const Offset(-30, 0));
    await tester.pump();
    expect(_panelWidth(tester), navigationSidebarMaxWidth - 30);
    await gesture.cancel();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('chosen width survives collapse and route changes', (
    tester,
  ) async {
    final expanded = ValueNotifier(true);
    final selectedRoute = ValueNotifier(
      ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat),
    );
    addTearDown(expanded.dispose);
    addTearDown(selectedRoute.dispose);
    await _pumpLayout(tester, expanded: expanded, selectedRoute: selectedRoute);
    await tester.drag(find.byKey(_handleKey), const Offset(-40, 0));
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), 240);

    expanded.value = false;
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), 0);
    expect(find.byKey(_handleKey), findsNothing);

    expanded.value = true;
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), 240);

    selectedRoute.value = ScreenRouteRegistry.routeIdOf(
      ScreenRouteRegistry.settings,
    );
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), 0);
    expect(find.byKey(_handleKey), findsNothing);

    selectedRoute.value = ScreenRouteRegistry.routeIdOf(
      ScreenRouteRegistry.aiChat,
    );
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), 240);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('narrow windows clamp immediately without losing chosen width', (
    tester,
  ) async {
    await _pumpLayout(tester);
    await tester.drag(find.byKey(_handleKey), const Offset(200, 0));
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), navigationSidebarMaxWidth);

    tester.view.physicalSize = const Size(600, 800);
    for (var frame = 0; frame < 20; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      final panel = tester.getRect(find.byKey(_panelKey));
      final content = tester.getRect(find.byKey(_contentKey));
      expect(panel.width, greaterThanOrEqualTo(navigationSidebarMinWidth));
      expect(content.width, greaterThanOrEqualTo(navigationMinContentWidth));
      expect(panel.right, content.left);
      expect(content.right, lessThanOrEqualTo(600));
      expect(tester.takeException(), isNull);
    }

    tester.view.physicalSize = const Size(1200, 800);
    await tester.pumpAndSettle();
    expect(_panelWidth(tester), navigationSidebarMaxWidth);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('minimum width keeps enlarged text and toolbar actions inside', (
    tester,
  ) async {
    await _pumpLayout(tester, textScale: 1.4);
    await tester.drag(find.byKey(_handleKey), const Offset(-100, 0));
    await tester.pumpAndSettle();
    final panel = tester.getRect(find.byKey(_panelKey));
    expect(panel.width, navigationSidebarMinWidth);
    for (final control in <Finder>[
      find.byKey(const ValueKey('workspace-create-chat')),
      find.byTooltip('搜索对话'),
      find.byTooltip('收起会话列表'),
      find.byKey(const ValueKey('chat-sidebar-workspace-tab')),
    ]) {
      final rect = tester.getRect(control);
      expect(rect.left, greaterThanOrEqualTo(panel.left));
      expect(rect.right, lessThanOrEqualTo(panel.right));
      expect(control.hitTestable(), findsOneWidget);
    }
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

double _panelWidth(WidgetTester tester) =>
    tester.getSize(find.byKey(_panelKey)).width;

Future<void> _pumpLayout(
  WidgetTester tester, {
  double zoom = 1,
  double textScale = 1,
  Widget content = const SizedBox.expand(),
  ValueNotifier<bool>? expanded,
  ValueNotifier<String>? selectedRoute,
}) async {
  tester.view.physicalSize = const Size(1200, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final expansion = expanded ?? ValueNotifier(true);
  if (expanded == null) addTearDown(expansion.dispose);
  final conversations = ValueNotifier(
    const DrawerConversationState(
      loading: false,
      currentChatId: 'resize-chat',
      histories: [
        core.ChatHistoryListItem(
          id: 'resize-chat',
          title: 'A long conversation title that must stay inside the sidebar',
          updatedAt: '2026-11-10T00:00:00Z',
          displayOrder: 0,
          parentChatId: null,
          group: 'A long group name that must remain inside the sidebar',
          workspaceId: null,
          workspaceName: null,
          locked: true,
          pinned: true,
        ),
      ],
    ),
  );
  addTearDown(conversations.dispose);
  final bridge = NativeSidebarBridge();
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: MediaQuery(
            data: MediaQueryData(
              size: const Size(1200, 800),
              textScaler: TextScaler.linear(textScale),
            ),
            child: Transform.scale(
              alignment: Alignment.topLeft,
              scale: zoom,
              child: AnimatedBuilder(
                animation: Listenable.merge([expansion, ?selectedRoute]),
                builder: (context, _) => TabletLayout(
                  bridge: bridge,
                  content: SizedBox(key: _contentKey, child: content),
                  navigationEntries: ScreenRouteRegistry.mainSidebarEntries(
                    AppLocalizationsEn(),
                  ),
                  pluginSidebarEntries: const [],
                  selectedRouteId:
                      selectedRoute?.value ??
                      ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat),
                  drawerConversationState: conversations,
                  isTabletSidebarExpanded: expansion.value,
                  collapsedTabletSidebarWidth: 56,
                  onExpandSidebar: () => expansion.value = true,
                  onCollapseSidebar: () => expansion.value = false,
                  onNavigationEntrySelected: (_) {},
                  onConversationActivated: () {},
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}
