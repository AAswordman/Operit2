// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:operit2/ui/common/code_editor/CodeEditorSurface.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorZoomTransform.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorScrollController.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLanguage.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxPalette.dart';

/// Edits one complete document while rendering only its viewport working set.
class CodeEditorInput extends StatefulWidget {
  /// Connects persistent text, history, scrolling, and the native input method.
  const CodeEditorInput({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.undoController,
    required this.scrollController,
    required this.groupId,
    required this.style,
    this.language = CodeSyntaxLanguage.plainText,
    required this.onChanged,
    required this.scrollPhysics,
    required this.active,
    this.readOnly = false,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final UndoHistoryController undoController;
  final CodeEditorScrollController scrollController;
  final Object groupId;
  final TextStyle style;
  final CodeSyntaxLanguage language;
  final ValueChanged<String> onChanged;
  final ScrollPhysics? scrollPhysics;
  final bool active;
  final bool readOnly;

  /// Creates the full-document input connection and viewport state.
  @override
  CodeEditorInputState createState() => CodeEditorInputState();
}

/// Implements Flutter input and selection protocols without a hidden TextField.
class CodeEditorInputState extends State<CodeEditorInput>
    with TextInputClient, TextSelectionDelegate {
  final GlobalKey _surfaceKey = GlobalKey();
  final LayerLink _startLink = LayerLink();
  final LayerLink _endLink = LayerLink();
  final LayerLink _toolbarLink = LayerLink();
  TextInputConnection? _connection;
  TextEditingValue? _remoteValue;
  TextSelectionOverlay? _overlay;
  Timer? _cursorTimer;
  bool _cursorVisible = true;
  late String _lastText;
  Offset? _tapPosition;
  int? _dragBase;
  bool _dragWords = false;
  final Set<int> _touchPointers = {};
  bool _geometryScheduled = false;
  ({
    TextInputConnection connection,
    Size size,
    Matrix4 transform,
    Rect caret,
    TextSelection selection,
    Rect? composing,
  })?
  _reportedGeometry;
  bool _multiTouchGesture = false;
  int? _revealOffset;
  Offset? _floatingCursorStart;
  Timer? _selectionScrollTimer;
  Offset? _selectionDragPoint;
  double? _verticalCaretX;

  /// Exposes native-compatible geometry to zooming, gutters, and regression tests.
  RenderCodeEditorText get renderEditable =>
      _surfaceKey.currentContext!.findRenderObject()! as RenderCodeEditorText;

  /// Returns the full UTF-16 document state supplied to the input method.
  @override
  TextEditingValue get currentTextEditingValue => widget.controller.value;

  /// Returns the same document state to Flutter selection handles and menus.
  @override
  TextEditingValue get textEditingValue => widget.controller.value;

  /// Disables autofill for a code/text file editor.
  @override
  AutofillScope? get currentAutofillScope => null;

  /// Observes document and focus changes without coupling them to zoom rebuilds.
  @override
  void initState() {
    super.initState();
    _lastText = widget.controller.text;
    widget.controller.addListener(_controllerChanged);
    widget.focusNode.addListener(_focusChanged);
    WidgetsBinding.instance.addPostFrameCallback(_initializeFocus);
  }

  /// Honors a retained focus node after the first surface layout is complete.
  void _initializeFocus(Duration timestamp) {
    if (mounted && widget.active && widget.focusNode.hasFocus) _focusChanged();
  }

  /// Stops inactive input and updates listeners when document ownership changes.
  @override
  void didUpdateWidget(CodeEditorInput oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_controllerChanged);
      widget.controller.addListener(_controllerChanged);
      _lastText = widget.controller.text;
    }
    if (oldWidget.focusNode != widget.focusNode) {
      oldWidget.focusNode.removeListener(_focusChanged);
      widget.focusNode.addListener(_focusChanged);
      WidgetsBinding.instance.addPostFrameCallback(_initializeFocus);
    }
    if (oldWidget.active != widget.active ||
        oldWidget.readOnly != widget.readOnly) {
      _focusChanged();
    }
  }

  /// Releases native input, overlays, listeners, and cursor animation resources.
  @override
  void dispose() {
    widget.controller.removeListener(_controllerChanged);
    widget.focusNode.removeListener(_focusChanged);
    _connection?.close();
    _cursorTimer?.cancel();
    _selectionScrollTimer?.cancel();
    _overlay?.dispose();
    super.dispose();
  }

  /// Synchronizes genuine editing changes with the document and native IME.
  void _controllerChanged() {
    final value = textEditingValue;
    if (_lastText != value.text) {
      _lastText = value.text;
      _verticalCaretX = null;
      renderEditable.synchronizeText(value.text);
      widget.onChanged(value.text);
    }
    if (_connection?.attached == true && _remoteValue != value) {
      _connection!.setEditingState(value);
      _remoteValue = value;
    }
    _cursorVisible = true;
    setState(() {});
    _overlay?.update(value);
    _scheduleGeometry();
  }

  /// Opens input only for an active focused editor, retaining history otherwise.
  void _focusChanged() {
    _cursorTimer?.cancel();
    if (widget.active && widget.focusNode.hasFocus && !widget.readOnly) {
      requestKeyboard();
      _cursorTimer = Timer.periodic(const Duration(milliseconds: 500), _blink);
    } else {
      _connection?.close();
      _connection = null;
      _remoteValue = null;
      hideToolbar();
    }
    if (mounted) setState(() {});
  }

  /// Repaints only cursor state while keeping paragraph geometry cached.
  void _blink(Timer timer) {
    if (!widget.active || !widget.focusNode.hasFocus) return;
    setState(() {
      _cursorVisible = !_cursorVisible;
    });
  }

  /// Attaches the complete editing value to Flutter's platform-neutral input API.
  void requestKeyboard() {
    if (!widget.active) return;
    widget.focusNode.requestFocus();
    if (widget.readOnly) return;
    if (_connection?.attached != true) {
      _connection = TextInput.attach(
        this,
        TextInputConfiguration(
          viewId: View.of(context).viewId,
          inputType: TextInputType.multiline,
          inputAction: TextInputAction.newline,
          obscureText: false,
          autocorrect: false,
          // Disabling suggestions adds password-variation flags in Flutter's embedder.
          enableSuggestions: true,
          smartDashesType: SmartDashesType.disabled,
          smartQuotesType: SmartQuotesType.disabled,
        ),
      );
      _connection!.setEditingState(textEditingValue);
      _remoteValue = textEditingValue;
    }
    _connection!.setStyle(
      fontFamily: widget.style.fontFamily,
      fontSize: widget.style.fontSize,
      fontWeight: widget.style.fontWeight,
      textDirection: Directionality.of(context),
      textAlign: TextAlign.start,
    );
    _connection!.show();
    _scheduleGeometry();
  }

  /// Accepts complete native edits, including cross-line IME composing ranges.
  @override
  void updateEditingValue(TextEditingValue value) {
    _remoteValue = value;
    userUpdateTextEditingValue(value, SelectionChangedCause.keyboard);
  }

  /// Routes local edits through one controller, native connection, and history.
  @override
  void userUpdateTextEditingValue(
    TextEditingValue value,
    SelectionChangedCause cause,
  ) {
    if (widget.readOnly && value.text != widget.controller.text) return;
    assert(value.selection.isValid && value.selection.end <= value.text.length);
    widget.controller.value = value;
    if (cause == SelectionChangedCause.keyboard ||
        cause == SelectionChangedCause.toolbar) {
      bringIntoView(value.selection.extent);
    }
  }

  /// Keeps IME action notifications separate from their already delivered text.
  @override
  void performAction(TextInputAction action) {}

  /// Resets a closed native connection without discarding any document content.
  @override
  void connectionClosed() {
    _connection = null;
    _remoteValue = null;
    widget.focusNode.unfocus();
  }

  /// Receives autocorrection prompts; autocorrection is disabled for file text.
  @override
  void showAutocorrectionPromptRect(int start, int end) {}

  /// Receives private IME commands; this plain-text editor has no private protocol.
  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  /// Routes input-control replacement through Flutter's shared input connection.
  @override
  void didChangeInputControl(
    TextInputControl? oldControl,
    TextInputControl? newControl,
  ) {
    if (_connection?.attached != true) return;
    oldControl?.hide();
    newControl?.show();
  }

  /// Maps floating-cursor updates to the same source-position hit-testing logic.
  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {
    if (point.state == FloatingCursorDragState.Start) {
      _floatingCursorStart = renderEditable
          .getLocalRectForCaret(textEditingValue.selection.extent)
          .center;
    } else if (point.state == FloatingCursorDragState.Update) {
      final position = renderEditable.getPositionForPoint(
        renderEditable.localToGlobal(_floatingCursorStart! + point.offset!),
      );
      _select(TextSelection.fromPosition(position), SelectionChangedCause.drag);
      bringIntoView(position);
    } else {
      _floatingCursorStart = null;
    }
  }

  /// Enables cutting nonempty global selections through the native toolbar.
  @override
  bool get cutEnabled =>
      !widget.readOnly && !textEditingValue.selection.isCollapsed;

  @override
  bool get pasteEnabled => !widget.readOnly;

  /// Enables copying nonempty global selections through the native toolbar.
  @override
  bool get copyEnabled => !textEditingValue.selection.isCollapsed;

  /// Copies exactly the selected source, preserving line separators and Unicode.
  @override
  void copySelection(SelectionChangedCause cause) {
    final value = textEditingValue;
    Clipboard.setData(
      ClipboardData(text: value.selection.textInside(value.text)),
    );
    hideToolbar(false);
  }

  /// Copies and removes a selection as one undoable full-document edit.
  @override
  void cutSelection(SelectionChangedCause cause) {
    if (!cutEnabled) return;
    copySelection(cause);
    _replaceSelection('');
    hideToolbar();
  }

  /// Applies clipboard text to the selection that is current when reading completes.
  @override
  Future<void> pasteText(SelectionChangedCause cause) async {
    if (!pasteEnabled) return;
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (!mounted || data?.text == null) return;
    _replaceSelection(data!.text!);
    hideToolbar();
  }

  /// Selects the full source document, not just its visible paragraphs.
  @override
  void selectAll(SelectionChangedCause cause) {
    _select(
      TextSelection(baseOffset: 0, extentOffset: widget.controller.text.length),
      cause,
    );
  }

  /// Replaces a global selection and atomically updates the resulting caret.
  void _replaceSelection(String text) {
    if (widget.readOnly) return;
    final value = textEditingValue;
    userUpdateTextEditingValue(
      TextEditingValue(
        text: value.text.replaceRange(
          value.selection.start,
          value.selection.end,
          text,
        ),
        selection: TextSelection.collapsed(
          offset: value.selection.start + text.length,
        ),
      ),
      SelectionChangedCause.keyboard,
    );
    requestKeyboard();
  }

  /// Selects a global range without making selection changes into undo entries.
  void _select(TextSelection selection, SelectionChangedCause cause) {
    userUpdateTextEditingValue(
      textEditingValue.copyWith(
        selection: selection,
        composing: TextRange.empty,
      ),
      cause,
    );
  }

  /// Clears menus and optionally handles without mutating source or focus.
  @override
  void hideToolbar([bool hideHandles = true]) {
    _overlay?.hideToolbar();
    if (hideHandles) _overlay?.hideHandles();
  }

  /// Creates Flutter's normal selection handles using virtualized caret geometry.
  void _showSelectionOverlay() {
    _overlay?.dispose();
    _overlay = TextSelectionOverlay(
      value: textEditingValue,
      context: context,
      renderObject: renderEditable,
      toolbarLayerLink: _toolbarLink,
      startHandleLayerLink: _startLink,
      endHandleLayerLink: _endLink,
      selectionControls: materialTextSelectionControls,
      selectionDelegate: this,
      handlesVisible: true,
      magnifierConfiguration: TextMagnifierConfiguration.disabled,
      contextMenuBuilder: _buildContextMenu,
    );
    _overlay!.showHandles();
    _overlay!.showToolbar();
  }

  /// Builds clipboard actions at real selection endpoints without an EditableText.
  Widget _buildContextMenu(BuildContext context) =>
      AdaptiveTextSelectionToolbar.buttonItems(
        anchors: TextSelectionToolbarAnchors.fromSelection(
          renderBox: renderEditable,
          selectionEndpoints: renderEditable.getEndpointsForSelection(
            textEditingValue.selection,
          ),
          startGlyphHeight: renderEditable.preferredLineHeight,
          endGlyphHeight: renderEditable.preferredLineHeight,
        ),
        buttonItems: [
          if (cutEnabled)
            ContextMenuButtonItem(
              type: ContextMenuButtonType.cut,
              onPressed: () => cutSelection(SelectionChangedCause.toolbar),
            ),
          if (copyEnabled)
            ContextMenuButtonItem(
              type: ContextMenuButtonType.copy,
              onPressed: () => copySelection(SelectionChangedCause.toolbar),
            ),
          if (pasteEnabled)
            ContextMenuButtonItem(
              type: ContextMenuButtonType.paste,
              onPressed: () => pasteText(SelectionChangedCause.toolbar),
            ),
          ContextMenuButtonItem(
            type: ContextMenuButtonType.selectAll,
            onPressed: () => selectAll(SelectionChangedCause.toolbar),
          ),
        ],
      );

  /// Opens selection controls requested through Flutter's text input protocol.
  @override
  void showToolbar() => _showSelectionOverlay();

  /// Coalesces geometry reporting to one callback after the next completed layout.
  void _scheduleGeometry() {
    if (_geometryScheduled) return;
    _geometryScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback(_reportGeometry);
  }

  /// Reports transformed caret and composing rectangles to the native IME.
  void _reportGeometry(Duration timestamp) {
    _geometryScheduled = false;
    if (!mounted || !widget.active || _connection?.attached != true) return;
    final render = renderEditable;
    if (!render.attached || !render.hasSize) return;
    final value = textEditingValue;
    final caret = render.getLocalRectForCaret(value.selection.extent);
    final composing = value.composing.isValid && !value.composing.isCollapsed
        ? render.getRectForComposingRange(value.composing)
        : null;
    final geometry = (
      connection: _connection!,
      size: render.size,
      transform: render.getTransformTo(null),
      caret: caret,
      selection: value.selection,
      composing: composing,
    );
    if (_reportedGeometry == geometry) return;
    _reportedGeometry = geometry;
    _connection!.setEditableSizeAndTransform(geometry.size, geometry.transform);
    _connection!.setCaretRect(caret);
    if (composing != null) _connection!.setComposingRect(composing);
    _overlay?.updateForScroll();
  }

  /// Measures a requested source caret before revealing it after ordinary edits.
  @override
  void bringIntoView(TextPosition position) {
    if (!widget.active || !widget.scrollController.hasClients) return;
    _revealOffset = position.offset;
    renderEditable.prepareAnchor(position);
    WidgetsBinding.instance.addPostFrameCallback(_revealCaret);
  }

  /// Scrolls only when an edited caret is outside the finished viewport geometry.
  void _revealCaret(Duration timestamp) {
    if (!mounted || !widget.active || _revealOffset == null) return;
    final position = TextPosition(offset: _revealOffset!);
    _revealOffset = null;
    final render = renderEditable;
    final rect = render.getLocalRectForCaret(position);
    final scroll = widget.scrollController;
    final delta = rect.top < 0
        ? rect.top
        : rect.bottom > render.size.height
        ? rect.bottom - render.size.height
        : 0.0;
    if (delta != 0) {
      scroll.jumpTo(
        (scroll.offset + delta).clamp(
          scroll.position.minScrollExtent,
          scroll.position.maxScrollExtent,
        ),
      );
    }
    _scheduleGeometry();
  }

  /// Records taps and prevents a second touch from starting text selection.
  void _pointerDown(PointerDownEvent event) {
    if (_touchPointers.isEmpty) _multiTouchGesture = false;
    if (event.kind == PointerDeviceKind.touch) {
      _touchPointers.add(event.pointer);
    }
    _tapPosition = event.position;
    if (_touchPointers.length > 1) {
      _multiTouchGesture = true;
      _dragBase = null;
      _selectionScrollTimer?.cancel();
      hideToolbar();
    }
  }

  /// Removes finished touches from the selection-versus-pinch gate.
  void _pointerUp(PointerEvent event) => _touchPointers.remove(event.pointer);

  /// Places the caret using the exact transformed paragraph hit-test geometry.
  void _tapUp(TapDragUpDetails details) {
    if (_multiTouchGesture) return;
    hideToolbar();
    final position = renderEditable.getPositionForPoint(details.globalPosition);
    _select(TextSelection.fromPosition(position), SelectionChangedCause.tap);
    _verticalCaretX = null;
    requestKeyboard();
  }

  /// Selects a complete native Unicode word without delaying ordinary single taps.
  void _doubleTap(TapDragDownDetails details) {
    if (_multiTouchGesture) return;
    _selectWord(details.globalPosition, SelectionChangedCause.doubleTap);
  }

  /// Selects a complete logical source line on a triple click or tap.
  void _tripleTap(TapDragDownDetails details) {
    if (_multiTouchGesture) return;
    final render = renderEditable;
    final position = render.getPositionForPoint(details.globalPosition);
    final line = render
        .textLayout
        .lines[render.textLayout.lineAtOffset(position.offset)];
    _select(
      TextSelection(
        baseOffset: line.start,
        extentOffset: line.start + line.content.length + line.separator.length,
      ),
      SelectionChangedCause.doubleTap,
    );
    requestKeyboard();
    _showSelectionOverlay();
  }

  /// Starts touch selection without stealing an ordinary single-finger scroll.
  void _longPressStart(LongPressStartDetails details) {
    if (_multiTouchGesture) return;
    _selectWord(details.globalPosition, SelectionChangedCause.longPress);
    _dragBase = textEditingValue.selection.baseOffset;
    _dragWords = true;
  }

  /// Selects a word using paragraph-native boundaries and global source offsets.
  void _selectWord(Offset point, SelectionChangedCause cause) {
    final position = renderEditable.getPositionForPoint(point);
    final word = renderEditable.getWordBoundary(position);
    _select(
      TextSelection(baseOffset: word.start, extentOffset: word.end),
      cause,
    );
    requestKeyboard();
    _showSelectionOverlay();
  }

  /// Extends a long-press selection across virtualized logical lines.
  void _longPressMove(LongPressMoveUpdateDetails details) =>
      _updateDrag(details.globalPosition);

  /// Starts mouse/stylus range selection through Flutter's supported-device gate.
  void _panStart(TapDragStartDetails details) {
    if (_multiTouchGesture || details.kind == PointerDeviceKind.touch) return;
    hideToolbar();
    final position = renderEditable.getPositionForPoint(details.globalPosition);
    _dragBase = position.offset;
    _dragWords = false;
    _select(TextSelection.fromPosition(position), SelectionChangedCause.drag);
    requestKeyboard();
  }

  /// Extends a desktop drag selection while preserving its original base.
  void _panUpdate(TapDragUpdateDetails details) =>
      _updateDrag(details.globalPosition);

  /// Updates a global selection and requests edge scrolling when necessary.
  void _updateDrag(Offset point) {
    if (_dragBase == null || _touchPointers.length > 1) return;
    _selectionDragPoint = point;
    final position = renderEditable.getPositionForPoint(point);
    final word = renderEditable.getWordBoundary(position);
    final extent = _dragWords
        ? (position.offset < _dragBase! ? word.start : word.end)
        : position.offset;
    _select(
      TextSelection(baseOffset: _dragBase!, extentOffset: extent),
      SelectionChangedCause.drag,
    );
    final y = renderEditable.globalToLocal(point).dy;
    if (y < 0 || y > renderEditable.size.height) {
      _selectionScrollTimer ??= Timer.periodic(
        const Duration(milliseconds: 16),
        _scrollSelection,
      );
    } else {
      _selectionScrollTimer?.cancel();
      _selectionScrollTimer = null;
    }
  }

  /// Scrolls a dragged selection at viewport edges using measured source geometry.
  void _scrollSelection(Timer timer) {
    if (!mounted || _dragBase == null) return;
    final render = renderEditable;
    final y = render.globalToLocal(_selectionDragPoint!).dy;
    final delta = y < 0 ? y : y - render.size.height;
    final scroll = widget.scrollController;
    scroll.jumpTo(
      (scroll.offset + delta.clamp(-24, 24)).clamp(
        scroll.position.minScrollExtent,
        scroll.position.maxScrollExtent,
      ),
    );
    _updateDrag(_selectionDragPoint!);
  }

  /// Ends selection dragging without altering the selected source range.
  void _endDrag() {
    _dragBase = null;
    _selectionScrollTimer?.cancel();
    _selectionScrollTimer = null;
  }

  /// Stops touch selection and restores its native handles after long press.
  void _longPressEnd(LongPressEndDetails details) {
    _endDrag();
    if (_touchPointers.length <= 1) _showSelectionOverlay();
  }

  /// Stops desktop selection after the mouse button is released.
  void _panEnd(TapDragEndDetails details) => _endDrag();

  /// Opens clipboard actions on a desktop secondary click at the actual caret.
  void _secondaryTap() {
    if (textEditingValue.selection.isCollapsed) {
      final position = renderEditable.getPositionForPoint(_tapPosition!);
      _select(TextSelection.fromPosition(position), SelectionChangedCause.tap);
    }
    requestKeyboard();
    _showSelectionOverlay();
  }

  /// Moves one complete grapheme, preserving emoji and combining sequences.
  int _graphemeOffset(int offset, bool forward) {
    final range = CharacterRange.at(widget.controller.text, offset);
    if (range.isEmpty) {
      if (forward) {
        range.moveNext();
      } else {
        range.moveBack();
      }
    }
    return range.stringBeforeLength + (forward ? range.current.length : 0);
  }

  /// Moves or extends a caret through the full document, not the cached page.
  void _moveCaret(
    LogicalKeyboardKey key,
    bool extend,
    bool byWord,
    bool documentEdge,
  ) {
    final value = textEditingValue;
    final selection = value.selection;
    final current = selection.extent;
    var next = current.offset;
    if (key == LogicalKeyboardKey.arrowLeft ||
        key == LogicalKeyboardKey.arrowRight) {
      final forward = key == LogicalKeyboardKey.arrowRight;
      if (!extend && !selection.isCollapsed) {
        next = forward ? selection.end : selection.start;
      } else if (byWord) {
        final offset = _graphemeOffset(current.offset, forward);
        final word = renderEditable.getWordBoundary(
          TextPosition(offset: offset),
        );
        next = forward ? word.end : word.start;
      } else {
        next = _graphemeOffset(current.offset, forward);
      }
      _verticalCaretX = null;
    } else if (key == LogicalKeyboardKey.home ||
        key == LogicalKeyboardKey.end) {
      final forward = key == LogicalKeyboardKey.end;
      final line = renderEditable.getLineAtOffset(current);
      next = documentEdge
          ? (forward ? value.text.length : 0)
          : (forward ? line.end : line.start);
      _verticalCaretX = null;
    } else {
      final render = renderEditable;
      final rect = render.getLocalRectForCaret(current);
      _verticalCaretX ??= rect.left;
      final forward =
          key == LogicalKeyboardKey.arrowDown ||
          key == LogicalKeyboardKey.pageDown;
      final distance =
          key == LogicalKeyboardKey.pageDown || key == LogicalKeyboardKey.pageUp
          ? render.size.height
          : render.preferredLineHeight;
      next = render
          .getPositionForPoint(
            render.localToGlobal(
              Offset(
                _verticalCaretX!,
                rect.center.dy + (forward ? distance : -distance),
              ),
            ),
          )
          .offset;
    }
    _select(
      TextSelection(
        baseOffset: extend ? selection.baseOffset : next,
        extentOffset: next,
        affinity: key == LogicalKeyboardKey.end && !documentEdge
            ? TextAffinity.upstream
            : TextAffinity.downstream,
      ),
      SelectionChangedCause.keyboard,
    );
  }

  /// Deletes one grapheme or selected global range as a single history edit.
  void _delete(bool forward, bool byWord) {
    final value = textEditingValue;
    if (!value.selection.isCollapsed) {
      _replaceSelection('');
      return;
    }
    final start = value.selection.extentOffset;
    var end = _graphemeOffset(start, forward);
    if (byWord) {
      final word = renderEditable.getWordBoundary(TextPosition(offset: end));
      end = forward ? word.end : word.start;
    }
    widget.controller.selection = TextSelection(
      baseOffset: start,
      extentOffset: end,
    );
    _replaceSelection('');
  }

  /// Converts Flutter editing intents to one full-document action pipeline.
  CallbackAction<T> _action<T extends Intent>(void Function(T) invoke) =>
      CallbackAction<T>(
        onInvoke: (intent) {
          invoke(intent);
          return null;
        },
      );

  /// Uses Flutter's shortcut compatibility layer instead of guessing modifiers.
  late final Map<Type, Action<Intent>> _editingActions = {
    DoNothingAndStopPropagationTextIntent: DoNothingAction(consumesKey: false),
    DeleteCharacterIntent: _action<DeleteCharacterIntent>(
      (intent) => _delete(intent.forward, false),
    ),
    DeleteToNextWordBoundaryIntent: _action<DeleteToNextWordBoundaryIntent>(
      (intent) => _delete(intent.forward, true),
    ),
    DeleteToLineBreakIntent: _action<DeleteToLineBreakIntent>(
      _deleteLineIntent,
    ),
    ExtendSelectionByCharacterIntent: _action<ExtendSelectionByCharacterIntent>(
      _caretIntent,
    ),
    ExtendSelectionToNextWordBoundaryIntent:
        _action<ExtendSelectionToNextWordBoundaryIntent>(_caretIntent),
    ExtendSelectionToNextWordBoundaryOrCaretLocationIntent:
        _action<ExtendSelectionToNextWordBoundaryOrCaretLocationIntent>(
          _caretIntent,
        ),
    ExtendSelectionVerticallyToAdjacentLineIntent:
        _action<ExtendSelectionVerticallyToAdjacentLineIntent>(_caretIntent),
    ExtendSelectionVerticallyToAdjacentPageIntent:
        _action<ExtendSelectionVerticallyToAdjacentPageIntent>(_caretIntent),
    ExtendSelectionToLineBreakIntent: _action<ExtendSelectionToLineBreakIntent>(
      _caretIntent,
    ),
    ExtendSelectionToDocumentBoundaryIntent:
        _action<ExtendSelectionToDocumentBoundaryIntent>(_caretIntent),
    ExtendSelectionToNextParagraphBoundaryIntent:
        _action<ExtendSelectionToNextParagraphBoundaryIntent>(_caretIntent),
    ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent:
        _action<ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent>(
          _caretIntent,
        ),
    ExpandSelectionToLineBreakIntent: _action<ExpandSelectionToLineBreakIntent>(
      _expandIntent,
    ),
    ExpandSelectionToDocumentBoundaryIntent:
        _action<ExpandSelectionToDocumentBoundaryIntent>(_expandIntent),
    ExtendSelectionByPageIntent: _action<ExtendSelectionByPageIntent>(
      (intent) => _moveCaret(
        intent.forward
            ? LogicalKeyboardKey.pageDown
            : LogicalKeyboardKey.pageUp,
        true,
        false,
        false,
      ),
    ),
    ScrollToDocumentBoundaryIntent: _action<ScrollToDocumentBoundaryIntent>(
      (intent) => widget.scrollController.jumpTo(
        intent.forward ? widget.scrollController.position.maxScrollExtent : 0,
      ),
    ),
    ScrollIntent: _action<ScrollIntent>(_scrollIntent),
    SelectAllTextIntent: _action<SelectAllTextIntent>(
      (intent) => selectAll(intent.cause),
    ),
    CopySelectionTextIntent: _action<CopySelectionTextIntent>((intent) {
      if (intent.collapseSelection && cutEnabled) cutSelection(intent.cause);
      if (!intent.collapseSelection && copyEnabled) copySelection(intent.cause);
    }),
    PasteTextIntent: _action<PasteTextIntent>(
      (intent) => pasteText(intent.cause),
    ),
    TransposeCharactersIntent: _action<TransposeCharactersIntent>(
      _transposeIntent,
    ),
    NextFocusIntent: _action<NextFocusIntent>(
      (intent) => _replaceSelection('\t'),
    ),
    DismissIntent: _action<DismissIntent>((intent) => hideToolbar()),
  };

  /// Executes character, word, row, page, paragraph, and document movements.
  void _caretIntent(DirectionalCaretMovementIntent intent) {
    final old = textEditingValue.selection;
    final forward = intent.forward;
    if (intent is ExtendSelectionToNextParagraphBoundaryIntent ||
        intent is ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent) {
      final layout = renderEditable.textLayout;
      final index = layout.lineAtOffset(old.extentOffset);
      final line = layout.lines[index];
      final next = forward
          ? line.start + line.content.length + line.separator.length
          : old.extentOffset == line.start && index > 0
          ? layout.lines[index - 1].start
          : line.start;
      _select(
        TextSelection(
          baseOffset: intent.collapseSelection ? next : old.baseOffset,
          extentOffset: next,
        ),
        SelectionChangedCause.keyboard,
      );
    } else {
      final key = switch (intent) {
        ExtendSelectionByCharacterIntent() ||
        ExtendSelectionToNextWordBoundaryIntent() ||
        ExtendSelectionToNextWordBoundaryOrCaretLocationIntent() =>
          forward
              ? LogicalKeyboardKey.arrowRight
              : LogicalKeyboardKey.arrowLeft,
        ExtendSelectionVerticallyToAdjacentLineIntent() =>
          forward ? LogicalKeyboardKey.arrowDown : LogicalKeyboardKey.arrowUp,
        ExtendSelectionVerticallyToAdjacentPageIntent() =>
          forward ? LogicalKeyboardKey.pageDown : LogicalKeyboardKey.pageUp,
        ExtendSelectionToLineBreakIntent() ||
        ExtendSelectionToDocumentBoundaryIntent() =>
          forward ? LogicalKeyboardKey.end : LogicalKeyboardKey.home,
        _ => throw StateError(
          'Unregistered caret intent: ${intent.runtimeType}',
        ),
      };
      _moveCaret(
        key,
        !intent.collapseSelection,
        intent is ExtendSelectionToNextWordBoundaryIntent ||
            intent is ExtendSelectionToNextWordBoundaryOrCaretLocationIntent,
        intent is ExtendSelectionToDocumentBoundaryIntent,
      );
    }
    if (intent.collapseAtReversal &&
        (old.extentOffset - old.baseOffset) *
                (textEditingValue.selection.extentOffset - old.baseOffset) <
            0) {
      _select(
        TextSelection.collapsed(offset: old.baseOffset),
        SelectionChangedCause.keyboard,
      );
    }
  }

  /// Expands selection boundaries without shrinking a reversed source range.
  void _expandIntent(DirectionalCaretMovementIntent intent) {
    if (intent is ExpandSelectionToDocumentBoundaryIntent) {
      final old = textEditingValue.selection;
      final next = intent.forward ? widget.controller.text.length : 0;
      _select(
        TextSelection(baseOffset: old.baseOffset, extentOffset: next),
        SelectionChangedCause.keyboard,
      );
      return;
    }
    final old = textEditingValue.selection;
    final outward = intent.forward ? old.end : old.start;
    final line = renderEditable.getLineAtOffset(TextPosition(offset: outward));
    final next = intent is ExpandSelectionToDocumentBoundaryIntent
        ? (intent.forward ? widget.controller.text.length : 0)
        : (intent.forward ? line.end : line.start);
    final start = intent.forward
        ? old.start
        : (next < old.start ? next : old.start);
    final end = intent.forward ? (next > old.end ? next : old.end) : old.end;
    _select(
      TextSelection(
        baseOffset: old.baseOffset <= old.extentOffset ? start : end,
        extentOffset: old.baseOffset <= old.extentOffset ? end : start,
      ),
      SelectionChangedCause.keyboard,
    );
    bringIntoView(TextPosition(offset: next));
  }

  /// Deletes to a visual line edge as one ordinary undoable source edit.
  void _deleteLineIntent(DeleteToLineBreakIntent intent) {
    final value = textEditingValue;
    if (value.selection.isCollapsed) {
      final line = renderEditable.getLineAtOffset(value.selection.extent);
      widget.controller.selection = TextSelection(
        baseOffset: value.selection.extentOffset,
        extentOffset: intent.forward ? line.end : line.start,
      );
    }
    _replaceSelection('');
  }

  /// Applies Flutter page-scrolling intents to the wrapped vertical viewport.
  void _scrollIntent(ScrollIntent intent) {
    final sign = switch (intent.direction) {
      AxisDirection.up => -1.0,
      AxisDirection.down => 1.0,
      AxisDirection.left || AxisDirection.right => 0.0,
    };
    final distance = intent.type == ScrollIncrementType.page
        ? renderEditable.size.height
        : renderEditable.preferredLineHeight;
    final scroll = widget.scrollController;
    scroll.jumpTo(
      (scroll.offset + sign * distance).clamp(
        0,
        scroll.position.maxScrollExtent,
      ),
    );
  }

  /// Transposes adjacent complete graphemes as one native undoable edit.
  void _transposeIntent(TransposeCharactersIntent intent) {
    final value = textEditingValue;
    if (!value.selection.isCollapsed || value.selection.extentOffset == 0) {
      return;
    }
    final caret = value.selection.extentOffset;
    final middle = caret == value.text.length
        ? _graphemeOffset(caret, false)
        : caret;
    final start = _graphemeOffset(middle, false);
    final end = _graphemeOffset(middle, true);
    if (start == middle) return;
    final replacement =
        value.text.substring(middle, end) + value.text.substring(start, middle);
    userUpdateTextEditingValue(
      TextEditingValue(
        text: value.text.replaceRange(start, end, replacement),
        selection: TextSelection.collapsed(offset: end),
      ),
      SelectionChangedCause.keyboard,
    );
  }

  /// Decodes native selectors using Flutter's shared command-to-intent table.
  @override
  void performSelector(String selectorName) {
    final intent = intentForMacOSSelector(selectorName);
    if (intent != null) Actions.invoke(widget.focusNode.context!, intent);
  }

  /// Inserts literal tabs while leaving standard editing shortcuts to Flutter.
  KeyEventResult _keyEvent(FocusNode node, KeyEvent event) {
    if ((event is KeyDownEvent || event is KeyRepeatEvent) &&
        event.logicalKey == LogicalKeyboardKey.tab &&
        !HardwareKeyboard.instance.isShiftPressed &&
        !HardwareKeyboard.instance.isControlPressed &&
        !HardwareKeyboard.instance.isMetaPressed) {
      _replaceSelection('\t');
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  /// Records committed edits while keeping transient composition out of history.
  bool _shouldRecord(TextEditingValue? oldValue, TextEditingValue value) =>
      value.composing.isCollapsed &&
      value.selection.isValid &&
      oldValue?.text != value.text;

  /// Restores native IME state after an undo or redo operation.
  void _historyTriggered(TextEditingValue value) =>
      userUpdateTextEditingValue(value, SelectionChangedCause.keyboard);

  /// Commits only plain-text snapshots to history, never an active IME session.
  TextEditingValue _historyValue(TextEditingValue value) =>
      value.copyWith(composing: TextRange.empty);

  /// Reports scroll-related geometry without reshaping cached paragraphs.
  bool _scrollNotification(ScrollNotification notification) {
    hideToolbar(false);
    _scheduleGeometry();
    return false;
  }

  /// Builds a single virtualized surface backed by a full-document input client.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final pan = CodeEditorViewportScope.maybeOf(context)?.panEnabled ?? false;
    return DefaultTextEditingShortcuts(
      child: Actions(
        actions: _editingActions,
        child: TextFieldTapRegion(
          groupId: widget.groupId,
          onTapOutside: _tapOutside,
          child: UndoHistory<TextEditingValue>(
            value: widget.controller,
            focusNode: widget.focusNode,
            controller: widget.undoController,
            onTriggered: _historyTriggered,
            shouldChangeUndoStack: _shouldRecord,
            undoStackModifier: _historyValue,
            child: Focus(
              focusNode: widget.focusNode,
              includeSemantics: false,
              onKeyEvent: _keyEvent,
              child: MouseRegion(
                cursor: SystemMouseCursors.text,
                child: NotificationListener<ScrollNotification>(
                  onNotification: _scrollNotification,
                  child: Listener(
                    onPointerDown: _pointerDown,
                    onPointerUp: _pointerUp,
                    onPointerCancel: _pointerUp,
                    child: TextSelectionGestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onSingleTapUp: _tapUp,
                      onDoubleTapDown: _doubleTap,
                      onTripleTapDown: _tripleTap,
                      onSingleLongTapStart: _longPressStart,
                      onSingleLongTapMoveUpdate: _longPressMove,
                      onSingleLongTapEnd: _longPressEnd,
                      onSingleLongTapCancel: _endDrag,
                      onSecondaryTap: _secondaryTap,
                      onDragSelectionStart: pan ? null : _panStart,
                      onDragSelectionUpdate: pan ? null : _panUpdate,
                      onDragSelectionEnd: pan ? null : _panEnd,
                      child: Scrollable(
                        controller: widget.scrollController,
                        physics:
                            CodeEditorViewportScope.maybeOf(context)?.physics ??
                            widget.scrollPhysics,
                        viewportBuilder: (context, offset) => CodeEditorSurface(
                          key: _surfaceKey,
                          value: textEditingValue,
                          style: widget.style,
                          language: widget.language,
                          palette: CodeSyntaxPalette(
                            Theme.of(context).brightness,
                          ),
                          offset: offset,
                          delegate: this,
                          onGeometryChanged: _scheduleGeometry,
                          requestKeyboard: requestKeyboard,
                          scrollController: widget.scrollController,
                          hasFocus: widget.focusNode.hasFocus,
                          readOnly: widget.readOnly,
                          showCursor: _cursorVisible && !widget.readOnly,
                          cursorColor: colors.primary,
                          selectionColor: colors.primary.withValues(
                            alpha: 0.25,
                          ),
                          startHandleLayerLink: _startLink,
                          endHandleLayerLink: _endLink,
                          toolbarLayerLink: _toolbarLink,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// Releases editor focus when a pointer presses outside its shared tap region.
  void _tapOutside(PointerDownEvent event) => widget.focusNode.unfocus();
}
