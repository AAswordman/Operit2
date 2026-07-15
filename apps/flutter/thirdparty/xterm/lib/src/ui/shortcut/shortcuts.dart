import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Returns the terminal shortcuts for the current target platform.
Map<ShortcutActivator, Intent> get defaultTerminalShortcuts {
  final platform = defaultTargetPlatform;
  if (platform == TargetPlatform.iOS || platform == TargetPlatform.macOS) {
    return _defaultAppleShortcuts;
  }
  return _defaultShortcuts;
}

final _defaultShortcuts = {
  SingleActivator(LogicalKeyboardKey.keyC, control: true, shift: true):
      CopySelectionTextIntent.copy,
  SingleActivator(LogicalKeyboardKey.keyV, control: true):
      const PasteTextIntent(SelectionChangedCause.keyboard),
  SingleActivator(LogicalKeyboardKey.keyA, control: true):
      const SelectAllTextIntent(SelectionChangedCause.keyboard),
};

final _defaultAppleShortcuts = {
  SingleActivator(LogicalKeyboardKey.keyC, meta: true):
      CopySelectionTextIntent.copy,
  SingleActivator(LogicalKeyboardKey.keyV, meta: true):
      const PasteTextIntent(SelectionChangedCause.keyboard),
  SingleActivator(LogicalKeyboardKey.keyA, meta: true):
      const SelectAllTextIntent(SelectionChangedCause.keyboard),
};
