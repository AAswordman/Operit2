// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../viewmodel/WorkspaceFileModels.dart';
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
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: Column(
        children: <Widget>[
          DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
              border: Border(
                bottom: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
              child: Row(
                children: <Widget>[
                  IconButton(
                    tooltip: '返回',
                    onPressed: _history.isEmpty ? null : _openPreviousPath,
                    icon: const Icon(Icons.arrow_back),
                  ),
                  Expanded(
                    child: Text(
                      _displayPath(),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: '刷新',
                    onPressed: () {
                      setState(_loadCurrentPath);
                    },
                    icon: const Icon(Icons.refresh),
                  ),
                ],
              ),
            ),
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
                  return const _WorkspaceFileMessage(
                    icon: Icons.folder_off_outlined,
                    message: '这个文件夹是空的',
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
                              : Text(_previewLabel(previewKind!)),
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
      ),
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
      _loadCurrentPath();
    });
  }

  String _displayPath() {
    if (_currentPath.isEmpty) {
      return widget.rootLabel;
    }
    return '${widget.rootLabel}/$_currentPath';
  }

  String _previewLabel(WorkspaceFilePreviewKind kind) {
    switch (kind) {
      case WorkspaceFilePreviewKind.image:
        return '图片预览';
      case WorkspaceFilePreviewKind.audio:
        return '音频预览';
      case WorkspaceFilePreviewKind.video:
        return '视频预览';
      case WorkspaceFilePreviewKind.pdf:
        return 'PDF 预览';
      case WorkspaceFilePreviewKind.word:
        return 'Word 预览';
      case WorkspaceFilePreviewKind.spreadsheet:
        return '表格预览';
      case WorkspaceFilePreviewKind.html:
        return '网页预览';
      case WorkspaceFilePreviewKind.markdown:
        return 'Markdown 预览';
      case WorkspaceFilePreviewKind.text:
        return '文本预览';
      case WorkspaceFilePreviewKind.binary:
        return '文件';
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
