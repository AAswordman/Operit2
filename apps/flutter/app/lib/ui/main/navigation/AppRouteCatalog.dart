// ignore_for_file: file_names

import '../screens/OperitScreens.dart';
import '../screens/ScreenRouteRegistry.dart';
import 'AppNavigationModels.dart';

class AppRouteCatalog {
  const AppRouteCatalog._();

  static AppNavigationModel build() {
    return AppNavigationModel(
      routes: ScreenRouteRegistry.hostRouteSpecs(),
      navigationEntries: ScreenRouteRegistry.mainSidebarEntries(),
    );
  }

  static OperitScreen resolveScreen(
    AppNavigationModel model,
    RouteEntry entry,
  ) {
    final routeSpec = model.routesById[entry.routeId];
    if (routeSpec == null) {
      throw StateError('Unknown routeId: ${entry.routeId}');
    }
    if (routeSpec.runtime != RouteRuntime.native) {
      throw StateError('Unsupported route runtime: ${routeSpec.runtime}');
    }
    return ScreenRouteRegistry.screenFromEntry(entry);
  }

  static RouteEntry initialEntry() {
    return ScreenRouteRegistry.initialEntry();
  }

  static RouteEntry toEntry({
    required OperitScreen screen,
    RouteEntrySource source = RouteEntrySource.defaultSource,
  }) {
    return ScreenRouteRegistry.toEntry(screen: screen, source: source);
  }
}
