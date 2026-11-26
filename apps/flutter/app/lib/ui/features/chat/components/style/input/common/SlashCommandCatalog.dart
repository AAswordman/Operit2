// ignore_for_file: file_names

import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../viewmodel/ChatViewModel.dart';

/// Identifies whether a slash command is handled by the desktop host or a ToolPkg plugin.
enum SlashCommandKind { builtin, plugin }

/// Enumerates the basic built-in desktop slash commands.
enum BuiltinSlashCommandId {
  newChat,
  undo,
  compact,
  fork,
  exportChat,
  terminal,
  mcp,
  model,
}

/// Describes one slash command option shown in the command suggestion popup.
class SlashCommandOption {
  const SlashCommandOption({
    required this.name,
    required this.title,
    required this.description,
    required this.usage,
    required this.argumentHint,
    required this.acceptsArguments,
    required this.kind,
    this.builtinId,
    this.containerPackageName,
  });

  final String name;
  final String title;
  final String description;
  final String usage;
  final String? argumentHint;
  final bool acceptsArguments;
  final SlashCommandKind kind;
  final BuiltinSlashCommandId? builtinId;
  final String? containerPackageName;
}

class _ScoredSlashCommandOption {
  const _ScoredSlashCommandOption({required this.option, required this.score});

  final SlashCommandOption option;
  final int score;
}

/// Returns the basic built-in desktop slash commands in display order.
List<SlashCommandOption> builtinSlashCommandOptions({
  required bool useEnglish,
}) {
  return <SlashCommandOption>[
    SlashCommandOption(
      name: 'new',
      title: useEnglish ? 'New chat' : '新建对话',
      description: useEnglish
          ? 'Start a new conversation'
          : '创建一个新的对话',
      usage: '/new',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.newChat,
    ),
    SlashCommandOption(
      name: 'undo',
      title: useEnglish ? 'Undo' : '撤销',
      description: useEnglish
          ? 'Undo the last user message'
          : '撤销上一条消息',
      usage: '/undo',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.undo,
    ),
    SlashCommandOption(
      name: 'compact',
      title: useEnglish ? 'Compact' : '压缩上下文',
      description: useEnglish
          ? 'Summarize conversation to reduce context size'
          : '总结会话以减少上下文大小',
      usage: '/compact',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.compact,
    ),
    SlashCommandOption(
      name: 'fork',
      title: useEnglish ? 'Fork' : '创建分支',
      description: useEnglish
          ? 'Create a new branch from the current conversation'
          : '从当前会话创建分支新会话',
      usage: '/fork',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.fork,
    ),
    SlashCommandOption(
      name: 'export',
      title: useEnglish ? 'Export' : '导出会话',
      description: useEnglish
          ? 'Export the current conversation history to JSON'
          : '将当前会话记录导出为 JSON',
      usage: '/export',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.exportChat,
    ),
    SlashCommandOption(
      name: 'terminal',
      title: useEnglish ? 'Terminal' : '终端',
      description: useEnglish
          ? 'Open a workspace terminal session'
          : '打开工作区终端',
      usage: '/terminal',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.terminal,
    ),
    SlashCommandOption(
      name: 'mcp',
      title: useEnglish ? 'MCP' : 'MCP 服务',
      description: useEnglish
          ? 'View or disable currently enabled MCP servers'
          : '查看或管理当前已开启的 MCP 服务器',
      usage: '/mcp',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.mcp,
    ),
    SlashCommandOption(
      name: 'model',
      title: useEnglish ? 'Model' : '切换模型',
      description: useEnglish
          ? 'Select a different chat model'
          : '选择不同的模型',
      usage: '/model',
      argumentHint: null,
      acceptsArguments: false,
      kind: SlashCommandKind.builtin,
      builtinId: BuiltinSlashCommandId.model,
    ),
  ];
}

/// Loads built-in basic slash commands and enabled ToolPkg Core commands.
Future<List<SlashCommandOption>> loadSlashCommandOptions(
  ChatViewModel viewModel, {
  required bool useEnglish,
}) async {
  final builtins = builtinSlashCommandOptions(useEnglish: useEnglish);
  final builtinNames = builtins.map((item) => item.name.toLowerCase()).toSet();
  final pluginCommands = await viewModel.clients.application
      .packageManager()
      .getToolPkgCoreCommands(useEnglish: useEnglish);

  final plugins = <SlashCommandOption>[];
  for (final command in pluginCommands) {
    final normalizedName = command.name.trim();
    if (normalizedName.isEmpty ||
        builtinNames.contains(normalizedName.toLowerCase())) {
      continue;
    }
    plugins.add(_pluginCommandOption(command));
  }

  return <SlashCommandOption>[...builtins, ...plugins];
}

/// Filters and ranks slash command options for the typed prefix query.
List<SlashCommandOption> filterSlashCommandOptions(
  List<SlashCommandOption> options,
  String searchQuery,
) {
  final query = searchQuery.trim().toLowerCase();
  if (query.isEmpty) {
    return options;
  }
  final scored = <_ScoredSlashCommandOption>[];
  for (final option in options) {
    final score = _scoreSlashCommand(option, query);
    if (score == null) {
      continue;
    }
    scored.add(_ScoredSlashCommandOption(option: option, score: score));
  }
  scored.sort((left, right) {
    final scoreOrder = left.score.compareTo(right.score);
    if (scoreOrder != 0) {
      return scoreOrder;
    }
    return left.option.name.toLowerCase().compareTo(
      right.option.name.toLowerCase(),
    );
  });
  return scored.map((entry) => entry.option).toList(growable: false);
}

/// Finds an exact case-insensitive command match by name.
SlashCommandOption? findSlashCommandByName(
  List<SlashCommandOption> options,
  String commandName,
) {
  final normalized = commandName.trim().toLowerCase();
  if (normalized.isEmpty) {
    return null;
  }
  for (final option in options) {
    if (option.name.toLowerCase() == normalized) {
      return option;
    }
  }
  return null;
}

SlashCommandOption _pluginCommandOption(core_proxy.ToolPkgCoreCommandInfo info) {
  final name = info.name.trim();
  final usage = info.usage.trim().isNotEmpty ? info.usage.trim() : '/$name';
  final hint = _extractArgumentHint(name: name, usage: usage, args: info.args);
  final acceptsArguments =
      info.args.isNotEmpty || (hint != null && hint.isNotEmpty);
  return SlashCommandOption(
    name: name,
    title: info.title.trim().isNotEmpty ? info.title.trim() : name,
    description: info.description.trim(),
    usage: usage,
    argumentHint: hint,
    acceptsArguments: acceptsArguments,
    kind: SlashCommandKind.plugin,
    containerPackageName: info.containerPackageName,
  );
}

String? _extractArgumentHint({
  required String name,
  required String usage,
  required List<core_proxy.ToolPkgCommandArgInfo> args,
}) {
  final commandPrefix = '/$name';
  if (usage.toLowerCase().startsWith(commandPrefix.toLowerCase())) {
    final remainder = usage.substring(commandPrefix.length).trim();
    if (remainder.isNotEmpty) {
      return remainder;
    }
  }
  if (args.isEmpty) {
    return null;
  }
  return args
      .map((arg) => arg.requiredValue ? '<${arg.name}>' : '[${arg.name}]')
      .join(' ');
}

int? _scoreSlashCommand(SlashCommandOption option, String query) {
  final name = option.name.toLowerCase();
  final title = option.title.toLowerCase();
  final description = option.description.toLowerCase();
  final usage = option.usage.toLowerCase();
  if (name.startsWith(query)) {
    return 0;
  }
  if (name.contains(query)) {
    return 1;
  }
  if (title.startsWith(query) || title.contains(query)) {
    return 2;
  }
  if (description.contains(query) || usage.contains(query)) {
    return 3;
  }
  return null;
}
