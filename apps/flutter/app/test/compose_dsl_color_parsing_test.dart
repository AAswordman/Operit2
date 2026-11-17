import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Exercises color resolution through real card and text rendering.
Future<void> mountColor(
  WidgetTester tester,
  Object color, {
  ThemeData? theme,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: theme,
      home: Scaffold(
        body: buildComposeDslLayoutForTest(
          node: {
            'type': 'Card',
            'props': {'containerColor': color, 'width': 260, 'height': 48},
            'children': [
              {
                'type': 'Text',
                'props': {'text': 'VoiceBar duration', 'color': color},
              },
            ],
          },
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'test.color.parsing',
            routeInstanceId: 'color-test',
            executionContextKey: 'color-test',
            dispatchAction: (id, [payload]) async => null,
            runtimeOptionsProvider: () => {},
          ),
          onTextInput: (_, _) async => null,
          splitMarkdownContent: (_) async => [],
        ),
      ),
    ),
  );
  expect(tester.takeException(), isNull);
}

/// Checks both component backgrounds and text without relying on theme fallbacks.
void expectColor(WidgetTester tester, int argb, Object input) {
  final card = tester.widget<Card>(find.byType(Card));
  final text = tester.widget<Text>(find.text('VoiceBar duration'));
  expect(card.color?.toARGB32(), argb, reason: 'Card color for $input');
  expect(text.style?.color?.toARGB32(), argb, reason: 'Text color for $input');
}

void main() {
  testWidgets('short RGB hex is opaque and preserves RGB and ARGB formats', (
    tester,
  ) async {
    for (final entry in {
      '#fff': 0xffffffff,
      '#000': 0xff000000,
      ' #AbC ': 0xffaabbcc,
      '#112233': 0xff112233,
      '#80112233': 0x80112233,
      '#00112233': 0x00112233,
    }.entries) {
      await mountColor(tester, entry.key);
      expectColor(tester, entry.value, entry.key);
    }
  });

  testWidgets('RGB and RGBA colors preserve channel values and transparency', (
    tester,
  ) async {
    for (final entry in {
      'rgb(255, 128, 0)': 0xffff8000,
      'rgba(255,255,255,0.9)': 0xe6ffffff,
      ' RGBA(10, 20, 30, .5) ': 0x800a141e,
      'rgba(255,0,0,0)': 0x00ff0000,
      'rgb(100%,50%,0%)': 0xffff8000,
      'rgba(100%,0%,0%,50%)': 0x80ff0000,
      'rgba(300,-20,0,2)': 0xffff0000,
    }.entries) {
      await mountColor(tester, entry.key);
      expectColor(tester, entry.value, entry.key);
    }
  });

  testWidgets(
    'invalid color literals fall back without producing partial colors',
    (tester) async {
      for (final input in [
        '#ff',
        '#fffff',
        '#112233445',
        '#ggg',
        'rgb(1,2,3,0.5)',
        'rgba(1,2,3)',
        'rgba(1,2,3,NaN)',
        'rgba(Infinity,2,3,1)',
        'rgba(1,2,3,)',
        'rgba(1,2,3,0.5) trailing',
      ]) {
        await mountColor(tester, input);
        expect(tester.widget<Card>(find.byType(Card)).color, isNull);
        final textFinder = find.text('VoiceBar duration');
        final theme = Theme.of(tester.element(textFinder));
        expect(
          tester.widget<Text>(textFinder).style?.color,
          theme.textTheme.bodyMedium!.color,
          reason: 'Invalid $input must retain the theme text color',
        );
      }
    },
  );

  testWidgets('theme colors and alpha composition still resolve normally', (
    tester,
  ) async {
    final theme = ThemeData(
      colorScheme: ColorScheme.fromSeed(seedColor: Colors.teal),
    );
    await mountColor(tester, 'primary', theme: theme);
    expectColor(tester, theme.colorScheme.primary.toARGB32(), 'primary');
    final composite = {'value': '#f00', 'alpha': 0.5, 'over': '#00f'};
    await mountColor(tester, composite);
    expectColor(
      tester,
      Color.alphaBlend(
        const Color(0xffff0000).withValues(alpha: 0.5),
        const Color(0xff0000ff),
      ).toARGB32(),
      composite,
    );
    final translucent = {'value': 'rgba(255,255,255,0.9)', 'alpha': 0.25};
    await mountColor(tester, translucent);
    expectColor(tester, 0x40ffffff, translucent);
  });
}
