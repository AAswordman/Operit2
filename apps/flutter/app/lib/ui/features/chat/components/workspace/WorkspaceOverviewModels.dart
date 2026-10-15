// ignore_for_file: file_names

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../common/contributions/ChatUiContributionModels.dart';

class WorkspaceContributionUsage {
  /// Creates a display record retaining the exact owner and opaque section ID.
  const WorkspaceContributionUsage({
    required this.ownerPackageName,
    required this.sectionId,
    required this.title,
    required this.avatarUri,
    required this.conversationCount,
    required this.preview,
  });

  /// Projects a validated catalog section without decoding its business input.
  factory WorkspaceContributionUsage.fromSection(ChatUiSection section) {
    return WorkspaceContributionUsage(
      ownerPackageName: section.ownerPackageName,
      sectionId: section.id,
      title: section.title,
      avatarUri: section.avatarUri,
      conversationCount: section.chatIds.length,
      preview: section.preview,
    );
  }

  final String ownerPackageName;
  final String sectionId;
  final String title;
  final String? avatarUri;
  final int conversationCount;
  final ChatUiAction preview;

  /// Keeps owner and section identities separate even when they contain delimiters.
  (String, String) get identity => (ownerPackageName, sectionId);
}

/// Counts only used sections, preserving catalog order for equal usage counts.
List<WorkspaceContributionUsage> workspaceContributionUsages(
  List<ChatUiSection> sections,
) {
  final usages = <(int, WorkspaceContributionUsage)>[
    for (final (index, section) in sections.indexed)
      if (section.chatIds.isNotEmpty)
        (index, WorkspaceContributionUsage.fromSection(section)),
  ];
  usages.sort((left, right) {
    final countOrder = right.$2.conversationCount.compareTo(
      left.$2.conversationCount,
    );
    return countOrder == 0 ? left.$1.compareTo(right.$1) : countOrder;
  });
  return List<WorkspaceContributionUsage>.unmodifiable(
    usages.map((entry) => entry.$2),
  );
}

sealed class WorkspaceContributionsState {
  /// Defines an explicit catalog state rather than treating failures as emptiness.
  const WorkspaceContributionsState();
}

final class WorkspaceContributionsLoading extends WorkspaceContributionsState {
  /// Marks a pending history snapshot or catalog request.
  const WorkspaceContributionsLoading();
}

final class WorkspaceContributionsReady extends WorkspaceContributionsState {
  /// Retains usage records produced by a successful catalog request.
  const WorkspaceContributionsReady(this.usages);

  final List<WorkspaceContributionUsage> usages;
}

final class WorkspaceContributionsFailed extends WorkspaceContributionsState {
  /// Retains the actual discovery, invocation, validation, or history error.
  const WorkspaceContributionsFailed(this.error, this.stackTrace);

  final Object error;
  final StackTrace stackTrace;
}

class WorkspaceOverviewChatScope {
  /// Retains only the generic summaries belonging to the selected chat's workspace.
  const WorkspaceOverviewChatScope({
    required this.workspaceName,
    required this.chats,
  });

  static const empty = WorkspaceOverviewChatScope(
    workspaceName: null,
    chats: <core_proxy.ChatHistoryListItem>[],
  );

  /// Requires one selected history for bound workspaces and projects exact workspace IDs.
  factory WorkspaceOverviewChatScope.fromHistories({
    required List<core_proxy.ChatHistoryListItem> histories,
    required String? currentChatId,
    required bool hasBoundWorkspace,
  }) {
    if (!hasBoundWorkspace || currentChatId == null) {
      return empty;
    }
    if (currentChatId.trim().isEmpty) {
      throw ArgumentError.value(
        currentChatId,
        'currentChatId',
        'Blank chat ID.',
      );
    }
    final matches = histories.where((history) => history.id == currentChatId);
    if (matches.isEmpty) {
      throw StateError(
        'The selected chat $currentChatId is missing from workspace histories.',
      );
    }
    if (matches.length != 1) {
      throw StateError(
        'The current chat must have exactly one history summary.',
      );
    }
    final currentHistory = matches.single;
    final workspaceId = currentHistory.workspaceId;
    if (workspaceId == null) {
      return WorkspaceOverviewChatScope(
        workspaceName: currentHistory.workspaceName,
        chats: const <core_proxy.ChatHistoryListItem>[],
      );
    }
    if (workspaceId.trim().isEmpty) {
      throw const FormatException('The workspace ID must not be blank.');
    }
    final chats = histories
        .where((history) => history.workspaceId == workspaceId)
        .toList(growable: false);
    final chatIds = <String>{};
    if (chats.any((chat) => !chatIds.add(chat.id))) {
      throw StateError(
        'Workspace history summaries must have unique chat IDs.',
      );
    }
    return WorkspaceOverviewChatScope(
      workspaceName: currentHistory.workspaceName,
      chats: List<core_proxy.ChatHistoryListItem>.unmodifiable(chats),
    );
  }

  final String? workspaceName;
  final List<core_proxy.ChatHistoryListItem> chats;
}

class WorkspaceMountedFolder {
  /// Creates one mounted folder record for a workspace overview.
  const WorkspaceMountedFolder({
    required this.name,
    required this.path,
    required this.relativePath,
  });

  final String name;
  final String path;
  final String relativePath;
}

class WorkspaceOverviewUsage {
  /// Creates the host counts and explicit contribution state for an overview.
  const WorkspaceOverviewUsage({
    required this.workspaceName,
    required this.conversationCount,
    required this.contributions,
    required this.mountedFolders,
    required this.mountedFoldersLoading,
    required this.mountedFoldersError,
  });

  static const WorkspaceOverviewUsage empty = WorkspaceOverviewUsage(
    workspaceName: null,
    conversationCount: 0,
    contributions: WorkspaceContributionsLoading(),
    mountedFolders: <WorkspaceMountedFolder>[],
    mountedFoldersLoading: false,
    mountedFoldersError: null,
  );

  final String? workspaceName;
  final int? conversationCount;
  final WorkspaceContributionsState contributions;
  final List<WorkspaceMountedFolder> mountedFolders;
  final bool mountedFoldersLoading;
  final String? mountedFoldersError;
}
