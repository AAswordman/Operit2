import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/settings/components/SettingsCategoryList.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';

/// Preserves the original conversation selection in recent and grouped lists.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
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
  setUp(ClientLogger.initialize);
  for (final brightness in Brightness.values) {
    for (final (name, primary, secondary) in [
      ('green-amber', 0xFF5B8C41, 0xFFB88D35),
      ('black-white', 0xFF000000, 0xFFFFFFFF),
      ('white-black', 0xFFFFFFFF, 0xFF000000),
      ('pink-cyan', 0xFFFF00FF, 0xFF00FFFF),
    ]) {
      for (final transparent in [false, true]) {
        testWidgets(
          'custom $name foreground roles in $brightness, glass=$transparent',
          (tester) async {
            final snapshot = ThemePreferenceSnapshot.fromJson(
              ThemePreferenceDefaults.snapshot.toJson()
                ..['themeMode'] = brightness.name
                ..['useCustomColors'] = true
                ..['customPrimaryColor'] = primary
                ..['customSecondaryColor'] = secondary
                ..['transparentSurfaceEnabled'] = transparent,
            );
            final scheme = operitColorScheme(brightness, snapshot);
            final palette = ThemeDrawerPalette.fromScheme(
              scheme,
              transparentSurface: transparent,
            );
            final boundary = GlobalKey();
            await tester.pumpWidget(
              OperitTheme(
                initialThemePreferenceSnapshot: snapshot,
                initialThemeIsReady: false,
                unconfiguredChildEnabled: true,
                hostInteractionHostsEnabled: false,
                child: RepaintBoundary(
                  key: boundary,
                  child: Scaffold(
                    backgroundColor: scheme.surface,
                    body: Column(
                      children: [
                        _sidebar('会话选中与文字对比度', palette),
                        ThemeBottomSidebarAction(
                          icon: Icons.chat,
                          label: '辅助选中项',
                          appearance: palette,
                          selected: true,
                          onClick: _noOp,
                        ),
                        SettingsCategoryTile(
                          spec: const SettingsCategorySpec(
                            title: '模型与 AI',
                            subtitle: '模型、Key、上下文',
                            description: '',
                            icon: Icons.smart_toy_outlined,
                          ),
                          selected: true,
                          onTap: _noOp,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            );
            await tester.pumpAndSettle();
            final rows = find.byType(ThemeConversationRow);
            for (var index = 0; index < 2; index++) {
              final row = rows.at(index);
              final title = tester.widget<Text>(
                find.descendant(of: row, matching: find.text('问候')),
              );
              expect(title.style!.color, scheme.onSecondaryContainer);
              if (name == 'green-amber') {
                expect(title.style!.color, Colors.white);
                expect(scheme.onPrimaryContainer, Colors.white);
              }
            }
            final auxiliary = tester.widget<Text>(find.text('辅助选中项'));
            expect(auxiliary.style!.color, scheme.onSecondaryContainer);
            final settings = find.byType(SettingsCategoryTile);
            final glass = tester.widget<ThemeGlassSurface>(
              find.descendant(
                of: settings,
                matching: find.byType(ThemeGlassSurface),
              ),
            );
            expect(glass.color, scheme.secondaryContainer);
            final title = tester.widget<Text>(find.text('模型与 AI'));
            expect(title.style!.color, scheme.onSecondaryContainer);
            final subtitle = tester.widget<Text>(find.text('模型、Key、上下文'));
            expect(subtitle.style!.color, scheme.onSecondaryContainer);
            final avatar = tester.widget<CircleAvatar>(
              find.descendant(
                of: settings,
                matching: find.byType(CircleAvatar),
              ),
            );
            expect(
              avatar.backgroundColor,
              scheme.secondary.withValues(alpha: 0.16),
            );
            expect(tester.takeException(), isNull);
            if (const bool.fromEnvironment('THEME_DESIGN_CAPTURE') &&
                name == 'green-amber' &&
                brightness == Brightness.dark &&
                !transparent) {
              await _capture(tester, boundary, 'custom-selection-dark');
            }
          },
        );
      }
    }
    testWidgets('original conversation selection in $brightness', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(960, 420);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final scheme = operitColorScheme(
        brightness,
        ThemePreferenceDefaults.snapshot,
      );
      final original = ColorScheme.fromSeed(
        seedColor: const Color(0xFF3B82F6),
        brightness: brightness,
        dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
      );
      final palette = ThemeDrawerPalette.fromScheme(
        scheme,
        transparentSurface: false,
      );
      final boundary = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData(colorScheme: scheme, fontFamily: 'Aptos'),
          home: RepaintBoundary(
            key: boundary,
            child: Scaffold(
              backgroundColor: scheme.surface,
              body: Row(
                children: [
                  Expanded(child: _sidebar('原始选中样式（已恢复）', palette)),
                  Expanded(
                    child: _sidebar(
                      '本次误改样式',
                      ThemeDrawerPalette.fromScheme(
                        scheme.copyWith(
                          secondaryContainer: scheme.primaryContainer,
                          onSecondaryContainer: scheme.onPrimaryContainer,
                        ),
                        transparentSurface: false,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final rows = find.descendant(
        of: find.byKey(const ValueKey('原始选中样式（已恢复）')),
        matching: find.byType(ThemeConversationRow),
      );
      for (var index = 0; index < 2; index++) {
        final row = rows.at(index);
        final material = tester.widget<Material>(
          find.descendant(of: row, matching: find.byType(Material)),
        );
        expect(
          material.color,
          original.secondaryContainer.withValues(
            alpha: index == 0 ? 0.50 : 0.62,
          ),
        );
        final title = tester.widget<Text>(
          find.descendant(of: row, matching: find.text('问候')),
        );
        expect(title.style!.color, original.onSecondaryContainer);
      }
      expect(tester.takeException(), isNull);
      if (const bool.fromEnvironment('THEME_DESIGN_CAPTURE')) {
        await _capture(
          tester,
          boundary,
          'sidebar-selection-${brightness.name}',
        );
      }
    });
  }
}

/// Renders both production conversation row variants with the supplied palette.
Widget _sidebar(String title, ThemeDrawerPalette palette) =>
    ThemeDesktopSidebarSurface(
      key: ValueKey(title),
      color: palette.containerColor,
      transparentSurface: false,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 28),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.all(20),
              child: Text(
                title,
                style: TextStyle(color: palette.titleColor, fontSize: 20),
              ),
            ),
            for (final workspace in [true, false]) ...[
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 20, 24, 12),
                child: Text(workspace ? '最近会话' : '项目分组 / 未分组'),
              ),
              ThemeConversationRow(
                title: '问候',
                selected: true,
                active: false,
                appearance: palette,
                leading: const SizedBox(width: 8),
                onTap: _noOp,
                onLongPress: _noOp,
                showActions: true,
                action: const Text('···'),
                workspaceStyle: workspace,
                nested: false,
              ),
            ],
          ],
        ),
      ),
    );

/// Keeps the isolated sidebar fixture free of navigation actions.
void _noOp() {}

/// Captures real Flutter components without altering their rendered image.
Future<void> _capture(
  WidgetTester tester,
  GlobalKey boundary,
  String name,
) async {
  await tester.runAsync(() async {
    final image =
        await (boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary)
            .toImage(pixelRatio: 2);
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    final folder = Platform.environment['OPERIT_THEME_RENDER_DIR']!;
    await Directory(folder).create(recursive: true);
    await File('$folder/$name.png').writeAsBytes(bytes!.buffer.asUint8List());
    image.dispose();
  });
}
