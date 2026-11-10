import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/semantic_markdown_fixture.dart';

/// Verifies prepared font propagation through real application and message widgets.
void main() {
  testWidgets('global font reaches text outside Material surfaces', (
    tester,
  ) async {
    final scheme = ColorScheme.fromSeed(seedColor: Colors.blue);
    await tester.pumpWidget(
      ThemeRenderRoot(
        themePreferenceSnapshot: ThemePreferenceDefaults.snapshot,
        lightColorScheme: scheme,
        darkColorScheme: scheme,
        fontFamily: 'GlobalCustomFont',
        fontFamilyFallback: const [],
        themeMode: ThemeMode.light,
        builder: (context, child) => child!,
        home: const Center(child: Text('Global text')),
      ),
    );
    expect(
      tester.widget<RichText>(find.byType(RichText)).text.style!.fontFamily,
      'GlobalCustomFont',
    );
  });

  for (final chatStyle in ['bubble', 'cursor']) {
    testWidgets(
      'prepared global font reaches $chatStyle messages and updates',
      (tester) async {
        final visual = (await tester.runAsync(
          () => _visual(chatStyle: chatStyle),
        ))!;
        addTearDown(visual.dispose);
        final settings = ThemePreferenceSnapshot.fromJson({
          ...ThemePreferenceDefaults.snapshot.toJson(),
          'userChatStyle': chatStyle,
          'aiChatStyle': chatStyle,
          'bubbleShowAvatar': false,
        });
        await _mountMessages(tester, settings, visual);
        expect(visual.snapshot(settings).bubbleUserUseCustomFont, isFalse);
        expect(visual.snapshot(settings).bubbleAiUseCustomFont, isFalse);
        expect(visual.snapshot(settings).userChatStyle, chatStyle);
        expect(visual.snapshot(settings).aiChatStyle, chatStyle);
        _expectMessageFonts(tester, visual.resources.sans.family);
        final device = ThemePreferenceSnapshot.fromJson({
          ...visual.snapshot(settings).toJson(),
          'fontType': 'device',
          'systemFontName': 'Installed device font',
        });
        for (final isUser in [true, false]) {
          expect(
            operitMessageFontFamily(device, isUser: isUser),
            visual.resources.sans.family,
          );
          expect(
            operitMessageFontFamilyFallback(device, isUser: isUser),
            isEmpty,
          );
        }

        final changed = (await tester.runAsync(
          () => _visual(global: kCascadiaMonoFontId, chatStyle: chatStyle),
        ))!;
        addTearDown(changed.dispose);
        await _mountMessages(tester, settings, changed);
        _expectMessageFonts(tester, changed.resources.sans.family);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'explicit user bubble font reaches selectable text and measurement',
    (tester) async {
      final visual = (await tester.runAsync(
        () => _visual(user: kCascadiaMonoFontId),
      ))!;
      addTearDown(visual.dispose);
      final settings = ThemePreferenceSnapshot.fromJson({
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'userChatStyle': 'bubble',
        'aiChatStyle': 'bubble',
        'bubbleShowAvatar': false,
      });
      await _mountMessages(tester, settings, visual);
      final user = tester.widget<EditableText>(find.byType(EditableText));
      expect(user.style.fontFamily, visual.resources.user.family);
      final context = tester.element(find.byType(SelectableText));
      expect(
        Theme.of(context).textTheme.bodyMedium!.fontFamily,
        user.style.fontFamily,
      );
      expect(tester.takeException(), isNull);
    },
  );
}

/// Registers real bundled font bytes using the same registry as theme previews.
Future<PreparedThemeVisual> _visual({
  String global = kNotoSansScFontId,
  String? user,
  String chatStyle = 'bubble',
}) async {
  final json =
      jsonDecode(
            await rootBundle.loadString(
              'packages/operit_ui/assets/fixtures/theme-v1.json',
            ),
          )
          as Map<String, dynamic>;
  json['typography'] = {'global': global, 'user': user, 'ai': null};
  (json['chat'] as Map<String, dynamic>)
    ..['style'] = chatStyle
    ..['showAvatar'] = false;
  return PreparedThemeVisual.prepare(
    spec: ThemeVisualSpecV1.fromCheckedContract(json),
    assets: const {},
    assetSha256: const {},
    builtinFonts: await BuiltinThemeFonts.load(bundle: rootBundle),
    fontRegistry: ThemeFontRegistry(),
  );
}

/// Renders production message components under the prepared theme boundary.
Future<void> _mountMessages(
  WidgetTester tester,
  ThemePreferenceSnapshot settings,
  PreparedThemeVisual visual,
) async {
  await tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: OperitThemePreview(
        snapshot: settings,
        visual: visual,
        child: Scaffold(
          body: Builder(
            builder: (context) {
              final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
              return SingleChildScrollView(
                child: Column(
                  children: [
                    const Text('Preview global text'),
                    for (final isUser in [true, false])
                      buildThemedChatMessage(
                        snapshot: snapshot,
                        colorScheme: Theme.of(context).colorScheme,
                        message: _message(isUser),
                        splitMarkdownContent: splitSemanticMarkdownFixture,
                        enableDialogs: false,
                      ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Checks final rendered text styles rather than the parent theme alone.
void _expectMessageFonts(WidgetTester tester, String family) {
  final user = tester.widget<EditableText>(find.byType(EditableText));
  expect(user.style.fontFamily, family);
  final texts = tester.widgetList<RichText>(find.byType(RichText));
  final heading = texts.singleWhere((text) => text.text.toPlainText() == '标题');
  expect(heading.text.style!.fontFamily, family);
  final bold = texts
      .expand((text) => _leaves(text.text))
      .singleWhere((span) => span.text == '粗体');
  expect(bold.style!.fontFamily, family);
  final preview = texts.singleWhere(
    (text) => text.text.toPlainText() == 'Preview global text',
  );
  expect(preview.text.style!.fontFamily, family);
}

/// Enumerates the final inline styles painted by the Markdown text renderer.
Iterable<TextSpan> _leaves(InlineSpan span) sync* {
  if (span is! TextSpan) return;
  if (span.text != null) yield span;
  for (final child in span.children ?? const <InlineSpan>[]) {
    yield* _leaves(child);
  }
}

/// Supplies completed messages with Core semantic fixtures and no native runtime.
ChatUiMessage _message(bool isUser) => ChatUiMessage(
  pluginExtensions: const {},
  sender: isUser ? 'user' : 'ai',
  parts: [
    core.MessagePart(
      partId: isUser ? 'user' : 'ai',
      sequence: 0,
      kind: core.MessagePartKind.markdown,
      content: isUser ? 'User 自定义字体' : semanticMarkdownBlocksSource,
      toolCallId: null,
      toolName: null,
      attributes: const {},
    ),
  ],
  timestamp: isUser ? 1 : 2,
  roleName: '',
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: '',
  modelName: '',
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  sentAt: 0,
  outputDurationMs: 0,
  waitDurationMs: 0,
  completedAt: 0,
  displayMode: core.ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);
