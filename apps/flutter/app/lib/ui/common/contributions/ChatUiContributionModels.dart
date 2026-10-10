// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'ContributionPresentationResult.dart';
import 'EmbeddedContributionDialog.dart';

/// Retains an owning catalog route and the plugin's opaque presentation input.
@immutable
class ChatUiAction {
  /// Creates an action from a route already validated by the public API catalog.
  const ChatUiAction({required this.route, required this.input});

  final core_proxy.ToolPkgUiRoute route;
  final Object? input;

  /// Opens the existing embedded host without decoding plugin business input.
  Future<ContributionPresentationResult?> present({
    required BuildContext context,
    required GeneratedCoreProxyClients clients,
    Map<String, Object?> hostState = const <String, Object?>{},
  }) async {
    final routes = await clients.application
        .packageManager()
        .getToolPkgUiRoutes(
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
        'A presentation must resolve to exactly one enabled registered route: '
        '${route.containerPackageName}/${route.routeId}',
      );
    }
    final registered = matches.single;
    if (registered.runtime != 'compose_dsl') {
      throw StateError('A presentation requires a registered Compose route.');
    }
    if (!context.mounted) return null;
    return showEmbeddedContributionDialog(
      context: context,
      clients: clients,
      route: EmbeddedContributionRoute(
        containerPackageName: registered.containerPackageName,
        routeId: registered.routeId,
        screenPath: registered.screen,
        title: registered.title,
        input: input,
        moduleSpec: registered.moduleSpec,
        initialState: hostState,
      ),
    );
  }
}

/// Describes a registered selector or attachment source using host display fields.
@immutable
class ChatUiChoice {
  /// Creates one choice without retaining domain identifiers or owner models.
  const ChatUiChoice({
    required this.id,
    required this.title,
    required this.icon,
    required this.action,
  });

  final String id;
  final String title;
  final String? icon;
  final ChatUiAction action;
}

/// Describes the current chat identity and its registered preview presentation.
@immutable
class ChatUiIdentity {
  /// Creates a display-only identity whose tap delegates to its owning plugin.
  const ChatUiIdentity({
    required this.title,
    required this.avatarUri,
    required this.action,
  });

  final String title;
  final String? avatarUri;
  final ChatUiAction action;
}

/// Contains the merged display context produced by the enabled public API owners.
@immutable
class ChatUiContext {
  /// Creates context with explicit absence when no identity or background exists.
  const ChatUiContext({required this.identity, required this.backgroundUri});

  final ChatUiIdentity? identity;
  final String? backgroundUri;
}

/// Projects chat IDs into a plugin-owned section while preserving opaque selection.
@immutable
class ChatUiSection {
  /// Creates a section without converting selection into a host binding model.
  const ChatUiSection({
    required this.ownerPackageName,
    required this.id,
    required this.title,
    required this.avatarUri,
    required this.chatIds,
    required this.selection,
    required this.preview,
  });

  final String ownerPackageName;
  final String id;
  final String title;
  final String? avatarUri;
  final List<String> chatIds;
  final Object? selection;
  final ChatUiAction preview;

  /// Namespaces presentation state using exact owner and section identities.
  String get key => '$ownerPackageName::$id';
}

/// Serializes only generic history metadata for the registered section API.
Map<String, Object?> chatUiHistorySummary(
  core_proxy.ChatHistoryListItem chat,
) => <String, Object?>{
  'id': chat.id,
  'title': chat.title,
  'updatedAt': chat.updatedAt,
  'displayOrder': chat.displayOrder,
  'workspaceId': chat.workspaceId,
  'workspaceName': chat.workspaceName,
  'locked': chat.locked,
  'pinned': chat.pinned,
};
