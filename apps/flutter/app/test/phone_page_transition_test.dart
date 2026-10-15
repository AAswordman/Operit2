import 'support/native_sidebar_bridge.dart';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/DrawerMotionScope.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies drawer snapshots do not hide page transitions or loading frames.
void main() {
  for (final effects in <bool>[true, false]) {
    testWidgets(
      'drawer motion snapshots only the established page (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        final pageState = tester.state(_page(0));
        final baselineBuilds = host.builds[0];
        final baselinePaints = host.paints[0];
        expect(
          find.ancestor(
            of: find.byType(AppContent),
            matching: find.byType(SnapshotWidget),
          ),
          findsNothing,
        );
        for (final open in <bool>[true, false]) {
          host.drawerOpen.value = open;
          await tester.pump();
          final snapshot = _snapshot(tester, 0);
          expect(snapshot.controller.allowSnapshotting, isTrue);
          expect(_boundary(tester, 0).debugLayer!.parent, isNull);
          for (var frame = 0; frame < 5; frame++) {
            await tester.pump(const Duration(milliseconds: 16));
            expect(snapshot.controller.allowSnapshotting, isTrue);
            expect(host.builds[0], baselineBuilds);
            expect(host.paints[0], baselinePaints);
          }
          await tester.pumpAndSettle();
          expect(snapshot.controller.allowSnapshotting, isFalse);
          expect(_boundary(tester, 0).debugLayer!.parent, isNotNull);
          expect(tester.state(_page(0)), same(pageState));
        }
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'outgoing image moves and fades while incoming loading stays live (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        host.drawerOpen.value = true;
        await tester.pumpAndSettle();
        tester
            .widget<DrawerContent>(find.byType(DrawerContent))
            .onNavigationEntrySelected(_nextEntry);
        await tester.pump();
        final outgoing = _snapshot(tester, 0);
        final incoming = _snapshot(tester, 1);
        expect(host.drawerOpen.value, isFalse);
        expect(outgoing.controller.allowSnapshotting, isTrue);
        expect(incoming.controller.allowSnapshotting, isFalse);
        expect(_boundary(tester, 0).debugLayer!.parent, isNull);
        expect(_boundary(tester, 1).debugLayer!.parent, isNotNull);
        final initialTranslation = _translation(tester, 1);
        final initialOpacity = _opacity(tester, 0);
        if (effects) {
          expect(initialTranslation, greaterThan(0));
        }
        await tester.pump(const Duration(milliseconds: 16));
        for (var frame = 0; frame < 5; frame++) {
          final previousPaints = host.paints[1];
          host.loadingTick.value++;
          await tester.pump(const Duration(milliseconds: 16));
          expect(incoming.controller.allowSnapshotting, isFalse);
          expect(_boundary(tester, 1).debugLayer!.parent, isNotNull);
          expect(host.paints[1], greaterThan(previousPaints));
          expect(host.lastPaintedTick[1], host.loadingTick.value);
        }
        expect(_opacity(tester, 0), lessThan(initialOpacity));
        if (effects) {
          expect(_translation(tester, 0), lessThan(0));
          expect(_translation(tester, 1), lessThan(initialTranslation));
        }
        await tester.pumpAndSettle();
        expect(incoming.controller.allowSnapshotting, isFalse);
        expect(host.initializations, <int>[1, 1, 0]);
        expect(find.text('Page 0'), findsNothing);
        expect(find.text('Page 1'), findsOneWidget);
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'navigation reuses an existing outgoing drawer snapshot (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        host.drawerOpen.value = true;
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 32));
        final outgoing = _snapshot(tester, 0);
        final outgoingBoundary = _boundary(tester, 0);
        final pageState = tester.state(_page(0));
        final paintCount = host.paints[0];
        tester
            .widget<DrawerContent>(find.byType(DrawerContent))
            .onNavigationEntrySelected(_nextEntry);
        await tester.pump();
        expect(_snapshot(tester, 0).controller, same(outgoing.controller));
        expect(outgoing.controller.allowSnapshotting, isTrue);
        expect(outgoingBoundary.debugLayer!.parent, isNull);
        expect(host.paints[0], paintCount);
        expect(tester.state(_page(0)), same(pageState));
        expect(_snapshot(tester, 1).controller.allowSnapshotting, isFalse);
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'rapid navigation and returning pages stay live during drawer motion (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        final original = tester.state(_page(0));
        host.drawerOpen.value = true;
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 32));
        host.drawerOpen.value = false;
        for (final index in <int>[1, 2, 0]) {
          host.selected.value = index;
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 32));
          final current = _snapshot(tester, index);
          expect(current.controller.allowSnapshotting, isFalse);
          expect(_boundary(tester, index).debugLayer!.parent, isNotNull);
          host.loadingTick.value++;
          await tester.pump(const Duration(milliseconds: 16));
          expect(host.lastPaintedTick[index], host.loadingTick.value);
        }
        expect(tester.state(_page(0)), same(original));
        expect(host.initializations, <int>[1, 1, 1]);
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'opening drawer cannot freeze an entering page (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        host.selected.value = 1;
        await tester.pump();
        host.drawerOpen.value = true;
        await tester.pump();
        final current = _snapshot(tester, 1);
        for (var frame = 0; frame < 20; frame++) {
          host.loadingTick.value++;
          await tester.pump(const Duration(milliseconds: 16));
          expect(current.controller.allowSnapshotting, isFalse);
          expect(_boundary(tester, 1).debugLayer!.parent, isNotNull);
          expect(host.lastPaintedTick[1], host.loadingTick.value);
        }
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'conversation activation keeps in-page loading live while closing (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        final original = tester.state(_page(0));
        host.drawerOpen.value = true;
        await tester.pumpAndSettle();
        tester
            .widget<DrawerContent>(find.byType(DrawerContent))
            .onConversationActivated();
        await tester.pump();
        final current = _snapshot(tester, 0);
        expect(host.drawerOpen.value, isFalse);
        expect(
          tester
              .widget<DrawerMotionScope>(find.byType(DrawerMotionScope))
              .isAnimating
              .value,
          isTrue,
        );
        for (var frame = 0; frame < 5; frame++) {
          host.loadingTick.value++;
          await tester.pump(const Duration(milliseconds: 16));
          expect(current.controller.allowSnapshotting, isFalse);
          expect(_boundary(tester, 0).debugLayer!.parent, isNotNull);
          expect(host.lastPaintedTick[0], host.loadingTick.value);
        }
        expect(tester.state(_page(0)), same(original));
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'page snapshots handle drawer reversal resize and disposal (effects: $effects)',
      (tester) async {
        final host = _PhonePageHarness();
        addTearDown(host.dispose);
        await host.pump(tester, effects: effects);
        await tester.pumpAndSettle();
        host.drawerOpen.value = true;
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 32));
        final snapshot = _snapshot(tester, 0);
        host.drawerOpen.value = false;
        await tester.pump(const Duration(milliseconds: 16));
        expect(snapshot.controller.allowSnapshotting, isTrue);
        tester.view.physicalSize = const Size(400, 600);
        await tester.pump(const Duration(milliseconds: 16));
        expect(tester.getSize(find.byType(PhoneLayout)), const Size(400, 600));
        expect(snapshot.controller.allowSnapshotting, isTrue);
        await tester.pumpAndSettle();
        expect(snapshot.controller.allowSnapshotting, isFalse);
        host.drawerOpen.value = true;
        await tester.pump();
        expect(snapshot.controller.allowSnapshotting, isTrue);
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      },
    );
  }
}

const _nextEntry = NavigationEntrySpec(
  entryId: 'next-page',
  routeId: 'route-1',
  surface: NavigationSurface.mainSidebarSystem,
  title: 'Next page',
  icon: Icons.settings,
);

/// Finds a page even while its cached slot is offstage.
Finder _page(int index) =>
    find.byKey(ValueKey<int>(index), skipOffstage: false);

/// Returns the only snapshot that owns the requested page's content.
SnapshotWidget _snapshot(WidgetTester tester, int index) =>
    tester.widget<SnapshotWidget>(
      find.ancestor(
        of: _page(index),
        matching: find.byType(SnapshotWidget, skipOffstage: false),
      ),
    );

/// Resolves the retained live layer inside a page-local snapshot.
RenderRepaintBoundary _boundary(WidgetTester tester, int index) =>
    tester.renderObject<RenderRepaintBoundary>(
      find.byWidget(_snapshot(tester, index).child!, skipOffstage: false),
    );

/// Reads page-local motion without including the drawer's outer transform.
double _translation(WidgetTester tester, int index) => tester
    .element(_page(index))
    .findAncestorWidgetOfExactType<Transform>()!
    .transform
    .storage[12];

/// Reads the outgoing page's animated opacity outside its snapshot.
double _opacity(WidgetTester tester, int index) => tester
    .element(_page(index))
    .findAncestorWidgetOfExactType<FadeTransition>()!
    .opacity
    .value;

class _PhonePageHarness {
  final selected = ValueNotifier<int>(0);
  final drawerOpen = ValueNotifier<bool>(false);
  final loadingTick = ValueNotifier<int>(0);
  final conversations = ValueNotifier<DrawerConversationState>(
    const DrawerConversationState(loading: false),
  );
  final layout = MainLayoutController();
  final topBar = TopBarController();
  final entries = List<RouteEntry>.generate(
    3,
    (index) => RouteEntry(instanceId: 'entry-$index', routeId: 'route-$index'),
  );
  late final router = AppRouterState(entries.first);
  final initializations = List<int>.filled(3, 0);
  final builds = List<int>.filled(3, 0);
  final paints = List<int>.filled(3, 0);
  final lastPaintedTick = List<int>.filled(3, -1);

  /// Installs the real phone drawer and page-transition hosts around probes.
  Future<void> pump(WidgetTester tester, {required bool effects}) async {
    tester.view.physicalSize = const Size(400, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final screens = List<_ProbeRoute>.generate(
      3,
      (index) => _ProbeRoute(index, this),
    );
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: MainLayoutScope(
          controller: layout,
          child: ValueListenableBuilder<int>(
            valueListenable: selected,
            builder: (context, index, _) => Scaffold(
              body: PhoneLayout(
                bridge: NativeSidebarBridge(),
                content: AppContent(
                  routerState: router,
                  currentScreen: screens[index],
                  currentRouteEntry: entries[index],
                  currentRouteTitle: 'Route $index',
                  useTabletLayout: false,
                  isTabletSidebarExpanded: false,
                  canGoBack: false,
                  enableNavigationAnimation: effects,
                  isNavigatingBack: false,
                  topBarController: topBar,
                  appBarEntries: const [],
                  onGoBack: () {},
                  onNavigationButtonPressed: () => drawerOpen.value = true,
                  onAppBarEntrySelected: (_) {},
                ),
                navigationEntries: const [],
                pluginSidebarEntries: const [],
                selectedRouteId: entries[index].routeId,
                drawerConversationState: conversations,
                drawerWidth: 300,
                drawerOpenState: drawerOpen,
                enableNavigationAnimation: effects,
                onOpenDrawer: () => drawerOpen.value = true,
                onCloseDrawer: () => drawerOpen.value = false,
                onNavigationEntrySelected: (_) {
                  drawerOpen.value = false;
                  selected.value = 1;
                },
                onConversationActivated: () => drawerOpen.value = false,
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// Releases notifier and host resources after each regression scenario.
  void dispose() {
    selected.dispose();
    drawerOpen.dispose();
    loadingTick.dispose();
    conversations.dispose();
    layout.dispose();
    topBar.dispose();
    router.dispose();
  }
}

class _ProbeRoute extends OperitScreen {
  /// Creates a retained route with observable live loading content.
  const _ProbeRoute(this.index, this.host)
    : super(routeTypeName: 'Probe', keepAlive: true);

  final int index;
  final _PhonePageHarness host;

  /// Identifies the page independently of a drawer motion or route entry.
  @override
  String stableScreenKey() => 'page-$index';

  /// Builds the stateful loading probe retained by the main page host.
  @override
  Widget build(BuildContext context) =>
      _LoadingPage(key: ValueKey<int>(index), index: index, host: host);
}

class _LoadingPage extends StatefulWidget {
  /// Creates a lightweight loading page with measurable lifecycle events.
  const _LoadingPage({super.key, required this.index, required this.host});

  final int index;
  final _PhonePageHarness host;

  /// Creates state that must survive drawer movement and cached navigation.
  @override
  State<_LoadingPage> createState() => _LoadingPageState();
}

class _LoadingPageState extends State<_LoadingPage> {
  /// Records initialization to detect page remounts during transitions.
  @override
  void initState() {
    super.initState();
    widget.host.initializations[widget.index]++;
  }

  /// Renders a repaint-driven loading surface without rebuilding each frame.
  @override
  Widget build(BuildContext context) {
    widget.host.builds[widget.index]++;
    return CustomPaint(
      painter: _LoadingPainter(widget.index, widget.host),
      child: Center(child: Text('Page ${widget.index}')),
    );
  }
}

class _LoadingPainter extends CustomPainter {
  /// Subscribes loading paints directly to the test's frame clock.
  _LoadingPainter(this.index, this.host) : super(repaint: host.loadingTick);

  final int index;
  final _PhonePageHarness host;

  /// Records and draws each loading frame to verify live scene updates.
  @override
  void paint(Canvas canvas, Size size) {
    host.paints[index]++;
    host.lastPaintedTick[index] = host.loadingTick.value;
    canvas.drawRect(
      Offset.zero & size,
      Paint()
        ..color = host.loadingTick.value.isEven ? Colors.white : Colors.blue,
    );
  }

  /// Retains the static delegate while its repaint source drives loading frames.
  @override
  bool shouldRepaint(covariant _LoadingPainter oldDelegate) => false;
}
