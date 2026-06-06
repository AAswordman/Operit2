// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspacePathBar.dart';
import 'WorkspaceTabModels.dart';

class WorkspaceFileBrowserContent extends StatefulWidget {
  const WorkspaceFileBrowserContent({
    super.key,
    required this.rootLabel,
    required this.rootRelativePath,
    required this.onListWorkspaceFiles,
    required this.onOpenFile,
  });

  final String rootLabel;
  final String rootRelativePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<void> Function(WorkspaceFileEntry entry) onOpenFile;

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
    if (oldWidget.rootRelativePath != widget.rootRelativePath) {
      _history.clear();
      _currentPath = widget.rootRelativePath;
      _loadCurrentPath();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return Column(
      children: <Widget>[
        WorkspacePathBar.editable(
          path: _displayPath(),
          controller: _pathController,
          isEditing: _editingPath,
          leading: WorkspacePathIconButton(
            tooltip: l10n.back,
            onPressed: _history.isEmpty ? null : _openPreviousPath,
            icon: Icons.arrow_back,
          ),
          onRefresh: () {
            setState(_loadCurrentPath);
          },
          onEditToggle: _startEditingPath,
          onSubmitted: _submitEditedPath,
        ),
        Expanded(
          child: FutureBuilder<List<WorkspaceFileEntry>>(
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
              final entries = snapshot.data ?? const <WorkspaceFileEntry>[];
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
                  child: ListView.separated(
                    controller: _scrollController,
                    primary: false,
                    physics: const AlwaysScrollableScrollPhysics(),
                    itemCount: entries.length,
                    separatorBuilder: (context, index) => Divider(
                      height: 1,
                      indent: 56,
                      color: theme.colorScheme.outlineVariant,
                    ),
                    itemBuilder: (context, index) {
                      final entry = entries[index];
                      final previewKind = entry.isDirectory
                          ? null
                          : workspacePreviewKindForPath(entry.path);
                      return ListTile(
                        dense: true,
                        leading: Icon(
                          entry.isDirectory
                              ? Icons.folder_outlined
                              : workspacePreviewIconForKind(previewKind!),
                          color: entry.isDirectory
                              ? theme.colorScheme.primary
                              : theme.colorScheme.onSurfaceVariant,
                        ),
                        title: Text(
                          entry.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: entry.isDirectory
                            ? null
                            : Text(_previewLabel(l10n, previewKind!)),
                        onTap: () {
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
      ],
    );
  }

  void _loadCurrentPath() {
    _entriesFuture = widget.onListWorkspaceFiles(_currentPath);
  }

  void _openDirectory(String path) {
    setState(() {
      _history.add(_currentPath);
      _currentPath = path;
      _loadCurrentPath();
    });
  }

  void _openPreviousPath() {
    setState(() {
      _currentPath = _history.removeLast();
      _editingPath = false;
      _loadCurrentPath();
    });
  }

  void _startEditingPath() {
    setState(() {
      _pathController.text = _displayPath();
      _pathController.selection = TextSelection.collapsed(
        offset: _pathController.text.length,
      );
      _editingPath = true;
    });
  }

  void _submitEditedPath(String value) {
    final normalizedPath = _relativePathFromDisplay(value);
    setState(() {
      _history.add(_currentPath);
      _currentPath = normalizedPath;
      _editingPath = false;
      _loadCurrentPath();
    });
  }

  String _displayPath() {
    if (_currentPath.isEmpty) {
      return widget.rootLabel;
    }
    return '${widget.rootLabel}/$_currentPath';
  }

  String _relativePathFromDisplay(String value) {
    final normalizedValue = value.trim().replaceAll('\\', '/');
    final normalizedRoot = widget.rootLabel.trim().replaceAll('\\', '/');
    if (normalizedValue == normalizedRoot) {
      return '';
    }
    if (normalizedValue.startsWith('$normalizedRoot/')) {
      return normalizedValue.substring(normalizedRoot.length + 1);
    }
    return normalizedValue.replaceFirst(RegExp(r'^/+'), '');
  }

  String _previewLabel(AppLocalizations l10n, WorkspaceFilePreviewKind kind) {
    switch (kind) {
      case WorkspaceFilePreviewKind.image:
        return l10n.imagePreview;
      case WorkspaceFilePreviewKind.audio:
        return l10n.audioPreview;
      case WorkspaceFilePreviewKind.video:
        return l10n.videoPreview;
      case WorkspaceFilePreviewKind.pdf:
        return l10n.pdfPreview;
      case WorkspaceFilePreviewKind.word:
        return l10n.wordPreview;
      case WorkspaceFilePreviewKind.spreadsheet:
        return l10n.spreadsheetPreview;
      case WorkspaceFilePreviewKind.presentation:
        return l10n.presentationPreview;
      case WorkspaceFilePreviewKind.html:
        return l10n.webPagePreview;
      case WorkspaceFilePreviewKind.markdown:
        return l10n.markdownPreview;
      case WorkspaceFilePreviewKind.text:
        return l10n.textPreview;
      case WorkspaceFilePreviewKind.binary:
        return l10n.file;
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
