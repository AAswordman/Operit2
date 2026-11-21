// ignore_for_file: file_names

import 'package:flutter/material.dart';

/// Shows API keys while focused and masks them without password input mode.
class ApiKeyTextField extends StatefulWidget {
  /// Creates a regular text input with focus-based visual masking.
  const ApiKeyTextField({
    super.key,
    required this.controller,
    required this.labelText,
    this.helperText,
    this.minLines = 1,
    this.maxLines = 1,
    this.validator,
  });

  final TextEditingController controller;
  final String labelText;
  final String? helperText;
  final int minLines;
  final int maxLines;
  final FormFieldValidator<String>? validator;

  /// Creates the field's focus and presentation state.
  @override
  State<ApiKeyTextField> createState() => _ApiKeyTextFieldState();
}

class _ApiKeyTextFieldState extends State<ApiKeyTextField> {
  final _focusNode = FocusNode();
  late _ApiKeyDisplayController _displayController;

  /// Connects the display controller to the caller's unchanged editing value.
  @override
  void initState() {
    super.initState();
    _displayController = _ApiKeyDisplayController(
      widget.controller,
      _focusNode,
    );
  }

  /// Rebinds the presentation when the caller replaces its controller.
  @override
  void didUpdateWidget(covariant ApiKeyTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      _displayController.dispose();
      _displayController = _ApiKeyDisplayController(
        widget.controller,
        _focusNode,
      );
    }
  }

  /// Releases presentation listeners without disposing the caller's controller.
  @override
  void dispose() {
    _displayController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// Keeps every input connection in ordinary text mode, including before focus.
  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: _displayController,
      focusNode: _focusNode,
      obscureText: false,
      keyboardType: widget.maxLines == 1
          ? TextInputType.text
          : TextInputType.multiline,
      minLines: widget.minLines,
      maxLines: widget.maxLines,
      autocorrect: false,
      // Disabling suggestions maps to VISIBLE_PASSWORD in Flutter's Android host.
      enableSuggestions: true,
      enableIMEPersonalizedLearning: false,
      smartDashesType: SmartDashesType.disabled,
      smartQuotesType: SmartQuotesType.disabled,
      decoration: InputDecoration(
        labelText: widget.labelText,
        helperText: widget.helperText,
      ),
      validator: widget.validator,
    );
  }
}

/// Delegates editing to the original controller and changes only rendered spans.
class _ApiKeyDisplayController extends TextEditingController {
  /// Refreshes the rendered text whenever its value or focus changes.
  _ApiKeyDisplayController(this.source, this.focusNode) {
    source.addListener(notifyListeners);
    focusNode.addListener(notifyListeners);
  }

  final TextEditingController source;
  final FocusNode focusNode;

  /// Reads the original text, selection, and composing range directly.
  @override
  TextEditingValue get value => source.value;

  /// Sends edits directly to the original controller without transforming them.
  @override
  set value(TextEditingValue newValue) => source.value = newValue;

  /// Masks unfocused text while preserving line breaks and editing offsets.
  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    if (focusNode.hasFocus) {
      return source.buildTextSpan(
        context: context,
        style: style,
        withComposing: withComposing,
      );
    }
    return TextSpan(
      style: style,
      text: text.replaceAll(RegExp(r'[^\r\n]'), '*'),
    );
  }

  /// Detaches listeners owned by this presentation controller.
  @override
  void dispose() {
    source.removeListener(notifyListeners);
    focusNode.removeListener(notifyListeners);
    super.dispose();
  }
}
