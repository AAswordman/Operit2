import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Checks that shared theme rendering preserves the main branch's default UI.
void main() {
  for (final brightness in Brightness.values) {
    final scheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFFBBDEFB),
      brightness: brightness,
      dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
    );
    for (final scale in <double>[1, 1.3]) {
      testWidgets(
        'default $brightness typography at scale $scale preserves main colors and geometry',
        (tester) async {
          final settings = ThemePreferenceSnapshot.fromJson(<String, Object?>{
            ...ThemePreferenceDefaults.snapshot.toJson(),
            'fontScale': scale,
          });
          final current = buildOperitThemeData(
            scheme,
            settings,
            fontFamily: 'Aptos',
            fontFamilyFallback: const <String>['sans-serif'],
          );
          final typography = Typography.material2021();
          final baseline = ThemeData(
            colorScheme: scheme,
            textTheme:
                (brightness == Brightness.dark
                        ? typography.white
                        : typography.black)
                    .apply(
                      fontFamily: 'Aptos',
                      fontFamilyFallback: const <String>['sans-serif'],
                      fontSizeFactor: 1,
                    ),
            fontFamily: 'Aptos',
            fontFamilyFallback: const <String>['sans-serif'],
            useMaterial3: true,
          );
          final expected = await _resolveTheme(tester, baseline);
          final actual = await _resolveTheme(tester, current);
          final expectedStyles = _textStyles(expected.textTheme);
          final actualStyles = _textStyles(actual.textTheme);
          for (final name in expectedStyles.keys) {
            final expectedStyle = expectedStyles[name]!;
            final actualStyle = actualStyles[name]!;
            expect(
              actualStyle.color,
              expectedStyle.color,
              reason: '$name color',
            );
            expect(
              actualStyle.fontSize,
              expectedStyle.fontSize! * scale,
              reason: '$name size',
            );
            expect(
              actualStyle.fontWeight,
              expectedStyle.fontWeight,
              reason: '$name weight',
            );
            expect(
              actualStyle.height,
              expectedStyle.height,
              reason: '$name height',
            );
            expect(
              actualStyle.letterSpacing,
              expectedStyle.letterSpacing,
              reason: '$name spacing',
            );
            expect(
              actualStyle.fontFamily,
              expectedStyle.fontFamily,
              reason: '$name family',
            );
          }
        },
      );
    }

    for (final transparent in <bool>[false, true]) {
      test(
        'default $brightness drawer palette preserves main, transparent=$transparent',
        () {
          final palette = ThemeDrawerPalette.fromScheme(
            scheme,
            transparentSurface: transparent,
          );
          expect(
            palette.containerColor,
            scheme.surface.withValues(alpha: transparent ? 0.04 : 0.88),
          );
          expect(palette.titleColor, scheme.onSurface);
          expect(palette.statusAvailableColor, scheme.primary);
          expect(palette.itemColor, scheme.onSurfaceVariant);
          expect(
            palette.buttonContainerColor,
            scheme.surfaceContainerLow.withValues(
              alpha: transparent ? 0.18 : 0.72,
            ),
          );
          expect(
            palette.selectedContainerColor,
            scheme.secondaryContainer.withValues(
              alpha: transparent ? 0.34 : 0.78,
            ),
          );
          expect(palette.selectedContentColor, scheme.onSecondaryContainer);
          expect(
            palette.dividerColor,
            scheme.outlineVariant.withValues(alpha: transparent ? 0.34 : 0.62),
          );
        },
      );
    }

    for (final classic in <bool>[false, true]) {
      testWidgets(
        'default $brightness ${classic ? 'classic' : 'agent'} input uses surface text colors',
        (tester) async {
          final controller = TextEditingController(text: 'Readable message');
          final focus = FocusNode();
          addTearDown(controller.dispose);
          addTearDown(focus.dispose);
          final editor = classic
              ? ThemeClassicComposerTextEditor(
                  controller: controller,
                  focusNode: focus,
                  hintText: 'Message',
                  inputExpanded: false,
                  readOnly: false,
                  hasDraftText: true,
                  settings: const SizedBox.shrink(),
                  onSubmitted: _ignoreSubmit,
                  onToggleExpansion: _ignoreToggle,
                  expandTooltip: 'Expand',
                  collapseTooltip: 'Collapse',
                )
              : ThemeComposerTextEditor(
                  controller: controller,
                  focusNode: focus,
                  hintText: 'Message',
                  inputExpanded: false,
                  readOnly: false,
                  onSubmitted: _ignoreSubmit,
                  onToggleExpansion: _ignoreToggle,
                  expandTooltip: 'Expand',
                  collapseTooltip: 'Collapse',
                );
          await tester.pumpWidget(
            MaterialApp(
              theme: buildOperitThemeData(
                scheme,
                ThemePreferenceDefaults.snapshot,
                fontFamily: 'Aptos',
                fontFamilyFallback: const <String>[],
              ),
              home: Scaffold(
                body: Column(
                  children: <Widget>[
                    ThemeComposerCard(
                      inputStyle: classic
                          ? ThemePreferenceDefaults.INPUT_STYLE_CLASSIC
                          : ThemePreferenceDefaults.INPUT_STYLE_AGENT,
                      floating: false,
                      transparentSurface: false,
                      child: editor,
                    ),
                    const ThemeComposerProviderInitial(initial: 'P', size: 18),
                  ],
                ),
              ),
            ),
          );
          final field = tester.widget<TextField>(find.byType(TextField));
          final inherited = Theme.of(tester.element(find.byType(TextField)));
          expect(field.style!.color, inherited.textTheme.bodyMedium!.color);
          expect(field.decoration!.hintStyle!.color, scheme.onSurfaceVariant);
          if (classic) {
            expect(
              field.decoration!.fillColor,
              scheme.surface.withValues(alpha: 0.84),
            );
          }
          expect(
            tester.widget<Text>(find.text('P')).style!.color,
            scheme.onSurfaceVariant,
          );
          final contrast = _contrast(field.style!.color!, scheme.surface);
          expect(
            contrast,
            greaterThanOrEqualTo(4.5),
            reason: 'Default input text must remain readable',
          );
          expect(tester.takeException(), isNull);
        },
      );
    }

    for (final classic in <bool>[false, true]) {
      for (final floating in <bool>[false, true]) {
        testWidgets(
          'default $brightness composer inset, classic=$classic floating=$floating',
          (tester) async {
            await tester.pumpWidget(
              MaterialApp(
                theme: ThemeData(colorScheme: scheme),
                home: Scaffold(
                  body: MediaQuery(
                    data: const MediaQueryData(
                      padding: EdgeInsets.only(bottom: 24),
                    ),
                    child: Align(
                      alignment: Alignment.bottomCenter,
                      child: ThemeComposerCard(
                        inputStyle: classic
                            ? ThemePreferenceDefaults.INPUT_STYLE_CLASSIC
                            : ThemePreferenceDefaults.INPUT_STYLE_AGENT,
                        floating: floating,
                        transparentSurface: false,
                        child: const SizedBox(
                          key: ValueKey<String>('composer-content'),
                          height: 30,
                          width: 100,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            );
            final card = tester.getRect(find.byType(ThemeComposerCard));
            final surface = tester.getRect(
              find.byKey(const ValueKey<String>('chat.input.surface')),
            );
            final content = tester.getRect(
              find.byKey(const ValueKey<String>('composer-content')),
            );
            final contentPadding = classic ? (floating ? 6.0 : 8.0) : 8.0;
            expect(
              surface.bottom - content.bottom,
              contentPadding + (floating ? 0 : 24),
            );
            expect(card.bottom - surface.bottom, floating ? 30 : 0);
            final decoration =
                tester
                        .widget<DecoratedBox>(
                          find.byKey(
                            const ValueKey<String>('chat.input.surface'),
                          ),
                        )
                        .decoration
                    as ShapeDecoration;
            expect(
              decoration.color,
              classic ? scheme.surface : scheme.surfaceContainer,
            );
            expect(tester.takeException(), isNull);
          },
        );
      }
    }

    testWidgets('preview $brightness uses the same surface roles as the app', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1280, 800);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.pumpWidget(
        MaterialApp(
          theme: buildOperitThemeData(
            scheme,
            ThemePreferenceDefaults.snapshot,
            fontFamily: 'Aptos',
            fontFamilyFallback: const <String>[],
          ),
          home: const ThemeClientScene(
            snapshot: ThemePreferenceDefaults.snapshot,
          ),
        ),
      );
      final topBar = tester.widget<ThemeTopBarChrome>(
        find.byType(ThemeTopBarChrome),
      );
      expect(topBar.contentColor, scheme.surface);
      expect(topBar.contentForeground, scheme.onSurface);
      final identity = tester.widget<ThemeComposerModelIdentity>(
        find.byType(ThemeComposerModelIdentity),
      );
      expect(identity.style.color, scheme.onSurface);
      expect(identity.chevronColor, scheme.onSurfaceVariant);
      final action = tester.widget<ThemeComposerActionButton>(
        find.byType(ThemeComposerActionButton),
      );
      expect(action.background, scheme.primary);
      expect(action.foreground, scheme.onPrimary);
      expect(tester.takeException(), isNull);
    });

    test(
      'default $brightness system bars preserve transparent main styling',
      () {
        final overlay = operitSystemUiOverlayStyle(scheme);
        expect(overlay.statusBarColor, Colors.transparent);
        expect(overlay.systemStatusBarContrastEnforced, false);
        expect(overlay.systemNavigationBarColor, Colors.transparent);
        expect(overlay.systemNavigationBarDividerColor, Colors.transparent);
        expect(overlay.systemNavigationBarContrastEnforced, false);
        expect(
          overlay.systemNavigationBarIconBrightness,
          brightness == Brightness.dark ? Brightness.light : Brightness.dark,
        );
      },
    );
  }
}

/// Resolves Material typography with the same localization step used by the app.
Future<ThemeData> _resolveTheme(WidgetTester tester, ThemeData theme) async {
  late ThemeData resolved;
  await tester.pumpWidget(
    MaterialApp(
      key: UniqueKey(),
      theme: theme,
      home: Builder(
        builder: (context) {
          resolved = Theme.of(context);
          return const SizedBox.shrink();
        },
      ),
    ),
  );
  await tester.pumpAndSettle();
  return resolved;
}

/// Enumerates every Material text role for default typography comparisons.
Map<String, TextStyle?> _textStyles(TextTheme theme) => <String, TextStyle?>{
  'displayLarge': theme.displayLarge,
  'displayMedium': theme.displayMedium,
  'displaySmall': theme.displaySmall,
  'headlineLarge': theme.headlineLarge,
  'headlineMedium': theme.headlineMedium,
  'headlineSmall': theme.headlineSmall,
  'titleLarge': theme.titleLarge,
  'titleMedium': theme.titleMedium,
  'titleSmall': theme.titleSmall,
  'bodyLarge': theme.bodyLarge,
  'bodyMedium': theme.bodyMedium,
  'bodySmall': theme.bodySmall,
  'labelLarge': theme.labelLarge,
  'labelMedium': theme.labelMedium,
  'labelSmall': theme.labelSmall,
};

/// Computes the WCAG contrast ratio between two opaque default colors.
double _contrast(Color foreground, Color background) {
  final first = foreground.computeLuminance();
  final second = background.computeLuminance();
  return first > second
      ? (first + 0.05) / (second + 0.05)
      : (second + 0.05) / (first + 0.05);
}

/// Supplies an inert submit callback for the isolated editor fixture.
void _ignoreSubmit(String value) {}

/// Supplies an inert expansion callback for the isolated editor fixture.
void _ignoreToggle() {}
