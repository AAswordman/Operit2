// ignore_for_file: file_names

import 'package:flutter/material.dart';

class OperitFormStyles {
  const OperitFormStyles._();

  /// Builds a form dropdown with the application's shared field and menu style.
  static Widget dropdownButtonFormField<T>(
    BuildContext context, {
    Key? key,
    required List<DropdownMenuItem<T>>? items,
    DropdownButtonBuilder? selectedItemBuilder,
    T? initialValue,
    Widget? hint,
    Widget? disabledHint,
    required ValueChanged<T?>? onChanged,
    VoidCallback? onTap,
    int elevation = 3,
    TextStyle? style,
    Widget? icon,
    Color? iconDisabledColor,
    Color? iconEnabledColor,
    double iconSize = 24.0,
    bool isDense = true,
    bool isExpanded = false,
    double? itemHeight,
    Color? focusColor,
    FocusNode? focusNode,
    bool autofocus = false,
    InputDecoration? decoration,
    FormFieldSetter<T>? onSaved,
    FormFieldValidator<T>? validator,
    AutovalidateMode? autovalidateMode,
    double? menuMaxHeight,
    bool? enableFeedback,
    AlignmentGeometry alignment = AlignmentDirectional.centerStart,
    EdgeInsetsGeometry? padding,
    bool barrierDismissible = true,
    MouseCursor? mouseCursor,
    MouseCursor? dropdownMenuItemMouseCursor,
  }) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return DropdownButtonFormField<T>(
      key: key,
      items: items,
      selectedItemBuilder: selectedItemBuilder,
      initialValue: initialValue,
      hint: hint,
      disabledHint: disabledHint,
      onChanged: onChanged,
      onTap: onTap,
      elevation: elevation,
      style: style ?? dropdownTextStyle(context),
      icon: icon,
      iconDisabledColor: iconDisabledColor,
      iconEnabledColor: iconEnabledColor,
      iconSize: iconSize,
      isDense: isDense,
      isExpanded: isExpanded,
      itemHeight: itemHeight,
      focusColor: focusColor,
      focusNode: focusNode,
      autofocus: autofocus,
      dropdownColor: colors.surfaceContainerHigh,
      decoration: decoration,
      onSaved: onSaved,
      validator: validator,
      autovalidateMode: autovalidateMode,
      menuMaxHeight: menuMaxHeight,
      enableFeedback: enableFeedback,
      alignment: alignment,
      borderRadius: BorderRadius.circular(8),
      padding: padding,
      barrierDismissible: barrierDismissible,
      mouseCursor: mouseCursor,
      dropdownMenuItemMouseCursor: dropdownMenuItemMouseCursor,
    );
  }

  /// Returns the shared text style for form dropdown controls.
  static TextStyle? dropdownTextStyle(BuildContext context) {
    final theme = Theme.of(context);
    return theme.textTheme.bodyMedium?.copyWith(
      color: theme.colorScheme.onSurface,
    );
  }
}
