// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspacePathBar.dart';
import 'WorkspaceFileIcon.dart';
import 'WorkspaceFileTree.dart';

class WorkspaceFileBrowserContent extends StatefulWidget {
  const WorkspaceFileBrowserContent({
    super.key,
    required this.rootLabel,
    required this.rootRelativePath,
    required this.onListWorkspaceFiles,
    required this.onOpenFile,
    this.onSelectCurrentDirectory,
    this.onPickLocalDirectory,
  });

  final String rootLabel;
  final String rootRelativePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<void> Function(WorkspaceFileEntry entry) onOpenFile;
  final Future<void> Function(String path)? onSelectCurrentDirectory;
  final Future<void> Function()? onPickLocalDirectory;

  @override
  State<WorkspaceFileBrowserContent> createState() =>
      _WorkspaceFileBrowserContentState();
}

class _WorkspaceFileBrowserContentState
    extends State<WorkspaceFileBrowserContent> {
  late String _currentPath;
  final List<String> _history = <String>[];
  final ScrollController _scrollController = ScrollController();
  final TextEditingController _pathController = TextEditingController();
  bool _editingPath = false;
  // Freeze navigation so a pending bind and its errors stay on the selected path.
  bool _selectingCurrentDirectory = false;
  String? _selectionError;
  String? _pathError;
  Future<List<WorkspaceFileEntry>>? _entriesFuture;

  @override
  void initState() {
    super.initState();
    _currentPath = widget.rootRelativePath;
    _loadCurrentPath();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _pathController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(WorkspaceFileBrowserContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.rootLabel != widget.rootLabel ||
        oldWidget.rootRelativePath != widget.rootRelativePath) {
      _history.clear();
      _currentPath = widget.rootRelativePath;
      _pathError = null;
      _loadCurrentPath();
    }
  }

  /// Bounds directory-selection diagnostics by the actual panel viewport.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return LayoutBuilder(
      builder: (context, constraints) => Column(
        children: <Widget>[
          WorkspacePathBar.editable(
            path: _displayPath(),
            controller: _pathController,
            isEditing: _editingPath && !_selectingCurrentDirectory,
            leading: WorkspacePathIconButton(
              tooltip: l10n.back,
              onPressed: _history.isEmpty || _selectingCurrentDirectory
                  ? null
                  : _openPreviousPath,
              icon: Icons.arrow_back,
            ),
            onRefresh: _selectingCurrentDirectory
                ? null
                : () => setState(_loadCurrentPath),
            onEditToggle: _selectingCurrentDirectory ? null : _startEditingPath,
            onSubmitted: _selectingCurrentDirectory ? null : _submitEditedPath,
          ),
          if (_pathError != null)
            Align(
              alignment: Alignment.centerLeft,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 6, 12, 0),
                child: Text(
                  _pathError!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.error,
                  ),
                ),
              ),
            ),
          Expanded(
            child: !_directorySelectionEnabled
                ? WorkspaceFileTree(
                    key: ValueKey(_currentPath),
                    rootPath: _currentPath,
                    rootLabel: _displayPath(),
                    rootEntries: _entriesFuture!,
                    onListDirectory: _listTreeDirectory,
                    onOpenFile: widget.onOpenFile,
                  )
                : FutureBuilder<List<WorkspaceFileEntry>>(
                    future: _entriesFuture,
                    builder: (context, snapshot) {
                      if (snapshot.connectionState != ConnectionState.done) {
                        return const Center(child: CircularProgressIndicator());
                      }
                      if (snapshot.hasError) {
                        return _WorkspaceFileMessage(
                          icon: Icons.error_outline,
                          message: snapshot.error.toString(),
                        );
                      }
                      final entries =
                          snapshot.data ?? const <WorkspaceFileEntry>[];
                      if (entries.isEmpty) {
                        return _WorkspaceFileMessage(
                          icon: Icons.folder_off_outlined,
                          message: l10n.emptyFolder,
                        );
                      }
                      return ScrollConfiguration(
                        behavior: ScrollConfiguration.of(context).copyWith(
                          dragDevices: const <PointerDeviceKind>{
                            PointerDeviceKind.touch,
                            PointerDeviceKind.mouse,
                            PointerDeviceKind.trackpad,
                            PointerDeviceKind.stylus,
                          },
                        ),
                        child: Scrollbar(
                          controller: _scrollController,
                          thumbVisibility: true,
                          child: ListView.builder(
                            controller: _scrollController,
                            primary: false,
                            padding: EdgeInsets.zero,
                            physics: const AlwaysScrollableScrollPhysics(),
                            itemCount: entries.length,
                            itemBuilder: (context, index) {
                              final entry = entries[index];
                              return ListTile(
                                dense: true,
                                visualDensity: VisualDensity.compact,
                                minTileHeight: 36,
                                minLeadingWidth: 16,
                                horizontalTitleGap: 8,
                                contentPadding: const EdgeInsets.symmetric(
                                  horizontal: 14,
                                ),
                                leading: entry.isDirectory
                                    ? const Icon(
                                        Icons.folder,
                                        size: 17,
                                        color: Color(0xFFD4AD6A),
                                      )
                                    : buildWorkspaceFileIcon(
                                        entry.name,
                                        size: 16,
                                      ),
                                title: Text(
                                  entry.name,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.bodySmall?.copyWith(
                                    fontSize: 13,
                                  ),
                                ),
                                trailing: entry.isDirectory
                                    ? Icon(
                                        Icons.chevron_right,
                                        size: 16,
                                        color:
                                            theme.colorScheme.onSurfaceVariant,
                                      )
                                    : null,
                                onTap: _selectingCurrentDirectory
                                    ? null
                                    : () {
                                        if (entry.isDirectory) {
                                          _openDirectory(entry.relativePath);
                                          return;
                                        }
                                        widget.onOpenFile(entry);
                                      },
                              );
                            },
                          ),
                        ),
                      );
                    },
                  ),
          ),
          if (_directorySelectionEnabled)
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: constraints.maxHeight / 2),
              child: _buildDirectorySelectionBar(context),
            ),
        ],
      ),
    );
  }

  bool get _directorySelectionEnabled =>
      widget.onSelectCurrentDirectory != null;

  /// Requests entries for the current path only after validating its namespace.
  void _loadCurrentPath() {
    if (!_directorySelectionEnabled &&
        !_isWorkspaceRelativePath(_currentPath)) {
      _entriesFuture = Future<List<WorkspaceFileEntry>>.error(
        StateError('工作区目录路径必须是相对路径'),
      );
      return;
    }
    _entriesFuture = widget.onListWorkspaceFiles(_currentPath);
  }

  Future<List<WorkspaceFileEntry>> _listTreeDirectory(String path) {
    if (!_isWorkspaceRelativePath(path)) {
      return Future.error(StateError('工作区目录路径必须是相对路径'));
    }
    return widget.onListWorkspaceFiles(path);
  }

  /// Opens a directory returned by the active workspace listing service.
  void _openDirectory(String path) {
    if (_selectingCurrentDirectory) {
      return;
    }
    if (!_directorySelectionEnabled && !_isWorkspaceRelativePath(path)) {
      setState(() {
        _pathError = '工作区目录路径必须是相对路径';
      });
      return;
    }
    setState(() {
      _history.add(_currentPath);
      _currentPath = path;
      _selectionError = null;
      _pathError = null;
      _loadCurrentPath();
    });
  }

  /// Restores the previous valid workspace-relative directory.
  void _openPreviousPath() {
    if (_selectingCurrentDirectory) {
      return;
    }
    setState(() {
      _currentPath = _history.removeLast();
      _editingPath = false;
      _selectionError = null;
      _pathError = null;
      _loadCurrentPath();
    });
  }

  /// Starts editing the displayed workspace path.
  void _startEditingPath() {
    if (_selectingCurrentDirectory) {
      return;
    }
    setState(() {
      _pathController.text = _displayPath();
      _pathController.selection = TextSelection.collapsed(
        offset: _pathController.text.length,
      );
      _editingPath = true;
    });
  }

  /// Converts and submits a displayed path within the current workspace root.
  void _submitEditedPath(String value) {
    if (_selectingCurrentDirectory) {
      return;
    }
    final normalizedPath = _relativePathFromDisplay(value);
    if (normalizedPath == null) {
      setState(() {
        _editingPath = false;
        _pathError = '路径必须位于当前工作区内';
      });
      return;
    }
    setState(() {
      _history.add(_currentPath);
      _currentPath = normalizedPath;
      _editingPath = false;
      _selectionError = null;
      _pathError = null;
      _loadCurrentPath();
    });
  }

  /// Builds the user-facing path while keeping API paths relative.
  String _displayPath() {
    if (_directorySelectionEnabled) {
      return _currentPath;
    }
    if (_currentPath.isEmpty) {
      return widget.rootLabel;
    }
    return '${widget.rootLabel}/$_currentPath';
  }

  /// Converts a displayed workspace path into its relative API path.
  String? _relativePathFromDisplay(String value) {
    final normalizedValue = value.trim().replaceAll('\\', '/');
    if (_directorySelectionEnabled) {
      return normalizedValue;
    }
    final normalizedRoot = widget.rootLabel
        .trim()
        .replaceAll('\\', '/')
        .replaceFirst(RegExp(r'/+$'), '');
    if (normalizedValue == normalizedRoot) {
      return '';
    }
    if (normalizedValue.startsWith('$normalizedRoot/')) {
      final relativePath = normalizedValue.substring(normalizedRoot.length + 1);
      return _isWorkspaceRelativePath(relativePath) ? relativePath : null;
    }
    return null;
  }

  /// Validates that a file-browser path stays in the workspace-relative namespace.
  bool _isWorkspaceRelativePath(String path) {
    final normalizedPath = path.trim().replaceAll('\\', '/');
    if (normalizedPath.isEmpty || normalizedPath.startsWith('/')) {
      return normalizedPath.isEmpty;
    }
    final segments = normalizedPath.split('/');
    return segments.every(
      (segment) => segment.isNotEmpty && segment != '.' && segment != '..',
    );
  }

  /// Keeps selection actions visible while long errors scroll within the footer.
  Widget _buildDirectorySelectionBar(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(
          top: BorderSide(color: theme.colorScheme.outlineVariant),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              _displayPath(),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.icon(
                  onPressed: _selectingCurrentDirectory
                      ? null
                      : _selectCurrentDirectory,
                  icon: _selectingCurrentDirectory
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.check),
                  label: Text(l10n.workspaceBindExistingTitle),
                ),
                if (widget.onPickLocalDirectory != null)
                  OutlinedButton.icon(
                    onPressed: _selectingCurrentDirectory
                        ? null
                        : _pickLocalDirectory,
                    icon: const Icon(Icons.folder_open),
                    label: Text(l10n.workspacePickLocalFolder),
                  ),
              ],
            ),
            if (_selectionError != null) ...<Widget>[
              const SizedBox(height: 6),
              Flexible(
                child: SingleChildScrollView(
                  child: Text(
                    _selectionError!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Future<void> _pickLocalDirectory() async {
    final picker = widget.onPickLocalDirectory;
    if (picker == null || _selectingCurrentDirectory) return;
    setState(() {
      _selectingCurrentDirectory = true;
      _selectionError = null;
    });
    try {
      await picker();
    } catch (error, stackTrace) {
      debugPrint(
        'Local workspace folder selection failed: $error\n$stackTrace',
      );
      if (mounted) setState(() => _selectionError = error.toString());
    } finally {
      if (mounted) setState(() => _selectingCurrentDirectory = false);
    }
  }

  Future<void> _selectCurrentDirectory() async {
    final onSelect = widget.onSelectCurrentDirectory;
    if (onSelect == null || _selectingCurrentDirectory) {
      return;
    }
    setState(() {
      _selectingCurrentDirectory = true;
      _selectionError = null;
    });
    try {
      await onSelect(_currentPath);
    } catch (error, stackTrace) {
      debugPrint('Workspace directory selection failed: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _selectionError = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _selectingCurrentDirectory = false;
        });
      }
    }
  }
}

class _WorkspaceFileMessage extends StatelessWidget {
  const _WorkspaceFileMessage({required this.icon, required this.message});

  final IconData icon;
  final String message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 36, color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(height: 10),
            Text(
              message,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
