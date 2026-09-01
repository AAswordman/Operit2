// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitGlassSurface.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import '../../../../viewmodel/WorkspaceFileModels.dart';
import 'MentionPackageOptions.dart';

/// Renders package and workspace suggestions for @ and / input mentions.
class MentionSuggestionPanel extends StatefulWidget {
  /// Creates a mention suggestion panel for the active input token.
  const MentionSuggestionPanel({
    super.key,
    required this.viewModel,
    required this.searchQuery,
    required this.triggerChar,
    required this.hasBoundWorkspace,
    required this.onPackageSelected,
    required this.onFileSelected,
  });

  final ChatViewModel viewModel;
  final String searchQuery;
  final String triggerChar;
  final bool hasBoundWorkspace;
  final ValueChanged<String> onPackageSelected;
  final ValueChanged<String> onFileSelected;

  /// Creates mutable state for loaded package and workspace suggestions.
  @override
  State<MentionSuggestionPanel> createState() => _MentionSuggestionPanelState();
}

class _MentionSuggestionPanelState extends State<MentionSuggestionPanel> {
  late Future<List<MentionPackageOption>> _packageOptionsFuture;
  Future<List<WorkspaceFileEntry>>? _fileSuggestionsFuture;
  String _fileSuggestionKey = '';

  /// Loads package options and prepares the first workspace query.
  @override
  void initState() {
    super.initState();
    _packageOptionsFuture = loadMentionPackageOptions(widget.viewModel);
    _syncFileSuggestions();
  }

  /// Refreshes futures when the active trigger or query changes.
  @override
  void didUpdateWidget(covariant MentionSuggestionPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.viewModel != widget.viewModel) {
      _packageOptionsFuture = loadMentionPackageOptions(widget.viewModel);
    }
    _syncFileSuggestions();
  }

  /// Builds the complete suggestion panel.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return SizedBox(
      width: double.infinity,
      child: OperitGlassSurface(
        color: colorScheme.surface,
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: BorderRadius.circular(22),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.32),
        ),
        shadows: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.10),
            blurRadius: 20,
            offset: const Offset(0, 8),
          ),
        ],
        material: true,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 280),
          child: ListView(
            shrinkWrap: true,
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
            children: <Widget>[
              _MentionSuggestionSectionHeader(
                title: _mentionText(
                  AppLocalizations.of(context)!,
                  zh: 'Packages',
                  en: 'Packages',
                ),
              ),
              _PackageSuggestionSection(
                future: _packageOptionsFuture,
                searchQuery: widget.searchQuery,
                onPackageSelected: widget.onPackageSelected,
              ),
              if (_showsWorkspaceSuggestions) ...<Widget>[
                Divider(
                  height: 12,
                  color: colorScheme.outlineVariant.withValues(alpha: 0.32),
                ),
                _MentionSuggestionSectionHeader(
                  title: _mentionText(
                    AppLocalizations.of(context)!,
                    zh: 'Files',
                    en: 'Files',
                  ),
                ),
                _WorkspaceSuggestionSection(
                  future: _fileSuggestionsFuture,
                  searchQuery: widget.searchQuery,
                  hasBoundWorkspace: widget.hasBoundWorkspace,
                  onFileSelected: widget.onFileSelected,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  /// Reports whether the active trigger can show workspace suggestions.
  bool get _showsWorkspaceSuggestions => widget.triggerChar == '@';

  /// Synchronizes the workspace suggestion future with the active query.
  void _syncFileSuggestions() {
    final key = <Object?>[
      widget.triggerChar,
      widget.searchQuery,
      widget.hasBoundWorkspace,
    ].join('\n');
    if (_fileSuggestionKey == key) {
      return;
    }
    _fileSuggestionKey = key;
    if (_showsWorkspaceSuggestions && widget.hasBoundWorkspace) {
      _fileSuggestionsFuture = widget.viewModel.listMentionWorkspaceSuggestions(
        widget.searchQuery,
      );
    } else {
      _fileSuggestionsFuture = null;
    }
  }
}

class _PackageSuggestionSection extends StatelessWidget {
  /// Creates the package section for a mention panel.
  const _PackageSuggestionSection({
    required this.future,
    required this.searchQuery,
    required this.onPackageSelected,
  });

  final Future<List<MentionPackageOption>> future;
  final String searchQuery;
  final ValueChanged<String> onPackageSelected;

  /// Builds package suggestion rows from the loaded package list.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<List<MentionPackageOption>>(
      future: future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return _MentionSuggestionEmptyRow(text: snapshot.error.toString());
        }
        if (snapshot.connectionState != ConnectionState.done) {
          return const _MentionSuggestionLoadingRow();
        }
        final suggestions = filterMentionPackageOptions(
          snapshot.requireData,
          searchQuery,
        );
        if (suggestions.isEmpty) {
          return _MentionSuggestionEmptyRow(
            text: searchQuery.trim().isEmpty
                ? l10n.attachmentPackageEmpty
                : l10n.attachmentPackageSearchEmpty,
          );
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: suggestions
              .map(
                (suggestion) => _MentionSuggestionPackageRow(
                  option: suggestion,
                  onTap: () => onPackageSelected(suggestion.packageName),
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}

class _WorkspaceSuggestionSection extends StatelessWidget {
  /// Creates the workspace file section for a mention panel.
  const _WorkspaceSuggestionSection({
    required this.future,
    required this.searchQuery,
    required this.hasBoundWorkspace,
    required this.onFileSelected,
  });

  final Future<List<WorkspaceFileEntry>>? future;
  final String searchQuery;
  final bool hasBoundWorkspace;
  final ValueChanged<String> onFileSelected;

  /// Builds workspace file suggestion rows for @ mentions.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (!hasBoundWorkspace) {
      return _MentionSuggestionEmptyRow(
        text: _mentionText(
          l10n,
          zh: '当前对话未绑定工作区',
          en: 'Current chat not bound to workspace',
        ),
      );
    }
    final activeFuture = future;
    if (activeFuture == null) {
      return const SizedBox.shrink();
    }
    return FutureBuilder<List<WorkspaceFileEntry>>(
      future: activeFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return _MentionSuggestionEmptyRow(text: snapshot.error.toString());
        }
        if (snapshot.connectionState != ConnectionState.done) {
          return const _MentionSuggestionLoadingRow();
        }
        final suggestions = snapshot.requireData;
        if (suggestions.isEmpty) {
          return _MentionSuggestionEmptyRow(
            text: searchQuery.trim().isEmpty
                ? _mentionText(
                    l10n,
                    zh: '输入名称或路径搜索工作区项目',
                    en: 'Enter name or path to search workspace items',
                  )
                : _mentionText(
                    l10n,
                    zh: '没有匹配的文件或文件夹',
                    en: 'No matching files or folders',
                  ),
          );
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: suggestions
              .map(
                (suggestion) => _MentionSuggestionFileRow(
                  entry: suggestion,
                  onTap: () => onFileSelected(suggestion.relativePath),
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}

class _MentionSuggestionSectionHeader extends StatelessWidget {
  /// Creates a compact section header.
  const _MentionSuggestionSectionHeader({required this.title});

  final String title;

  /// Builds the section header text.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
      child: Text(
        title,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _MentionSuggestionLoadingRow extends StatelessWidget {
  /// Creates a loading row for mention suggestions.
  const _MentionSuggestionLoadingRow();

  /// Builds the loading spinner row.
  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.symmetric(vertical: 8),
      child: Center(
        child: SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      ),
    );
  }
}

class _MentionSuggestionEmptyRow extends StatelessWidget {
  /// Creates an empty or error row for mention suggestions.
  const _MentionSuggestionEmptyRow({required this.text});

  final String text;

  /// Builds the empty row text.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
      child: Text(
        text,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _MentionSuggestionPackageRow extends StatelessWidget {
  /// Creates one package suggestion row.
  const _MentionSuggestionPackageRow({
    required this.option,
    required this.onTap,
  });

  final MentionPackageOption option;
  final VoidCallback onTap;

  /// Builds the package suggestion row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(
                Icons.auto_awesome,
                size: 14,
                color: _mentionPackageColor(option.kind, colors),
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      option.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontSize: 12,
                        height: 14 / 12,
                        color: colors.onSurface,
                      ),
                    ),
                    if (buildMentionPackageSubtitle(
                      AppLocalizations.of(context)!,
                      option,
                    ).isNotEmpty)
                      Text(
                        buildMentionPackageSubtitle(
                          AppLocalizations.of(context)!,
                          option,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          fontSize: 10,
                          height: 12 / 10,
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MentionSuggestionFileRow extends StatelessWidget {
  /// Creates one workspace file suggestion row.
  const _MentionSuggestionFileRow({required this.entry, required this.onTap});

  final WorkspaceFileEntry entry;
  final VoidCallback onTap;

  /// Builds the workspace file suggestion row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final title = _workspaceEntryTitle(entry);
    final detail = _workspaceEntryParentPath(entry.relativePath);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
          child: Row(
            children: <Widget>[
              Icon(
                entry.isDirectory ? Icons.folder : Icons.description,
                size: 14,
                color: entry.isDirectory ? colors.secondary : colors.primary,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: RichText(
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  text: TextSpan(
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontSize: 12,
                      height: 14 / 12,
                      color: colors.onSurface,
                    ),
                    children: <InlineSpan>[
                      TextSpan(
                        text: title,
                        style: const TextStyle(fontWeight: FontWeight.w600),
                      ),
                      if (detail.isNotEmpty) ...<InlineSpan>[
                        const TextSpan(text: '  '),
                        TextSpan(
                          text: detail,
                          style: TextStyle(color: colors.onSurfaceVariant),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Resolves the color used for one package kind.
Color _mentionPackageColor(MentionPackageKind kind, ColorScheme colors) {
  return switch (kind) {
    MentionPackageKind.package => colors.primary,
    MentionPackageKind.skill => colors.secondary,
    MentionPackageKind.mcp => colors.tertiary,
  };
}

/// Resolves locale-specific text used only by this compact panel.
String _mentionText(
  AppLocalizations l10n, {
  required String zh,
  required String en,
}) {
  if (l10n.localeName.toLowerCase().startsWith('zh')) {
    return zh;
  }
  return en;
}

/// Returns the visible workspace entry name.
String _workspaceEntryTitle(WorkspaceFileEntry entry) {
  final name = entry.name.trim();
  if (name.isNotEmpty) {
    return name;
  }
  return entry.relativePath;
}

/// Returns the parent path for a workspace entry.
String _workspaceEntryParentPath(String relativePath) {
  final separatorIndex = relativePath.lastIndexOf('/');
  if (separatorIndex < 0) {
    return '';
  }
  return relativePath.substring(0, separatorIndex);
}
