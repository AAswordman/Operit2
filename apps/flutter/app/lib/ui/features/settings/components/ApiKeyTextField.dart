// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';

/// API key text field with an eye button that toggles masked and plain text.
///
/// The field starts masked. Toggling only changes how the text is displayed;
/// it never modifies the controller content.
class ApiKeyTextField extends StatefulWidget {
  const ApiKeyTextField({
    super.key,
    required this.controller,
    required this.labelText,
  });

  final TextEditingController controller;
  final String labelText;

  @override
  State<ApiKeyTextField> createState() => _ApiKeyTextFieldState();
}

class _ApiKeyTextFieldState extends State<ApiKeyTextField> {
  bool _obscured = true;

  /// Flips only the display mode; the controller text stays untouched.
  void _toggleObscured() {
    setState(() => _obscured = !_obscured);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return TextFormField(
      controller: widget.controller,
      obscureText: _obscured,
      maxLines: 1,
      autocorrect: false,
      enableSuggestions: false,
      decoration: InputDecoration(
        labelText: widget.labelText,
        suffixIcon: Semantics(
          button: true,
          toggled: !_obscured,
          child: IconButton(
            tooltip: _obscured
                ? l10n.settingsModelShowApiKey
                : l10n.settingsModelHideApiKey,
            icon: Icon(
              _obscured
                  ? Icons.visibility_outlined
                  : Icons.visibility_off_outlined,
            ),
            onPressed: _toggleObscured,
          ),
        ),
      ),
    );
  }
}
