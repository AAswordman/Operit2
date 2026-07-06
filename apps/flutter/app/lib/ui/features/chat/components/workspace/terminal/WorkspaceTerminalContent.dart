// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert' as convert;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import '../../../../../theme/OperitGlassSurface.dart';
import '../../../../../theme/OperitTheme.dart';
import 'WorkspacePtyProcess.dart';

class WorkspaceTerminalContent extends StatefulWidget {
  const WorkspaceTerminalContent({
    super.key,
    required this.sessionId,
    required this.terminalType,
    required this.workingDir,
  });

  final String sessionId;
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
  StreamSubscription<String>? _outputSubscription;
  WorkspacePtyProcess? _pty;
  Object? _startupError;
  int? _pendingRows;
  int? _pendingColumns;
  final StringBuffer _pendingTerminalOutput = StringBuffer();
  bool _terminalFlushScheduled = false;
  bool _exited = false;
  bool _ctrlLatched = false;
  bool _altLatched = false;

  @override
  void initState() {
    super.initState();
    _terminal = Terminal(
      maxLines: 10000,
      inputHandler: _LatchedTerminalInputHandler(
        ctrlLatched: () => _ctrlLatched,
        altLatched: () => _altLatched,
        releaseLatchedModifiers: _releaseLatchedModifiers,
      ),
    );
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
    if (oldWidget.sessionId != widget.sessionId) {
      _restartSession();
    }
  }

  @override
  void dispose() {
    _outputSubscription?.cancel();
    _pty?.kill();
    _pendingTerminalOutput.clear();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final themeSnapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final hasBackgroundMedia =
        themeSnapshot.useBackgroundImage &&
        (themeSnapshot.backgroundImageUri?.trim().isNotEmpty ?? false);
    final translucentTerminal =
        themeSnapshot.transparentSurfaceEnabled || hasBackgroundMedia;
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
        color: translucentTerminal
            ? Colors.black.withValues(alpha: 0.56)
            : Colors.black,
        layer: OperitGlassSurfaceLayer.panel,
        transparentAlpha: translucentTerminal ? 0.56 : 1.0,
        clip: false,
        child: Column(
          children: [
            Expanded(
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
                  theme: _workspaceTerminalTheme,
                  backgroundOpacity: 0,
                  textStyle: _terminalStyleFromTheme(
                    Theme.of(context).textTheme.bodySmall!,
                  ),
                  onSecondaryTapDown: (details, offset) =>
                      _copyOrPasteSelection(),
                ),
              ),
            ),
            SafeArea(
              top: false,
              left: false,
              right: false,
              child: _TerminalShortcutBar(
                ctrlLatched: _ctrlLatched,
                altLatched: _altLatched,
                onToggleCtrl: () {
                  setState(() => _ctrlLatched = !_ctrlLatched);
                  _focusNode.requestFocus();
                },
                onToggleAlt: () {
                  setState(() => _altLatched = !_altLatched);
                  _focusNode.requestFocus();
                },
                onKey: _sendTerminalKey,
                onText: _sendTerminalText,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _attachSession() async {
    await _attachPty();
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
          .transform(const convert.Utf8Decoder())
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

  Future<void> _restartSession() async {
    await _outputSubscription?.cancel();
    _outputSubscription = null;
    _pty?.kill();
    _pty = null;
    _pendingTerminalOutput.clear();
    _terminalFlushScheduled = false;
    _terminal.eraseDisplay();
    _startupError = null;
    _exited = true;
    if (mounted) {
      setState(() {});
    }
    await _attachSession();
  }

  void _writeToSession(String data) {
    _pty?.write(const convert.Utf8Encoder().convert(data));
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

  void _sendTerminalKey(TerminalKey key) {
    _terminal.keyInput(key);
    _releaseLatchedModifiers();
    _focusNode.requestFocus();
  }

  void _sendTerminalText(String text, {required TerminalKey modifiedKey}) {
    if (_ctrlLatched || _altLatched) {
      _terminal.keyInput(modifiedKey);
    } else {
      _terminal.textInput(text);
    }
    _releaseLatchedModifiers();
    _focusNode.requestFocus();
  }

  void _releaseLatchedModifiers() {
    if (!_ctrlLatched && !_altLatched) {
      return;
    }
    setState(() {
      _ctrlLatched = false;
      _altLatched = false;
    });
  }

}

class _LatchedTerminalInputHandler implements TerminalInputHandler {
  const _LatchedTerminalInputHandler({
    required this.ctrlLatched,
    required this.altLatched,
    required this.releaseLatchedModifiers,
  });

  final bool Function() ctrlLatched;
  final bool Function() altLatched;
  final VoidCallback releaseLatchedModifiers;

  @override
  String? call(TerminalKeyboardEvent event) {
    final ctrl = ctrlLatched();
    final alt = altLatched();
    final hasLatchedModifiers = ctrl || alt;
    final effectiveEvent = hasLatchedModifiers
        ? event.copyWith(ctrl: event.ctrl || ctrl, alt: event.alt || alt)
        : event;
    final output = defaultInputHandler(effectiveEvent);
    if (hasLatchedModifiers && output != null) {
      releaseLatchedModifiers();
    }
    return output;
  }
}

class _TerminalShortcutBar extends StatelessWidget {
  const _TerminalShortcutBar({
    required this.ctrlLatched,
    required this.altLatched,
    required this.onToggleCtrl,
    required this.onToggleAlt,
    required this.onKey,
    required this.onText,
  });

  final bool ctrlLatched;
  final bool altLatched;
  final VoidCallback onToggleCtrl;
  final VoidCallback onToggleAlt;
  final ValueChanged<TerminalKey> onKey;
  final void Function(String text, {required TerminalKey modifiedKey}) onText;

  @override
  Widget build(BuildContext context) {
    final borderColor = Colors.white.withValues(alpha: 0.08);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.72),
        border: Border(top: BorderSide(color: borderColor)),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(0, 4, 0, 4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            _TerminalShortcutRow(
              children: [
                _TerminalShortcutButton(
                  label: 'ESC',
                  onPressed: () => onKey(TerminalKey.escape),
                ),
                _TerminalShortcutButton(
                  label: '/',
                  onPressed: () => onText(
                    '/',
                    modifiedKey: TerminalKey.slash,
                  ),
                ),
                _TerminalShortcutButton(
                  label: '―',
                  onPressed: () => onText(
                    '-',
                    modifiedKey: TerminalKey.minus,
                  ),
                  onSwipeUp: () => onText(
                    '|',
                    modifiedKey: TerminalKey.backslash,
                  ),
                  popupLabel: '|',
                ),
                _TerminalShortcutButton(
                  label: 'HOME',
                  onPressed: () => onKey(TerminalKey.home),
                ),
                _TerminalShortcutButton(
                  label: '↑',
                  onPressed: () => onKey(TerminalKey.arrowUp),
                ),
                _TerminalShortcutButton(
                  label: 'END',
                  onPressed: () => onKey(TerminalKey.end),
                ),
                _TerminalShortcutButton(
                  label: 'PGUP',
                  onPressed: () => onKey(TerminalKey.pageUp),
                ),
              ],
            ),
            _TerminalShortcutRow(
              children: [
                _TerminalShortcutButton(
                  label: '↹',
                  onPressed: () => onKey(TerminalKey.tab),
                ),
                _TerminalShortcutButton(
                  label: 'CTRL',
                  selected: ctrlLatched,
                  onPressed: onToggleCtrl,
                ),
                _TerminalShortcutButton(
                  label: 'ALT',
                  selected: altLatched,
                  onPressed: onToggleAlt,
                ),
                _TerminalShortcutButton(
                  label: '←',
                  onPressed: () => onKey(TerminalKey.arrowLeft),
                ),
                _TerminalShortcutButton(
                  label: '↓',
                  onPressed: () => onKey(TerminalKey.arrowDown),
                ),
                _TerminalShortcutButton(
                  label: '→',
                  onPressed: () => onKey(TerminalKey.arrowRight),
                ),
                _TerminalShortcutButton(
                  label: 'PGDN',
                  onPressed: () => onKey(TerminalKey.pageDown),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _TerminalShortcutRow extends StatelessWidget {
  const _TerminalShortcutRow({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final rowChildren = <Widget>[];
    for (var i = 0; i < children.length; i++) {
      rowChildren.add(Expanded(child: children[i]));
    }
    return Row(children: rowChildren);
  }
}

class _TerminalShortcutButton extends StatefulWidget {
  const _TerminalShortcutButton({
    required this.label,
    this.selected = false,
    required this.onPressed,
    this.onSwipeUp,
    this.popupLabel,
  });

  final String label;
  final bool selected;
  final VoidCallback onPressed;
  final VoidCallback? onSwipeUp;
  final String? popupLabel;

  @override
  State<_TerminalShortcutButton> createState() => _TerminalShortcutButtonState();
}

class _TerminalShortcutButtonState extends State<_TerminalShortcutButton> {
  double _dragDy = 0;
  bool _showPopup = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final background = widget.selected
        ? Colors.white.withValues(alpha: 0.24)
        : Colors.transparent;
    final foreground = Colors.white.withValues(alpha: 0.92);
    return SizedBox(
      height: 34,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onVerticalDragStart: (_) {
          _dragDy = 0;
          _setPopupVisible(false);
        },
        onVerticalDragUpdate: (details) {
          _dragDy += details.delta.dy;
          _setPopupVisible(widget.popupLabel != null && _dragDy < -12);
        },
        onVerticalDragEnd: (_) {
          if (_dragDy < -18) {
            widget.onSwipeUp?.call();
          }
          _dragDy = 0;
          _setPopupVisible(false);
        },
        child: Stack(
          clipBehavior: Clip.none,
          alignment: Alignment.center,
          children: [
            Positioned.fill(
              child: Material(
                color: background,
                child: InkWell(
                  onTap: widget.onPressed,
                  child: Center(
                    child: FittedBox(
                      fit: BoxFit.scaleDown,
                      child: Text(
                        widget.label,
                        maxLines: 1,
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: foreground,
                          fontWeight: FontWeight.w700,
                          letterSpacing: 0,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
            if (_showPopup)
              Positioned(
                top: -32,
                width: 42,
                height: 30,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: Colors.black.withValues(alpha: 0.9),
                    border: Border.all(
                      color: Colors.white.withValues(alpha: 0.18),
                    ),
                  ),
                  child: Center(
                    child: Text(
                      widget.popupLabel!,
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: foreground,
                        fontWeight: FontWeight.w700,
                        letterSpacing: 0,
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  void _setPopupVisible(bool visible) {
    if (_showPopup == visible) {
      return;
    }
    setState(() => _showPopup = visible);
  }
}

TerminalStyle _terminalStyleFromTheme(TextStyle style) {
  return TerminalStyle.fromTextStyle(
    style.copyWith(
      fontFamily: _terminalMonospaceFontFamily,
      fontFamilyFallback: _terminalMonospaceFontFamilies,
      fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
      letterSpacing: 0,
      height: 1.25,
    ),
  );
}

const String _terminalMonospaceFontFamily = 'OperitTerminalMono';

const List<String> _terminalMonospaceFontFamilies = <String>[
  'Cascadia Mono',
  'Consolas',
  'JetBrains Mono',
  'SF Mono',
  'Menlo',
  'monospace',
];

const TerminalTheme _workspaceTerminalTheme = TerminalTheme(
  cursor: Color(0xFFAEAFAD),
  selection: Color(0x66569CD6),
  foreground: Color(0xFFFFFFFF),
  background: Color(0xFF000000),
  black: Color(0xFF000000),
  red: Color(0xFFE53935),
  green: Color(0xFF66BB6A),
  yellow: Color(0xFFFBC02D),
  blue: Color(0xFF569CD6),
  magenta: Color(0xFFD69D85),
  cyan: Color(0xFF4EC9B0),
  white: Color(0xFFE5E5E5),
  brightBlack: Color(0xFF666666),
  brightRed: Color(0xFFE53935),
  brightGreen: Color(0xFF6A9955),
  brightYellow: Color(0xFFB5CEA8),
  brightBlue: Color(0xFF569CD6),
  brightMagenta: Color(0xFFD69D85),
  brightCyan: Color(0xFF4EC9B0),
  brightWhite: Color(0xFFFFFFFF),
  searchHitBackground: Color(0xFFFBC02D),
  searchHitBackgroundCurrent: Color(0xFF4EC9B0),
  searchHitForeground: Color(0xFF000000),
);
