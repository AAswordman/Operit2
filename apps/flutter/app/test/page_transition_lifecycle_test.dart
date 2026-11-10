import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/components/AnimatedLazyIndexedStack.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/test_platform_view.dart';

/// Verifies page transitions preserve cached state and isolate animation work.
void main() {
  testWidgets('native WebView layer survives tab exits and reactivation', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<int>(
            valueListenable: selected,
            builder: (context, index, child) => AnimatedLazyIndexedStack(
              index: index,
              itemCount: 2,
              itemBuilder: (context, page) => page == 0
                  ? const TestPlatformView()
                  : const ColoredBox(color: Colors.white),
            ),
          ),
        ),
      ),
    );
    expect(tester.layers.whereType<PlatformViewLayer>(), hasLength(1));
    selected.value = 1;
    await tester.pump();
    for (var frame = 0; frame < 5; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      expect(tester.layers.whereType<PlatformViewLayer>(), hasLength(1));
    }
    await tester.pumpAndSettle();
    expect(tester.layers.whereType<PlatformViewLayer>(), isEmpty);
    selected.value = 0;
    await tester.pumpAndSettle();
    expect(tester.layers.whereType<PlatformViewLayer>(), hasLength(1));
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets('cached main pages remain mounted when hidden and restored', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    final counts = _PageCounts();
    await _pumpMainPages(tester, selected, counts);
    await tester.pumpAndSettle();
    final original = tester.state(_probe(0));
    await tester.tap(find.text('Page 0: 0'));
    await tester.pump();

    selected.value = 1;
    await tester.pumpAndSettle();
    expect(counts.initialized[0], 1);
    expect(counts.disposed[0], 0);
    expect(tester.state(_probe(0)), same(original));
    expect(TickerMode.valuesOf(tester.element(_probe(0))).enabled, isFalse);

    selected.value = 0;
    await tester.pumpAndSettle();
    expect(tester.state(_probe(0)), same(original));
    expect(find.text('Page 0: 1'), findsOneWidget);
    expect(counts.initialized, <int>[1, 1, 0]);
    expect(TickerMode.valuesOf(tester.element(_probe(0))).enabled, isTrue);
  });

  testWidgets('an earlier transition cannot hide the next outgoing page', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    await _pumpMainPages(tester, selected, _PageCounts());
    await tester.pumpAndSettle();

    selected.value = 1;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 120));
    selected.value = 2;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 170));
    expect(find.text('Page 1: 0'), findsOneWidget);

    await tester.pumpAndSettle();
    expect(find.text('Page 1: 0'), findsNothing);
    expect(find.text('Page 2: 0'), findsOneWidget);
  });

  testWidgets('non-retained main pages are disposed after their exit', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    final counts = _PageCounts();
    await _pumpMainPages(tester, selected, counts, keepFirstPage: false);
    await tester.pumpAndSettle();
    selected.value = 1;
    await tester.pumpAndSettle();
    expect(_probe(0), findsNothing);
    expect(counts.initialized[0], 1);
    expect(counts.disposed[0], 1);
  });

  testWidgets('tab pages survive outgoing, hidden and current roles', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    final counts = _PageCounts();
    await _pumpTabs(tester, selected, counts);
    final original = tester.state(_probe(0));
    await tester.tap(find.text('Page 0: 0'));
    await tester.pump();

    selected.value = 1;
    await tester.pump();
    expect(tester.state(_probe(0)), same(original));
    await tester.pumpAndSettle();
    expect(tester.state(_probe(0)), same(original));
    expect(TickerMode.valuesOf(tester.element(_probe(0))).enabled, isFalse);

    selected.value = 0;
    await tester.pumpAndSettle();
    expect(tester.state(_probe(0)), same(original));
    expect(find.text('Page 0: 1'), findsOneWidget);
    expect(counts.initialized, <int>[1, 1, 0]);
    expect(counts.disposed, <int>[0, 0, 0]);
    expect(TickerMode.valuesOf(tester.element(_probe(0))).enabled, isTrue);
  });

  testWidgets('rapid tab switches keep each visited page instance', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    final counts = _PageCounts();
    await _pumpTabs(tester, selected, counts);
    for (final index in <int>[1, 2, 0, 2]) {
      selected.value = index;
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 40));
    }
    await tester.pumpAndSettle();
    expect(counts.initialized, <int>[1, 1, 1]);
    expect(counts.disposed, <int>[0, 0, 0]);
    expect(find.text('Page 2: 0'), findsOneWidget);
    expect(find.text('Page 0: 0'), findsNothing);
    expect(find.text('Page 1: 0'), findsNothing);
    for (final snapshot in tester.widgetList<SnapshotWidget>(
      find.byType(SnapshotWidget, skipOffstage: false),
    )) {
      expect(snapshot.controller.allowSnapshotting, isFalse);
    }
  });

  testWidgets('tab animation frames do not rebuild their page contents', (
    tester,
  ) async {
    final selected = ValueNotifier<int>(0);
    addTearDown(selected.dispose);
    final counts = _PageCounts();
    await _pumpTabs(tester, selected, counts);
    selected.value = 1;
    await tester.pump();
    final builds = List<int>.of(counts.built);
    for (var frame = 0; frame < 8; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
    }
    expect(counts.built, builds);
    await tester.pumpAndSettle();
  });

  for (final mainPages in <bool>[true, false]) {
    for (final nextPage in <int>[0, 2]) {
      testWidgets(
        'interrupted motion preserves visible positions (main: $mainPages, next: $nextPage)',
        (tester) async {
          final selected = ValueNotifier<int>(0);
          addTearDown(selected.dispose);
          final counts = _PageCounts();
          if (mainPages) {
            await _pumpMainPages(tester, selected, counts);
          } else {
            await _pumpTabs(tester, selected, counts);
          }
          await tester.pumpAndSettle();
          selected.value = 1;
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 16));
          await tester.pump(const Duration(milliseconds: 40));
          final currentPosition = tester.getTopLeft(_probe(1));
          final previousPosition = tester.getTopLeft(_probe(0));

          selected.value = nextPage;
          await tester.pump();
          expect(tester.getTopLeft(_probe(1)), currentPosition);
          if (nextPage == 0) {
            expect(tester.getTopLeft(_probe(0)), previousPosition);
          }
          await tester.pumpAndSettle();
          expect(tester.getTopLeft(_probe(nextPage)).dx, 0);
          expect(find.text('Page $nextPage: 0'), findsOneWidget);
          expect(counts.initialized[0], 1);
          expect(counts.initialized[1], 1);
        },
      );
    }

    testWidgets(
      'page motion reuses incoming page paint (main pages: $mainPages)',
      (tester) async {
        final selected = ValueNotifier<int>(0);
        addTearDown(selected.dispose);
        final counts = _PageCounts();
        if (mainPages) {
          await _pumpMainPages(tester, selected, counts);
        } else {
          await _pumpTabs(tester, selected, counts);
        }
        await tester.pumpAndSettle();
        selected.value = 1;
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 16));
        final paints = counts.painted[1];
        expect(paints, greaterThan(0));
        for (var frame = 0; frame < 8; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
        }
        expect(counts.painted[1], paints);
        await tester.pumpAndSettle();
      },
    );
  }
}

/// Finds a page probe even when its retained page is offstage.
Finder _probe(int index) =>
    find.byKey(ValueKey<String>('probe-$index'), skipOffstage: false);

/// Installs the real main transition host around lightweight test pages.
Future<void> _pumpMainPages(
  WidgetTester tester,
  ValueNotifier<int> selected,
  _PageCounts counts, {
  bool keepFirstPage = true,
}) async {
  final layout = MainLayoutController();
  final topBar = TopBarController();
  final entries = List<RouteEntry>.generate(
    3,
    (index) => RouteEntry(instanceId: 'entry-$index', routeId: 'route-$index'),
  );
  final router = AppRouterState(entries.first);
  addTearDown(layout.dispose);
  addTearDown(topBar.dispose);
  addTearDown(router.dispose);
  final screens = List<_ProbeScreen>.generate(
    3,
    (index) => _ProbeScreen(
      index: index,
      counts: counts,
      keepAlive: index != 0 || keepFirstPage,
    ),
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
          builder: (context, index, child) => AppContent(
            routerState: router,
            currentScreen: screens[index],
            currentRouteEntry: entries[index],
            currentRouteTitle: 'Route $index',
            useTabletLayout: false,
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
  );
}

/// Installs the shared package and market tab transition host.
Future<void> _pumpTabs(
  WidgetTester tester,
  ValueNotifier<int> selected,
  _PageCounts counts,
) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: ValueListenableBuilder<int>(
        valueListenable: selected,
        builder: (context, index, child) => AnimatedLazyIndexedStack(
          index: index,
          itemCount: 3,
          itemBuilder: (context, page) => _PageProbe(
            key: ValueKey<String>('probe-$page'),
            index: page,
            counts: counts,
          ),
        ),
      ),
    ),
  ),
);

class _PageCounts {
  final initialized = List<int>.filled(3, 0);
  final disposed = List<int>.filled(3, 0);
  final built = List<int>.filled(3, 0);
  final painted = List<int>.filled(3, 0);
}

class _ProbeScreen extends OperitScreen {
  /// Creates a route with observable page lifecycle events.
  const _ProbeScreen({
    required this.index,
    required this.counts,
    required super.keepAlive,
  }) : super(routeTypeName: 'Probe');

  final int index;
  final _PageCounts counts;

  /// Identifies the cached page independently of the route entry instance.
  @override
  String stableScreenKey() => 'screen-$index';

  /// Builds a page whose state can be checked across transitions.
  @override
  Widget build(BuildContext context) => _PageProbe(
    key: ValueKey<String>('probe-$index'),
    index: index,
    counts: counts,
  );
}

class _PageProbe extends StatefulWidget {
  /// Creates an observable stateful page.
  const _PageProbe({super.key, required this.index, required this.counts});

  final int index;
  final _PageCounts counts;

  /// Creates local state that must survive transitions for retained pages.
  @override
  State<_PageProbe> createState() => _PageProbeState();
}

class _PageProbeState extends State<_PageProbe> {
  int _value = 0;

  /// Counts page initialization, including accidental remounts.
  @override
  void initState() {
    super.initState();
    widget.counts.initialized[widget.index]++;
  }

  /// Counts page disposal.
  @override
  void dispose() {
    widget.counts.disposed[widget.index]++;
    super.dispose();
  }

  /// Exposes editable state and records page builds during animation.
  @override
  Widget build(BuildContext context) {
    widget.counts.built[widget.index]++;
    return CustomPaint(
      painter: _PagePainter(widget.counts, widget.index),
      child: Center(
        child: TextButton(
          onPressed: () => setState(() => _value++),
          child: Text('Page ${widget.index}: $_value'),
        ),
      ),
    );
  }
}

class _PagePainter extends CustomPainter {
  /// Records page painting independently of widget builds.
  _PagePainter(this.counts, this.index);

  final _PageCounts counts;
  final int index;

  /// Draws a static page background and counts each repaint.
  @override
  void paint(Canvas canvas, Size size) {
    counts.painted[index]++;
    canvas.drawRect(Offset.zero & size, Paint()..color = Colors.white);
  }

  /// Keeps the static page paint valid across widget rebuilds.
  @override
  bool shouldRepaint(covariant _PagePainter oldDelegate) => false;
}
