// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'ContributionPresentationResult.dart';

/// Opens an existing Compose host and returns only an explicit presentation result.
Future<ContributionPresentationResult?> showEmbeddedContributionDialog({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required core_proxy.ToolPkgUiRoute route,
  required Object? input,
  Map<String, Object?> hostState = const <String, Object?>{},
}) async {
  final routes = await clients.application.packageManager().getToolPkgUiRoutes(
    runtime: 'compose_dsl',
    useEnglish: Localizations.localeOf(context).languageCode == 'en',
  );
  final matches = routes.where(
    (registered) =>
        registered.containerPackageName == route.containerPackageName &&
        registered.routeId == route.routeId,
  );
  if (matches.length != 1) {
    throw StateError(
      'A presentation must resolve to exactly one enabled registered route: ${route.containerPackageName}/${route.routeId}',
    );
  }
  final registered = matches.single;
  if (registered.runtime != 'compose_dsl') {
    throw StateError('A presentation requires a registered Compose route.');
  }

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
  if (hostState.containsKey('presentation')) {
    throw ArgumentError('Host state cannot replace the presentation envelope.');
  }
  final presentation = ContributionPresentationContext.create(input: input);
  var completed = false;
  return showDialog<ContributionPresentationResult>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => PopScope(
      canPop: false,
      child: ToolPkgUiLauncherScreen(
        clients: clients,
        plugin: plugin,
        initialRouteId: registered.routeId,
        embeddedScreenPath: registered.screen,
        showLauncherChrome: false,
        dialogTitle: registered.title,
        initialState: <String, Object?>{
          ...hostState,
          'presentation': presentation.toJson(),
        },
        initialModuleSpec: Map<String, Object?>.unmodifiable(
          registered.moduleSpec,
        ),
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
