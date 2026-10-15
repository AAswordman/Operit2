// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'ContributionPresentationResult.dart';

/// Supplies resolved host render inputs for an already registered plugin route.
@immutable
class EmbeddedContributionRoute {
  /// Creates a route without assuming a particular package or business feature.
  const EmbeddedContributionRoute({
    required this.containerPackageName,
    required this.routeId,
    required this.screenPath,
    required this.title,
    required this.input,
    required this.moduleSpec,
    this.initialState = const <String, Object?>{},
  });

  final String containerPackageName;
  final String routeId;
  final String screenPath;
  final String title;
  final Object? input;
  final Map<String, Object?> moduleSpec;

  /// Carries host context separately from the locked presentation envelope.
  final Map<String, Object?> initialState;
}

/// Opens an existing Compose host and returns only an explicit presentation result.
Future<ContributionPresentationResult?> showEmbeddedContributionDialog({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required EmbeddedContributionRoute route,
}) async {
  final plugin = await clients.application
      .packageManager()
      .getToolPkgContainerRuntime(
        containerPackageName: route.containerPackageName,
      );
  if (plugin == null) {
    throw StateError(
      'Contribution container not registered: ${route.containerPackageName}',
    );
  }
  if (!context.mounted) {
    return null;
  }
  if (route.initialState.containsKey('presentation')) {
    throw ArgumentError('Host state cannot replace the presentation envelope.');
  }
  final presentation = ContributionPresentationContext.create(
    input: route.input,
  );
  var completed = false;
  return showDialog<ContributionPresentationResult>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => PopScope(
      canPop: false,
      child: ToolPkgUiLauncherScreen(
        clients: clients,
        plugin: plugin,
        initialRouteId: route.routeId,
        embeddedScreenPath: route.screenPath,
        showLauncherChrome: false,
        dialogTitle: route.title,
        initialState: <String, Object?>{
          ...route.initialState,
          'presentation': presentation.toJson(),
        },
        initialModuleSpec: Map<String, Object?>.unmodifiable(route.moduleSpec),
        onActionResult: (raw) {
          if (completed || !dialogContext.mounted) {
            return;
          }
          final result = ContributionPresentationResult.fromActionResult(
            raw,
            request: presentation,
          );
          if (result == null) {
            return;
          }
          final modal = ModalRoute.of(dialogContext);
          if (modal == null || !modal.isCurrent) {
            throw StateError(
              'A contribution can only complete its own current dialog route.',
            );
          }
          completed = true;
          Navigator.of(dialogContext).pop(result);
        },
      ),
    ),
  );
}
