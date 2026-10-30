// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Requests a generic workspace page without granting authority over another owner.
@immutable
class PluginWorkspaceRequest {
  /// Keeps plugin input opaque and uses a plugin-local page reuse key.
  const PluginWorkspaceRequest({
    required this.routeId,
    required this.instanceKey,
    required this.input,
  });

  final String routeId;
  final String instanceKey;
  final Object? input;

  /// Recognizes only the explicit workspace presentation action contract.
  static PluginWorkspaceRequest? fromActionResult(Object? value) {
    if (value is! Map || value['type'] != 'toolpkg.ui.open') return null;
    if (value['placement'] != 'workspace') return null;
    final routeId = value['routeId'];
    final instanceKey = value['instanceKey'];
    if (routeId is! String ||
        routeId.isEmpty ||
        instanceKey is! String ||
        instanceKey.isEmpty) {
      throw const FormatException(
        'Workspace presentations require routeId and instanceKey.',
      );
    }
    return PluginWorkspaceRequest(
      routeId: routeId,
      instanceKey: instanceKey,
      input: jsonDecode(jsonEncode(value['input'])),
    );
  }
}

/// Carries an enabled registered page and separates opaque input from host context.
@immutable
class PluginWorkspacePresentation {
  /// Accepts only a route and owner resolved together through the live catalog.
  const PluginWorkspacePresentation._({
    required this.clients,
    required this.route,
    required this.plugin,
    required this.request,
    required this.hostState,
    required this.icon,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final PluginWorkspaceRequest request;
  final Map<String, Object?> hostState;
  final String? icon;

  /// Namespaces reuse keys by the authenticated source owner and exact route.
  String get identity => jsonEncode([
    route.containerPackageName,
    route.routeId,
    request.instanceKey,
  ]);

  /// Resolves a fresh enabled declaration; output cannot select another package's UI.
  static Future<PluginWorkspacePresentation> resolve({
    required GeneratedCoreProxyClients clients,
    required String sourceOwnerPackageName,
    required PluginWorkspaceRequest request,
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
    final manager = clients.application.packageManager();
    final declarations = await manager.getToolPkgNavigationEntries(
      useEnglish: language == 'en',
    );
    final entries = declarations.where(
      (entry) =>
          entry.containerPackageName == sourceOwnerPackageName &&
          entry.surface == 'workspace' &&
          entry.routeId == request.routeId &&
          entry.action == null,
    );
    if (entries.length != 1) {
      throw StateError(
        'Workspace presentation requires one enabled owning contribution.',
      );
    }
    final routes = await manager.getToolPkgUiRoutes(
      runtime: 'compose_dsl',
      useEnglish: language == 'en',
    );
    final matches = routes.where(
      (route) =>
          route.containerPackageName == sourceOwnerPackageName &&
          route.routeId == request.routeId,
    );
    if (matches.length != 1 ||
        matches.single.runtime != 'compose_dsl' ||
        matches.single.screen.isEmpty) {
      throw StateError(
        'Workspace presentation requires one registered Compose route.',
      );
    }
    final plugin = await manager.getToolPkgContainerRuntime(
      containerPackageName: sourceOwnerPackageName,
    );
    if (plugin == null) {
      throw StateError('Workspace presentation owner is disabled.');
    }
    return PluginWorkspacePresentation._(
      clients: clients,
      route: matches.single,
      plugin: plugin,
      request: request,
      hostState: Map<String, Object?>.unmodifiable(hostState),
      icon: entries.single.icon,
    );
  }
}
