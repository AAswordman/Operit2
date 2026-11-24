// ignore_for_file: file_names
import 'package:flutter/material.dart';
import '../../core/host/RuntimeWindowCamera.dart';
import '../../data/preferences/UserPreferencesManager.dart';

/// Exposes window presentation resources to explicitly declared UI components.
class FloatingWindowScope extends InheritedWidget {
  const FloatingWindowScope({
    super.key,
    required this.camera,
    required this.appearance,
    required this.fullscreen,
    required this.toolbar,
    required this.resizeHandle,
    required super.child,
  });
  final RuntimeWindowCamera? camera;
  final ThemePreferenceSnapshot appearance;
  final bool fullscreen;
  final Widget toolbar;
  final Widget? resizeHandle;

  static FloatingWindowScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<FloatingWindowScope>();

  @override
  bool updateShouldNotify(FloatingWindowScope oldWidget) =>
      camera != oldWidget.camera ||
      appearance != oldWidget.appearance ||
      fullscreen != oldWidget.fullscreen ||
      toolbar != oldWidget.toolbar ||
      resizeHandle != oldWidget.resizeHandle;
}
