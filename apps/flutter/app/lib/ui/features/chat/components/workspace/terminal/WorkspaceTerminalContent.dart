// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import '../../../../../theme/OperitGlassSurface.dart';
import 'WorkspacePtyProcess.dart';
import 'WorkspaceTerminalSessions.dart';

class WorkspaceTerminalContent extends StatefulWidget {
  const WorkspaceTerminalContent({
    super.key,
    required this.sessionId,
    required this.sessionKind,
    required this.terminalType,
    required this.workingDir,
  });

  final String sessionId;
  final String sessionKind;
  final String terminalType;
  final String workingDir;

  @override
  State<WorkspaceTerminalContent> createState() =>
      _WorkspaceTerminalContentState();
}

class _WorkspaceTerminalContentState extends State<WorkspaceTerminalContent> {
  late final Terminal _terminal;
  late final TerminalController _controller;
  late final FocusNode _focusNode;
  final WorkspaceTerminalSessions _terminalSessions =
      const WorkspaceTerminalSessions();
  StreamSubscription<String>? _outputSubscription;
  WorkspacePtyProcess? _pty;
  Timer? _screenTimer;
  Object? _startupError;
  int? _pendingRows;
  int? _pendingColumns;
  final StringBuffer _pendingTerminalOutput = StringBuffer();
  String _lastShellContent = '';
  bool _terminalFlushScheduled = false;
  bool _shellRefreshRunning = false;
  bool _exited = false;

  @override
  void initState() {
    super.initState();
    _terminal = Terminal(maxLines: 10000);
    _controller = TerminalController();
    _focusNode = FocusNode(debugLabel: 'WorkspaceTerminal');
    _terminal.onOutput = _writeToSession;
    _terminal.onResize = (columns, rows, pixelWidth, pixelHeight) {
      _pendingRows = rows;
      _pendingColumns = columns;
      _pty?.resize(rows, columns);
    };
    WidgetsBinding.instance.endOfFrame.then((_) {
      if (mounted) {
        _focusNode.requestFocus();
        _attachSession();
      }
    });
  }

  @override
  void didUpdateWidget(covariant WorkspaceTerminalContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sessionId != widget.sessionId ||
        oldWidget.sessionKind != widget.sessionKind) {
      _restartSession();
    }
  }

  @override
  void dispose() {
    _outputSubscription?.cancel();
    _screenTimer?.cancel();
    _pty?.kill();
    _pendingTerminalOutput.clear();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final startupError = _startupError;
    if (startupError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: OperitGlassSurface(
            color: theme.colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.42,
            ),
            layer: OperitGlassSurfaceLayer.card,
            borderRadius: BorderRadius.circular(18),
            border: Border.all(
              color: theme.colorScheme.outlineVariant.withValues(alpha: 0.2),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 20),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(
                    Icons.terminal_outlined,
                    size: 42,
                    color: theme.colorScheme.error,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    '终端启动失败',
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    startupError.toString(),
                    textAlign: TextAlign.center,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    onPressed: _restartSession,
                    icon: const Icon(Icons.refresh),
                    label: const Text('重试'),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
    }

    return Listener(
      behavior: HitTestBehavior.opaque,
      onPointerDown: (_) => _focusNode.requestFocus(),
      child: OperitGlassSurface(
        color: TerminalThemes.defaultTheme.background.withValues(alpha: 0.42),
        layer: OperitGlassSurfaceLayer.panel,
        transparentAlpha: 0.08,
        clip: false,
        child: MediaQuery.removePadding(
          context: context,
          removeLeft: true,
          removeTop: true,
          removeRight: true,
          removeBottom: true,
          child: TerminalView(
            _terminal,
            controller: _controller,
            focusNode: _focusNode,
            autofocus: true,
            padding: const EdgeInsets.all(8),
            theme: TerminalThemes.defaultTheme,
            backgroundOpacity: 0.38,
            textStyle: const TerminalStyle(fontSize: 13, height: 1.25),
            onSecondaryTapDown: (details, offset) => _copyOrPasteSelection(),
          ),
        ),
      ),
    );
  }

  Future<void> _attachSession() async {
    if (widget.sessionKind == 'pty') {
      await _attachPty();
      return;
    }
    if (widget.sessionKind == 'shell') {
      await _attachShell();
      return;
    }
    _startupError = StateError('未知终端会话类型: ${widget.sessionKind}');
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _attachPty() async {
    try {
      final pty = attachWorkspacePty(widget.sessionId);
      _pty = pty;
      _syncPtySize();
      _exited = false;
      _startupError = null;
      _outputSubscription = pty.output
          .cast<List<int>>()
          .transform(const Utf8Decoder(allowMalformed: true))
          .listen(_queueTerminalWrite);
      unawaited(
        pty.exitCode.then((code) {
          if (!_exited) {
            _queueTerminalWrite('\r\n[process exited with code $code]\r\n');
          }
          _exited = true;
        }),
      );
      if (mounted) {
        setState(() {});
      }
    } catch (error) {
      _startupError = error;
      if (mounted) {
        setState(() {});
      }
    }
  }

  Future<void> _attachShell() async {
    try {
      _startupError = null;
      await _refreshShellScreen();
      _screenTimer = Timer.periodic(
        const Duration(milliseconds: 400),
        (_) => unawaited(_refreshShellScreen()),
      );
      if (mounted) {
        setState(() {});
      }
    } catch (error) {
      _startupError = error;
      if (mounted) {
        setState(() {});
      }
    }
  }

  Future<void> _restartSession() async {
    await _outputSubscription?.cancel();
    _outputSubscription = null;
    _screenTimer?.cancel();
    _screenTimer = null;
    _pty?.kill();
    _pty = null;
    _pendingTerminalOutput.clear();
    _terminalFlushScheduled = false;
    _terminal.eraseDisplay();
    _startupError = null;
    _lastShellContent = '';
    _exited = true;
    if (mounted) {
      setState(() {});
    }
    await _attachSession();
  }

  void _writeToSession(String data) {
    if (widget.sessionKind == 'pty') {
      _pty?.write(const Utf8Encoder().convert(data));
      return;
    }
    unawaited(
      _terminalSessions.inputSession(sessionId: widget.sessionId, input: data),
    );
  }

  Future<void> _refreshShellScreen() async {
    if (_shellRefreshRunning) {
      return;
    }
    _shellRefreshRunning = true;
    try {
      final screen = await _terminalSessions.getSessionScreen(widget.sessionId);
      if (screen.content == _lastShellContent) {
        return;
      }
      _lastShellContent = screen.content;
      _terminal.eraseDisplay();
      _terminal.write(screen.content.replaceAll('\n', '\r\n'));
    } finally {
      _shellRefreshRunning = false;
    }
  }

  void _queueTerminalWrite(String data) {
    if (data.isEmpty) {
      return;
    }
    _pendingTerminalOutput.write(data);
    if (_terminalFlushScheduled) {
      return;
    }
    _terminalFlushScheduled = true;
    WidgetsBinding.instance.scheduleFrameCallback((_) {
      if (!mounted) {
        _pendingTerminalOutput.clear();
        _terminalFlushScheduled = false;
        return;
      }
      final output = _pendingTerminalOutput.toString();
      _pendingTerminalOutput.clear();
      _terminalFlushScheduled = false;
      if (output.isNotEmpty) {
        _terminal.write(output);
      }
    });
  }

  void _syncPtySize() {
    final rows = _pendingRows ?? _terminal.viewHeight;
    final columns = _pendingColumns ?? _terminal.viewWidth;
    _pty?.resize(rows, columns);
  }

  Future<void> _copyOrPasteSelection() async {
    final selection = _controller.selection;
    if (selection != null) {
      final text = _terminal.buffer.getText(selection);
      _controller.clearSelection();
      await Clipboard.setData(ClipboardData(text: text));
      return;
    }
    final data = await Clipboard.getData('text/plain');
    final text = data?.text;
    if (text != null && text.isNotEmpty) {
      _terminal.paste(text);
    }
  }
}
