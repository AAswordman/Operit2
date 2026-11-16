// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'package:operit2/ui/common/code_editor/CodeTextLayout.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorScrollController.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLanguage.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxPalette.dart';

/// Uses Flutter's editable geometry contract with viewport-only paragraphs.
class CodeEditorSurface extends LeafRenderObjectWidget {
  /// Connects the virtual renderer to the full-document editing delegate.
  const CodeEditorSurface({
    super.key,
    required this.value,
    required this.style,
    required this.language,
    required this.palette,
    required this.offset,
    required this.delegate,
    required this.onGeometryChanged,
    required this.requestKeyboard,
    required this.scrollController,
    required this.hasFocus,
    this.readOnly = false,
    required this.showCursor,
    required this.cursorColor,
    required this.selectionColor,
    required this.startHandleLayerLink,
    required this.endHandleLayerLink,
    required this.toolbarLayerLink,
  });

  final TextEditingValue value;
  final TextStyle style;
  final CodeSyntaxLanguage language;
  final CodeSyntaxPalette palette;
  final ViewportOffset offset;
  final TextSelectionDelegate delegate;
  final VoidCallback onGeometryChanged;
  final VoidCallback requestKeyboard;
  final CodeEditorScrollController scrollController;
  final bool hasFocus;
  final bool readOnly;
  final bool showCursor;
  final Color cursorColor;
  final Color selectionColor;
  final LayerLink startHandleLayerLink;
  final LayerLink endHandleLayerLink;
  final LayerLink toolbarLayerLink;

  /// Creates a renderer whose paragraph count follows the viewport, not the file.
  @override
  RenderCodeEditorText createRenderObject(BuildContext context) =>
      RenderCodeEditorText(
        value: value,
        style: style,
        language: language,
        palette: palette,
        offset: offset,
        delegate: delegate,
        onGeometryChanged: onGeometryChanged,
        requestKeyboard: requestKeyboard,
        scrollController: scrollController,
        readOnly: readOnly,
        hasFocus: hasFocus,
        cursorVisible: showCursor,
        cursorColor: cursorColor,
        selectionColor: selectionColor,
        textScaler: MediaQuery.textScalerOf(context),
        textDirection: Directionality.of(context),
        locale: Localizations.maybeLocaleOf(context),
        startHandleLayerLink: startHandleLayerLink,
        endHandleLayerLink: endHandleLayerLink,
        toolbarLayerLink: toolbarLayerLink,
      );

  /// Updates text and input state without replacing the paragraph cache.
  @override
  void updateRenderObject(
    BuildContext context,
    RenderCodeEditorText renderObject,
  ) => renderObject.update(
    value: value,
    style: style,
    language: language,
    palette: palette,
    offset: offset,
    readOnly: readOnly,
    hasFocus: hasFocus,
    cursorVisible: showCursor,
    cursorColor: cursorColor,
    selectionColor: selectionColor,
    textScaler: MediaQuery.textScalerOf(context),
    textDirection: Directionality.of(context),
    locale: Localizations.maybeLocaleOf(context),
  );
}

/// Paints only intersecting logical-line paragraphs and keeps global offsets.
class RenderCodeEditorText extends RenderEditable {
  /// Keeps the native editing contract while replacing whole-document layout.
  RenderCodeEditorText({
    required TextEditingValue value,
    required TextStyle style,
    required CodeSyntaxLanguage language,
    required CodeSyntaxPalette palette,
    required super.offset,
    required TextSelectionDelegate delegate,
    required this.onGeometryChanged,
    required this.requestKeyboard,
    required this.scrollController,
    super.readOnly,
    required super.hasFocus,
    required bool cursorVisible,
    required super.cursorColor,
    required super.selectionColor,
    required super.textScaler,
    required super.textDirection,
    required super.locale,
    required super.startHandleLayerLink,
    required super.endHandleLayerLink,
    required this.toolbarLayerLink,
  }) : _value = value,
       _style = style,
       _cursorVisible = cursorVisible,
       textLayout = CodeTextLayout(
         value.text,
         language: language,
         palette: palette,
       ),
       super(
         text: const TextSpan(text: ''),
         ignorePointer: true,
         maxLines: null,
         selection: value.selection,
         textSelectionDelegate: delegate,
       );

  TextEditingValue _value;
  TextStyle _style;
  bool _cursorVisible;
  final CodeTextLayout textLayout;
  final VoidCallback onGeometryChanged;
  final VoidCallback requestKeyboard;
  final CodeEditorScrollController scrollController;
  final LayerLink toolbarLayerLink;
  final List<int> _visible = [];
  TextPosition? _nextAnchor;
  int debugPaintedParagraphs = 0;

  /// Tracks native handle visibility independently of the unused base paragraph.
  @override
  final ValueNotifier<bool> selectionStartInViewport = ValueNotifier<bool>(
    false,
  );

  /// Tracks the selection end visibility for Flutter's overlay.
  @override
  final ValueNotifier<bool> selectionEndInViewport = ValueNotifier<bool>(false);

  /// Exposes the actual source to native handles without a whole paragraph.
  @override
  String get plainText => _value.text;

  /// Returns row height without laying out a whole-document paragraph.
  @override
  double get preferredLineHeight => textLayout.lineHeight;

  /// Exposes already laid out logical lines to the gutter without hidden work.
  List<({int number, double centerY})> get visibleLogicalLines => [
    for (final index in _visible)
      (
        number: index + 1,
        centerY:
            textLayout.topOf(index) - offset.pixels + preferredLineHeight / 2,
      ),
  ];

  /// Reports paragraphs retained by the virtualized working set.
  @visibleForTesting
  int get debugCachedParagraphs => textLayout.cachedParagraphs;

  /// Reports actual native paragraph layouts, including overscan and anchors.
  @visibleForTesting
  int get debugParagraphLayouts => textLayout.debugParagraphLayouts;

  /// Reports visible paragraph widths without touching unseen source lines.
  @visibleForTesting
  List<double> get debugVisibleWidths => [
    for (final index in _visible)
      for (final metric in textLayout.paragraph(index).computeLineMetrics())
        metric.width,
  ];

  /// Synchronizes source indices immediately for multiple input events per frame.
  void synchronizeText(String text) {
    textLayout.updateText(text);
    _visible.clear();
    markNeedsLayout();
    markNeedsSemanticsUpdate();
  }

  /// Marks a source anchor for measurement before scroll dimensions are applied.
  void prepareAnchor(TextPosition position) {
    _nextAnchor = position;
    markNeedsLayout();
  }

  /// Invalidates only geometry or paint affected by the changed editing state.
  void update({
    required TextEditingValue value,
    required TextStyle style,
    required CodeSyntaxLanguage language,
    required CodeSyntaxPalette palette,
    required ViewportOffset offset,
    required bool readOnly,
    required bool hasFocus,
    required bool cursorVisible,
    required Color cursorColor,
    required Color selectionColor,
    required TextScaler textScaler,
    required TextDirection textDirection,
    required Locale? locale,
  }) {
    this.readOnly = readOnly;
    if (this.offset != offset) {
      if (attached) this.offset.removeListener(markNeedsLayout);
      this.offset = offset;
      if (attached) offset.addListener(markNeedsLayout);
    }
    if (_value.text != value.text) {
      textLayout.updateText(value.text);
      markNeedsLayout();
    }
    if (textLayout.configureSyntax(language, palette)) markNeedsLayout();
    if (_style != style ||
        this.textScaler != textScaler ||
        this.textDirection != textDirection ||
        this.locale != locale) {
      markNeedsLayout();
    }
    _value = value;
    _style = style;
    _cursorVisible = cursorVisible;
    this.hasFocus = hasFocus;
    this.cursorColor = cursorColor;
    this.selectionColor = selectionColor;
    this.textScaler = textScaler;
    this.textDirection = textDirection;
    this.locale = locale;
    selection = value.selection;
    markNeedsPaint();
    markNeedsSemanticsUpdate();
  }

  /// Measures only the working set and commits scroll corrections before paint.
  @override
  void performLayout() {
    size = constraints.biggest;
    textLayout.configure(
      size.width,
      _style,
      textScaler,
      textDirection,
      locale: locale,
    );
    visitChildren(_layoutAuxiliaryChild);
    offset.applyViewportDimension(size.height);
    final anchor = _nextAnchor;
    _nextAnchor = null;
    if (anchor != null) {
      textLayout.paragraph(textLayout.lineAtOffset(anchor.offset));
    }
    // Source-anchor correction remains active while measured heights settle.
    scrollController.beginVirtualLayout();
    try {
      offset.applyContentDimensions(
        0,
        math.max(0, textLayout.height - size.height),
      );
      var accepted = false;
      while (!accepted) {
        _layoutVisible();
        accepted = offset.applyContentDimensions(
          0,
          math.max(0, textLayout.height - size.height),
        );
      }
    } finally {
      scrollController.endVirtualLayout();
    }
  }

  /// Sizes Flutter's auxiliary handle-paint nodes without paragraph work.
  void _layoutAuxiliaryChild(RenderObject child) {
    (child as RenderBox).layout(BoxConstraints.tight(size));
  }

  /// Measures one backward and one forward overscan page around a stable source origin.
  void _layoutVisible() {
    _visible.clear();
    final retained = <CodeTextLine>{};
    final working = <int>[];
    final first = textLayout.lineAtY(offset.pixels);
    final oldTop = textLayout.topOf(first);
    final withinLine = offset.pixels - oldTop;
    var beforeHeight = 0.0;
    for (var i = first - 1; i >= 0 && beforeHeight < size.height; i--) {
      beforeHeight += textLayout.paragraph(i).height;
      retained.add(textLayout.lines[i]);
    }
    // Relative origins stay stable even when the unvisited-height mean changes.
    final limit = withinLine + size.height * 2;
    for (
      var i = first;
      i < textLayout.lines.length &&
          textLayout.topOf(i) - textLayout.topOf(first) <= limit;
      i++
    ) {
      textLayout.paragraph(i);
      retained.add(textLayout.lines[i]);
      working.add(i);
    }
    // Commit discovered prefix geometry before any text, gutter, or handle paint.
    offset.correctBy(textLayout.topOf(first) - oldTop);
    for (final i in working) {
      if (textLayout.topOf(i) < offset.pixels + size.height &&
          textLayout.topOf(i + 1) > offset.pixels) {
        _visible.add(i);
      }
    }
    textLayout.cacheCapacity = math.max(
      128,
      (size.height * 3 / preferredLineHeight).ceil() + 4,
    );
    textLayout.trimCache(retained);
  }

  /// Reads caret geometry in document coordinates using the owning paragraph.
  Rect _documentCaret(TextPosition position) {
    final index = textLayout.lineAtOffset(position.offset);
    final line = textLayout.lines[index];
    final painter = textLayout.paragraph(index, updateHeight: false);
    final local = TextPosition(
      offset: (position.offset - line.start).clamp(0, line.content.length),
      affinity: position.affinity,
    );
    final point = painter.getOffsetForCaret(
      local,
      Rect.fromLTWH(0, 0, cursorWidth, preferredLineHeight),
    );
    return Rect.fromLTWH(
      point.dx,
      textLayout.topOf(index) + point.dy,
      cursorWidth,
      preferredLineHeight,
    );
  }

  /// Returns unsnapped viewport caret geometry for input and zoom anchors.
  @override
  Rect getLocalRectForCaret(TextPosition caretPosition) =>
      _documentCaret(caretPosition).shift(Offset(0, -offset.pixels));

  /// Leaves pointer recognition to the editor; this leaf has no inline children.
  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      false;

  /// Maps transformed input coordinates back into full-document UTF-16 offsets.
  @override
  TextPosition getPositionForPoint(Offset globalPosition) {
    final point = globalToLocal(globalPosition) + Offset(0, offset.pixels);
    final index = textLayout.lineAtY(point.dy);
    final line = textLayout.lines[index];
    final position = textLayout
        .paragraph(index, updateHeight: false)
        .getPositionForOffset(point - Offset(0, textLayout.topOf(index)));
    return TextPosition(
      offset: line.start + position.offset,
      affinity: position.affinity,
    );
  }

  /// Exposes native handle endpoints in the same unsnapped coordinate space.
  @override
  List<TextSelectionPoint> getEndpointsForSelection(TextSelection selection) {
    final start = getLocalRectForCaret(
      selection.start == selection.extentOffset
          ? selection.extent
          : TextPosition(offset: selection.start),
    );
    if (selection.isCollapsed) {
      return [TextSelectionPoint(start.bottomLeft, textDirection)];
    }
    final end = getLocalRectForCaret(
      selection.end == selection.extentOffset
          ? selection.extent
          : TextPosition(offset: selection.end),
    );
    return [
      TextSelectionPoint(start.bottomLeft, textDirection),
      TextSelectionPoint(end.bottomLeft, textDirection),
    ];
  }

  /// Computes selection rectangles only for already visible paragraphs.
  @override
  List<ui.TextBox> getBoxesForSelection(TextSelection selection) {
    final result = <ui.TextBox>[];
    for (final i in _visible) {
      final line = textLayout.lines[i];
      final start = math.max(0, selection.start - line.start);
      final end = math.min(line.content.length, selection.end - line.start);
      if (end <= start) continue;
      final dy = textLayout.topOf(i) - offset.pixels;
      for (final box
          in textLayout
              .paragraph(i, updateHeight: false)
              .getBoxesForSelection(
                TextSelection(baseOffset: start, extentOffset: end),
                boxHeightStyle: ui.BoxHeightStyle.strut,
              )) {
        result.add(
          ui.TextBox.fromLTRBD(
            box.left,
            box.top + dy,
            box.right,
            box.bottom + dy,
            box.direction,
          ),
        );
      }
    }
    return result;
  }

  /// Unites visible composition boxes without shaping offscreen selections.
  @override
  Rect? getRectForComposingRange(TextRange range) {
    final boxes = getBoxesForSelection(
      TextSelection(baseOffset: range.start, extentOffset: range.end),
    );
    if (boxes.isEmpty) return null;
    return boxes
        .map((box) => box.toRect())
        .reduce((a, b) => a.expandToInclude(b));
  }

  /// Resolves a visual-row boundary within its original logical source line.
  @override
  TextSelection getLineAtOffset(TextPosition position) {
    final i = textLayout.lineAtOffset(position.offset);
    final line = textLayout.lines[i];
    final range = textLayout
        .paragraph(i, updateHeight: false)
        .getLineBoundary(
          TextPosition(
            offset: (position.offset - line.start).clamp(
              0,
              line.content.length,
            ),
            affinity: position.affinity,
          ),
        );
    return TextSelection(
      baseOffset: line.start + range.start,
      extentOffset: line.start + range.end,
    );
  }

  /// Resolves a native Unicode word boundary while preserving global offsets.
  @override
  TextRange getWordBoundary(TextPosition position) {
    final i = textLayout.lineAtOffset(position.offset);
    final line = textLayout.lines[i];
    final range = textLayout
        .paragraph(i, updateHeight: false)
        .getWordBoundary(
          TextPosition(
            offset: (position.offset - line.start).clamp(
              0,
              line.content.length,
            ),
          ),
        );
    return TextRange(
      start: line.start + range.start,
      end: line.start + range.end,
    );
  }

  /// Paints visible selections, paragraphs, composing underlines, and the caret.
  @override
  void paint(PaintingContext context, Offset origin) {
    if (hasFocus) onGeometryChanged();
    final canvas = context.canvas;
    canvas.save();
    canvas.clipRect(origin & size);
    final paint = Paint()..color = selectionColor!;
    if (!_value.selection.isCollapsed) {
      for (final box in getBoxesForSelection(_value.selection)) {
        canvas.drawRect(box.toRect().shift(origin), paint);
      }
    }
    debugPaintedParagraphs = 0;
    for (final i in _visible) {
      textLayout
          .paragraph(i)
          .paint(
            canvas,
            origin + Offset(0, textLayout.topOf(i) - offset.pixels),
          );
      assert(() {
        debugPaintedParagraphs++;
        return true;
      }());
    }
    if (_value.composing.isValid && !_value.composing.isCollapsed) {
      paint.color = _style.color!;
      paint.strokeWidth = 1;
      for (final box in getBoxesForSelection(
        TextSelection(
          baseOffset: _value.composing.start,
          extentOffset: _value.composing.end,
        ),
      )) {
        final rect = box.toRect().shift(origin);
        canvas.drawLine(rect.bottomLeft, rect.bottomRight, paint);
      }
    }
    if (hasFocus &&
        _cursorVisible &&
        _value.selection.isCollapsed &&
        _visible.contains(
          textLayout.lineAtOffset(_value.selection.extentOffset),
        )) {
      final rect = getLocalRectForCaret(_value.selection.extent);
      canvas.drawRect(rect.shift(origin), paint..color = cursorColor!);
    }
    canvas.restore();
    _paintHandle(
      context,
      origin,
      startHandleLayerLink,
      TextPosition(offset: _value.selection.start),
      selectionStartInViewport,
    );
    _paintHandle(
      context,
      origin,
      endHandleLayerLink,
      TextPosition(offset: _value.selection.end),
      selectionEndInViewport,
    );
    context.pushLayer(
      LeaderLayer(link: toolbarLayerLink, offset: origin),
      _paintLeader,
      Offset.zero,
    );
  }

  /// Positions native selection handles without shaping their hidden endpoints.
  void _paintHandle(
    PaintingContext context,
    Offset origin,
    LayerLink link,
    TextPosition position,
    ValueNotifier<bool> visible,
  ) {
    final index = textLayout.lineAtOffset(position.offset);
    final isVisible = _visible.contains(index);
    visible.value = isVisible;
    if (!isVisible) return;
    final rect = getLocalRectForCaret(position);
    visible.value = rect.bottom >= 0 && rect.top <= size.height;
    context.pushLayer(
      LeaderLayer(link: link, offset: origin + rect.bottomLeft),
      _paintLeader,
      Offset.zero,
    );
  }

  /// Supplies an empty leader painter for Flutter's selection overlay links.
  void _paintLeader(PaintingContext context, Offset offset) {}

  /// Exposes the complete document to accessibility without a hidden paragraph.
  @override
  void describeSemanticsConfiguration(SemanticsConfiguration config) {
    config
      ..isSemanticBoundary = true
      ..isTextField = true
      ..onTap = requestKeyboard
      ..isMultiline = true
      ..isFocused = hasFocus
      ..textDirection = textDirection
      ..value = _value.text
      ..textSelection = _value.selection
      ..onSetText = _setSemanticText
      ..onSetSelection = _setSemanticSelection;
  }

  /// Publishes one complete editable semantics node without paragraph children.
  @override
  void assembleSemanticsNode(
    SemanticsNode node,
    SemanticsConfiguration config,
    Iterable<SemanticsNode> children,
  ) {
    node.updateWith(
      config: config,
      childrenInInversePaintOrder: children.toList(),
    );
  }

  /// Applies accessibility text edits through the full-document delegate.
  void _setSemanticText(String text) =>
      textSelectionDelegate.userUpdateTextEditingValue(
        TextEditingValue(
          text: text,
          selection: TextSelection.collapsed(offset: text.length),
        ),
        SelectionChangedCause.keyboard,
      );

  /// Applies accessibility selection changes through the same editing delegate.
  void _setSemanticSelection(TextSelection selection) =>
      textSelectionDelegate.userUpdateTextEditingValue(
        _value.copyWith(selection: selection, composing: TextRange.empty),
        SelectionChangedCause.keyboard,
      );

  /// Invalidates owned native paragraphs when Flutter's font collection changes.
  @override
  void systemFontsDidChange() {
    textLayout.invalidateFonts();
    super.systemFontsDidChange();
  }

  /// Schedules working-set layout whenever scrolling changes the visible range.
  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    offset.addListener(markNeedsLayout);
  }

  /// Stops viewport work while the editor is retained but inactive.
  @override
  void detach() {
    offset.removeListener(markNeedsLayout);
    super.detach();
  }

  /// Releases all native paragraphs when this editor surface is destroyed.
  @override
  void dispose() {
    textLayout.dispose();
    selectionStartInViewport.dispose();
    selectionEndInViewport.dispose();
    super.dispose();
  }
}
