// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceSetupContent.dart';
import 'WorkspaceProjectConfig.dart';
import 'WorkspaceTabContent.dart';
import 'WorkspaceTabModels.dart';
import 'WorkspaceTabStrip.dart';
import 'browser/automation/WorkspaceBrowserSessionRegistry.dart';

class WorkspacePanel extends StatefulWidget {
  const WorkspacePanel({
    super.key,
    required this.currentChatId,
    required this.hasBoundWorkspace,
    required this.workspacePath,
    required this.onListWorkspaceFiles,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onCreateDefaultWorkspace,
    required this.onBindWorkspace,
    required this.onRevealRequested,
  });

  final String? currentChatId;
  final bool hasBoundWorkspace;
  final String? workspacePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(String? projectType) onCreateDefaultWorkspace;
  final Future<void> Function(String workspace, String? workspaceEnv)
  onBindWorkspace;
  final VoidCallback onRevealRequested;

  @override
  State<WorkspacePanel> createState() => _WorkspacePanelState();
}

class _WorkspacePanelState extends State<WorkspacePanel> {
  final WorkspaceBrowserSessionRegistry _browserSessionRegistry =
      WorkspaceBrowserSessionRegistry.instance;
  final List<WorkspaceTab> _tabs = <WorkspaceTab>[
    const WorkspaceTab(
      kind: WorkspaceTabKind.home,
      title: '',
      icon: Icons.home_outlined,
      closable: false,
    ),
  ];
  int _selectedIndex = 0;

  @override
  void initState() {
    super.initState();
    _registerBrowserControls();
  }

  @override
  void didUpdateWidget(covariant WorkspacePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId) {
      _removeBrowserTabsForChatSwitch();
      final oldChatId = oldWidget.currentChatId;
      if (oldChatId != null && oldChatId.trim().isNotEmpty) {
        _browserSessionRegistry.clearChatControls(oldChatId);
      }
      _registerBrowserControls();
    }
  }

  @override
  void dispose() {
    final chatId = widget.currentChatId;
    if (chatId != null && chatId.trim().isNotEmpty) {
      _browserSessionRegistry.clearChatControls(chatId);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final showTabs = widget.hasBoundWorkspace || _hasBrowserTabs;
    return Material(
      color: theme.colorScheme.surfaceContainerLowest,
      child: SizedBox.expand(
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: BorderDirectional(
              start: BorderSide(color: theme.colorScheme.outlineVariant),
            ),
          ),
          child: showTabs
              ? Column(
                  children: <Widget>[
                    WorkspaceTabStrip(
                      tabs: _tabs,
                      selectedIndex: _selectedIndex,
                      onSelected: _selectTab,
                      onClosed: _closeTab,
                    ),
                    Expanded(
                      child: IndexedStack(
                        index: _selectedIndex,
                        children: <Widget>[
                          for (final tab in _tabs)
                            KeyedSubtree(
                              key: ValueKey<String>(_tabIdentity(tab)),
                              child: _buildTabContent(tab),
                            ),
                        ],
                      ),
                    ),
                  ],
                )
              : WorkspaceSetupContent(
                  onCreateDefaultWorkspace: widget.onCreateDefaultWorkspace,
                  onBindWorkspace: widget.onBindWorkspace,
                ),
        ),
      ),
    );
  }

  bool get _hasBrowserTabs {
    return _tabs.any((tab) => tab.kind == WorkspaceTabKind.browser);
  }

  void _registerBrowserControls() {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      return;
    }
    _browserSessionRegistry.setChatControls(
      chatId: chatId,
      openBrowserTab: _openBrowserTab,
    );
  }

  void _removeBrowserTabsForChatSwitch() {
    if (!_hasBrowserTabs) {
      return;
    }
    setState(() {
      _tabs.removeWhere((tab) => tab.kind == WorkspaceTabKind.browser);
      if (_selectedIndex >= _tabs.length) {
        _selectedIndex = _tabs.length - 1;
      }
    });
  }

  void _selectTab(int index) {
    setState(() {
      _selectedIndex = index;
    });
  }

  void _openSingletonTab(WorkspaceTab tab) {
    final existingIndex = _tabs.indexWhere((item) => item.kind == tab.kind);
    setState(() {
      if (existingIndex >= 0) {
        _selectedIndex = existingIndex;
      } else {
        _tabs.add(tab);
        _selectedIndex = _tabs.length - 1;
      }
    });
  }

  void _openBrowserTab({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  }) {
    widget.onRevealRequested();
    final title = _browserTabTitle(
      url: url,
      localFilePath: localFilePath,
      workspaceHtmlPath: workspaceHtmlPath,
    );
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.browser,
      title: title,
      icon: Icons.public,
      url: url,
      absolutePath: localFilePath,
      workspaceHtmlPath: workspaceHtmlPath,
    );
    setState(() {
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
    });
  }

  String _browserTabTitle({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  }) {
    final htmlPath = workspaceHtmlPath?.trim();
    if (htmlPath != null && htmlPath.isNotEmpty) {
      return htmlPath.split(RegExp(r'[\\/]')).last;
    }
    final filePath = localFilePath?.trim();
    if (filePath != null && filePath.isNotEmpty) {
      return filePath.split(RegExp(r'[\\/]')).last;
    }
    final rawUrl = url?.trim();
    if (rawUrl != null && rawUrl.isNotEmpty) {
      final uri = Uri.tryParse(rawUrl);
      return uri?.host.isNotEmpty == true ? uri!.host : rawUrl;
    }
    return '';
  }

  Widget _buildTabContent(WorkspaceTab tab) {
    final currentChatId = widget.currentChatId;
    if (currentChatId == null || currentChatId.trim().isEmpty) {
      return const Center(child: Icon(Icons.public_off_outlined, size: 40));
    }
    return WorkspaceTabContent(
      tab: tab,
      currentChatId: currentChatId,
      workspacePath: widget.workspacePath,
      onListWorkspaceFiles: widget.onListWorkspaceFiles,
      onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: widget.onWriteWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      onOpenFile: _openFileTab,
      onOpenFiles: () {
        _openSingletonTab(
          const WorkspaceTab(
            kind: WorkspaceTabKind.files,
            title: '',
            icon: Icons.folder_outlined,
          ),
        );
      },
      onOpenTerminal: () {
        _openSingletonTab(
          const WorkspaceTab(
            kind: WorkspaceTabKind.terminal,
            title: '',
            icon: Icons.terminal,
          ),
        );
      },
      onOpenBrowser: _openBrowserTab,
      onOpenProjectBrowser: _openProjectBrowserTab,
    );
  }

  String _tabIdentity(WorkspaceTab tab) {
    return <String>[
      tab.kind.name,
      tab.filePath ?? '',
      tab.absolutePath ?? '',
      tab.url ?? '',
      tab.workspaceHtmlPath ?? '',
      tab.title,
    ].join('|');
  }

  void _closeTab(int index) {
    if (index <= 0 || index >= _tabs.length) {
      return;
    }
    setState(() {
      _tabs.removeAt(index);
      if (_selectedIndex == index) {
        _selectedIndex = (index - 1).clamp(0, _tabs.length - 1);
      } else if (_selectedIndex > index) {
        _selectedIndex -= 1;
      }
    });
  }

  Future<void> _openFileTab(WorkspaceFileEntry entry) async {
    final previewKind = workspacePreviewKindForPath(entry.path);
    var content = '';
    if (previewKind == WorkspaceFilePreviewKind.text ||
        previewKind == WorkspaceFilePreviewKind.markdown ||
        previewKind == WorkspaceFilePreviewKind.html) {
      content = await widget.onReadWorkspaceTextFile(entry.relativePath);
    }

    if (!mounted) {
      return;
    }

    final existingIndex = _tabs.indexWhere(
      (item) => item.filePath == entry.path,
    );
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.filePreview,
      title: entry.name,
      icon: workspacePreviewIconForKind(previewKind),
      filePath: entry.relativePath,
      absolutePath: entry.path,
      fileContent: content,
      previewKind: previewKind,
    );
    setState(() {
      if (existingIndex >= 0) {
        _tabs[existingIndex] = tab;
        _selectedIndex = existingIndex;
      } else {
        _tabs.add(tab);
        _selectedIndex = _tabs.length - 1;
      }
    });
  }

  Future<void> _openProjectBrowserTab() async {
    final configText = await widget.onReadWorkspaceTextFile(
      '.operit/config.json',
    );
    final config = WorkspaceProjectConfig.fromJsonText(configText);
    _openBrowserTab(
      url: config.previewUrl.trim(),
      workspaceHtmlPath: 'index.html',
    );
  }
}
