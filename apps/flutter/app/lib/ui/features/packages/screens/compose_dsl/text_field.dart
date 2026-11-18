// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

class _ComposeTextField extends StatefulWidget {
  /// Creates the compose text field instance.
  const _ComposeTextField({
    required this.identity,
    required this.value,
    required this.enabled,
    required this.readOnly,
    required this.obscureText,
    required this.singleLine,
    this.expandToFill = false,
    required this.minLines,
    required this.maxLines,
    required this.keyboardType,
    required this.textInputAction,
    required this.isError,
    required this.textStyle,
    required this.labelText,
    required this.label,
    required this.hintText,
    required this.hint,
    required this.prefixIcon,
    required this.suffixIcon,
    required this.prefix,
    required this.suffix,
    required this.supportingText,
    required this.helperText,
    required this.border,
    this.decoration,
    this.cursorWidth,
    this.cursorHeight,
    this.suffixIconConstraints,
    required this.onChanged,
    this.onSubmitted,
  });

  final String identity;
  final String value;
  final bool enabled;
  final bool readOnly;
  final bool obscureText;
  final bool singleLine;
  final bool expandToFill;
  final int? minLines;
  final int? maxLines;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final bool isError;
  final TextStyle? textStyle;
  final String? labelText;
  final Widget? label;
  final String? hintText;
  final Widget? hint;
  final Widget? prefixIcon;
  final Widget? suffixIcon;
  final Widget? prefix;
  final Widget? suffix;
  final Widget? supportingText;
  final String? helperText;
  final InputBorder? border;
  final InputDecoration? decoration;
  final double? cursorWidth;
  final double? cursorHeight;
  final BoxConstraints? suffixIconConstraints;

  /// Resolves function for the Compose DSL renderer.
  final Future<Object?> Function(String) onChanged;
  final Future<Object?> Function(String)? onSubmitted;

  /// Creates persistent state for this DSL widget.
  @override
  State<_ComposeTextField> createState() => _ComposeTextFieldState();
}

class _ComposeTextFieldState extends State<_ComposeTextField> {
  late TextEditingController _controller;
  late FocusNode _focusNode;
  late String _lastAppliedExternalValue;
  final List<String> _pendingEchoes = <String>[];
  Future<void> _dispatchTail = Future<void>.value();
  bool _submitting = false;

  /// Flushes pending text edits before submitting the exact current value once.
  Future<void> _submit(String value) async {
    if (_submitting ||
        !widget.enabled ||
        widget.readOnly ||
        widget.onSubmitted == null) {
      return;
    }
    _submitting = true;
    final identity = widget.identity;
    try {
      await _dispatchTail;
      if (!mounted ||
          widget.identity != identity ||
          !widget.enabled ||
          widget.readOnly) {
        return;
      }
      await widget.onSubmitted?.call(value);
    } finally {
      _submitting = false;
    }
  }

  /// Serializes edits and records values before the JavaScript round trip starts.
  void _enqueueEdit(String value) {
    _pendingEchoes.add(value);
    _dispatchTail = widget.onChanged(value).then((_) {});
  }

  /// Waits for dispatched edits before reconciling a field that lost focus.
  Future<void> _flushOnBlur() async {
    if (_focusNode.hasFocus) return;
    final identity = widget.identity;
    await _dispatchTail;
    if (!mounted || widget.identity != identity || _focusNode.hasFocus) return;
    _reconcileValue();
  }

  /// Initializes resources owned by this DSL widget.
  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.value);
    _controller.selection = TextSelection.collapsed(
      offset: widget.value.length,
    );
    _focusNode = FocusNode();
    _focusNode.addListener(_flushOnBlur);
    _lastAppliedExternalValue = widget.value;
  }

  /// Synchronizes widget state with the latest DSL node.
  @override
  void didUpdateWidget(_ComposeTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.identity != oldWidget.identity) {
      _controller.dispose();
      _focusNode.dispose();
      _controller = TextEditingController(text: widget.value);
      _controller.selection = TextSelection.collapsed(
        offset: widget.value.length,
      );
      _focusNode = FocusNode();
      _focusNode.addListener(_flushOnBlur);
      _lastAppliedExternalValue = widget.value;
      _pendingEchoes.clear();
      return;
    }
    _reconcileValue();
  }

  /// Mirrors Kotlin's converged, own-echo, and external-change reconciliation.
  void _reconcileValue() {
    if (widget.value == _controller.text) {
      _pendingEchoes.clear();
      _lastAppliedExternalValue = widget.value;
      return;
    }
    if (_focusNode.hasFocus || _pendingEchoes.isNotEmpty) {
      if (widget.value == _lastAppliedExternalValue) return;
      final echoIndex = _pendingEchoes.indexOf(widget.value);
      if (echoIndex >= 0) {
        _pendingEchoes.removeRange(0, echoIndex + 1);
        return;
      }
    }
    final selection = _controller.selection;
    _pendingEchoes.clear();
    final start = selection.start.clamp(0, widget.value.length);
    final end = selection.end.clamp(0, widget.value.length);
    _controller.value = TextEditingValue(
      text: widget.value,
      selection: TextSelection(baseOffset: start, extentOffset: end),
    );
    _lastAppliedExternalValue = widget.value;
  }

  /// Releases resources owned by this DSL widget.
  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// Builds the widget for the current DSL state.
  @override
  Widget build(BuildContext context) {
    if (widget.expandToFill) {
      return LayoutBuilder(
        builder: (context, constraints) =>
            _buildField(expands: constraints.hasBoundedHeight),
      );
    }
    return _buildField();
  }

  /// Fills bounded multiline editors while retaining internal text scrolling.
  Widget _buildField({bool expands = false}) {
    return TextField(
      controller: _controller,
      focusNode: _focusNode,
      enabled: widget.enabled,
      readOnly: widget.readOnly,
      obscureText: widget.obscureText,
      expands: expands,
      maxLines: expands ? null : widget.maxLines,
      minLines: expands ? null : widget.minLines,
      textAlignVertical: expands ? TextAlignVertical.top : null,
      keyboardType: widget.keyboardType,
      textInputAction: widget.textInputAction,
      style: widget.textStyle,
      cursorWidth: widget.cursorWidth ?? 2,
      cursorHeight: widget.cursorHeight,
      decoration: (widget.decoration ?? const InputDecoration()).copyWith(
        labelText: widget.labelText,
        label: widget.label,
        hintText: widget.hintText,
        hint: widget.hint,
        errorText: widget.isError ? '' : null,
        prefixIcon: widget.prefixIcon,
        suffixIcon: widget.suffixIcon,
        suffixIconConstraints: widget.suffixIconConstraints,
        prefix: widget.prefix,
        suffix: widget.suffix,
        helperText: widget.helperText,
        helper: widget.supportingText,
        border: widget.decoration?.border ?? widget.border,
      ),
      onChanged: _enqueueEdit,
      onSubmitted: widget.onSubmitted == null ? null : _submit,
    );
  }
}
