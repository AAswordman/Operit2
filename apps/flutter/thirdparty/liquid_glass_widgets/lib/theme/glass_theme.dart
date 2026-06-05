import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'glass_theme_data.dart';

/// Provides glass theme configuration to descendant widgets.
///
/// This [InheritedWidget] allows glass widgets to access centralized theme
/// settings without passing them explicitly through the widget tree.
///
/// ## Usage
///
/// Wrap your app (or a section of it) with [GlassTheme]:
///
/// ```dart
/// MaterialApp(
///   builder: (context, child) => GlassTheme(
///     data: GlassThemeData(
///       light: GlassThemeVariant(
///         settings: LiquidGlassSettings(thickness: 30),
///         quality: GlassQuality.standard,
///       ),
///       dark: GlassThemeVariant(
///         settings: LiquidGlassSettings(thickness: 40),
///         quality: GlassQuality.standard,
///       ),
///     ),
///     child: child!,
///   ),
/// )
/// ```
///
/// Access theme in widgets:
///
/// ```dart
/// final theme = GlassThemeData.of(context);
/// final settings = theme.settingsFor(context); // Auto light/dark
/// ```
///
/// ## Theme Inheritance
///
/// Widgets automatically inherit theme settings from the nearest [GlassTheme]
/// ancestor. Individual widgets can override theme values by explicitly
/// passing `settings`, `quality`, or `glowColor` parameters.
///
/// ## Light/Dark Mode
///
/// The theme automatically switches between light and dark variants based on
/// [MediaQuery.platformBrightnessOf]. You don't need to manually check
/// brightness in your widgets.
class GlassTheme extends InheritedWidget {
  /// Creates a glass theme.
  ///
  /// The [data] parameter contains theme configuration for light and dark modes.
  const GlassTheme({
    required this.data,
    required super.child,
    super.key,
  });

  /// The theme data containing light and dark configurations.
  final GlassThemeData data;

  /// Retrieves the [GlassTheme] from the widget tree.
  ///
  /// Returns null if no [GlassTheme] ancestor exists.
  static GlassTheme? maybeOf(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<GlassTheme>();
  }

  /// Retrieves the [GlassTheme] from the widget tree.
  ///
  /// Throws if no [GlassTheme] ancestor exists. Use [maybeOf] for a
  /// null-safe alternative.
  static GlassTheme of(BuildContext context) {
    final theme = maybeOf(context);
    assert(
      theme != null,
      'No GlassTheme found in context. '
      'Wrap your app with GlassTheme to provide theme configuration.',
    );
    return theme!;
  }

  @override
  bool updateShouldNotify(GlassTheme oldWidget) {
    return data != oldWidget.data;
  }

  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(DiagnosticsProperty<GlassThemeData>('data', data));
  }
}
