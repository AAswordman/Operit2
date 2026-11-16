// ignore_for_file: file_names

import 'dart:collection';
import 'dart:math' as math;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'package:operit2/ui/common/code_editor/CodeEditorSurface.dart';

/// Reserves a zoom-scaled gutter for logical lines, not soft-wrapped rows.
class CodeEditorLineNumbers extends SingleChildRenderObjectWidget {
  /// Creates line numbers synchronized with the actual editable text layout.
  const CodeEditorLineNumbers({
    super.key,
    required this.enabled,
    required this.scale,
    required this.text,
    required this.scrollController,
    required this.color,
    required this.dividerColor,
    required super.child,
  });

  final bool enabled;
  final double scale;
  final String text;
  final ScrollController scrollController;
  final Color color;
  final Color dividerColor;

  /// Creates the gutter renderer and its document line index.
  @override
  RenderCodeEditorLineNumbers createRenderObject(BuildContext context) =>
      RenderCodeEditorLineNumbers(
        enabled: enabled,
        scale: scale,
        text: text,
        scrollController: scrollController,
        color: color,
        dividerColor: dividerColor,
      );

  /// Updates document and appearance without recreating the editable child.
  @override
  void updateRenderObject(
    BuildContext context,
    RenderCodeEditorLineNumbers renderObject,
  ) => renderObject.update(
    enabled: enabled,
    scale: scale,
    text: text,
    scrollController: scrollController,
    color: color,
    dividerColor: dividerColor,
  );
}

/// Paints visible logical line numbers using the editor's own line geometry.
class RenderCodeEditorLineNumbers extends RenderShiftedBox {
  /// Indexes the document once and observes ordinary scrolling for repainting.
  RenderCodeEditorLineNumbers({
    required bool enabled,
    required double scale,
    required String text,
    required ScrollController scrollController,
    required Color color,
    required Color dividerColor,
  }) : _enabled = enabled,
       _scale = scale,
       _text = text,
       _scrollController = scrollController,
       _color = color,
       _dividerColor = dividerColor,
       _lineStarts = _indexLines(text),
       super(null);

  bool _enabled;
  double _scale;
  String _text;
  ScrollController _scrollController;
  Color _color;
  Color _dividerColor;
  List<int> _lineStarts;
  double _gutterWidth = 0;
  final LinkedHashMap<int, TextPainter> _labels =
      LinkedHashMap<int, TextPainter>();
  int _debugLabelLayouts = 0;

  /// Counts label shaping in debug tests without adding release-mode work.
  @visibleForTesting
  int get debugLabelLayouts => _debugLabelLayouts;

  /// Exposes the bounded label cache size for regression checks.
  @visibleForTesting
  int get debugCachedLabels => _labels.length;
  final TextPainter _numberPainter = TextPainter(
    textDirection: TextDirection.ltr,
  );

  /// Counts source lines, including empty lines and the final empty line.
  static List<int> _indexLines(String text) => [
    0,
    for (final match in RegExp(r'\r\n|\r|\n').allMatches(text)) match.end,
  ];

  /// Exposes the source line count independently of visual wrapping.
  int get lineCount => _lineStarts.length;

  /// Exposes the scaled gutter width for layout and geometry checks.
  double get gutterWidth => _gutterWidth;

  /// Returns the painted label height after applying document zoom.
  double get lineNumberHeight => _numberPainter.height * _scale;

  /// Updates only the document index when the actual source text changes.
  void update({
    required bool enabled,
    required double scale,
    required String text,
    required ScrollController scrollController,
    required Color color,
    required Color dividerColor,
  }) {
    final textChanged = _text != text;
    final geometryChanged =
        textChanged || _enabled != enabled || _scale != scale;
    final colorChanged = _color != color;
    final paintChanged =
        geometryChanged || colorChanged || _dividerColor != dividerColor;
    if (textChanged) {
      _text = text;
      _lineStarts = _indexLines(text);
    }
    if (colorChanged) _clearLabels();
    if (_scrollController != scrollController) {
      if (attached) _scrollController.removeListener(markNeedsPaint);
      _scrollController = scrollController;
      if (attached) _scrollController.addListener(markNeedsPaint);
    }
    _enabled = enabled;
    _scale = scale;
    _color = color;
    _dividerColor = dividerColor;
    if (geometryChanged) markNeedsLayout();
    if (paintChanged) markNeedsPaint();
  }

  /// Releases cached native paragraphs when their color or owner changes.
  void _clearLabels() {
    for (final label in _labels.values) {
      label.dispose();
    }
    _labels.clear();
  }

  /// Shapes each visible number once and bounds memory during long scrolling.
  TextPainter _label(int number) {
    final painter = _labels.putIfAbsent(number, () {
      final label = TextPainter(
        textDirection: TextDirection.ltr,
        text: TextSpan(
          text: '$number',
          style: TextStyle(
            fontFamily: 'monospace',
            fontSize: 12,
            color: _color,
          ),
        ),
      )..layout();
      assert(() {
        _debugLabelLayouts++;
        return true;
      }());
      return label;
    });
    // Tall or heavily zoomed-out viewports must retain a full visible page;
    // the bound depends on viewport height, never on total document length.
    final capacity = math.max(128, (size.height / lineNumberHeight).ceil() + 2);
    _labels.remove(number);
    _labels[number] = painter;
    while (_labels.length > capacity) {
      _labels.remove(_labels.keys.first)!.dispose();
    }
    return painter;
  }

  /// Subscribes only while this render object belongs to the active pipeline.
  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _scrollController.addListener(markNeedsPaint);
  }

  /// Stops observing scrolling while the document render subtree is detached.
  @override
  void detach() {
    _scrollController.removeListener(markNeedsPaint);
    super.detach();
  }

  /// Releases the cached paragraph used for gutter labels.
  @override
  void dispose() {
    _clearLabels();
    _numberPainter.dispose();
    super.dispose();
  }

  /// Measures base labels before uniformly scaling their paint and spacing.
  void _layoutNumber(int number) {
    _numberPainter.text = TextSpan(
      text: '$number',
      style: TextStyle(fontFamily: 'monospace', fontSize: 12, color: _color),
    );
    _numberPainter.layout();
  }

  /// Reserves scaled label width and padding before wrapping the editor.
  @override
  void performLayout() {
    size = constraints.biggest;
    _layoutNumber(lineCount);
    _gutterWidth = _enabled
        ? math.min(size.width, math.max(40, _numberPainter.width + 20) * _scale)
        : 0;
    child!.layout(
      BoxConstraints.tight(Size(size.width - _gutterWidth, size.height)),
      parentUsesSize: true,
    );
    (child!.parentData! as BoxParentData).offset = Offset(_gutterWidth, 0);
  }

  /// Locates the single editor by render type rather than by widget internals.
  void _collectEditable(
    RenderObject object,
    List<RenderCodeEditorText> matches,
  ) {
    if (object is RenderCodeEditorText) {
      matches.add(object);
      return;
    }
    object.visitChildren((child) => _collectEditable(child, matches));
  }

  /// Returns visible source lines and their actual first-row vertical centers.
  List<({int number, double centerY})> visibleLines() {
    if (!_enabled) return [];
    // Retained children may replace their render tree in an isolated build scope.
    // Resolve the live editor after layout rather than caching an old render box.
    final matches = <RenderCodeEditorText>[];
    _collectEditable(child!, matches);
    final editable = matches.single;
    final transform = editable.getTransformTo(this);
    final result = <({int number, double centerY})>[
      for (final line in editable.visibleLogicalLines)
        (
          number: line.number,
          centerY: MatrixUtils.transformPoint(
            transform,
            Offset(0, line.centerY),
          ).dy,
        ),
    ];
    return result;
  }

  /// Paints clipped labels after text layout and its same-frame zoom correction.
  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    if (!_enabled) return;
    final canvas = context.canvas;
    canvas.save();
    canvas.clipRect(offset & Size(_gutterWidth, size.height));
    canvas.drawLine(
      offset + Offset(_gutterWidth - 0.5, 0),
      offset + Offset(_gutterWidth - 0.5, size.height),
      Paint()..color = _dividerColor,
    );
    for (final line in visibleLines()) {
      final label = _label(line.number);
      canvas.save();
      canvas.translate(
        offset.dx + _gutterWidth - (10 + label.width) * _scale,
        offset.dy + line.centerY - lineNumberHeight / 2,
      );
      canvas.scale(_scale);
      label.paint(canvas, Offset.zero);
      canvas.restore();
    }
    canvas.restore();
  }
}
