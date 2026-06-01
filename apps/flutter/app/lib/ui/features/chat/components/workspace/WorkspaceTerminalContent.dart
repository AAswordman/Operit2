// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import 'WorkspacePtyProcess.dart';

class WorkspaceTerminalContent extends StatefulWidget {
  const WorkspaceTerminalContent({super.key, required this.workspacePath});

  final String workspacePath;

  @override
  State<WorkspaceTerminalContent> createState() =>
      _WorkspaceTerminalContentState();
}

class _WorkspaceTerminalContentState extends State<WorkspaceTerminalContent> {
  late final Terminal _terminal;
  late final TerminalController _controller;
  late final FocusNode _focusNode;
  StreamSubscription<String>? _outputSubscription;
  WorkspacePtyProcess? _pty;
  Object? _startupError;
  int? _pendingRows;
  int? _pendingColumns;
  bool _exited = false;

  @override
  void initState() {
    super.initState();
    _terminal = Terminal(maxLines: 10000);
    _controller = TerminalController();
    _focusNode = FocusNode(debugLabel: 'WorkspaceTerminal');
    _terminal.onOutput = _writeToPty;
    _terminal.onResize = (columns, rows, pixelWidth, pixelHeight) {
      _pendingRows = rows;
      _pendingColumns = columns;
      _pty?.resize(rows, columns);
    };
    WidgetsBinding.instance.endOfFrame.then((_) {
      if (mounted) {
        _focusNode.requestFocus();
        _startPty();
      }
    });
  }

  @override
  void didUpdateWidget(covariant WorkspaceTerminalContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.workspacePath != widget.workspacePath) {
      _restartPty();
    }
  }

  @override
  void dispose() {
    _outputSubscription?.cancel();
    _pty?.kill();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final startupError = _startupError;
    if (startupError != null) {
      return ColoredBox(
        color: theme.colorScheme.surface,
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
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
                  onPressed: _restartPty,
                  icon: const Icon(Icons.refresh),
                  label: const Text('重试'),
                ),
              ],
            ),
          ),
        ),
      );
    }

    return Listener(
      behavior: HitTestBehavior.opaque,
      onPointerDown: (_) => _focusNode.requestFocus(),
      child: ColoredBox(
        color: TerminalThemes.defaultTheme.background,
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
            textStyle: const TerminalStyle(fontSize: 13, height: 1.25),
            onSecondaryTapDown: (details, offset) => _copyOrPasteSelection(),
          ),
        ),
      ),
    );
  }

  Future<void> _startPty() async {
    try {
      final pty = await startWorkspacePty(
        workingDirectory: widget.workspacePath,
        rows: _pendingRows ?? _terminal.viewHeight,
        columns: _pendingColumns ?? _terminal.viewWidth,
      );
      _pty = pty;
      _syncPtySize();
      _exited = false;
      _startupError = null;
      _outputSubscription = pty.output
          .cast<List<int>>()
          .transform(const Utf8Decoder(allowMalformed: true))
          .listen(_terminal.write);
      unawaited(
        pty.exitCode.then((code) {
          if (!_exited) {
            _terminal.write('\r\n[process exited with code $code]\r\n');
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

  Future<void> _restartPty() async {
    await _outputSubscription?.cancel();
    _outputSubscription = null;
    _pty?.kill();
    _pty = null;
    _terminal.eraseDisplay();
    _startupError = null;
    _exited = true;
    if (mounted) {
      setState(() {});
    }
    await _startPty();
  }

  void _writeToPty(String data) {
    _pty?.write(const Utf8Encoder().convert(data));
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
