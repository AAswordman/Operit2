// ignore_for_file: file_names

import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../packages/utils/PackageDisplayUtils.dart';
import '../../../../viewmodel/ChatViewModel.dart';

/// Identifies the source type for a mention package option.
enum MentionPackageKind { package, skill, mcp }

/// Describes one package option shown by the mention panel.
class MentionPackageOption {
  /// Creates a package suggestion row model.
  const MentionPackageOption({
    required this.packageName,
    required this.title,
    required this.description,
    required this.kind,
  });

  final String packageName;
  final String title;
  final String description;
  final MentionPackageKind kind;
}

class _ScoredMentionPackageOption {
  /// Creates a scored package suggestion used for sorting.
  const _ScoredMentionPackageOption({
    required this.option,
    required this.score,
  });

  final MentionPackageOption option;
  final int score;
}

/// Loads package mention choices from package, skill, and MCP sources.
Future<List<MentionPackageOption>> loadMentionPackageOptions(
  ChatViewModel viewModel,
) async {
  final packageManager = viewModel.clients.application.packageManager();
  final skillRepository = viewModel.clients.application.skillRepository();
  final permissionsMcpRuntimeMcpLocalServer =
      viewModel.clients.permissionsMcpRuntimeMcpLocalServer;
  final options = <String, MentionPackageOption>{};

  final availablePackages = await packageManager.getAvailablePackages();
  final packageEntries = availablePackages.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in packageEntries) {
    final packageName = entry.key;
    final isContainer = await packageManager.isToolPkgContainer(
      packageName: packageName,
    );
    if (isContainer) {
      continue;
    }
    options.putIfAbsent(
      packageName,
      () => MentionPackageOption(
        packageName: packageName,
        title: _toolPackageTitle(packageName, entry.value),
        description: localizedText(entry.value.description),
        kind: MentionPackageKind.package,
      ),
    );
  }

  final skillPackages = await skillRepository.getAiVisibleSkillPackages();
  final skillEntries = skillPackages.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in skillEntries) {
    options.putIfAbsent(
      entry.key,
      () => MentionPackageOption(
        packageName: entry.key,
        title: entry.key,
        description: entry.value.description,
        kind: MentionPackageKind.skill,
      ),
    );
  }

  final mcpServers = await permissionsMcpRuntimeMcpLocalServer
      .getAllMcpServers();
  final mcpMetadata = await permissionsMcpRuntimeMcpLocalServer
      .getAllPluginMetadata();
  final mcpEntries = mcpServers.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in mcpEntries) {
    final metadata = mcpMetadata[entry.key];
    options.putIfAbsent(
      entry.key,
      () => MentionPackageOption(
        packageName: entry.key,
        title: _mcpPackageTitle(entry.key, metadata),
        description: metadata?.description ?? '',
        kind: MentionPackageKind.mcp,
      ),
    );
  }

  return options.values.toList(growable: false);
}

/// Filters and ranks package options for the mention query.
List<MentionPackageOption> filterMentionPackageOptions(
  List<MentionPackageOption> options,
  String searchQuery,
) {
  final query = searchQuery.trim().toLowerCase();
  final scoredOptions = <_ScoredMentionPackageOption>[];
  for (final option in options) {
    final score = _scoreMentionPackageOption(option, query);
    if (score == null) {
      continue;
    }
    scoredOptions.add(
      _ScoredMentionPackageOption(option: option, score: score),
    );
  }
  scoredOptions.sort((left, right) {
    final scoreOrder = left.score.compareTo(right.score);
    if (scoreOrder != 0) {
      return scoreOrder;
    }
    return left.option.title.toLowerCase().compareTo(
      right.option.title.toLowerCase(),
    );
  });
  return scoredOptions.map((entry) => entry.option).toList(growable: false);
}

/// Builds the compact subtitle used by mention package rows.
String buildMentionPackageSubtitle(
  AppLocalizations l10n,
  MentionPackageOption option,
) {
  final typeLabel = switch (option.kind) {
    MentionPackageKind.package => l10n.attachmentPackageKindPackage,
    MentionPackageKind.skill => l10n.attachmentPackageKindSkill,
    MentionPackageKind.mcp => l10n.attachmentPackageKindMcp,
  };
  final metaParts = <String>[];
  if (option.packageName != option.title) {
    metaParts.add(option.packageName);
  }
  metaParts.add(typeLabel);
  final description = option.description.trim();
  if (description.isNotEmpty) {
    metaParts.add(description);
  }
  return metaParts.join(' · ');
}

/// Scores one package option for mention ranking.
int? _scoreMentionPackageOption(MentionPackageOption option, String query) {
  if (query.isEmpty) {
    return 4;
  }

  final title = option.title.toLowerCase();
  final packageName = option.packageName.toLowerCase();
  final description = option.description.toLowerCase();
  if (packageName.startsWith(query)) {
    return 0;
  }
  if (title.startsWith(query)) {
    return 1;
  }
  if (_hasQuery(packageName, query)) {
    return 2;
  }
  if (_hasQuery(title, query)) {
    return 3;
  }
  if (_hasQuery(description, query)) {
    return 4;
  }
  return null;
}

/// Reports whether text has the normalized mention query.
bool _hasQuery(String text, String query) {
  return RegExp(RegExp.escape(query)).firstMatch(text) != null;
}

/// Resolves a tool package title for mention rows.
String _toolPackageTitle(String packageName, core_proxy.ToolPackage package) {
  final displayName = toolPackageDisplayName(package).trim();
  if (displayName.isEmpty) {
    return packageName;
  }
  return displayName;
}

/// Resolves the visible MCP package title.
String _mcpPackageTitle(
  String serverName,
  core_proxy.PluginMetadata? metadata,
) {
  final title = metadata?.name.trim() ?? '';
  if (title.isEmpty) {
    return serverName;
  }
  return title;
}
