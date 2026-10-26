import 'support/native_sidebar_bridge.dart';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/AdaptiveMainLayout.dart';
import 'package:operit2/ui/main/layout/NavigationLayoutMetrics.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/layout/TabletLayout.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Tests cached main pages through the real navigation and transition host.
void main() {
  for (final initialWidth in <double>[420, 900]) {
    testWidgets(
      'page cache survives repeated layout changes from $initialWidth',
      (tester) async {
        tester.view.physicalSize = Size(initialWidth, 900);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final topBar = TopBarController();
        final layout = MainLayoutController();
        final router = AppRouterState(
          RouteEntry(instanceId: 'settings-fixture', routeId: 'Settings'),
        );
        final drawerOpen = ValueNotifier<bool>(false);
        final conversations = ValueNotifier<DrawerConversationState>(
          const DrawerConversationState(loading: false),
        );
        addTearDown(topBar.dispose);
        addTearDown(layout.dispose);
        addTearDown(router.dispose);
        addTearDown(drawerOpen.dispose);
        addTearDown(conversations.dispose);
        final settingsCounts = _PageCounts();
        final chatCounts = _PageCounts();
        const settingsKey = ValueKey<String>('resize-settings');
        const chatKey = ValueKey<String>('resize-chat');
        final settings = _FixtureScreen(
          'Settings',
          _PageProbe(key: settingsKey, counts: settingsCounts),
        );
        final chat = _FixtureScreen(
          'AiChat',
          _PageProbe(key: chatKey, counts: chatCounts),
        );

        Future<void> pumpScreen(OperitScreen screen, double width) async {
          tester.view.physicalSize = Size(width, 900);
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot:
                  UserPreferencesManager.defaultThemePreferenceSnapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: MainLayoutScope(
                controller: layout,
                child: TopBarScope(
                  controller: topBar,
                  child: Scaffold(
                    body: Builder(
                      builder: (context) => AdaptiveMainLayout(
                        bridge: NativeSidebarBridge(),
                        navigationEntries: const <NavigationEntrySpec>[
                          NavigationEntrySpec(
                            entryId: 'main.ai_chat',
                            routeId: 'AiChat',
                            surface: NavigationSurface.mainSidebarAi,
                            title: 'Chat',
                            icon: Icons.chat_bubble_outline,
                          ),
                          NavigationEntrySpec(
                            entryId: 'main.package_manager',
                            routeId: 'PackageManager',
                            surface: NavigationSurface.mainSidebarSystem,
                            title: 'Packages',
                            icon: Icons.inventory_2_outlined,
                          ),
                          NavigationEntrySpec(
                            entryId: 'main.settings',
                            routeId: 'Settings',
                            surface: NavigationSurface.mainSidebarSystem,
                            title: 'Settings',
                            icon: Icons.settings_outlined,
                          ),
                        ],
                        pluginSidebarEntries: const [],
                        selectedRouteId: router.currentEntry.routeId,
                        drawerConversationState: conversations,
                        isTabletSidebarExpanded: false,
                        drawerOpenState: drawerOpen,
                        enableNavigationAnimation: true,
                        onOpenDrawer: () => drawerOpen.value = true,
                        onCloseDrawer: () => drawerOpen.value = false,
                        onNavigationEntrySelected: (_) {},
                        onConversationActivated: () {},
                        content: AppContent(
                          routerState: router,
                          currentScreen: screen,
                          currentRouteEntry: router.currentEntry,
                          currentRouteTitle: screen.routeTypeName,
                          useTabletLayout: useTabletLayoutForContext(context),
                          isTabletSidebarExpanded: false,
                          canGoBack: false,
                          enableNavigationAnimation: true,
                          isNavigatingBack: false,
                          topBarController: topBar,
                          appBarEntries: const [],
                          onGoBack: () {},
                          onNavigationButtonPressed: () {},
                          onAppBarEntrySelected: (_) {},
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

        await pumpScreen(settings, initialWidth);
        final hostState = tester.state(find.byType(AppContent));
        final settingsFinder = find.byKey(settingsKey, skipOffstage: false);
        final settingsState = tester.state<_PageProbeState>(settingsFinder);
        settingsState.controller.text = 'settings draft';
        router.navigate(routeId: 'AiChat');
        await pumpScreen(chat, initialWidth);
        final chatFinder = find.byKey(chatKey, skipOffstage: false);
        final chatState = tester.state<_PageProbeState>(chatFinder);
        chatState.controller.value = const TextEditingValue(
          text: 'unsent chat draft',
          selection: TextSelection(baseOffset: 2, extentOffset: 8),
        );
        chatState.scroll.jumpTo(240);
        await tester.pump();

        for (var cycle = 0; cycle < 3; cycle++) {
          for (final width in <double>[599, 600, 900, 420]) {
            await pumpScreen(chat, width);
            expect(
              find.byType(width < 600 ? PhoneLayout : TabletLayout),
              findsOneWidget,
            );
            expect(tester.state(find.byType(AppContent)), same(hostState));
            expect(tester.state(chatFinder), same(chatState));
            expect(tester.state(settingsFinder), same(settingsState));
            expect(chatState.controller.text, 'unsent chat draft');
            expect(
              chatState.controller.selection,
              const TextSelection(baseOffset: 2, extentOffset: 8),
            );
            expect(chatState.scroll.offset, 240);
            expect(chatCounts.disposals, 0);
            expect(settingsCounts.disposals, 0);
            expect(tester.takeException(), isNull);
          }
        }
        router.navigate(routeId: 'Settings');
        await pumpScreen(settings, 900);
        expect(tester.state(settingsFinder), same(settingsState));
        expect(settingsState.controller.text, 'settings draft');
        await tester.pumpWidget(const SizedBox.shrink());
        expect(chatCounts.disposals, 1);
        expect(settingsCounts.disposals, 1);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('cached settings do no UI work through four keyboard cycles', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(420, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final topBar = TopBarController();
    final layout = MainLayoutController();
    final entry = RouteEntry(
      instanceId: 'settings-fixture',
      routeId: 'Settings',
    );
    final router = AppRouterState(entry);
    addTearDown(topBar.dispose);
    addTearDown(layout.dispose);
    addTearDown(router.dispose);
    final settingsKey = GlobalKey<_PageProbeState>();
    final chatKey = GlobalKey<_PageProbeState>();
    final settingsCounts = _PageCounts();
    final chatCounts = _PageCounts();
    final settings = _FixtureScreen(
      'Settings',
      _PageProbe(key: settingsKey, counts: settingsCounts),
    );
    final chat = _FixtureScreen(
      'AiChat',
      _PageProbe(key: chatKey, counts: chatCounts),
    );

    /// Pumps the real cached page host with a chosen keyboard viewport.
    Future<void> pumpScreen(OperitScreen screen, double inset) async {
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: MainLayoutScope(
            controller: layout,
            child: TopBarScope(
              controller: topBar,
              child: Scaffold(
                body: Builder(
                  builder: (context) => MediaQuery(
                    data: MediaQuery.of(
                      context,
                    ).copyWith(viewInsets: EdgeInsets.only(bottom: inset)),
                    child: SizedBox(
                      height: 760 - inset,
                      child: AppContent(
                        routerState: router,
                        currentScreen: screen,
                        currentRouteEntry: router.currentEntry,
                        currentRouteTitle: screen.routeTypeName,
                        useTabletLayout: false,
                        isTabletSidebarExpanded: false,
                        canGoBack: false,
                        enableNavigationAnimation: true,
                        isNavigatingBack: false,
                        topBarController: topBar,
                        appBarEntries: const <NavigationEntrySpec>[],
                        onGoBack: () {},
                        onNavigationButtonPressed: () {},
                        onAppBarEntrySelected: (_) {},
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
    }

    await pumpScreen(settings, 0);
    await tester.pumpAndSettle();
    final state = settingsKey.currentState!;
    state.controller.text = 'keep my settings draft';
    router.navigate(routeId: 'AiChat');
    await pumpScreen(chat, 0);
    await tester.pump(const Duration(milliseconds: 350));
    await tester.pumpAndSettle();
    final before = settingsCounts.snapshot;
    expect(settingsKey.currentState, same(state));
    expect(settingsCounts.disposals, 0);
    for (var cycle = 0; cycle < 4; cycle += 1) {
      for (final inset in <double>[40, 110, 210, 300, 210, 110, 40, 0]) {
        await pumpScreen(chat, inset);
        await tester.pump(const Duration(milliseconds: 16));
        expect(settingsCounts.snapshot, before);
        expect(settingsKey.currentState, same(state));
        expect(tester.takeException(), isNull);
      }
    }
    router.navigate(routeId: 'Settings');
    await pumpScreen(settings, 120);
    await tester.pump(const Duration(milliseconds: 350));
    await tester.pumpAndSettle();
    expect(settingsKey.currentState, same(state));
    expect(state.controller.text, 'keep my settings draft');
    expect(state.lastInset, 120);
    expect(settingsCounts.layouts, greaterThan(before[1]));
    expect(settingsCounts.disposals, 0);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(settingsCounts.disposals, 1);
    expect(tester.takeException(), isNull);
  });
}

class _FixtureScreen extends OperitScreen {
  /// Creates a keep-alive route with a deterministic widget fixture.
  const _FixtureScreen(String name, this.content)
    : super(routeTypeName: name, keepAlive: true);
  final Widget content;

  /// Returns the stable route identity used by the production page cache.
  @override
  String stableScreenKey() => routeTypeName;

  /// Mounts the same page widget when its route is reactivated.
  @override
  Widget build(BuildContext context) => content;
}

class _PageCounts {
  int builds = 0;
  int layouts = 0;
  int paints = 0;
  int disposals = 0;

  /// Copies the page work counters across keyboard animation frames.
  List<int> get snapshot => <int>[builds, layouts, paints];
}

class _PageProbe extends StatefulWidget {
  /// Creates a page with draft state and a constraint-dependent subtree.
  const _PageProbe({super.key, required this.counts});
  final _PageCounts counts;

  /// Creates the state retained by the main page cache.
  @override
  State<_PageProbe> createState() => _PageProbeState();
}

class _PageProbeState extends State<_PageProbe> {
  final controller = TextEditingController();
  final scroll = ScrollController();
  double lastInset = 0;

  /// Exercises both inherited and layout-driven rebuild requests.
  @override
  Widget build(BuildContext context) {
    widget.counts.builds += 1;
    lastInset = MediaQuery.viewInsetsOf(context).bottom;
    return LayoutBuilder(
      builder: (context, constraints) {
        widget.counts.builds += 1;
        return _PageRenderProbe(
          counts: widget.counts,
          child: Column(
            children: <Widget>[
              Expanded(
                child: ListView.builder(
                  controller: scroll,
                  itemCount: 100,
                  itemExtent: 40,
                  itemBuilder: (context, index) => Text('Message $index'),
                ),
              ),
              TextField(controller: controller),
            ],
          ),
        );
      },
    );
  }

  /// Releases state only when the retained main host is removed.
  @override
  void dispose() {
    widget.counts.disposals += 1;
    controller.dispose();
    scroll.dispose();
    super.dispose();
  }
}

class _PageRenderProbe extends SingleChildRenderObjectWidget {
  /// Creates an observable page render root.
  const _PageRenderProbe({required this.counts, required super.child});
  final _PageCounts counts;

  /// Creates the layout and paint counters beneath the cached page boundary.
  @override
  _PageRenderObject createRenderObject(BuildContext context) =>
      _PageRenderObject(counts);
}

class _PageRenderObject extends RenderProxyBox {
  /// Records actual layout and paint work in the page subtree.
  _PageRenderObject(this.counts);
  final _PageCounts counts;

  /// Counts layouts caused by viewport constraint changes.
  @override
  void performLayout() {
    counts.layouts += 1;
    super.performLayout();
  }

  /// Counts paints caused by page or ancestor invalidations.
  @override
  void paint(PaintingContext context, Offset offset) {
    counts.paints += 1;
    super.paint(context, offset);
  }
}
