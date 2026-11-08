import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/components/ExtensionTopBar.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies the shell top bar shows only the current page's title and actions.
void main() {
  testWidgets('updating an outgoing bar cannot clear the returning bar', (
    tester,
  ) async {
    final topBar = TopBarController();
    final showList = ValueNotifier<bool>(false);
    addTearDown(topBar.dispose);
    addTearDown(showList.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: TopBarScope(
          controller: topBar,
          child: Scaffold(
            appBar: AppBar(
              title: ListenableBuilder(
                listenable: topBar,
                builder: (context, child) =>
                    topBar.titleContent?.content(context) ?? const SizedBox(),
              ),
              actions: <Widget>[
                ListenableBuilder(
                  listenable: topBar,
                  builder: (context, child) =>
                      Row(children: topBar.actions?.call(context) ?? const []),
                ),
              ],
            ),
            body: ValueListenableBuilder<bool>(
              valueListenable: showList,
              builder: (context, inList, child) => Stack(
                children: <Widget>[
                  MainScreenActivityScope(
                    isCurrentScreen: inList,
                    child: ExtensionTopBar(
                      title: (context) => const Text('List title'),
                      actions: (context) => const <Widget>[
                        Text('List actions'),
                      ],
                      child: const SizedBox(),
                    ),
                  ),
                  MainScreenActivityScope(
                    isCurrentScreen: !inList,
                    child: ExtensionTopBar(
                      title: (context) => const Text('Detail title'),
                      child: const SizedBox(),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Detail title'), findsOneWidget);
    showList.value = true;
    await tester.pumpAndSettle();
    expect(find.text('List title'), findsOneWidget);
    expect(find.text('List actions'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a parked list reclaims its bar after an immediate transition', (
    tester,
  ) async {
    final topBar = TopBarController();
    final catalog = ValueNotifier<int>(0);
    final list = _Location(
      const _ListScreen(),
      RouteEntry(instanceId: 'list', routeId: 'List'),
    );
    final detail = _Location(
      _DetailScreen(catalog, crossfade: false),
      RouteEntry(instanceId: 'detail', routeId: 'Detail'),
      canGoBack: true,
    );
    final location = ValueNotifier<_Location>(list);
    addTearDown(topBar.dispose);
    addTearDown(catalog.dispose);
    addTearDown(location.dispose);
    await _pumpShell(tester, topBar, location);
    await tester.pumpAndSettle();

    topBar.clear();
    location.value = detail;
    await tester.pumpAndSettle();
    expect(find.text('Detail title'), findsOneWidget);

    catalog.value += 1;
    topBar.clear();
    location.value = _Location(list.screen, list.entry, navigatingBack: true);
    await tester.pumpAndSettle();
    expect(find.text('List title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a catalog change in the return frame preserves list actions', (
    tester,
  ) async {
    final topBar = TopBarController();
    final catalog = ValueNotifier<int>(0);
    final list = _Location(
      const _ListScreen(),
      RouteEntry(instanceId: 'list', routeId: 'List'),
    );
    final detail = _Location(
      _DetailScreen(catalog),
      RouteEntry(instanceId: 'detail', routeId: 'Detail'),
      canGoBack: true,
    );
    final location = ValueNotifier<_Location>(list);
    addTearDown(topBar.dispose);
    addTearDown(catalog.dispose);
    addTearDown(location.dispose);
    await _pumpShell(tester, topBar, location);
    await tester.pumpAndSettle();

    topBar.clear();
    location.value = detail;
    await tester.pumpAndSettle();
    expect(find.text('Detail title'), findsOneWidget);

    // Deletion refreshes the catalog and returns before the next frame.
    catalog.value += 1;
    topBar.clear();
    location.value = _Location(list.screen, list.entry, navigatingBack: true);
    await tester.pump();
    await tester.pump();
    expect(find.text('List title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsOneWidget);

    await tester.pumpAndSettle();
    expect(find.text('List title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a detail page fading out cannot take the bar from its list', (
    tester,
  ) async {
    final topBar = TopBarController();
    final catalog = ValueNotifier<int>(0);
    addTearDown(topBar.dispose);
    addTearDown(catalog.dispose);
    final list = _Location(
      const _ListScreen(),
      RouteEntry(instanceId: 'list', routeId: 'List'),
    );
    final detail = _Location(
      _DetailScreen(catalog),
      RouteEntry(instanceId: 'detail', routeId: 'Detail'),
      canGoBack: true,
    );
    final backToList = _Location(list.screen, list.entry, navigatingBack: true);
    final location = ValueNotifier<_Location>(list);
    addTearDown(location.dispose);
    await _pumpShell(tester, topBar, location);
    await tester.pumpAndSettle();
    expect(find.text('List title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsOneWidget);

    // The shell clears the bar whenever navigation brings another page.
    topBar.clear();
    location.value = detail;
    await tester.pumpAndSettle();
    expect(find.text('Detail title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsNothing);

    topBar.clear();
    location.value = backToList;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 80));
    // Returning to the list refreshes the shared catalog while the detail
    // page is still fading out, so the detail page rebuilds. The second frame
    // shows whatever the rebuild did to the bar.
    catalog.value += 1;
    await tester.pump();
    await tester.pump();
    expect(find.text('Detail body 1', skipOffstage: false), findsOneWidget);
    expect(find.text('Detail title'), findsNothing);
    expect(find.text('List title'), findsOneWidget);

    await tester.pumpAndSettle();
    expect(find.text('Detail body 1', skipOffstage: false), findsNothing);
    expect(find.text('List title'), findsOneWidget);
    expect(find.byTooltip('Import'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _Location {
  const _Location(
    this.screen,
    this.entry, {
    this.canGoBack = false,
    this.navigatingBack = false,
  });

  final OperitScreen screen;
  final RouteEntry entry;
  final bool canGoBack;
  final bool navigatingBack;
}

/// Installs the real main page host and its top bar around fixture pages.
Future<void> _pumpShell(
  WidgetTester tester,
  TopBarController topBar,
  ValueNotifier<_Location> location,
) async {
  final layout = MainLayoutController();
  final router = AppRouterState(location.value.entry);
  addTearDown(layout.dispose);
  addTearDown(router.dispose);
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
            body: ValueListenableBuilder<_Location>(
              valueListenable: location,
              builder: (context, current, child) => AppContent(
                routerState: router,
                currentScreen: current.screen,
                currentRouteEntry: current.entry,
                currentRouteTitle: current.screen.title!,
                useTabletLayout: false,
                isTabletSidebarExpanded: false,
                canGoBack: current.canGoBack,
                enableNavigationAnimation: true,
                isNavigatingBack: current.navigatingBack,
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
  );
}

/// A retained list page with its own title and an action, like the package
/// manager.
class _ListScreen extends OperitScreen {
  const _ListScreen()
    : super(routeTypeName: 'List', title: 'List route', keepAlive: true);

  @override
  String stableScreenKey() => 'List';

  @override
  Widget build(BuildContext context) => ExtensionTopBar(
    title: (context) => const Text('List title'),
    actions: (context) => <Widget>[
      IconButton(
        tooltip: 'Import',
        onPressed: () {},
        icon: const Icon(Icons.add_rounded),
      ),
    ],
    child: const Center(child: Text('List body')),
  );
}

/// A detail page that rebuilds on every catalog change, like the extension
/// detail page.
class _DetailScreen extends OperitScreen {
  const _DetailScreen(this.catalog, {bool crossfade = true})
    : super(
        routeTypeName: 'Detail',
        title: 'Detail route',
        participatesInCrossfadeTransition: crossfade,
      );

  final ValueNotifier<int> catalog;

  @override
  String stableScreenKey() => 'Detail';

  @override
  Widget build(BuildContext context) => _DetailPage(catalog: catalog);
}

class _DetailPage extends StatefulWidget {
  const _DetailPage({required this.catalog});

  final ValueNotifier<int> catalog;

  @override
  State<_DetailPage> createState() => _DetailPageState();
}

class _DetailPageState extends State<_DetailPage> {
  @override
  void initState() {
    super.initState();
    widget.catalog.addListener(_onCatalogChanged);
  }

  @override
  void dispose() {
    widget.catalog.removeListener(_onCatalogChanged);
    super.dispose();
  }

  void _onCatalogChanged() => setState(() {});

  @override
  Widget build(BuildContext context) => ExtensionTopBar(
    title: (context) => const Text('Detail title'),
    child: Center(child: Text('Detail body ${widget.catalog.value}')),
  );
}
