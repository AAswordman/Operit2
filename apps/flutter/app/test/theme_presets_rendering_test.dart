import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/settings/components/SettingsCategoryList.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/theme/ThemeColorPresets.dart';

/// Captures production selection widgets for every built-in theme in both modes.
void main() {
  setUp(ClientLogger.initialize);
  setUpAll(() async {
    if (const bool.fromEnvironment('THEME_DESIGN_CAPTURE')) {
      await (FontLoader('Aptos')..addFont(
            rootBundle.load('packages/operit_ui/assets/fonts/noto-sans-sc.ttf'),
          ))
          .load();
      await (FontLoader(
        'MaterialIcons',
      )..addFont(rootBundle.load('fonts/MaterialIcons-Regular.otf'))).load();
    }
  });
  for (final brightness in Brightness.values) {
    for (final preset in themeColorPresets) {
      test('${preset.id} retains M3 tone roles in $brightness', () {
        final snapshot = _presetSnapshot(preset);
        final scheme = operitColorScheme(brightness, snapshot);
        final expected = ColorScheme.fromSeed(
          seedColor: Color(preset.primaryColor ?? 0xFF3B82F6),
          brightness: brightness,
          dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
        );
        expect(themeColorPresetForSnapshot(snapshot)?.id, preset.id);
        for (final (actual, reference) in [
          (scheme.primary, expected.primary),
          (scheme.onPrimary, expected.onPrimary),
          (scheme.primaryContainer, expected.primaryContainer),
          (scheme.onPrimaryContainer, expected.onPrimaryContainer),
          (scheme.secondary, expected.secondary),
          (scheme.onSecondary, expected.onSecondary),
          (scheme.secondaryContainer, expected.secondaryContainer),
          (scheme.onSecondaryContainer, expected.onSecondaryContainer),
          (scheme.tertiaryContainer, expected.tertiaryContainer),
        ]) {
          expect(actual, reference);
        }
        expect(scheme.primary, isNot(scheme.primaryContainer));
        expect(scheme.secondary, isNot(scheme.secondaryContainer));
        expect(
          scheme.surface,
          operitColorScheme(
            brightness,
            ThemePreferenceDefaults.snapshot,
          ).surface,
        );
      });
    }
    testWidgets('built-in presets in ${brightness.name}', (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1600, 520);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final boundary = GlobalKey();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot: ThemePreferenceDefaults.snapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: RepaintBoundary(
            key: boundary,
            child: Row(
              children: [
                for (final preset in themeColorPresets)
                  Expanded(child: _presetPreview(preset, brightness)),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      if (const bool.fromEnvironment('THEME_DESIGN_CAPTURE')) {
        await tester.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1.5);
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          final folder = Platform.environment['OPERIT_THEME_RENDER_DIR']!;
          await Directory(folder).create(recursive: true);
          await File(
            '$folder/presets-${brightness.name}.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
          image.dispose();
        });
      }
    });
  }
}

/// Uses the persisted preset and real settings/sidebar components for each sample.
Widget _presetPreview(ThemeColorPreset preset, Brightness brightness) {
  final snapshot = _presetSnapshot(preset);
  final scheme = operitColorScheme(brightness, snapshot);
  final palette = ThemeDrawerPalette.fromScheme(
    scheme,
    transparentSurface: false,
  );
  final label = switch (preset.id) {
    'default' => '默认',
    'sky' => '天蓝',
    'matcha' => '抹茶',
    'ember' => '暖橙',
    'rose' => '玫瑰',
    _ => throw StateError(preset.id),
  };
  return Theme(
    data: buildOperitThemeData(
      scheme,
      snapshot,
      fontFamily: 'Aptos',
      fontFamilyFallback: const [],
    ),
    child: Scaffold(
      backgroundColor: scheme.surface,
      body: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: 14),
            Text(
              '$label · ${brightness == Brightness.dark ? '深色' : '浅色'}',
              style: TextStyle(fontSize: 22, color: scheme.onSurface),
            ),
            const SizedBox(height: 28),
            ThemeUnifiedCreateBar(
              onCreateConversation: _noOp,
              onCreateGroup: _noOp,
            ),
            const SizedBox(height: 24),
            const Text('最近会话'),
            const SizedBox(height: 12),
            ThemeConversationRow(
              title: '问候',
              selected: true,
              active: false,
              appearance: palette,
              leading: const SizedBox(width: 8),
              onTap: _noOp,
              onLongPress: _noOp,
              showActions: true,
              action: Icon(
                Icons.more_horiz,
                size: 18,
                color: scheme.onSecondaryContainer,
              ),
              workspaceStyle: true,
              nested: false,
            ),
            const SizedBox(height: 24),
            SettingsCategoryTile(
              spec: const SettingsCategorySpec(
                title: '外观与交互',
                subtitle: '主题、气泡与交互',
                description: '',
                icon: Icons.palette_outlined,
              ),
              selected: true,
              onTap: _noOp,
            ),
            const SizedBox(height: 24),
            Row(
              children: [
                Switch(value: true, onChanged: (_) {}),
                const SizedBox(width: 8),
                const Text('跟随主题'),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}

/// Reuses the existing persisted fields without a settings migration.
ThemePreferenceSnapshot _presetSnapshot(ThemeColorPreset preset) =>
    ThemePreferenceSnapshot.fromJson({
      ...ThemePreferenceDefaults.snapshot.toJson(),
      'useCustomColors': preset.useCustomColors,
      'customPrimaryColor': preset.primaryColor,
      'customSecondaryColor': preset.secondaryColor,
    });

/// Keeps preview actions inert.
void _noOp() {}
