import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/navigation/AppRouteCatalog.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';

/// Pins which navigations land on the page already on screen. The shell keeps
/// that page's top bar and layout attachment, because the page stays current
/// and never claims them again.
void main() {
  final chat = ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat);
  final packages = ScreenRouteRegistry.routeIdOf(
    ScreenRouteRegistry.packageManager,
  );
  final market = ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.market);
  final detail = ScreenRouteRegistry.routeIdOf(
    const ExtensionDetailScreenRoute(kind: '', id: ''),
  );
  final model = AppNavigationModel(
    routes: <RouteSpec>[
      for (final routeId in <String>[chat, packages, market, detail])
        RouteSpec(routeId: routeId, runtime: RouteRuntime.native),
    ],
    navigationEntries: const <NavigationEntrySpec>[],
  );

  // The page the shell shows for a route opened with these arguments.
  String page(String routeId, [Map<String, Object?> args = const {}]) {
    final entry = RouteEntry(routeId: routeId, args: args);
    return AppRouteCatalog.resolveScreen(model, entry).pageKey(entry);
  }

  test('a tab link shows the page the package sidebar entry opened', () {
    expect(page(packages, {'initialTab': 'extensions'}), page(packages));
    expect(page(packages, {'initialTab': 'devices'}), page(packages));
    expect(page(packages, {'initialTab': 'market'}), isNot(page(packages)));
  });

  test('the market entry shares the package page on its market tab', () {
    expect(page(market), page(packages, {'initialTab': 'market'}));
  });

  test('activating another conversation keeps the single chat page', () {
    expect(
      page(chat, {'conversationActivatedAt': 1}),
      page(chat, {'conversationActivatedAt': 2}),
    );
  });

  test('reopening a detail keeps its page; another extension gets its own', () {
    final opened = page(detail, {'kind': 'toolpkg', 'id': 'a'});
    expect(page(detail, {'kind': 'toolpkg', 'id': 'a'}), opened);
    expect(page(detail, {'kind': 'toolpkg', 'id': 'b'}), isNot(opened));
  });
}
