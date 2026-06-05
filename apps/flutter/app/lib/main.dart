import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'ui/main/OperitApp.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LiquidGlassWidgets.initialize();
  runApp(
    LiquidGlassWidgets.wrap(
      respectSystemAccessibility: false,
      theme: GlassThemeData.simple(
        blur: 2.5,
        thickness: 36,
        quality: GlassQuality.standard,
      ),
      child: const OperitApp(),
    ),
  );
}
