// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'EmbeddedPluginSurfaceCatalog.dart';
import 'PluginWorkspacePresentation.dart';

/// Hosts plugin-owned workspace content and revokes it with its catalog entry.
class PluginWorkspacePageHost extends StatefulWidget {
  /// Receives a validated page without interpreting its feature-specific input.
  const PluginWorkspacePageHost({
    super.key,
    required this.presentation,
    required this.workspacePath,
    this.hostState = const {},
  });

  final PluginWorkspacePresentation presentation;
  final String? workspacePath;
  final Map<String, Object?> hostState;

  /// Retains one catalog observer and one native Compose page instance.
  @override
  State<PluginWorkspacePageHost> createState() =>
      _PluginWorkspacePageHostState();
}

class _PluginWorkspacePageHostState extends State<PluginWorkspacePageHost> {
  late EmbeddedPluginSurfaceCatalog _catalog;

  /// Observes package changes independently of the lifetime of the workspace tab.
  @override
  void initState() {
    super.initState();
    _catalog = EmbeddedPluginSurfaceCatalog(
      clients: widget.presentation.clients,
      surfaces: const {'workspace'},
    );
  }

  /// Refreshes localized registrations when the host language changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _catalog.setLanguage(Localizations.localeOf(context).languageCode);
  }

  /// Rebinds observers when the page moves to a different runtime client.
  @override
  void didUpdateWidget(covariant PluginWorkspacePageHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.presentation.clients != widget.presentation.clients) {
      _catalog.dispose();
      _catalog = EmbeddedPluginSurfaceCatalog(
        clients: widget.presentation.clients,
        surfaces: const {'workspace'},
      );
      _catalog.setLanguage(Localizations.localeOf(context).languageCode);
    }
  }

  /// Releases the observer and lets the launcher dispose its runtime session.
  @override
  void dispose() {
    _catalog.dispose();
    super.dispose();
  }

  /// Renders only a currently enabled exact route, never a cached disabled owner.
  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _catalog,
    builder: (context, _) {
      if (_catalog.error case final error?) {
        return Center(child: Text('$error'));
      }
      final requested = widget.presentation;
      final matches = _catalog.entries.where(
        (entry) =>
            entry.route.containerPackageName ==
                requested.route.containerPackageName &&
            entry.route.routeId == requested.route.routeId,
      );
      if (matches.isEmpty) return const SizedBox.shrink();
      final entry = matches.single;
      return ToolPkgUiLauncherScreen(
        key: ValueKey((requested.clients, entry.mountIdentity)),
        clients: requested.clients,
        plugin: entry.plugin,
        initialRouteId: entry.route.routeId,
        embeddedScreenPath: entry.route.screen,
        initialModuleSpec: entry.route.moduleSpec,
        showLauncherChrome: false,
        initialState: {
          ...requested.hostState,
          ...widget.hostState,
          'workspacePath': widget.workspacePath,
          'locale': Localizations.localeOf(context).languageCode,
          'input': requested.input,
        },
      );
    },
  );
}
