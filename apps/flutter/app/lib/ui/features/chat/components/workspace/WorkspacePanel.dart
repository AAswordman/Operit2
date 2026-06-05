// ignore_for_file: file_names

import 'dart:typed_data';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';

import '../../../../theme/OperitGlassSurface.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceTabContent.dart';
import 'WorkspaceTabModels.dart';
import 'WorkspaceTabStrip.dart';
import 'browser/automation/WorkspaceBrowserSessionRegistry.dart';
import 'browser/automation/WorkspaceWebVisitSessionRegistry.dart';
import 'terminal/WorkspaceTerminalSessions.dart';

class WorkspacePanel extends StatefulWidget {
  const WorkspacePanel({
    super.key,
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
  final WorkspaceWebVisitSessionRegistry _webVisitSessionRegistry =
      WorkspaceWebVisitSessionRegistry.instance;
  final WorkspaceTerminalSessions _terminalSessions =
      const WorkspaceTerminalSessions();
  final Map<String, Completer<WebVisitResponse>> _webVisitCompleters =
      <String, Completer<WebVisitResponse>>{};
  final List<WorkspaceTab> _tabs = <WorkspaceTab>[
    const WorkspaceTab(
      kind: WorkspaceTabKind.home,
      title: '',
      icon: Icons.home_outlined,
      closable: false,
    ),
  ];
  int _selectedIndex = 0;
  List<WorkspaceTerminalSessionInfo> _terminalSessionEntries =
      const <WorkspaceTerminalSessionInfo>[];
  final ValueNotifier<int> _terminalSessionCount = ValueNotifier<int>(0);
  Timer? _terminalSessionRefreshTimer;

  @override
  void initState() {
    super.initState();
    _registerBrowserControls();
    _registerWebVisitControls();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refreshTerminalSessions();
    });
    _terminalSessionRefreshTimer = Timer.periodic(
      const Duration(seconds: 2),
      (_) => _refreshTerminalSessions(),
    );
  }

  @override
  void didUpdateWidget(covariant WorkspacePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    _registerBrowserControls();
    _registerWebVisitControls();
    if (!oldWidget.hasBoundWorkspace && widget.hasBoundWorkspace) {
      _replaceSetupTabWithFilesTab();
    }
  }

  @override
  void dispose() {
    _browserSessionRegistry.clearBrowserControls();
    _webVisitSessionRegistry.clearControls();
    for (final entry in _webVisitCompleters.entries) {
      final completer = entry.value;
      if (!completer.isCompleted) {
        completer.complete(
          WebVisitResponse(
            requestId: entry.key,
            success: false,
            error: 'visit_web workspace closed',
          ),
        );
      }
    }
    _webVisitCompleters.clear();
    _terminalSessionRefreshTimer?.cancel();
    _terminalSessionCount.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: theme.colorScheme.surface,
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.035,
      borderRadius: BorderRadius.zero,
      clip: false,
      child: SizedBox.expand(
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: BorderDirectional(
              start: BorderSide(color: theme.colorScheme.outlineVariant),
            ),
          ),
          child: Column(
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
          ),
        ),
      ),
    );
  }

  void _registerBrowserControls() {
    _browserSessionRegistry.setBrowserControls(openBrowserTab: _openBrowserTab);
  }

  void _registerWebVisitControls() {
    _webVisitSessionRegistry.setControls(openWebVisitTab: _openWebVisitTab);
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
    String? userAgent,
    Map<String, String>? headers,
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
      userAgent: userAgent,
      headers: headers,
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

  Future<WebVisitResponse> _openWebVisitTab(WebVisitRequest request) {
    widget.onRevealRequested();
    final completer = Completer<WebVisitResponse>();
    _webVisitCompleters[request.requestId] = completer;
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.webVisit,
      title: _webVisitTabTitle(request),
      icon: Icons.travel_explore,
      webVisitRequest: request,
    );
    setState(() {
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
    });
    return completer.future;
  }

  String _webVisitTabTitle(WebVisitRequest request) {
    final uri = Uri.tryParse(request.url.trim());
    if (uri != null && uri.host.isNotEmpty) {
      return uri.host;
    }
    return 'visit_web';
  }

  Widget _buildTabContent(WorkspaceTab tab) {
    return WorkspaceTabContent(
      tab: tab,
      workspacePath: widget.workspacePath,
      terminalSessionCountListenable: _terminalSessionCount,
      browserSessionRegistry: _browserSessionRegistry,
      onListWorkspaceFiles: widget.onListWorkspaceFiles,
      onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: widget.onWriteWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      onOpenFile: _openFileTab,
      onOpenFiles: _openFilesTab,
      onOpenTerminal: _createAndOpenTerminalSession,
      onOpenTerminalSessions: _showTerminalSessionPicker,
      onOpenBrowserSessions: _showBrowserSessionPicker,
      onOpenBrowser: _openBrowserTab,
      onFinishWebVisit: _finishWebVisitTab,
      onActivateCurrentTab: () => _selectWorkspaceTab(tab),
      onCloseCurrentTab: () => _closeWorkspaceTab(tab),
      onCreateDefaultWorkspace: widget.onCreateDefaultWorkspace,
      onBindWorkspace: widget.onBindWorkspace,
    );
  }

  Future<void> _refreshTerminalSessions() async {
    final sessions = await _terminalSessions.listSessions();
    if (!mounted) {
      return;
    }
    _updateTerminalSessionEntries(sessions);
  }

  Future<void> _createAndOpenTerminalSession() async {
    final workingDirectory = widget.workspacePath?.trim();
    if (workingDirectory == null || workingDirectory.isEmpty) {
      _openWorkspaceSetupTab();
      return;
    }
    final sessionId = await _terminalSessions.startPtySession(
      sessionName: _nextManualTerminalSessionName(),
      workingDirectory: workingDirectory,
      rows: 24,
      columns: 80,
    );
    final sessions = await _terminalSessions.listSessions();
    final session = sessions.firstWhere((item) => item.sessionId == sessionId);
    if (!mounted) {
      return;
    }
    _updateTerminalSessionEntries(sessions);
    _openTerminalSessionTab(session);
  }

  String _nextManualTerminalSessionName() {
    final manualCount = _terminalSessionEntries
        .where(
          (session) =>
              session.sessionKind == 'pty' &&
              session.sessionName.trim().startsWith('手动终端'),
        )
        .length;
    return '手动终端 ${manualCount + 1}';
  }

  Future<void> _showTerminalSessionPicker() async {
    final sessions = await _terminalSessions.listSessions();
    if (!mounted) {
      return;
    }
    _updateTerminalSessionEntries(sessions);
    await showDialog<void>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        var dialogSessions = sessions;
        final closingSessionIds = <String>{};
        return StatefulBuilder(
          builder: (context, setDialogState) {
            Future<void> closeSession(
              WorkspaceTerminalSessionInfo session,
            ) async {
              setDialogState(() {
                closingSessionIds.add(session.sessionId);
              });
              await _terminalSessions.closePtySession(session.sessionId);
              _removeTerminalTabsForSession(session.sessionId);
              final updatedSessions = await _terminalSessions.listSessions();
              if (!mounted) {
                return;
              }
              _updateTerminalSessionEntries(updatedSessions);
              setDialogState(() {
                closingSessionIds.remove(session.sessionId);
                dialogSessions = updatedSessions;
              });
            }

            return AlertDialog(
              title: const Text('终端会话'),
              contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
              content: SizedBox(
                width: 520,
                child: dialogSessions.isEmpty
                    ? Text(
                        '当前没有终端会话',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    : ConstrainedBox(
                        constraints: const BoxConstraints(maxHeight: 420),
                        child: ListView.separated(
                          shrinkWrap: true,
                          itemCount: dialogSessions.length,
                          separatorBuilder: (context, index) =>
                              const Divider(height: 1),
                          itemBuilder: (context, index) {
                            final session = dialogSessions[index];
                            final canClose = session.sessionKind == 'pty';
                            final isClosing = closingSessionIds.contains(
                              session.sessionId,
                            );
                            return ListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              title: Text(session.title),
                              subtitle: Padding(
                                padding: const EdgeInsets.only(top: 4),
                                child: Text(
                                  _terminalSessionSubtitle(session),
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              trailing: canClose
                                  ? IconButton(
                                      tooltip: '结束进程',
                                      onPressed: isClosing
                                          ? null
                                          : () => closeSession(session),
                                      icon: isClosing
                                          ? const SizedBox.square(
                                              dimension: 18,
                                              child: CircularProgressIndicator(
                                                strokeWidth: 2,
                                              ),
                                            )
                                          : const Icon(
                                              Icons.stop_circle_outlined,
                                            ),
                                    )
                                  : null,
                              onTap: () {
                                Navigator.of(context).pop();
                                _openTerminalSessionTab(session);
                              },
                            );
                          },
                        ),
                      ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('关闭'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _showBrowserSessionPicker() async {
    await showDialog<void>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        var dialogSessions = _browserSessionRegistry.sessions;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            void refreshDialogSessions() {
              setDialogState(() {
                dialogSessions = _browserSessionRegistry.sessions;
              });
            }

            return AlertDialog(
              title: const Text('浏览器会话'),
              contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
              content: SizedBox(
                width: 520,
                child: dialogSessions.isEmpty
                    ? Text(
                        '当前没有浏览器会话',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    : ConstrainedBox(
                        constraints: const BoxConstraints(maxHeight: 420),
                        child: ListView.separated(
                          shrinkWrap: true,
                          itemCount: dialogSessions.length,
                          separatorBuilder: (context, index) =>
                              const Divider(height: 1),
                          itemBuilder: (context, index) {
                            final session = dialogSessions[index];
                            return ListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              title: Text(session.title),
                              subtitle: Padding(
                                padding: const EdgeInsets.only(top: 4),
                                child: Text(
                                  _browserSessionSubtitle(session),
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              trailing: IconButton(
                                tooltip: '关闭会话',
                                onPressed: () {
                                  _browserSessionRegistry.closeTab(
                                    session.sessionId,
                                  );
                                  refreshDialogSessions();
                                },
                                icon: const Icon(Icons.close),
                              ),
                              onTap: () {
                                Navigator.of(context).pop();
                                widget.onRevealRequested();
                                _browserSessionRegistry.selectTab(
                                  session.sessionId,
                                );
                              },
                            );
                          },
                        ),
                      ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('关闭'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  void _updateTerminalSessionEntries(
    List<WorkspaceTerminalSessionInfo> sessions,
  ) {
    _terminalSessionEntries = sessions;
    _terminalSessionCount.value = sessions.length;
  }

  void _removeTerminalTabsForSession(String sessionId) {
    final selectedTab = _tabs[_selectedIndex];
    setState(() {
      _tabs.removeWhere((tab) => tab.terminalSessionId == sessionId);
      final preservedIndex = _tabs.indexOf(selectedTab);
      if (preservedIndex >= 0) {
        _selectedIndex = preservedIndex;
      } else {
        _selectedIndex = _selectedIndex.clamp(0, _tabs.length - 1).toInt();
      }
    });
  }

  String _terminalSessionSubtitle(WorkspaceTerminalSessionInfo session) {
    final workingDir = session.workingDir.trim();
    final prefix = session.sessionKind == 'shell' ? 'AI' : 'PTY';
    if (workingDir.isNotEmpty) {
      return 'UUID · ${session.sessionId}\n$prefix · $workingDir';
    }
    return 'UUID · ${session.sessionId}\n$prefix · ${session.terminalType}';
  }

  String _browserSessionSubtitle(WorkspaceBrowserSessionInfo session) {
    return 'UUID · ${session.sessionId}\n${session.url}';
  }

  void _openTerminalSessionTab(WorkspaceTerminalSessionInfo session) {
    final existingIndex = _tabs.indexWhere(
      (tab) => tab.terminalSessionId == session.sessionId,
    );
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.terminal,
      title: session.title,
      icon: Icons.terminal,
      terminalSessionId: session.sessionId,
      terminalSessionKind: session.sessionKind,
      terminalType: session.terminalType,
      terminalWorkingDir: session.workingDir,
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

  void _openFilesTab() {
    if (!widget.hasBoundWorkspace) {
      _openWorkspaceSetupTab();
      return;
    }
    _openSingletonTab(
      const WorkspaceTab(
        kind: WorkspaceTabKind.files,
        title: '',
        icon: Icons.folder_outlined,
      ),
    );
  }

  void _openWorkspaceSetupTab() {
    _openSingletonTab(
      const WorkspaceTab(
        kind: WorkspaceTabKind.setup,
        title: '',
        icon: Icons.tune_outlined,
      ),
    );
  }

  void _replaceSetupTabWithFilesTab() {
    final setupIndex = _tabs.indexWhere(
      (tab) => tab.kind == WorkspaceTabKind.setup,
    );
    if (setupIndex < 0) {
      return;
    }
    final selectedTab = _tabs[_selectedIndex];
    setState(() {
      _tabs.removeWhere((tab) => tab.kind == WorkspaceTabKind.setup);
      var filesIndex = _tabs.indexWhere(
        (tab) => tab.kind == WorkspaceTabKind.files,
      );
      if (filesIndex < 0) {
        _tabs.add(
          const WorkspaceTab(
            kind: WorkspaceTabKind.files,
            title: '',
            icon: Icons.folder_outlined,
          ),
        );
        filesIndex = _tabs.length - 1;
      }
      if (selectedTab.kind == WorkspaceTabKind.setup) {
        _selectedIndex = filesIndex;
        return;
      }
      final preservedIndex = _tabs.indexOf(selectedTab);
      _selectedIndex = preservedIndex >= 0
          ? preservedIndex
          : _selectedIndex.clamp(0, _tabs.length - 1).toInt();
    });
  }

  String _tabIdentity(WorkspaceTab tab) {
    return <String>[
      tab.kind.name,
      tab.filePath ?? '',
      tab.absolutePath ?? '',
      tab.url ?? '',
      tab.workspaceHtmlPath ?? '',
      tab.webVisitRequest?.requestId ?? '',
      tab.terminalSessionId ?? '',
      tab.title,
    ].join('|');
  }

  void _closeTab(int index) {
    if (index <= 0 || index >= _tabs.length) {
      return;
    }
    final tab = _tabs[index];
    setState(() {
      _tabs.removeAt(index);
      if (_selectedIndex == index) {
        _selectedIndex = (index - 1).clamp(0, _tabs.length - 1);
      } else if (_selectedIndex > index) {
        _selectedIndex -= 1;
      }
    });
    if (tab.kind == WorkspaceTabKind.terminal &&
        tab.terminalSessionKind == 'pty') {
      final sessionId = tab.terminalSessionId;
      if (sessionId != null && sessionId.trim().isNotEmpty) {
        unawaited(_closeManualTerminalSession(sessionId));
      }
    }
    if (tab.kind == WorkspaceTabKind.webVisit) {
      final request = tab.webVisitRequest;
      if (request != null) {
        _completeWebVisitRequest(
          request.requestId,
          WebVisitResponse(
            requestId: request.requestId,
            success: false,
            error: 'visit_web cancelled',
          ),
        );
      }
    }
  }

  void _selectWorkspaceTab(WorkspaceTab tab) {
    final index = _tabs.indexOf(tab);
    if (index < 0) {
      return;
    }
    widget.onRevealRequested();
    setState(() {
      _selectedIndex = index;
    });
  }

  void _closeWorkspaceTab(WorkspaceTab tab) {
    final index = _tabs.indexOf(tab);
    if (index <= 0) {
      return;
    }
    _closeTab(index);
  }

  void _finishWebVisitTab(WorkspaceTab tab, WebVisitResponse response) {
    final request = tab.webVisitRequest;
    if (request == null) {
      return;
    }
    _completeWebVisitRequest(request.requestId, response);
    final index = _tabs.indexOf(tab);
    if (index > 0) {
      setState(() {
        _tabs.removeAt(index);
        if (_selectedIndex == index) {
          _selectedIndex = (index - 1).clamp(0, _tabs.length - 1);
        } else if (_selectedIndex > index) {
          _selectedIndex -= 1;
        }
      });
    }
  }

  void _completeWebVisitRequest(String requestId, WebVisitResponse response) {
    final completer = _webVisitCompleters.remove(requestId);
    if (completer != null && !completer.isCompleted) {
      completer.complete(response);
    }
  }

  Future<void> _closeManualTerminalSession(String sessionId) async {
    await _terminalSessions.closePtySession(sessionId);
    await _refreshTerminalSessions();
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
}
