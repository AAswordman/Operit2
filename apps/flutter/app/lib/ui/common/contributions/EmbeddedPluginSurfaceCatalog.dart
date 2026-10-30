// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../../core/application/PluginHotReload.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../main/navigation/ToolPkgCatalogChangeBus.dart';

/// Resolves one contribution to its enabled owner and exact registered route.
@immutable
class EmbeddedPluginSurfaceEntry {
  /// Retains opaque plugin input without interpreting feature-specific fields.
  const EmbeddedPluginSurfaceEntry({
    required this.entry,
    required this.route,
    required this.plugin,
    this.codeRevision = 0,
  });

  final core_proxy.ToolPkgNavigationEntry entry;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final int codeRevision;

  /// Namespaces entries without ambiguous delimiter concatenation.
  String get identity =>
      jsonEncode([entry.containerPackageName, entry.entryId]);

  /// Preserves sessions for catalog toggles but replaces changed executable routes.
  String get mountIdentity => jsonEncode([
    identity,
    route.uiModuleId,
    route.screen,
    route.moduleSpec,
    plugin.version,
    codeRevision,
  ]);
}

/// Shares catalog reads across visible message rows instead of querying per row.
class EmbeddedPluginSurfaceCatalog extends ChangeNotifier {
  /// Observes enabled-package changes for the requested host-owned surfaces.
  EmbeddedPluginSurfaceCatalog({
    required GeneratedCoreProxyClients clients,
    required Set<String> surfaces,
  }) : _clients = clients,
       surfaces = Set<String>.unmodifiable(surfaces) {
    _subscription = ToolPkgCatalogChangeBus.listenChanges((change) {
      if (change.reloadPluginCode) _codeRevision++;
      _refresh();
    });
    PluginHotReload.revision.addListener(_refresh);
  }

  GeneratedCoreProxyClients _clients;

  /// Uses the current receiving runtime without invalidating unrelated native transcript widgets.
  GeneratedCoreProxyClients get clients => _clients;
  final Set<String> surfaces;
  late final StreamSubscription<ToolPkgCatalogChange> _subscription;
  List<EmbeddedPluginSurfaceEntry> _entries = const [];
  String? _language;
  int _generation = 0;
  int _codeRevision = 0;
  bool _disposed = false;
  Object? error;

  /// Exposes only entries whose route and enabled owner were validated together.
  List<EmbeddedPluginSurfaceEntry> get entries => _entries;

  /// Revokes the old runtime's entries immediately while retaining the shared catalog identity.
  void bindClients(GeneratedCoreProxyClients clients) {
    if (identical(_clients.bridge, clients.bridge)) return;
    _clients = clients;
    _generation++;
    _entries = const [];
    error = null;
    notifyListeners();
    _refresh();
  }

  /// Refreshes localized declarations once for the shared receiving surface.
  void setLanguage(String language) {
    if (_language == language) return;
    _language = language;
    _refresh();
  }

  /// Invalidates pending loads before removing catalog and hot-reload observers.
  @override
  void dispose() {
    _disposed = true;
    _generation++;
    unawaited(_subscription.cancel());
    PluginHotReload.revision.removeListener(_refresh);
    super.dispose();
  }

  /// Launches one generation whose completion cannot resurrect disabled owners.
  void _refresh() {
    if (_disposed || _language == null) return;
    unawaited(_load(++_generation));
  }

  /// Validates exact route ownership and fetches each enabled owner only once.
  Future<void> _load(int generation) async {
    try {
      final manager = clients.application.packageManager();
      final results = await Future.wait<Object>([
        manager.getToolPkgNavigationEntries(useEnglish: _language == 'en'),
        manager.getToolPkgUiRoutes(
          runtime: 'compose_dsl',
          useEnglish: _language == 'en',
        ),
      ]);
      final declarations =
          results[0] as List<core_proxy.ToolPkgNavigationEntry>;
      final routes = results[1] as List<core_proxy.ToolPkgUiRoute>;
      final owners = <String, core_proxy.ToolPkgContainerRuntime>{};
      final identities = <String>{};
      final entries = <EmbeddedPluginSurfaceEntry>[];
      for (final entry in declarations) {
        if (!surfaces.contains(entry.surface)) continue;
        final identity = jsonEncode([
          entry.containerPackageName,
          entry.entryId,
        ]);
        if (entry.containerPackageName.isEmpty ||
            entry.entryId.isEmpty ||
            !identities.add(identity) ||
            entry.action != null) {
          throw StateError('Invalid embedded plugin contribution: $identity');
        }
        final matches = routes.where(
          (route) =>
              route.containerPackageName == entry.containerPackageName &&
              route.routeId == entry.routeId,
        );
        if (matches.length != 1 ||
            matches.single.runtime != 'compose_dsl' ||
            matches.single.screen.isEmpty) {
          throw StateError('Embedded plugin route is not unique: $identity');
        }
        if (!owners.containsKey(entry.containerPackageName)) {
          final owner = await manager.getToolPkgContainerRuntime(
            containerPackageName: entry.containerPackageName,
          );
          if (owner == null) {
            throw StateError('Embedded plugin owner is disabled: $identity');
          }
          owners[entry.containerPackageName] = owner;
        }
        entries.add(
          EmbeddedPluginSurfaceEntry(
            entry: entry,
            route: matches.single,
            plugin: owners[entry.containerPackageName]!,
            codeRevision: _codeRevision,
          ),
        );
      }
      entries.sort((left, right) {
        final order = left.entry.order.compareTo(right.entry.order);
        return order != 0 ? order : left.identity.compareTo(right.identity);
      });
      if (_disposed || generation != _generation) return;
      _entries = List<EmbeddedPluginSurfaceEntry>.unmodifiable(entries);
      error = null;
    } catch (failure) {
      if (_disposed || generation != _generation) return;
      _entries = const [];
      error = failure;
    }
    notifyListeners();
  }
}
