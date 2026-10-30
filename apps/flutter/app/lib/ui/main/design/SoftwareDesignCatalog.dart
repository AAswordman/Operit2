// ignore_for_file: file_names

import 'package:operit_ui/operit_ui.dart';

import '../../features/packages/components/PackageTab.dart';
import '../../features/packages/screens/UnifiedMarketScreen.dart';
import '../../features/settings/models/SettingsModels.dart';
import '../navigation/AppNavigationModels.dart';
import '../screens/OperitScreens.dart';
import '../screens/ScreenRouteRegistry.dart';

/// Enumerates production routes instead of maintaining a theme-only mock UI.
/// Opening a scenario pushes a temporary route; restoring keeps the original
/// route instances, back stack and selected conversation intact.
class SoftwareDesignCatalog {
  SoftwareDesignCatalog({
    required this.router,
    required this.routes,
    required this.navigate,
  });

  final AppRouterState router;
  final List<RouteSpec> Function() routes;
  final void Function(String, Map<String, Object?>, RouteEntrySource) navigate;
  RouteEntry? _returnEntry;

  void install() {
    if (!operitDesignBridgeEnabled) return;
    OperitDesignScenarios.install(
      owner: this,
      list: scenarios,
      restore: restore,
    );
  }

  void dispose() => OperitDesignScenarios.uninstall(this);

  void restore() {
    final entry = _returnEntry;
    if (entry == null) return;
    if (!router.backStack.any((candidate) => identical(entry, candidate))) {
      throw StateError(
        'Navigation changed during capture; original stack is no longer available',
      );
    }
    while (!identical(router.currentEntry, entry)) {
      router.pop();
    }
    _returnEntry = null;
  }

  List<OperitDesignScenario> scenarios() {
    final result = <OperitDesignScenario>[];
    void add(
      RouteSpec route,
      String suffix,
      String name,
      Map<String, Object?> args,
      String file, {
      String? widget,
    }) {
      result.add(
        OperitDesignScenario(
          id: '${route.routeId}$suffix',
          name: name,
          group: route.runtime == RouteRuntime.toolPkgComposeDsl
              ? 'plugins'
              : route.routeId.contains('settings')
              ? 'settings'
              : route.routeId.contains('package_manager')
              ? 'packages'
              : route.routeId.contains('market')
              ? 'market'
              : 'chat',
          source: {
            'kind': 'application-route',
            'routeId': route.routeId,
            'args': args,
            if (file.isNotEmpty) 'file': file,
            if (widget != null) 'widget': widget,
            if (route.ownerPackageName != null)
              'package': route.ownerPackageName,
            if (route.toolPkgUiModuleId != null)
              'uiModuleId': route.toolPkgUiModuleId,
          },
          open: () {
            _returnEntry = router.currentEntry;
            navigate(route.routeId, args, RouteEntrySource.script);
          },
        ),
      );
    }

    for (final route in routes()) {
      if (route.routeId ==
          ScreenRouteRegistry.routeIdOf(const SettingsScreenRoute())) {
        add(
          route,
          '',
          '设置 / 分区入口',
          {},
          'ui/features/settings/screens/SettingsScreen.dart',
          widget: 'SettingsScreen',
        );
        for (final category in SettingsCategory.values) {
          add(
            route,
            '/${category.name}',
            '设置 / ${category.name}',
            {'category': category.name},
            'ui/features/settings/screens/SettingsScreen.dart',
            widget: 'SettingsScreen',
          );
        }
      } else if (route.routeId ==
          ScreenRouteRegistry.routeIdOf(const PackageManagerScreenRoute())) {
        for (final tab in PackageTab.values) {
          add(
            route,
            '/${tab.name}',
            '包管理 / ${tab.name}',
            {'initialTab': tab.name},
            'ui/features/packages/screens/PackageManagerScreen.dart',
            widget: 'PackageManagerScreen',
          );
        }
      } else if (route.routeId ==
          ScreenRouteRegistry.routeIdOf(const MarketScreenRoute())) {
        for (final tab in MarketHomeTab.values) {
          add(
            route,
            '/${tab.name}',
            '市场 / ${tab.name}',
            {'initialTab': tab.name},
            'ui/features/packages/screens/UnifiedMarketScreen.dart',
            widget: 'UnifiedMarketScreen',
          );
        }
      } else {
        add(
          route,
          '',
          route.title ?? route.routeId,
          {},
          route.runtime == RouteRuntime.native
              ? 'ui/features/chat/screens/AIChatScreen.dart'
              : '',
          widget: route.runtime == RouteRuntime.native
              ? 'AIChatScreen'
              : 'ToolPkgUiLauncherScreen',
        );
      }
    }
    return result;
  }
}
