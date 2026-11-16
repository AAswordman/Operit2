// ignore_for_file: file_names

import 'dart:math' as math;
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'CodeTextLayout.dart';
import 'CodeEditorInput.dart';
import 'CodeEditorLineNumbers.dart';
import 'CodeEditorScrollController.dart';
import 'CodeEditorZoomTransform.dart';
import 'syntax/CodeSyntaxLanguage.dart';

export 'CodeEditorScrollController.dart' show CodeEditorScrollController;
export 'CodeEditorZoomTransform.dart' show CodeEditorViewportScope;
export 'syntax/CodeSyntaxLanguage.dart' show CodeSyntaxLanguage;

/// Source and reserved gutter space for a custom body's proportional column.
/// Width is measured by the editor's paragraph layout, in logical pixels.
class CodeEditorColumn {
  const CodeEditorColumn({
    required this.text,
    this.fraction = 1,
    this.horizontalPadding = 0,
  }) : assert(fraction > 0 && fraction <= 1);

  final String text;
  final double fraction;
  final double horizontalPadding;
}

/// The application's virtual code editor, also used by plugin code views.
///
/// Supply a controller for editing, or [text] and [readOnly] for a source preview.
/// [contentBuilder] replaces the document body for decorated rows, paired columns
/// or other presentations while retaining the same zoom and scroll behavior.
class CodeEditor extends StatefulWidget {
  const CodeEditor({
    super.key,
    this.controller,
    this.text = '',
    this.focusNode,
    this.undoController,
    this.scrollController,
    this.groupId,
    this.onChanged,
    this.style,
    this.filePath = '',
    this.language,
    this.readOnly = false,
    this.active = true,
    this.showLineNumbers = true,
    this.padding = const EdgeInsets.all(12),
    this.scale,
    this.onScaleChanged,
    this.minScale = 0.6,
    this.maxScale = 2.5,
    this.panEnabled,
    this.wrapToViewport = true,
    this.contentWidth,
    this.contentColumns = const [],
    this.identity,
    this.scrollPhysics,
    this.contentBuilder,
  }) : assert(minScale > 0 && maxScale >= minScale);

  final TextEditingController? controller;
  final String text;
  final FocusNode? focusNode;
  final UndoHistoryController? undoController;
  final CodeEditorScrollController? scrollController;
  final Object? groupId;
  final ValueChanged<String>? onChanged;
  final TextStyle? style;
  final String filePath;
  final CodeSyntaxLanguage? language;
  final bool readOnly;
  final bool active;
  final bool showLineNumbers;

  /// Remains the same size on screen while the document is zoomed.
  final EdgeInsets padding;
  final double? scale;
  final ValueChanged<double>? onScaleChanged;
  final double minScale;
  final double maxScale;

  /// Defaults to panning for read-only views and selecting text for editing.
  final bool? panEnabled;
  final bool wrapToViewport;
  final double? contentWidth;

  /// Measures unwrapped custom columns; wrapping always uses live constraints.
  /// Column fractions must match the custom body's flex proportions.
  final List<CodeEditorColumn> contentColumns;
  final Object? identity;
  final ScrollPhysics? scrollPhysics;

  /// Builds under [CodeEditorViewportScope], sharing its controller and physics.
  /// Custom bodies own their gutters and must contain one vertical scrollable.
  final WidgetBuilder? contentBuilder;

  @override
  State<CodeEditor> createState() => CodeEditorState();
}

class CodeEditorState extends State<CodeEditor> {
  final _inputKey = GlobalKey<CodeEditorInputState>();
  final _viewportKey = GlobalKey<CodeEditorViewportState>();
  CodeEditorInput? _input;
  List<Object?>? _widthKey;
  List<double> _columnWidths = const [];

  double? _contentWidth(BuildContext context, TextStyle style) {
    if (widget.wrapToViewport || widget.contentColumns.isEmpty) {
      return widget.contentWidth;
    }
    final scaler = MediaQuery.textScalerOf(context);
    final direction = Directionality.of(context);
    final locale = Localizations.maybeLocaleOf(context);
    final key = <Object?>[
      style,
      scaler,
      direction,
      locale,
      for (final column in widget.contentColumns) column.text,
    ];
    if (!listEquals(key, _widthKey)) {
      _widthKey = key;
      _columnWidths = [
        for (final column in widget.contentColumns)
          CodeTextLayout.measureUnwrappedWidth(
            column.text,
            style,
            scaler,
            direction,
            locale: locale,
          ),
      ];
    }
    var width = widget.contentWidth ?? 0.0;
    for (var i = 0; i < _columnWidths.length; i++) {
      final column = widget.contentColumns[i];
      width = math.max(
        width,
        (_columnWidths[i] + column.horizontalPadding) / column.fraction + 1,
      );
    }
    return width;
  }

  TextEditingController? _ownedController;
  FocusNode? _ownedFocus;
  UndoHistoryController? _ownedUndo;
  CodeEditorScrollController? _ownedScroll;

  TextEditingController get _controller =>
      widget.controller ??
      (_ownedController ??= TextEditingController.fromValue(
        TextEditingValue(
          text: widget.text,
          selection: const TextSelection.collapsed(offset: 0),
        ),
      ));
  FocusNode get _focus => widget.focusNode ?? (_ownedFocus ??= FocusNode());
  UndoHistoryController get _undo =>
      widget.undoController ?? (_ownedUndo ??= UndoHistoryController());
  CodeEditorScrollController get _scroll =>
      widget.scrollController ??
      (_ownedScroll ??= CodeEditorScrollController());

  @override
  void initState() {
    super.initState();
    PaintingBinding.instance.systemFonts.addListener(_fontsChanged);
  }

  void _fontsChanged() {
    _widthKey = null;
    if (!widget.wrapToViewport && widget.contentColumns.isNotEmpty) {
      setState(() {});
    }
  }

  /// Provides native input/selection operations to an optional editing toolbar.
  CodeEditorInputState get input => _inputKey.currentState!;

  void zoomTo(double scale) => _viewportKey.currentState!.zoomTo(scale);

  @override
  void didUpdateWidget(CodeEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.controller != oldWidget.controller ||
        widget.focusNode != oldWidget.focusNode ||
        widget.undoController != oldWidget.undoController ||
        widget.scrollController != oldWidget.scrollController ||
        widget.groupId != oldWidget.groupId ||
        widget.style != oldWidget.style ||
        widget.filePath != oldWidget.filePath ||
        widget.language != oldWidget.language ||
        widget.onChanged != oldWidget.onChanged ||
        widget.active != oldWidget.active ||
        widget.readOnly != oldWidget.readOnly ||
        widget.scrollPhysics != oldWidget.scrollPhysics) {
      _input = null;
    }
    if (widget.controller == null && widget.text != oldWidget.text) {
      _ownedController?.text = widget.text;
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _input = null;
  }

  @override
  void dispose() {
    PaintingBinding.instance.systemFonts.removeListener(_fontsChanged);
    _ownedController?.dispose();
    _ownedFocus?.dispose();
    _ownedUndo?.dispose();
    _ownedScroll?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final custom = widget.contentBuilder != null;
    final style =
        widget.style ??
        TextStyle(
          fontFamily: 'monospace',
          fontSize: 12,
          color: theme.colorScheme.onSurface,
        );
    Widget body(double scale) => custom
        ? Builder(builder: widget.contentBuilder!)
        : Padding(
            padding: widget.padding / scale,
            child: _input ??= CodeEditorInput(
              key: _inputKey,
              controller: _controller,
              focusNode: _focus,
              undoController: _undo,
              scrollController: _scroll,
              groupId: widget.groupId ?? this,
              style: style,
              language:
                  widget.language ??
                  CodeSyntaxLanguage.forPath(widget.filePath),
              onChanged: widget.onChanged ?? (_) {},
              scrollPhysics: widget.scrollPhysics,
              active: widget.active,
              readOnly: widget.readOnly,
            ),
          );
    return CodeEditorViewport(
      key: _viewportKey,
      identity: widget.identity ?? widget.controller,
      scale: widget.scale,
      onScaleChanged: widget.onScaleChanged,
      minScale: widget.minScale,
      maxScale: widget.maxScale,
      scrollController: _scroll,
      targetKey: custom ? null : _inputKey,
      wrapToViewport: widget.wrapToViewport,
      panEnabled: widget.panEnabled ?? widget.readOnly,
      contentWidth: _contentWidth(context, style),
      chromeBuilder: (context, scale, child) => custom
          ? child
          : ListenableBuilder(
              listenable: _controller,
              child: child,
              builder: (context, child) => CodeEditorLineNumbers(
                enabled: widget.showLineNumbers,
                scale: scale,
                text: _controller.text,
                scrollController: _scroll,
                color: theme.colorScheme.onSurfaceVariant,
                dividerColor: theme.colorScheme.outlineVariant,
                child: child!,
              ),
            ),
      contentBuilder: body,
    );
  }
}
