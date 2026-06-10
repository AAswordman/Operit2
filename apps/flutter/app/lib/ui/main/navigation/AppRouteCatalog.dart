// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';
import '../../common/icons/MaterialIconNameResolver.dart';
import '../../features/packages/utils/PackageDisplayUtils.dart';
import '../screens/OperitScreens.dart';
import '../screens/ScreenRouteRegistry.dart';
import 'AppNavigationModels.dart';

class AppRouteCatalog {
  const AppRouteCatalog._();

  static const String _toolPkgRuntimeComposeDsl = 'compose_dsl';
  static const String _toolPkgNavSurfaceToolbox = 'toolbox';
  static const String _toolPkgNavSurfaceMainSidebarPlugins =
      'main_sidebar_plugins';

  static AppNavigationModel build(
    BuildContext context, {
    List<core_proxy.ToolPkgContainerRuntime> toolPkgContainers =
        const <core_proxy.ToolPkgContainerRuntime>[],
  }) {
    final l10n = AppLocalizations.of(context)!;
    final toolPkgRoutes = <RouteSpec>[
      for (final container in toolPkgContainers)
        for (final route in container.uiRoutes)
          if (route.runtime.trim().toLowerCase() == _toolPkgRuntimeComposeDsl)
            RouteSpec(
              routeId: route.routeId,
              runtime: RouteRuntime.toolPkgComposeDsl,
              title: localizedText(route.title),
              icon: Icons.extension_outlined,
              ownerPackageName: container.packageName,
              toolPkgUiModuleId: route.id,
              keepAlive: route.keepAlive,
            ),
    ];
    final toolPkgNavigationEntries = <NavigationEntrySpec>[
      for (final container in toolPkgContainers)
        for (final entry in container.navigationEntries)
          if (_navigationSurface(entry.surface) != null)
            NavigationEntrySpec(
              entryId: 'toolpkg:${container.packageName}:${entry.id}',
              routeId: entry.routeId,
              surface: _navigationSurface(entry.surface)!,
              title: localizedText(entry.title),
              icon: MaterialIconNameResolver.resolveOrDefault(
                entry.icon,
                Icons.extension_outlined,
              ),
              order: entry.order,
              action: entry.action == null
                  ? null
                  : NavigationEntryActionSpec(
                      functionName: entry.action!.function,
                      functionSource: entry.action!.functionSource,
                    ),
              kind: NavigationEntryKind.plugin,
              ownerPackageName: container.packageName,
            ),
    ];
    final navigationEntries =
        ScreenRouteRegistry.mainSidebarEntries(l10n) + toolPkgNavigationEntries;
    final sortedNavigationEntries =
        List<NavigationEntrySpec>.of(navigationEntries)..sort((left, right) {
          final surfaceOrder = left.surface.index.compareTo(
            right.surface.index,
          );
          if (surfaceOrder != 0) {
            return surfaceOrder;
          }
          final entryOrder = left.order.compareTo(right.order);
          if (entryOrder != 0) {
            return entryOrder;
          }
          return left.title.compareTo(right.title);
        });
    return AppNavigationModel(
      routes: ScreenRouteRegistry.hostRouteSpecs(l10n) + toolPkgRoutes,
      navigationEntries: sortedNavigationEntries,
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
    if (routeSpec.runtime == RouteRuntime.native) {
      return ScreenRouteRegistry.screenFromEntry(entry);
    }
    if (routeSpec.runtime == RouteRuntime.toolPkgComposeDsl) {
      final containerPackageName = routeSpec.ownerPackageName;
      if (containerPackageName == null) {
        throw StateError('ToolPkg route missing ownerPackageName');
      }
      final uiModuleId = routeSpec.toolPkgUiModuleId;
      if (uiModuleId == null) {
        throw StateError('ToolPkg route missing toolPkgUiModuleId');
      }
      return ToolPkgComposeDslScreenRoute(
        containerPackageName: containerPackageName,
        uiModuleId: uiModuleId,
        title: routeSpec.title,
        keepAlive: routeSpec.keepAlive,
      );
    }
    throw StateError('Unsupported route runtime: ${routeSpec.runtime}');
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

  static NavigationSurface? _navigationSurface(String surface) {
    return switch (surface.trim().toLowerCase()) {
      _toolPkgNavSurfaceToolbox => NavigationSurface.toolbox,
      _toolPkgNavSurfaceMainSidebarPlugins =>
        NavigationSurface.mainSidebarPlugins,
      _ => null,
    };
  }
}
