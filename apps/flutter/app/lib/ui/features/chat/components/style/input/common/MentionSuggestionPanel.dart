// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitGlassSurface.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import '../../../../viewmodel/WorkspaceFileModels.dart';
import 'MentionPackageOptions.dart';
import 'SlashCommandCatalog.dart';

/// Callback invoked when a slash command suggestion is selected.
typedef SlashCommandSelectedCallback =
    void Function(SlashCommandOption option, {required bool completeOnly});

/// Represents one selectable item in the mention/command suggestion panel.
sealed class MentionSuggestionItem {
  const MentionSuggestionItem();
}

/// A slash command entry in the suggestion list.
class CommandMentionSuggestionItem extends MentionSuggestionItem {
  const CommandMentionSuggestionItem(this.option);

  final SlashCommandOption option;
}

/// An enabled package/skill/MCP entry in the suggestion list.
class PackageMentionSuggestionItem extends MentionSuggestionItem {
  const PackageMentionSuggestionItem(this.option);

  final MentionPackageOption option;
}

/// A workspace file/folder entry in the suggestion list.
class FileMentionSuggestionItem extends MentionSuggestionItem {
  const FileMentionSuggestionItem(this.entry);

  final WorkspaceFileEntry entry;
}

/// Coordinates keyboard highlight and acceptance with [MentionSuggestionPanel].
class MentionSuggestionController extends ChangeNotifier {
  int _selectedIndex = 0;
  List<MentionSuggestionItem> _items = const <MentionSuggestionItem>[];
  VoidCallback? _acceptHandler;

  int get selectedIndex => _selectedIndex;

  bool get hasItems => _items.isNotEmpty;

  MentionSuggestionItem? get selectedItem {
    if (_items.isEmpty) {
      return null;
    }
    return _items[_selectedIndex.clamp(0, _items.length - 1)];
  }

  /// Registers the active panel callback that accepts the highlighted item.
  void attachAcceptHandler(VoidCallback? handler) {
    _acceptHandler = handler;
  }

  /// Replaces the visible suggestion items and keeps the highlight index valid.
  void setItems(List<MentionSuggestionItem> items, {bool resetIndex = false}) {
    _items = items;
    if (_items.isEmpty) {
      _selectedIndex = 0;
    } else if (resetIndex || _selectedIndex >= _items.length) {
      _selectedIndex = 0;
    }
    notifyListeners();
  }

  /// Moves the active highlight cyclically by [delta].
  void moveSelection(int delta) {
    if (_items.isEmpty) {
      return;
    }
    var next = (_selectedIndex + delta) % _items.length;
    if (next < 0) {
      next += _items.length;
    }
    if (next == _selectedIndex) {
      return;
    }
    _selectedIndex = next;
    notifyListeners();
  }

  /// Clears stored items when the suggestion panel closes.
  void resetSelection() {
    _selectedIndex = 0;
    _items = const <MentionSuggestionItem>[];
    _acceptHandler = null;
  }

  /// Accepts the currently highlighted item if one is available.
  bool acceptSelected() {
    if (_items.isEmpty || _acceptHandler == null) {
      return false;
    }
    _acceptHandler!();
    return true;
  }
}

/// Renders command, package, and workspace suggestions for @ and / input mentions.
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
    this.showsCommands = false,
    this.onCommandSelected,
    this.controller,
  });

  final ChatViewModel viewModel;
  final String searchQuery;
  final String triggerChar;
  final bool hasBoundWorkspace;
  final ValueChanged<String> onPackageSelected;
  final ValueChanged<String> onFileSelected;
  final bool showsCommands;
  final SlashCommandSelectedCallback? onCommandSelected;
  final MentionSuggestionController? controller;

  /// Creates mutable state for loaded command, package, and workspace suggestions.
  @override
  State<MentionSuggestionPanel> createState() => _MentionSuggestionPanelState();
}

class _MentionSuggestionPanelState extends State<MentionSuggestionPanel> {
  final Map<int, GlobalKey> _itemKeys = <int, GlobalKey>{};
  int _commandLoadGeneration = 0;
  int _packageLoadGeneration = 0;
  int _fileLoadGeneration = 0;
  bool? _useEnglish;
  String _fileSuggestionKey = '';

  bool _loadingCommands = false;
  Object? _commandsError;
  List<SlashCommandOption> _loadedCommands = const <SlashCommandOption>[];

  bool _loadingPackages = true;
  Object? _packagesError;
  List<MentionPackageOption> _loadedPackages = const <MentionPackageOption>[];

  bool _loadingFiles = false;
  Object? _filesError;
  List<WorkspaceFileEntry> _loadedFiles = const <WorkspaceFileEntry>[];

  @override
  void initState() {
    super.initState();
    widget.controller?.addListener(_onControllerChanged);
    widget.controller?.attachAcceptHandler(_acceptCurrentSelection);
    _loadPackages();
    _syncFileSuggestions();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final useEnglish = !Localizations.localeOf(
      context,
    ).languageCode.toLowerCase().startsWith('zh');
    if (_useEnglish != useEnglish) {
      _useEnglish = useEnglish;
      if (widget.showsCommands) {
        _loadCommands(useEnglish: useEnglish);
      }
    }
  }

  @override
  void didUpdateWidget(covariant MentionSuggestionPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller?.removeListener(_onControllerChanged);
      oldWidget.controller?.attachAcceptHandler(null);
      widget.controller?.addListener(_onControllerChanged);
      widget.controller?.attachAcceptHandler(_acceptCurrentSelection);
    }
    if (oldWidget.viewModel != widget.viewModel) {
      _loadPackages();
      if (widget.showsCommands && _useEnglish != null) {
        _loadCommands(useEnglish: _useEnglish!);
      }
    } else if (!oldWidget.showsCommands &&
        widget.showsCommands &&
        _useEnglish != null) {
      _loadCommands(useEnglish: _useEnglish!);
    }
    _syncFileSuggestions();
    final queryOrTriggerChanged =
        oldWidget.searchQuery != widget.searchQuery ||
        oldWidget.triggerChar != widget.triggerChar ||
        oldWidget.showsCommands != widget.showsCommands ||
        oldWidget.controller != widget.controller;
    if (queryOrTriggerChanged) {
      _syncControllerItems(resetIndex: true);
    }
  }

  @override
  void dispose() {
    widget.controller?.removeListener(_onControllerChanged);
    widget.controller?.attachAcceptHandler(null);
    super.dispose();
  }

  void _onControllerChanged() {
    if (!mounted) {
      return;
    }
    final selectedIndex = widget.controller?.selectedIndex ?? 0;
    final key = _itemKeys[selectedIndex];
    if (key != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final targetContext = key.currentContext;
        if (targetContext != null && targetContext.mounted) {
          Scrollable.ensureVisible(
            targetContext,
            alignmentPolicy: ScrollPositionAlignmentPolicy.keepVisibleAtEnd,
          );
          Scrollable.ensureVisible(
            targetContext,
            alignmentPolicy: ScrollPositionAlignmentPolicy.keepVisibleAtStart,
          );
        }
      });
    }
  }

  void _loadCommands({required bool useEnglish}) {
    final generation = ++_commandLoadGeneration;
    _loadingCommands = true;
    _commandsError = null;
    unawaited(
      loadSlashCommandOptions(widget.viewModel, useEnglish: useEnglish)
          .then((commands) {
            if (!mounted || generation != _commandLoadGeneration) {
              return;
            }
            setState(() {
              _loadingCommands = false;
              _commandsError = null;
              _loadedCommands = commands;
            });
            _syncControllerItems();
          })
          .catchError((Object error) {
            if (!mounted || generation != _commandLoadGeneration) {
              return;
            }
            setState(() {
              _loadingCommands = false;
              _commandsError = error;
              _loadedCommands = const <SlashCommandOption>[];
            });
            _syncControllerItems();
          }),
    );
  }

  void _loadPackages() {
    final generation = ++_packageLoadGeneration;
    _loadingPackages = true;
    _packagesError = null;
    unawaited(
      loadMentionPackageOptions(widget.viewModel)
          .then((packages) {
            if (!mounted || generation != _packageLoadGeneration) {
              return;
            }
            setState(() {
              _loadingPackages = false;
              _packagesError = null;
              _loadedPackages = packages;
            });
            _syncControllerItems();
          })
          .catchError((Object error) {
            if (!mounted || generation != _packageLoadGeneration) {
              return;
            }
            setState(() {
              _loadingPackages = false;
              _packagesError = error;
              _loadedPackages = const <MentionPackageOption>[];
            });
            _syncControllerItems();
          }),
    );
  }

  bool get _showsWorkspaceSuggestions => widget.triggerChar == '@';

  void _syncFileSuggestions() {
    final key = <Object?>[
      widget.triggerChar,
      widget.searchQuery,
      widget.hasBoundWorkspace,
      widget.viewModel,
    ].join('\n');
    if (_fileSuggestionKey == key) {
      return;
    }
    _fileSuggestionKey = key;
    if (_showsWorkspaceSuggestions && widget.hasBoundWorkspace) {
      final generation = ++_fileLoadGeneration;
      _loadingFiles = true;
      _filesError = null;
      unawaited(
        widget.viewModel
            .listMentionWorkspaceSuggestions(widget.searchQuery)
            .then((files) {
              if (!mounted || generation != _fileLoadGeneration) {
                return;
              }
              setState(() {
                _loadingFiles = false;
                _filesError = null;
                _loadedFiles = files;
              });
              _syncControllerItems();
            })
            .catchError((Object error) {
              if (!mounted || generation != _fileLoadGeneration) {
                return;
              }
              setState(() {
                _loadingFiles = false;
                _filesError = error;
                _loadedFiles = const <WorkspaceFileEntry>[];
              });
              _syncControllerItems();
            }),
      );
    } else {
      _fileLoadGeneration++;
      _loadingFiles = false;
      _filesError = null;
      _loadedFiles = const <WorkspaceFileEntry>[];
      _syncControllerItems();
    }
  }

  List<SlashCommandOption> get _filteredCommands {
    if (!widget.showsCommands || _loadingCommands || _commandsError != null) {
      return const <SlashCommandOption>[];
    }
    return filterSlashCommandOptions(_loadedCommands, widget.searchQuery);
  }

  List<MentionPackageOption> get _filteredPackages {
    if (_loadingPackages || _packagesError != null) {
      return const <MentionPackageOption>[];
    }
    return filterMentionPackageOptions(_loadedPackages, widget.searchQuery);
  }

  List<WorkspaceFileEntry> get _filteredFiles {
    if (!_showsWorkspaceSuggestions ||
        !widget.hasBoundWorkspace ||
        _loadingFiles ||
        _filesError != null) {
      return const <WorkspaceFileEntry>[];
    }
    return _loadedFiles;
  }

  void _syncControllerItems({bool resetIndex = false}) {
    final controller = widget.controller;
    if (controller == null) {
      return;
    }
    final items = <MentionSuggestionItem>[
      for (final command in _filteredCommands)
        CommandMentionSuggestionItem(command),
      for (final pkg in _filteredPackages) PackageMentionSuggestionItem(pkg),
      for (final file in _filteredFiles) FileMentionSuggestionItem(file),
    ];
    controller.setItems(items, resetIndex: resetIndex);
  }

  void _acceptCurrentSelection() {
    final selected = widget.controller?.selectedItem;
    if (selected == null) {
      return;
    }
    switch (selected) {
      case CommandMentionSuggestionItem(:final option):
        widget.onCommandSelected?.call(
          option,
          completeOnly: option.acceptsArguments,
        );
      case PackageMentionSuggestionItem(:final option):
        widget.onPackageSelected(option.packageName);
      case FileMentionSuggestionItem(:final entry):
        widget.onFileSelected(entry.relativePath);
    }
  }

  GlobalKey _keyForFlatIndex(int index) {
    return _itemKeys.putIfAbsent(index, GlobalKey.new);
  }

  /// Builds the complete suggestion panel.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final commands = _filteredCommands;
    final packages = _filteredPackages;
    final files = _filteredFiles;
    final hasQuery = widget.searchQuery.trim().isNotEmpty;

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
          child: widget.controller != null
              ? ListenableBuilder(
                  listenable: widget.controller!,
                  builder: (context, _) => _buildListContent(
                    context,
                    l10n: l10n,
                    colorScheme: colorScheme,
                    commands: commands,
                    packages: packages,
                    files: files,
                    hasQuery: hasQuery,
                    selectedIndex: widget.controller!.selectedIndex,
                  ),
                )
              : _buildListContent(
                  context,
                  l10n: l10n,
                  colorScheme: colorScheme,
                  commands: commands,
                  packages: packages,
                  files: files,
                  hasQuery: hasQuery,
                  selectedIndex: -1,
                ),
        ),
      ),
    );
  }

  Widget _buildListContent(
    BuildContext context, {
    required AppLocalizations l10n,
    required ColorScheme colorScheme,
    required List<SlashCommandOption> commands,
    required List<MentionPackageOption> packages,
    required List<WorkspaceFileEntry> files,
    required bool hasQuery,
    required int selectedIndex,
  }) {
    final children = <Widget>[];
    var flatIndex = 0;

    if (widget.showsCommands) {
      final showCommandSection =
          !hasQuery ||
          _loadingCommands ||
          _commandsError != null ||
          commands.isNotEmpty;
      if (showCommandSection) {
        children.add(
          _MentionSuggestionSectionHeader(
            title: _mentionText(l10n, zh: '命令', en: 'Commands'),
          ),
        );
        if (_commandsError != null) {
          children.add(
            _MentionSuggestionEmptyRow(text: _commandsError.toString()),
          );
        } else if (_loadingCommands) {
          children.add(const _MentionSuggestionLoadingRow());
        } else if (commands.isEmpty) {
          children.add(
            _MentionSuggestionEmptyRow(
              text: _mentionText(
                l10n,
                zh: '暂无可用命令',
                en: 'No available commands',
              ),
            ),
          );
        } else {
          for (final command in commands) {
            final index = flatIndex++;
            children.add(
              KeyedSubtree(
                key: _keyForFlatIndex(index),
                child: _MentionSuggestionCommandRow(
                  option: command,
                  selected: index == selectedIndex,
                  onTap: () => widget.onCommandSelected?.call(
                    command,
                    completeOnly: command.acceptsArguments,
                  ),
                ),
              ),
            );
          }
        }
      }
    }

    final showPackageSection =
        !widget.showsCommands ||
        !hasQuery ||
        _loadingPackages ||
        _packagesError != null ||
        packages.isNotEmpty;
    if (showPackageSection) {
      if (children.isNotEmpty) {
        children.add(
          Divider(
            height: 12,
            color: colorScheme.outlineVariant.withValues(alpha: 0.32),
          ),
        );
      }
      children.add(
        _MentionSuggestionSectionHeader(
          title: _mentionText(l10n, zh: '工具包', en: 'Packages'),
        ),
      );
      if (_packagesError != null) {
        children.add(
          _MentionSuggestionEmptyRow(text: _packagesError.toString()),
        );
      } else if (_loadingPackages) {
        children.add(const _MentionSuggestionLoadingRow());
      } else if (packages.isEmpty) {
        children.add(
          _MentionSuggestionEmptyRow(
            text: hasQuery
                ? l10n.attachmentPackageSearchEmpty
                : l10n.attachmentPackageEmpty,
          ),
        );
      } else {
        for (final pkg in packages) {
          final index = flatIndex++;
          children.add(
            KeyedSubtree(
              key: _keyForFlatIndex(index),
              child: _MentionSuggestionPackageRow(
                option: pkg,
                selected: index == selectedIndex,
                onTap: () => widget.onPackageSelected(pkg.packageName),
              ),
            ),
          );
        }
      }
    }

    if (widget.showsCommands &&
        hasQuery &&
        !_loadingCommands &&
        !_loadingPackages &&
        _commandsError == null &&
        _packagesError == null &&
        commands.isEmpty &&
        packages.isEmpty) {
      children.add(
        _MentionSuggestionEmptyRow(
          text: _mentionText(
            l10n,
            zh: '没有匹配的命令或工具包',
            en: 'No matching commands or packages',
          ),
        ),
      );
    }

    if (_showsWorkspaceSuggestions) {
      children.add(
        Divider(
          height: 12,
          color: colorScheme.outlineVariant.withValues(alpha: 0.32),
        ),
      );
      children.add(
        _MentionSuggestionSectionHeader(
          title: _mentionText(l10n, zh: 'Files', en: 'Files'),
        ),
      );
      if (!widget.hasBoundWorkspace) {
        children.add(
          _MentionSuggestionEmptyRow(
            text: _mentionText(
              l10n,
              zh: '当前对话未绑定工作区',
              en: 'Current chat not bound to workspace',
            ),
          ),
        );
      } else if (_filesError != null) {
        children.add(_MentionSuggestionEmptyRow(text: _filesError.toString()));
      } else if (_loadingFiles) {
        children.add(const _MentionSuggestionLoadingRow());
      } else if (files.isEmpty) {
        children.add(
          _MentionSuggestionEmptyRow(
            text: !hasQuery
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
          ),
        );
      } else {
        for (final file in files) {
          final index = flatIndex++;
          children.add(
            KeyedSubtree(
              key: _keyForFlatIndex(index),
              child: _MentionSuggestionFileRow(
                entry: file,
                selected: index == selectedIndex,
                onTap: () => widget.onFileSelected(file.relativePath),
              ),
            ),
          );
        }
      }
    }

    return ListView(
      shrinkWrap: true,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
      children: children,
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

class _MentionSuggestionCommandRow extends StatelessWidget {
  const _MentionSuggestionCommandRow({
    required this.option,
    required this.selected,
    required this.onTap,
  });

  final SlashCommandOption option;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Material(
      color: selected
          ? colors.primaryContainer.withValues(alpha: 0.42)
          : Colors.transparent,
      borderRadius: BorderRadius.circular(10),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
          child: SizedBox(
            width: double.infinity,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                RichText(
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
                        text: '/${option.name}',
                        style: const TextStyle(fontWeight: FontWeight.w600),
                      ),
                      if (option.argumentHint != null &&
                          option.argumentHint!.isNotEmpty) ...<InlineSpan>[
                        const TextSpan(text: '  '),
                        TextSpan(
                          text: option.argumentHint!,
                          style: TextStyle(
                            fontSize: 11,
                            color: colors.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                if (option.description.isNotEmpty)
                  Text(
                    option.description,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontSize: 10,
                      height: 13 / 10,
                      color: colors.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _MentionSuggestionPackageRow extends StatelessWidget {
  /// Creates one package suggestion row.
  const _MentionSuggestionPackageRow({
    required this.option,
    required this.selected,
    required this.onTap,
  });

  final MentionPackageOption option;
  final bool selected;
  final VoidCallback onTap;

  /// Builds the package suggestion row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final subtitle = buildMentionPackageSubtitle(
      AppLocalizations.of(context)!,
      option,
    );
    return Material(
      color: selected
          ? colors.primaryContainer.withValues(alpha: 0.42)
          : Colors.transparent,
      borderRadius: BorderRadius.circular(10),
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
                    if (subtitle.isNotEmpty)
                      Text(
                        subtitle,
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
  const _MentionSuggestionFileRow({
    required this.entry,
    required this.selected,
    required this.onTap,
  });

  final WorkspaceFileEntry entry;
  final bool selected;
  final VoidCallback onTap;

  /// Builds the workspace file suggestion row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final title = _workspaceEntryTitle(entry);
    final detail = _workspaceEntryParentPath(entry.relativePath);
    return Material(
      color: selected
          ? colors.primaryContainer.withValues(alpha: 0.42)
          : Colors.transparent,
      borderRadius: BorderRadius.circular(10),
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
