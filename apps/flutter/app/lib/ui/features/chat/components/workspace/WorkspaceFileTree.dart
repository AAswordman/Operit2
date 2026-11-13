// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceFileIcon.dart';
import 'WorkspaceFileRenameDialog.dart';
import '../../../../main/components/SidebarMoreMenu.dart';

/// A lazily loaded explorer. Only visible rows are built and collapsed folders
/// keep their cached children until the user refreshes the listing.
class WorkspaceFileTree extends StatefulWidget {
  const WorkspaceFileTree({
    super.key,
    required this.rootPath,
    required this.rootLabel,
    required this.rootEntries,
    required this.onListDirectory,
    required this.onOpenFile,
    this.protectWorkspaceRoots = false,
    this.onMutate,
    this.clipboard,
    this.onFilesChanged,
  });

  final WorkspaceFileMutation? onMutate;
  final WorkspaceFileClipboard? clipboard;
  final VoidCallback? onFilesChanged;
  final bool protectWorkspaceRoots;
  final String rootPath;
  final String rootLabel;
  final Future<List<WorkspaceFileEntry>> rootEntries;
  final Future<List<WorkspaceFileEntry>> Function(String) onListDirectory;
  final Future<void> Function(WorkspaceFileEntry) onOpenFile;

  @override
  State<WorkspaceFileTree> createState() => _WorkspaceFileTreeState();
}

class _WorkspaceFileTreeState extends State<WorkspaceFileTree> {
  final _children = <String, List<WorkspaceFileEntry>>{};
  final _expanded = <String>{};
  final _loading = <String>{};
  final _errors = <String, Object>{};
  final _scroll = ScrollController();
  final _focus = FocusNode();
  final _localClipboard = WorkspaceFileClipboard();
  bool _mutating = false;
  bool _menuOpen = false;
  String? _selected;
  bool _rootExpanded = true;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _clipboard.addListener(_clipboardChanged);
    _reload();
  }

  @override
  void didUpdateWidget(covariant WorkspaceFileTree oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clipboard != widget.clipboard) {
      (oldWidget.clipboard ?? _localClipboard).removeListener(
        _clipboardChanged,
      );
      _clipboard.addListener(_clipboardChanged);
    }
    final rootChanged =
        oldWidget.rootPath != widget.rootPath ||
        oldWidget.rootLabel != widget.rootLabel;
    if (rootChanged) {
      _expanded.clear();
      _selected = null;
      _rootExpanded = true;
    }
    if (rootChanged || oldWidget.rootEntries != widget.rootEntries) _reload();
  }

  @override
  void dispose() {
    _clipboard.removeListener(_clipboardChanged);
    _localClipboard.dispose();
    _scroll.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _reload() {
    _generation++;
    _children.clear();
    _loading.clear();
    _errors.clear();
    _load(widget.rootPath, source: widget.rootEntries);
  }

  Future<void> _load(
    String path, {
    Future<List<WorkspaceFileEntry>>? source,
  }) async {
    if (_loading.contains(path)) return;
    final generation = _generation;
    _loading.add(path);
    _errors.remove(path);
    try {
      final entries = await (source ?? widget.onListDirectory(path));
      if (!mounted || generation != _generation) return;
      final sorted = List<WorkspaceFileEntry>.of(entries)
        ..sort((a, b) {
          if (a.isDirectory != b.isDirectory) return a.isDirectory ? -1 : 1;
          final order = a.name.toLowerCase().compareTo(b.name.toLowerCase());
          return order == 0 ? a.name.compareTo(b.name) : order;
        });
      setState(() {
        _loading.remove(path);
        _children[path] = sorted;
      });
      // Refresh expanded branches only once their parent has been confirmed to
      // still exist. Old requests cannot populate a newer refresh generation.
      for (final entry in sorted) {
        if (entry.isDirectory && _expanded.contains(entry.relativePath)) {
          _load(entry.relativePath);
        }
      }
    } catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _loading.remove(path);
        _errors[path] = error;
      });
    }
  }

  List<_TreeRow> _visibleRows() {
    final rows = <_TreeRow>[];
    void visit(String path, int depth) {
      final entries = _children[path];
      if (entries == null || entries.isEmpty) {
        rows.add(_TreeRow.message(path, depth));
        return;
      }
      for (final entry in entries) {
        rows.add(_TreeRow.entry(entry, depth));
        if (entry.isDirectory && _expanded.contains(entry.relativePath)) {
          visit(entry.relativePath, depth + 1);
        }
      }
    }

    if (_rootExpanded) visit(widget.rootPath, 0);
    return rows;
  }

  void _activate(WorkspaceFileEntry entry) {
    _focus.requestFocus();
    setState(() => _selected = entry.relativePath);
    if (!entry.isDirectory) {
      widget.onOpenFile(entry);
      return;
    }
    _toggleDirectory(entry.relativePath);
  }

  void _toggleDirectory(String path) {
    setState(() {
      if (!_expanded.remove(path)) {
        _expanded.add(path);
        if (!_children.containsKey(path)) _load(path);
      }
    });
  }

  KeyEventResult _onKey(KeyEvent event, List<_TreeRow> rows, double rowHeight) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final key = event.logicalKey;
    final index = rows.indexWhere(
      (row) => row.entry != null && row.path == _selected,
    );
    final entry = index < 0 ? null : rows[index].entry;
    var next = index;
    if (key == LogicalKeyboardKey.arrowDown ||
        key == LogicalKeyboardKey.arrowUp) {
      final step = key == LogicalKeyboardKey.arrowDown ? 1 : -1;
      next = index < 0 ? (step == 1 ? 0 : rows.length - 1) : index + step;
      while (next >= 0 && next < rows.length && rows[next].entry == null) {
        next += step;
      }
    } else if (entry != null &&
        (key == LogicalKeyboardKey.enter || key == LogicalKeyboardKey.space)) {
      _activate(entry);
    } else if (entry != null && key == LogicalKeyboardKey.arrowRight) {
      if (entry.isDirectory && !_expanded.contains(entry.relativePath)) {
        _toggleDirectory(entry.relativePath);
      } else if (entry.isDirectory) {
        next = index + 1;
      }
    } else if (entry != null && key == LogicalKeyboardKey.arrowLeft) {
      if (entry.isDirectory && _expanded.contains(entry.relativePath)) {
        _toggleDirectory(entry.relativePath);
      } else {
        for (var i = index - 1; i >= 0; i--) {
          if (rows[i].depth < rows[index].depth) {
            next = i;
            break;
          }
        }
      }
    } else {
      return KeyEventResult.ignored;
    }
    if (next >= 0 && next < rows.length && rows[next].entry != null) {
      setState(() => _selected = rows[next].path);
      if (_scroll.hasClients) {
        final top = next * rowHeight;
        final bottom = top + rowHeight;
        final position = _scroll.position;
        if (top < position.pixels ||
            bottom > position.pixels + position.viewportDimension) {
          _scroll.jumpTo(
            (top < position.pixels ? top : bottom - position.viewportDimension)
                .clamp(0.0, position.maxScrollExtent),
          );
        }
      }
    }
    return KeyEventResult.handled;
  }

  WorkspaceFileClipboard get _clipboard => widget.clipboard ?? _localClipboard;

  void _clipboardChanged() {
    if (mounted) setState(() {});
  }

  String _parentPath(String path) {
    final slash = path.lastIndexOf('/');
    return slash < 0 ? '' : path.substring(0, slash);
  }

  String _joinPath(String parent, String name) =>
      parent.isEmpty ? name : '$parent/$name';

  bool _canPaste(String directory) {
    final value = _clipboard.value;
    if (value == null ||
        _mutating ||
        (widget.protectWorkspaceRoots && directory.isEmpty)) {
      return false;
    }
    final source = value.entry.relativePath;
    if (value.cut && _parentPath(source) == directory) return false;
    return !value.entry.isDirectory ||
        (directory != source && !directory.startsWith('$source/'));
  }

  Future<void> _showActions(
    Offset position, {
    WorkspaceFileEntry? entry,
    required String directory,
  }) async {
    if (widget.onMutate == null || _mutating || _menuOpen) return;
    _menuOpen = true;
    if (entry != null) setState(() => _selected = entry.relativePath);
    final l10n = AppLocalizations.of(context)!;
    final generation = _generation;
    final mutable =
        entry != null &&
        !(widget.protectWorkspaceRoots && !entry.relativePath.contains('/'));
    final actions = <SidebarMenuAction<_FileAction>>[
      if (entry != null) ...[
        SidebarMenuAction(
          value: _FileAction.rename,
          icon: Icons.edit_outlined,
          label: l10n.workspaceFileRename,
          enabled: mutable,
        ),
        SidebarMenuAction(
          value: _FileAction.copy,
          icon: Icons.copy_outlined,
          label: l10n.workspaceFileCopy,
        ),
        SidebarMenuAction(
          value: _FileAction.cut,
          icon: Icons.content_cut,
          label: l10n.workspaceFileCut,
          enabled: mutable,
        ),
      ],
      SidebarMenuAction(
        value: _FileAction.paste,
        icon: Icons.content_paste,
        label: l10n.workspaceFilePaste,
        enabled: _canPaste(directory),
      ),
      if (entry != null)
        SidebarMenuAction(
          value: _FileAction.delete,
          icon: Icons.delete_outline,
          label: l10n.delete,
          enabled: mutable,
          danger: true,
          dividerBefore: true,
        ),
    ];
    final action = await showSidebarContextMenu(
      context,
      position: position,
      actions: actions,
    );
    _menuOpen = false;
    if (!mounted || generation != _generation || action == null) return;
    switch (action) {
      case _FileAction.copy:
      case _FileAction.cut:
        _clipboard.value = WorkspaceFileClipboardEntry(
          entry!,
          cut: action == _FileAction.cut,
        );
      case _FileAction.rename:
        await _rename(entry!);
      case _FileAction.paste:
        await _paste(directory);
      case _FileAction.delete:
        await _delete(entry!);
    }
  }

  Future<void> _rename(WorkspaceFileEntry entry) async {
    final generation = _generation;
    final name = await showDialog<String>(
      context: context,
      useRootNavigator: false,
      builder: (_) => WorkspaceFileRenameDialog(entry: entry),
    );
    if (!mounted ||
        generation != _generation ||
        name == null ||
        name == entry.name) {
      return;
    }
    await _mutate(
      WorkspaceFileOperation.move,
      entry.relativePath,
      _joinPath(_parentPath(entry.relativePath), name),
    );
  }

  Future<void> _delete(WorkspaceFileEntry entry) async {
    final generation = _generation;
    if (entry.isDirectory) {
      final l10n = AppLocalizations.of(context)!;
      final confirmed = await showDialog<bool>(
        context: context,
        useRootNavigator: false,
        builder: (context) => AlertDialog(
          title: Text(l10n.workspaceDeleteFolderTitle),
          content: Text(l10n.workspaceDeleteFolderMessage(entry.name)),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(l10n.cancel),
            ),
            FilledButton(
              style: FilledButton.styleFrom(
                backgroundColor: Theme.of(context).colorScheme.error,
              ),
              onPressed: () => Navigator.pop(context, true),
              child: Text(l10n.delete),
            ),
          ],
        ),
      );
      if (confirmed != true) return;
    }
    if (!mounted || generation != _generation) return;
    await _mutate(WorkspaceFileOperation.delete, entry.relativePath, null);
  }

  Future<void> _paste(String directory) async {
    if (!_canPaste(directory)) return;
    final clipboard = _clipboard.value!;
    final entry = clipboard.entry;
    final generation = _generation;
    setState(() => _mutating = true);
    try {
      var destination = _joinPath(directory, entry.name);
      if (!clipboard.cut && destination == entry.relativePath) {
        final entries = await widget.onListDirectory(directory);
        if (!mounted || generation != _generation) return;
        final names = entries.map((entry) => entry.name).toSet();
        final dot = entry.isDirectory ? -1 : entry.name.lastIndexOf('.');
        final stem = dot > 0 ? entry.name.substring(0, dot) : entry.name;
        final suffix = dot > 0 ? entry.name.substring(dot) : '';
        var index = 1;
        String name;
        do {
          name =
              '${AppLocalizations.of(context)!.workspaceFileCopyName(stem, index++)}$suffix';
        } while (names.contains(name));
        destination = _joinPath(directory, name);
      }
      if (!mounted || generation != _generation) return;
      await _mutate(
        clipboard.cut
            ? WorkspaceFileOperation.move
            : WorkspaceFileOperation.copy,
        entry.relativePath,
        destination,
      );
    } catch (error) {
      if (mounted) _showError(error);
    } finally {
      if (mounted) setState(() => _mutating = false);
    }
  }

  Future<void> _mutate(
    WorkspaceFileOperation operation,
    String path,
    String? destination,
  ) async {
    final mutate = widget.onMutate;
    if (mutate == null) return;
    setState(() => _mutating = true);
    try {
      await mutate(operation, path, destination);
      if (!mounted) return;
      final clipboard = _clipboard.value;
      if (operation != WorkspaceFileOperation.copy &&
          clipboard != null &&
          (clipboard.entry.relativePath == path ||
              clipboard.entry.relativePath.startsWith('$path/'))) {
        _clipboard.value = null;
      }
      // Forget stale expanded branches below a moved/deleted entry.
      if (operation != WorkspaceFileOperation.copy) {
        _expanded.removeWhere(
          (value) => value == path || value.startsWith('$path/'),
        );
        _selected = destination;
      }
      widget.onFilesChanged?.call();
    } catch (error) {
      if (mounted) _showError(error);
    } finally {
      if (mounted) setState(() => _mutating = false);
    }
  }

  void _showError(Object error) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          AppLocalizations.of(
            context,
          )!.workspaceFileOperationFailed(error.toString()),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final rows = _visibleRows();
    final rowHeight = math.max(
      30.0,
      MediaQuery.textScalerOf(context).scale(13) *
              (theme.textTheme.bodySmall?.height ?? 1.4) +
          8,
    );
    final rootName =
        widget.rootLabel
            .replaceAll('\\', '/')
            .split('/')
            .where((part) => part.isNotEmpty)
            .lastOrNull ??
        widget.rootLabel;
    return Column(
      children: [
        Semantics(
          expanded: _rootExpanded,
          child: InkWell(
            onLongPress: widget.onMutate == null
                ? null
                : () => _showActions(
                    (context.findRenderObject()! as RenderBox).localToGlobal(
                      const Offset(20, 20),
                    ),
                    directory: widget.rootPath,
                  ),
            onSecondaryTapUp: widget.onMutate == null
                ? null
                : (details) => _showActions(
                    details.globalPosition,
                    directory: widget.rootPath,
                  ),
            onTap: () => setState(() => _rootExpanded = !_rootExpanded),
            child: SizedBox(
              height: rowHeight + 4,
              child: Row(
                children: [
                  const SizedBox(width: 6),
                  Icon(
                    _rootExpanded ? Icons.expand_more : Icons.chevron_right,
                    size: 18,
                  ),
                  const SizedBox(width: 4),
                  Icon(Icons.folder_outlined, size: 16, color: colors.primary),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      rootName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                ],
              ),
            ),
          ),
        ),
        Expanded(
          child: Focus(
            focusNode: _focus,
            onKeyEvent: (_, event) => _onKey(event, rows, rowHeight),
            child: ScrollConfiguration(
              behavior: ScrollConfiguration.of(context).copyWith(
                dragDevices: const {
                  PointerDeviceKind.touch,
                  PointerDeviceKind.mouse,
                  PointerDeviceKind.trackpad,
                  PointerDeviceKind.stylus,
                },
              ),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onLongPressStart: widget.onMutate == null
                    ? null
                    : (details) => _showActions(
                        details.globalPosition,
                        directory: widget.rootPath,
                      ),
                onSecondaryTapUp: widget.onMutate == null
                    ? null
                    : (details) => _showActions(
                        details.globalPosition,
                        directory: widget.rootPath,
                      ),
                child: Scrollbar(
                  controller: _scroll,
                  child: ListView.builder(
                    controller: _scroll,
                    primary: false,
                    padding: const EdgeInsets.only(bottom: 8),
                    itemExtent: rowHeight,
                    itemCount: rows.length,
                    itemBuilder: (context, index) =>
                        _buildRow(context, rows[index]),
                  ),
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildRow(BuildContext context, _TreeRow row) {
    final colors = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final entry = row.entry;
    final selected = entry != null && _selected == row.path;
    final opened = entry != null && _expanded.contains(row.path);
    final loading = _loading.contains(row.path);
    final error = _errors[row.path];
    final label =
        entry?.name ?? (error != null ? l10n.retry : l10n.emptyFolder);
    return LayoutBuilder(
      builder: (context, constraints) {
        final indent = math.min(
          row.depth * 16.0,
          math.max(0.0, constraints.maxWidth - 110),
        );
        return Semantics(
          selected: selected,
          expanded: entry?.isDirectory == true ? opened : null,
          child: Material(
            color: selected
                ? colors.primary.withValues(alpha: 0.12)
                : Colors.transparent,
            child: GestureDetector(
              onLongPressStart: widget.onMutate == null
                  ? null
                  : (details) => _showActions(
                      details.globalPosition,
                      entry: entry,
                      directory: entry?.isDirectory == true
                          ? row.path
                          : entry != null
                          ? _parentPath(row.path)
                          : row.path,
                    ),
              child: InkWell(
                onSecondaryTapUp: widget.onMutate == null
                    ? null
                    : (details) => _showActions(
                        details.globalPosition,
                        entry: entry,
                        directory: entry?.isDirectory == true
                            ? row.path
                            : entry != null
                            ? _parentPath(row.path)
                            : row.path,
                      ),
                canRequestFocus: false,
                onTap: entry != null
                    ? () => _activate(entry)
                    : error != null
                    ? () => setState(() {
                        _load(row.path);
                      })
                    : null,
                child: Opacity(
                  opacity:
                      entry != null &&
                          _clipboard.value?.cut == true &&
                          _clipboard.value?.entry.relativePath == row.path
                      ? 0.45
                      : 1,
                  child: Tooltip(
                    triggerMode: widget.onMutate == null
                        ? null
                        : TooltipTriggerMode.manual,
                    message: entry?.relativePath ?? error?.toString() ?? label,
                    child: Stack(
                      alignment: Alignment.centerLeft,
                      children: [
                        for (
                          var level = 0;
                          level < row.depth && level * 16 < indent;
                          level++
                        )
                          Positioned(
                            left: 14 + level * 16,
                            top: 0,
                            bottom: 0,
                            child: VerticalDivider(
                              width: 1,
                              thickness: 1,
                              color: colors.outlineVariant.withValues(
                                alpha: 0.45,
                              ),
                            ),
                          ),
                        Padding(
                          padding: EdgeInsets.only(left: 6 + indent, right: 10),
                          child: Row(
                            children: [
                              SizedBox(
                                width: 18,
                                child: entry?.isDirectory == true
                                    ? Icon(
                                        opened
                                            ? Icons.expand_more
                                            : Icons.chevron_right,
                                        size: 16,
                                        color: colors.onSurfaceVariant,
                                      )
                                    : null,
                              ),
                              const SizedBox(width: 4),
                              if (entry != null)
                                entry.isDirectory
                                    ? Icon(
                                        opened
                                            ? Icons.folder_open
                                            : Icons.folder,
                                        size: 16,
                                        color: const Color(0xFFD4AD6A),
                                      )
                                    : buildWorkspaceFileIcon(
                                        entry.name,
                                        size: 16,
                                      )
                              else if (loading)
                                const SizedBox.square(
                                  dimension: 14,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 1.5,
                                  ),
                                )
                              else
                                Icon(
                                  error != null
                                      ? Icons.error_outline
                                      : Icons.folder_off_outlined,
                                  size: 14,
                                  color: error != null
                                      ? colors.error
                                      : colors.onSurfaceVariant,
                                ),
                              const SizedBox(width: 6),
                              Expanded(
                                child: Text(
                                  loading && entry == null ? '' : label,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: Theme.of(context).textTheme.bodySmall
                                      ?.copyWith(
                                        fontSize: 13,
                                        color: error != null && entry == null
                                            ? colors.error
                                            : entry == null
                                            ? colors.onSurfaceVariant
                                            : colors.onSurface,
                                      ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

enum _FileAction { rename, copy, cut, paste, delete }

class _TreeRow {
  const _TreeRow.entry(WorkspaceFileEntry this.entry, this.depth)
    : _messagePath = null;
  const _TreeRow.message(String path, this.depth)
    : entry = null,
      _messagePath = path;

  final WorkspaceFileEntry? entry;
  final String? _messagePath;
  final int depth;
  String get path => entry?.relativePath ?? _messagePath!;
}
