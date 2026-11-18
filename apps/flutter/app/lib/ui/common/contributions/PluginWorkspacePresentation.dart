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
    required this.tabId,
    required this.title,
    this.fileOpenerId,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final Object? input;
  final Map<String, Object?> hostState;
  final String? icon;

  /// Optional instance identity scoped to the authenticated plugin and route.
  final String? tabId;

  /// Uses the requested tab title or the localized route title.
  final String title;
  final String? fileOpenerId;

  /// Reuses the same route instance across both workspace panes.
  String get identity => jsonEncode([
    route.containerPackageName,
    route.routeId,
    if (fileOpenerId != null) fileOpenerId,
    if (tabId != null) tabId,
  ]);

  /// Resolves enabled openers in catalog order without executing plugin callbacks.
  static Future<List<PluginWorkspacePresentation>> forFile({
    required GeneratedCoreProxyClients clients,
    required String path,
    required String relativePath,
    required String language,
    String? chatId,
  }) async {
    final name = path.split(RegExp(r'[/\\]')).last;
    if (!name.contains('.')) return const [];
    final manager = clients.application.packageManager();
    final declarations = await manager.getToolPkgNavigationEntries(
      useEnglish: language == 'en',
    );
    final matching =
        declarations.where((entry) {
          final params = entry.params;
          if (entry.surface != 'workspace_file' ||
              entry.action != null ||
              params is! Map ||
              params['extensions'] is! List)
            return false;
          return (params['extensions'] as List).whereType<String>().any(
            (extension) =>
                extension.isNotEmpty &&
                name.toLowerCase().endsWith('.$extension'),
          );
        }).toList()..sort((a, b) {
          final order = a.order.compareTo(b.order);
          if (order != 0) return order;
          final owner = a.containerPackageName.compareTo(
            b.containerPackageName,
          );
          return owner != 0 ? owner : a.entryId.compareTo(b.entryId);
        });
    if (matching.isEmpty) return const [];
    final routes = await manager.getToolPkgUiRoutes(
      runtime: 'compose_dsl',
      useEnglish: language == 'en',
    );
    final result = <PluginWorkspacePresentation>[];
    for (final entry in matching) {
      final owned = routes.where(
        (route) =>
            route.containerPackageName == entry.containerPackageName &&
            route.routeId == entry.routeId &&
            route.runtime == 'compose_dsl' &&
            route.screen.isNotEmpty,
      );
      if (owned.length != 1) continue;
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: entry.containerPackageName,
      );
      if (plugin == null) continue;
      result.add(
        PluginWorkspacePresentation._(
          clients: clients,
          route: owned.single,
          plugin: plugin,
          input: {
            'path': path,
            'relativePath': relativePath,
            'name': name,
            'chatId': chatId,
          },
          hostState: const {},
          icon: entry.icon,
          tabId: path,
          title: entry.title,
          fileOpenerId: entry.entryId,
        ),
      );
    }
    return result;
  }

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
    final input = Map<String, Object?>.from(
      jsonDecode(jsonEncode(args)) as Map,
    );
    // These navigation fields belong to the workspace; all remaining input is opaque.
    String? takeTabText(String key) {
      final value = input.remove(key);
      if (value == null) return null;
      if (value is! String || value.trim().isEmpty) {
        throw ArgumentError.value(value, key, 'Expected nonblank text.');
      }
      return value;
    }

    final tabId = takeTabText('workspaceTabId');
    final tabTitle = takeTabText('workspaceTabTitle');
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
      tabId: tabId,
      title: tabTitle ?? matches.single.title,
    );
  }
}
