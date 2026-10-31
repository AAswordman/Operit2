// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Carries an enabled registered workspace page and its opaque navigation arguments.
@immutable
class PluginWorkspacePresentation {
  /// Accepts only a route and owner resolved together through the live catalog.
  const PluginWorkspacePresentation._({
    required this.clients,
    required this.route,
    required this.plugin,
    required this.input,
    required this.hostState,
    required this.icon,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final Object? input;
  final Map<String, Object?> hostState;
  final String? icon;

  /// Reuses one tab per authenticated owner and route in either workspace pane.
  String get identity =>
      jsonEncode([route.containerPackageName, route.routeId]);

  /// Uses ordinary page navigation; the registered surface determines placement.
  /// Returns null for routes that should continue through the application router.
  static Future<PluginWorkspacePresentation?> fromNavigation({
    required GeneratedCoreProxyClients clients,
    required String sourceOwnerPackageName,
    required String routeId,
    required Map<String, Object?> args,
    required String language,
    Map<String, Object?> hostState = const {},
  }) async {
    if (sourceOwnerPackageName.isEmpty) {
      throw ArgumentError.value(
        sourceOwnerPackageName,
        'sourceOwnerPackageName',
      );
    }
    if (hostState.containsKey('input')) {
      throw ArgumentError(
        'Host state cannot replace plugin presentation input.',
      );
    }
    final input = jsonDecode(jsonEncode(args));
    final state = Map<String, Object?>.unmodifiable(hostState);
    final manager = clients.application.packageManager();
    final declarations = await manager.getToolPkgNavigationEntries(
      useEnglish: language == 'en',
    );
    final entries = declarations.where(
      (entry) =>
          entry.containerPackageName == sourceOwnerPackageName &&
          entry.surface == 'workspace' &&
          entry.routeId == routeId,
    );
    if (entries.isEmpty) return null;
    if (entries.length != 1 || entries.single.action != null) {
      throw StateError(
        'Workspace navigation requires one enabled owning contribution.',
      );
    }
    final routes = await manager.getToolPkgUiRoutes(
      runtime: 'compose_dsl',
      useEnglish: language == 'en',
    );
    final matches = routes.where(
      (route) =>
          route.containerPackageName == sourceOwnerPackageName &&
          route.routeId == routeId,
    );
    if (matches.length != 1 ||
        matches.single.runtime != 'compose_dsl' ||
        matches.single.screen.isEmpty) {
      throw StateError(
        'Workspace navigation requires one registered Compose route.',
      );
    }
    final plugin = await manager.getToolPkgContainerRuntime(
      containerPackageName: sourceOwnerPackageName,
    );
    if (plugin == null) {
      throw StateError('Workspace navigation owner is disabled.');
    }
    return PluginWorkspacePresentation._(
      clients: clients,
      route: matches.single,
      plugin: plugin,
      input: input,
      hostState: state,
      icon: entries.single.icon,
    );
  }
}
