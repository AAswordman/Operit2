// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

enum RouteRuntime { native, toolPkgComposeDsl }

enum NavigationSurface {
  mainSidebarAi,
  mainSidebarTools,
  mainSidebarPlugins,
  mainSidebarSystem,
  toolbox,
}

enum NavigationEntryKind { host, plugin }

enum RouteEntrySource { defaultSource, drawer, script }

@immutable
class NavigationEntryActionSpec {
  const NavigationEntryActionSpec({
    required this.functionName,
    this.functionSource,
  });

  final String functionName;
  final String? functionSource;
}

@immutable
class RouteSpec {
  const RouteSpec({
    required this.routeId,
    required this.runtime,
    this.title,
    this.icon,
    this.ownerPackageName,
    this.toolPkgUiModuleId,
    this.keepAlive = false,
    this.reuseOnTop = true,
  });

  final String routeId;
  final RouteRuntime runtime;
  final String? title;
  final IconData? icon;
  final String? ownerPackageName;
  final String? toolPkgUiModuleId;
  final bool keepAlive;
  final bool reuseOnTop;
}

@immutable
class RouteEntry {
  RouteEntry({
    String? instanceId,
    required this.routeId,
    this.args = const <String, Object?>{},
    this.source = RouteEntrySource.defaultSource,
  }) : instanceId = instanceId ?? _newInstanceId();

  final String instanceId;
  final String routeId;
  final Map<String, Object?> args;
  final RouteEntrySource source;

  static String _newInstanceId() {
    return DateTime.now().microsecondsSinceEpoch.toString();
  }
}

@immutable
class NavigationEntrySpec {
  const NavigationEntrySpec({
    required this.entryId,
    required this.routeId,
    required this.surface,
    required this.title,
    required this.icon,
    this.description,
    this.order = 0,
    this.routeArgs = const <String, Object?>{},
    this.action,
    this.kind = NavigationEntryKind.host,
    this.ownerPackageName,
  });

  final String entryId;
  final String routeId;
  final NavigationSurface surface;
  final String title;
  final IconData icon;
  final String? description;
  final int order;
  final Map<String, Object?> routeArgs;
  final NavigationEntryActionSpec? action;
  final NavigationEntryKind kind;
  final String? ownerPackageName;
}

@immutable
class AppNavigationModel {
  AppNavigationModel({required this.routes, required this.navigationEntries})
    : routesById = <String, RouteSpec>{
        for (final route in routes) route.routeId: route,
      },
      navigationEntriesById = <String, NavigationEntrySpec>{
        for (final entry in navigationEntries) entry.entryId: entry,
      };

  final List<RouteSpec> routes;
  final List<NavigationEntrySpec> navigationEntries;
  final Map<String, RouteSpec> routesById;
  final Map<String, NavigationEntrySpec> navigationEntriesById;
}

class AppRouterState extends ChangeNotifier {
  AppRouterState(RouteEntry initialEntry)
    : _stack = <RouteEntry>[initialEntry],
      _currentEntry = initialEntry;

  final List<RouteEntry> _stack;
  RouteEntry _currentEntry;

  RouteEntry get currentEntry => _currentEntry;

  List<RouteEntry> get backStack => List<RouteEntry>.unmodifiable(_stack);

  bool get canPop => _stack.length > 1;

  RouteEntry navigate({
    required String routeId,
    Map<String, Object?> args = const <String, Object?>{},
    RouteEntrySource source = RouteEntrySource.defaultSource,
    RouteSpec? routeSpec,
  }) {
    final current = _currentEntry;
    if (routeSpec?.reuseOnTop != false &&
        current.routeId == routeId &&
        mapEquals(current.args, args)) {
      return current;
    }

    final nextEntry = RouteEntry(routeId: routeId, args: args, source: source);
    _stack.add(nextEntry);
    _currentEntry = nextEntry;
    notifyListeners();
    return nextEntry;
  }

  void resetTo(RouteEntry entry) {
    _stack
      ..clear()
      ..add(entry);
    _currentEntry = entry;
    notifyListeners();
  }

  RouteEntry? pop() {
    if (!canPop) {
      return null;
    }
    _stack.removeLast();
    _currentEntry = _stack.last;
    notifyListeners();
    return _currentEntry;
  }
}

class AppRouterGateway {
  const AppRouterGateway._();

  static void Function(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  )?
  _navigateHandler;

  static void Function(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  )?
  _resetHandler;

  static void install({
    required void Function(
      String routeId,
      Map<String, Object?> args,
      RouteEntrySource source,
    )
    handler,
    required void Function(
      String routeId,
      Map<String, Object?> args,
      RouteEntrySource source,
    )
    reset,
  }) {
    _navigateHandler = handler;
    _resetHandler = reset;
  }

  static void clear() {
    _navigateHandler = null;
    _resetHandler = null;
  }

  static void navigate({
    required String routeId,
    Map<String, Object?> args = const <String, Object?>{},
    RouteEntrySource source = RouteEntrySource.script,
  }) {
    final handler = _navigateHandler;
    if (handler == null) {
      throw StateError('AppRouterGateway is not installed');
    }
    handler(routeId, args, source);
  }

  static void resetTo({
    required String routeId,
    Map<String, Object?> args = const <String, Object?>{},
    RouteEntrySource source = RouteEntrySource.script,
  }) {
    final reset = _resetHandler;
    if (reset == null) {
      throw StateError('AppRouterGateway is not installed');
    }
    reset(routeId, args, source);
  }
}

class AppRouteDiscoveryGateway {
  const AppRouteDiscoveryGateway._();

  static List<RouteSpec> Function()? _routesProvider;

  static void install(List<RouteSpec> Function() provider) {
    _routesProvider = provider;
  }

  static void clear() {
    _routesProvider = null;
  }

  static List<RouteSpec> listRoutes() {
    final provider = _routesProvider;
    if (provider == null) {
      throw StateError('AppRouteDiscoveryGateway is not installed');
    }
    return provider();
  }
}
