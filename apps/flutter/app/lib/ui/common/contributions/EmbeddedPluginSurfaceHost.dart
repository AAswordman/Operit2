// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'EmbeddedPluginSurfaceCatalog.dart';

/// Mounts enabled Compose contributions without decoding feature-specific input.
class EmbeddedPluginSurfaceHost extends StatelessWidget {
  /// Shares catalog reads and retains one independent session per mounted entry.
  const EmbeddedPluginSurfaceHost({
    super.key,
    required this.catalog,
    required this.surface,
    required this.instanceIdentity,
    required this.hostState,
    required this.onActionResult,
    this.entryPadding = EdgeInsets.zero,
  });

  final EmbeddedPluginSurfaceCatalog catalog;
  final String surface;
  final Object instanceIdentity;
  final Map<String, Object?> hostState;
  final void Function(EmbeddedPluginSurfaceEntry entry, Object? result)
  onActionResult;
  final EdgeInsetsGeometry entryPadding;

  /// Reconciles only currently enabled entries and delegates all content to plugins.
  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: catalog,
    builder: (context, _) {
      if (catalog.error case final error?) {
        return Text(
          '$error',
          style: TextStyle(color: Theme.of(context).colorScheme.error),
        );
      }
      final entries = catalog.entries.where(
        (entry) => entry.entry.surface == surface,
      );
      if (entries.isEmpty) return const SizedBox.shrink();
      return Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final entry in entries)
            Padding(
              key: ValueKey((
                catalog.clients.bridge,
                instanceIdentity,
                entry.mountIdentity,
              )),
              padding: entryPadding,
              child: ToolPkgUiLauncherScreen(
                clients: catalog.clients,
                plugin: entry.plugin,
                initialRouteId: entry.route.routeId,
                embeddedScreenPath: entry.route.screen,
                initialModuleSpec: entry.route.moduleSpec,
                showLauncherChrome: false,
                showLoadingIndicator: false,
                initialState: {
                  'input': entry.entry.params,
                  ...hostState,
                  'locale': Localizations.localeOf(context).languageCode,
                },
                onActionResult: (result) => onActionResult(entry, result),
              ),
            ),
        ],
      );
    },
  );
}
