// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../common/components/RetainedPage.dart';
import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/MarkdownLink.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import '../WorkspaceTabModels.dart';
import 'WorkspaceTextDocument.dart';
import 'WorkspaceVirtualTextEditor.dart';
import 'WorkspaceVirtualTextSurface.dart';
import 'WorkspaceTextLineNumbers.dart';
import 'WorkspaceTextZoomViewport.dart';
import 'WorkspaceZoomScrollController.dart';
import 'syntax/WorkspaceSyntaxLanguage.dart';

/// Edits workspace text through the shared file API without browser chrome.
class WorkspaceTextPreview extends StatefulWidget {
  /// Creates an editor for a text or Markdown workspace tab.
  const WorkspaceTextPreview({
    super.key,
    required this.tab,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenBrowser,
    required this.splitMarkdownContent,
  });

  final WorkspaceTab tab;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Creates the editor's input and gesture state.
  @override
  State<WorkspaceTextPreview> createState() => _WorkspaceTextPreviewState();
}

class _WorkspaceTextPreviewState extends State<WorkspaceTextPreview> {
  late TextEditingController _controller;
  late WorkspaceSyntaxLanguage _syntaxLanguage;
  final ValueNotifier<int> _viewportChanges = ValueNotifier<int>(0);
  final ValueNotifier<int> _chromeChanges = ValueNotifier<int>(0);
  late ({bool dirty, bool saving, Object? error}) _chromeState;
  RenderBox? _zoomTarget;
  final FocusNode _editorFocus = FocusNode();
  late UndoHistoryController _undoController;
  final Object _editorTapGroup = Object();
  final WorkspaceZoomScrollController _editorScroll =
      WorkspaceZoomScrollController();
  final WorkspaceZoomScrollController _previewScroll =
      WorkspaceZoomScrollController();
  GlobalKey _editorKey = GlobalKey();
  final GlobalKey _previewKey = GlobalKey();
  final GlobalKey _viewportKey = GlobalKey();
  final Map<int, Offset> _touches = <int, Offset>{};
  Offset? _zoomSceneAnchor;
  TextPosition? _zoomTextAnchor;
  double _zoomLineFraction = 0;
  Offset? _zoomLocalFocalPoint;
  double _zoomRenderOriginY = 0;
  double _horizontalOffset = 0;
  bool _trackpadPinching = false;
  double _startScale = 1;
  double _panZoomScale = 1;
  double? _startDistance;
  bool _showMarkdown = false;

  /// Returns the draft owned by this workspace tab.
  WorkspaceTextDocument get _document => widget.tab.textDocument!;

  /// Subscribes to the persistent draft and creates the editable input.
  @override
  void initState() {
    super.initState();
    _syntaxLanguage = WorkspaceSyntaxLanguage.forPath(widget.tab.filePath!);
    _controller = TextEditingController.fromValue(
      TextEditingValue(
        text: _document.text,
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
    _undoController = UndoHistoryController();
    _chromeState = _readChromeState();
    _document.addListener(_documentChanged);
  }

  /// Rebinds the input when the surrounding widget changes its document.
  @override
  void didUpdateWidget(covariant WorkspaceTextPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.tab.filePath != widget.tab.filePath) {
      _syntaxLanguage = WorkspaceSyntaxLanguage.forPath(widget.tab.filePath!);
    }
    if (oldWidget.tab.textDocument == _document) return;
    oldWidget.tab.textDocument!.removeListener(_documentChanged);
    _controller.dispose();
    _undoController.dispose();
    _editorKey = GlobalKey();
    _controller = TextEditingController.fromValue(
      TextEditingValue(
        text: _document.text,
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
    _undoController = UndoHistoryController();
    _chromeState = _readChromeState();
    _document.addListener(_documentChanged);
    _showMarkdown = false;
    _touches.clear();
    _startDistance = null;
    _zoomTarget = null;
    _zoomSceneAnchor = null;
    _zoomTextAnchor = null;
    _zoomLocalFocalPoint = null;
    _horizontalOffset = 0;
    _editorScroll.cancelZoomAnchor();
    _previewScroll.cancelZoomAnchor();
    _trackpadPinching = false;
  }

  /// Releases input resources without discarding the tab's draft.
  @override
  void dispose() {
    _document.removeListener(_documentChanged);
    _controller.dispose();
    _undoController.dispose();
    _editorFocus.dispose();
    _editorScroll.dispose();
    _previewScroll.dispose();
    _viewportChanges.dispose();
    _chromeChanges.dispose();
    super.dispose();
  }

  /// Captures only the state that can change toolbar and error presentation.
  ({bool dirty, bool saving, Object? error}) _readChromeState() => (
    dirty: _document.isDirty,
    saving: _document.isSaving,
    error: _document.saveError,
  );

  /// Updates text and zoom independently from the static controls and symbols.
  void _documentChanged() {
    if (_controller.text != _document.text) {
      _controller.value = TextEditingValue(
        text: _document.text,
        selection: TextSelection.collapsed(offset: _document.text.length),
      );
    }
    final chrome = _readChromeState();
    if (_chromeState != chrome) {
      _chromeState = chrome;
      _chromeChanges.value++;
    }
    _viewportChanges.value++;
  }

  /// Saves UTF-8 text using the existing workspace host capability.
  Future<void> _save() => _document.save(
    (text) => widget.onWriteWorkspaceFileBytes(
      widget.tab.filePath!,
      Uint8List.fromList(utf8.encode(text)),
    ),
  );

  /// Records touch pointers without competing with text scrolling gestures.
  void _pointerDown(PointerDownEvent event) {
    if (event.kind != PointerDeviceKind.touch) return;
    final wasPinching = _touches.length == 2;
    _touches[event.pointer] = event.position;
    _resetPinchBaseline();
    if (wasPinching != (_touches.length == 2)) setState(() {});
  }

  /// Starts a new two-finger zoom baseline after the touch count changes.
  void _resetPinchBaseline() {
    _startScale = _document.scale;
    _startDistance = _touches.length == 2 ? _touchDistance : null;
    if (_touches.length == 2) {
      final points = _touches.values.toList(growable: false);
      _captureZoomAnchor((points[0] + points[1]) / 2);
    }
  }

  /// Returns the scroll controller for the currently displayed presentation.
  WorkspaceZoomScrollController get _zoomScroll =>
      _showMarkdown ? _previewScroll : _editorScroll;

  /// Returns the outer viewport whose coordinates remain independent of zoom.
  RenderBox get _viewport =>
      _viewportKey.currentContext!.findRenderObject()! as RenderBox;

  /// Finds the exact scroll render box without inspecting character positions.
  RenderBox get _zoomRender {
    final matches = <RenderBox>[];
    final key = _showMarkdown ? _previewKey : _editorKey;
    _collectZoomRenders(key.currentContext!.findRenderObject()!, matches);
    return matches.single;
  }

  /// Collects this presentation's viewport by its Flutter render contract.
  void _collectZoomRenders(RenderObject render, List<RenderBox> matches) {
    if ((_showMarkdown && render is RenderAbstractViewport) ||
        (!_showMarkdown && render is RenderEditable)) {
      matches.add(render as RenderBox);
      return;
    }
    render.visitChildren((child) => _collectZoomRenders(child, matches));
  }

  /// Locks wrapped text to the left edge and bounds preview translation.
  double _boundHorizontalOffset(double offset, double scale) {
    if (!_showMarkdown) return 0;
    final extent = _viewport.size.width * (1 - scale);
    return offset.clamp(extent < 0 ? extent : 0.0, 0.0);
  }

  /// Reads unsnapped line geometry without consulting unfinished ancestors.
  double _anchorLineTop(RenderEditable render) =>
      render
          .getEndpointsForSelection(
            TextSelection.fromPosition(_zoomTextAnchor!),
          )
          .single
          .point
          .dy -
      render.preferredLineHeight;

  /// Captures one text position per gesture, retaining its within-line fraction.
  void _captureZoomAnchor(Offset focalPoint) {
    _zoomScroll.jumpTo(_zoomScroll.offset);
    _horizontalOffset = _boundHorizontalOffset(
      _horizontalOffset,
      _document.scale,
    );
    final localFocalPoint = _viewport.globalToLocal(focalPoint);
    final render = _zoomRender;
    _zoomTarget = render;
    final renderFocalY = render.globalToLocal(focalPoint).dy;
    _zoomLocalFocalPoint = localFocalPoint;
    _zoomRenderOriginY = localFocalPoint.dy - renderFocalY * _document.scale;
    if (!_showMarkdown) {
      final editable = render as RenderEditable;
      _zoomTextAnchor = editable.getPositionForPoint(focalPoint);
      _zoomLineFraction =
          (renderFocalY - _anchorLineTop(editable)) /
          editable.preferredLineHeight;
    }
    _zoomSceneAnchor = Offset(
      (localFocalPoint.dx - _horizontalOffset) / _document.scale,
      _zoomScroll.offset + renderFocalY,
    );
  }

  /// Resolves the exact unscaled scroll offset during the next layout pass.
  double _resolveZoomOffset() {
    // Use the new paragraph geometry and captured viewport coordinates only.
    // Global transforms are unsafe while ancestors are still laying out.
    final double sceneY;
    if (_showMarkdown) {
      sceneY = _zoomSceneAnchor!.dy;
    } else {
      final render = _zoomTarget! as RenderWorkspaceVirtualText;
      sceneY =
          _anchorLineTop(render) +
          _zoomScroll.offset +
          _zoomLineFraction * render.preferredLineHeight;
    }
    return sceneY -
        (_zoomLocalFocalPoint!.dy - _zoomRenderOriginY) / _document.scale;
  }

  /// Commits scaling and focal scrolling together before the next paint pass.
  void _zoomAroundFocalPoint(double scale, Offset focalPoint) {
    final next = scale.clamp(0.6, 2.5);
    final localFocalPoint = _viewport.globalToLocal(focalPoint);
    final scaleChanged = next != _document.scale;
    if (!scaleChanged && _zoomLocalFocalPoint == localFocalPoint) return;
    _zoomLocalFocalPoint = localFocalPoint;
    _horizontalOffset = _boundHorizontalOffset(
      _zoomLocalFocalPoint!.dx - _zoomSceneAnchor!.dx * next,
      next,
    );
    _zoomScroll.anchorNextLayout(_resolveZoomOffset);
    // A center-only pan also needs a layout transaction, even at equal scale.
    if (!_showMarkdown) {
      (_zoomTarget! as RenderWorkspaceVirtualText).prepareAnchor(
        _zoomTextAnchor!,
      );
    }
    _zoomTarget!.markNeedsLayout();
    _document.updateScale(next);
    // Scale notifications already dirty the viewport once; only a center-only
    // pan needs an explicit rebuild to update its horizontal translation.
    if (!scaleChanged) _viewportChanges.value++;
  }

  /// Anchors keyboard zoom at the center of the visible viewport.
  void _zoomTo(double scale) {
    final center = _viewport.localToGlobal(_viewport.size.center(Offset.zero));
    _captureZoomAnchor(center);
    _zoomAroundFocalPoint(scale, center);
  }

  /// Measures the distance between the two active touch pointers.
  double get _touchDistance {
    final points = _touches.values.toList(growable: false);
    return (points[0] - points[1]).distance;
  }

  /// Resizes the text only while exactly two touch pointers are active.
  void _pointerMove(PointerMoveEvent event) {
    if (!_touches.containsKey(event.pointer)) return;
    _touches[event.pointer] = event.position;
    final distance = _startDistance;
    if (_touches.length != 2 || distance == null || distance < 1) return;
    final points = _touches.values.toList(growable: false);
    _zoomAroundFocalPoint(
      _startScale * _touchDistance / distance,
      (points[0] + points[1]) / 2,
    );
  }

  /// Ends the released pointer and resets the remaining zoom baseline.
  void _pointerUp(PointerEvent event) {
    final wasPinching = _touches.length == 2;
    if (_touches.remove(event.pointer) == null) return;
    _resetPinchBaseline();
    if (wasPinching != (_touches.length == 2)) setState(() {});
  }

  /// Captures the current zoom before a trackpad pinch starts.
  void _panZoomStart(PointerPanZoomStartEvent event) {
    _panZoomScale = _document.scale;
    _trackpadPinching = true;
    _captureZoomAnchor(event.position);
    setState(() {});
  }

  /// Applies trackpad pinch zoom through Flutter's unified pointer events.
  void _panZoomUpdate(PointerPanZoomUpdateEvent event) {
    _zoomAroundFocalPoint(
      _panZoomScale * event.scale,
      event.position + event.pan,
    );
  }

  /// Restores ordinary scrolling when the trackpad gesture ends.
  void _panZoomEnd(PointerPanZoomEndEvent event) {
    _trackpadPinching = false;
    setState(() {});
  }

  /// Switches Markdown presentation without carrying an old zoom anchor.
  void _toggleMarkdown() {
    setState(() {
      _showMarkdown = !_showMarkdown;
      _zoomTarget = null;
      _zoomSceneAnchor = null;
      _zoomTextAnchor = null;
      _zoomLocalFocalPoint = null;
      _editorScroll.cancelZoomAnchor();
      _previewScroll.cancelZoomAnchor();
    });
  }

  /// Finds the full-document native input client retained by this editor.
  WorkspaceVirtualTextEditorState get _editableState =>
      _editorKey.currentState! as WorkspaceVirtualTextEditorState;

  /// Inserts exactly one shortcut token and lets Flutter update IME and history.
  void _insertSymbol(String symbol) {
    final value = _controller.value;
    final selection = value.selection;
    assert(selection.isValid);
    final next = TextEditingValue(
      text: value.text.replaceRange(selection.start, selection.end, symbol),
      selection: TextSelection.collapsed(
        offset: selection.start + symbol.length,
      ),
    );
    final editable = _editableState;
    editable.userUpdateTextEditingValue(next, SelectionChangedCause.keyboard);
    editable.requestKeyboard();
  }

  /// Keeps editor focus while invoking its native undo history.
  void _undo() {
    _undoController.undo();
    _editableState.requestKeyboard();
  }

  /// Keeps editor focus while replaying its native redo history.
  void _redo() {
    _undoController.redo();
    _editableState.requestKeyboard();
  }

  /// Fixes toolbar geometry without padded tap targets or inherited density.
  Widget _buildToolbarButton({
    required String tooltip,
    required VoidCallback? onPressed,
    required IconData icon,
  }) => IconButton(
    tooltip: tooltip,
    onPressed: onPressed,
    icon: Icon(icon),
    iconSize: 24,
    padding: EdgeInsets.zero,
    constraints: const BoxConstraints.tightFor(width: 40, height: 40),
    visualDensity: VisualDensity.standard,
    style: IconButton.styleFrom(
      fixedSize: const Size.square(40),
      padding: EdgeInsets.zero,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      visualDensity: VisualDensity.standard,
    ),
  );

  /// Uses the top row for editing actions instead of gesture zoom controls.
  Widget _buildControls(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final isMarkdown =
        widget.tab.previewKind == WorkspaceFilePreviewKind.markdown;
    return TextFieldTapRegion(
      groupId: _editorTapGroup,
      child: ValueListenableBuilder<UndoHistoryValue>(
        valueListenable: _undoController,
        builder: (context, history, child) => Row(
          children: <Widget>[
            _buildToolbarButton(
              tooltip: l10n.workspaceUndo,
              onPressed: !_showMarkdown && history.canUndo ? _undo : null,
              icon: Icons.undo,
            ),
            _buildToolbarButton(
              tooltip: l10n.workspaceRedo,
              onPressed: !_showMarkdown && history.canRedo ? _redo : null,
              icon: Icons.redo,
            ),
            const Spacer(),
            if (isMarkdown)
              _buildToolbarButton(
                tooltip: _showMarkdown ? l10n.edit : l10n.markdownPreview,
                onPressed: _toggleMarkdown,
                icon: _showMarkdown
                    ? Icons.edit_outlined
                    : Icons.visibility_outlined,
              ),
            if (_document.isSaving)
              const SizedBox.square(
                dimension: 40,
                child: Center(
                  child: SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                ),
              )
            else
              _buildToolbarButton(
                tooltip: l10n.save,
                onPressed: _document.isDirty ? _save : null,
                icon: _document.isDirty ? Icons.save : Icons.save_outlined,
              ),
          ],
        ),
      ),
    );
  }

  /// Provides a compact scrollable symbol row without stealing input focus.
  Widget _buildQuickInput(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    const symbols = <String>[
      '{',
      '}',
      '[',
      ']',
      '(',
      ')',
      '<',
      '>',
      '"',
      "'",
      '`',
      ';',
      ':',
      ',',
      '.',
      '=',
      '/',
      r'\',
      '_',
      '-',
      '+',
      '*',
      '&',
      '|',
      '!',
      '?',
      '#',
      r'$',
    ];
    return TextFieldTapRegion(
      groupId: _editorTapGroup,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(
            top: BorderSide(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
          ),
        ),
        child: SingleChildScrollView(
          key: const ValueKey('workspace-quick-input'),
          scrollDirection: Axis.horizontal,
          child: Row(
            children: <Widget>[
              for (final symbol in symbols)
                Tooltip(
                  message: l10n.workspaceInsertSymbol(symbol),
                  child: TextButton(
                    key: ValueKey('workspace-insert-$symbol'),
                    style: TextButton.styleFrom(
                      minimumSize: Size.zero,
                      fixedSize: const Size(36, 32),
                      padding: EdgeInsets.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.standard,
                      textStyle: const TextStyle(
                        fontFamily: 'monospace',
                        fontSize: 18,
                        height: 1,
                      ),
                    ),
                    onPressed: () => _insertSymbol(symbol),
                    child: Text(symbol),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  /// Builds an editable, scrollable document with touch and keyboard zoom.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodyMedium!.copyWith(
      color: theme.colorScheme.onSurface,
      fontFamily: 'monospace',
      fontSize: 14,
      height: 1.45,
    );

    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        const SingleActivator(LogicalKeyboardKey.keyS, control: true): _save,
        const SingleActivator(LogicalKeyboardKey.keyS, meta: true): _save,
        const SingleActivator(LogicalKeyboardKey.equal, control: true): () =>
            _zoomTo(_document.scale + 0.1),
        const SingleActivator(LogicalKeyboardKey.minus, control: true): () =>
            _zoomTo(_document.scale - 0.1),
        const SingleActivator(LogicalKeyboardKey.digit0, control: true): () =>
            _zoomTo(1),
      },
      child: OperitGlassSurface(
        color: theme.colorScheme.surface,
        layer: OperitGlassSurfaceLayer.panel,
        transparentAlpha: 0.025,
        child: Column(
          children: <Widget>[
            RepaintBoundary(
              child: ListenableBuilder(
                listenable: _chromeChanges,
                builder: (context, child) => _buildControls(context),
              ),
            ),
            ListenableBuilder(
              listenable: _chromeChanges,
              builder: (context, child) {
                final error = _document.saveError;
                return error == null
                    ? const SizedBox.shrink()
                    : Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 4,
                        ),
                        child: Text(
                          error.toString(),
                          style: TextStyle(color: theme.colorScheme.error),
                        ),
                      );
              },
            ),
            Expanded(
              child: RepaintBoundary(
                child: ListenableBuilder(
                  listenable: _viewportChanges,
                  child: WorkspaceVirtualTextEditor(
                    key: _editorKey,
                    groupId: _editorTapGroup,
                    controller: _controller,
                    undoController: _undoController,
                    focusNode: _editorFocus,
                    scrollController: _editorScroll,
                    scrollPhysics: _touches.length == 2 || _trackpadPinching
                        ? const NeverScrollableScrollPhysics()
                        : null,
                    onChanged: _document.updateText,
                    style: style,
                    language: _syntaxLanguage,
                    active: !_showMarkdown,
                  ),
                  builder: (context, child) => WorkspaceTextLineNumbers(
                    enabled: !_showMarkdown,
                    scale: _document.scale,
                    text: _controller.text,
                    scrollController: _editorScroll,
                    color: theme.colorScheme.onSurfaceVariant,
                    dividerColor: theme.colorScheme.outlineVariant,
                    child: Listener(
                      key: _viewportKey,
                      behavior: HitTestBehavior.opaque,
                      onPointerDown: _pointerDown,
                      onPointerMove: _pointerMove,
                      onPointerUp: _pointerUp,
                      onPointerCancel: _pointerUp,
                      onPointerPanZoomStart: _panZoomStart,
                      onPointerPanZoomUpdate: _panZoomUpdate,
                      onPointerPanZoomEnd: _panZoomEnd,
                      child: ClipRect(
                        child: WorkspaceTextZoomViewport(
                          scale: _document.scale,
                          horizontalOffset: _horizontalOffset,
                          wrapToViewport: !_showMarkdown,
                          child: IndexedStack(
                            index: _showMarkdown ? 1 : 0,
                            children: <Widget>[
                              RetainedPage(
                                active: !_showMarkdown,
                                child: Padding(
                                  padding: EdgeInsets.all(12 / _document.scale),
                                  child: child,
                                ),
                              ),
                              if (_showMarkdown)
                                SingleChildScrollView(
                                  key: _previewKey,
                                  controller: _previewScroll,
                                  physics:
                                      _touches.length == 2 || _trackpadPinching
                                      ? const NeverScrollableScrollPhysics()
                                      : null,
                                  padding: const EdgeInsets.all(12),
                                  child: StreamMarkdownRenderer(
                                    content: _document.text,
                                    isStreaming: false,
                                    textColor: theme.colorScheme.onSurface,
                                    backgroundColor: Colors.transparent,
                                    onLinkClick: (url) {
                                      final path = markdownFileLinkPath(url);
                                      if (path == null) {
                                        widget.onOpenBrowser(url: url);
                                        return;
                                      }
                                      final file =
                                          widget.tab.absolutePath ??
                                          widget.tab.filePath!;
                                      final separator = file.lastIndexOf('/');
                                      final base = separator < 0
                                          ? null
                                          : file.substring(0, separator + 1);
                                      openMarkdownFileLink(
                                        url,
                                        null,
                                        basePath: base,
                                        onOpenFile: MarkdownLinkScope.maybeOf(
                                          context,
                                        )?.onOpenFile,
                                      );
                                    },
                                    splitMarkdownContent:
                                        widget.splitMarkdownContent,
                                  ),
                                )
                              else
                                const SizedBox.shrink(),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
            if (!_showMarkdown)
              RepaintBoundary(child: _buildQuickInput(context)),
          ],
        ),
      ),
    );
  }
}

/// Displays extracted text from read-only document formats.
class WorkspaceTextBody extends StatelessWidget {
  /// Creates a selectable document text view.
  const WorkspaceTextBody({
    super.key,
    required this.text,
    required this.monospace,
  });

  final String text;
  final bool monospace;

  /// Builds the read-only text surface for extracted document contents.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: theme.colorScheme.surface.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.025,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: SelectableText(
          text,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurface,
            fontFamily: monospace ? 'monospace' : null,
            height: 1.45,
          ),
        ),
      ),
    );
  }
}
