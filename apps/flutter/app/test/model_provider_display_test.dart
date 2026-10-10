import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/MessageHeaderMetadata.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises model/provider switches in production message renderers, not samples.
void main() {
  for (final style in ['cursor', 'bubble', 'bubble-wide']) {
    for (final showModel in [false, true]) {
      for (final showProvider in [false, true]) {
        testWidgets('$style: model=$showModel, provider=$showProvider', (
          tester,
        ) async {
          final snapshot = _snapshot(style, showModel, showProvider);
          final message = _message();
          await _pumpMessage(tester, message, snapshot);
          await tester.pumpAndSettle();

          final expected = [
            if (showModel) message.modelName,
            if (showProvider) message.provider,
          ].join(' · ');
          expect(formatModelProviderLabel(message, snapshot), expected);
          if (expected.isNotEmpty) {
            expect(find.textContaining(expected), findsOneWidget);
          }
          expect(
            find.textContaining(message.modelName),
            showModel ? findsOneWidget : findsNothing,
          );
          expect(
            find.textContaining(message.provider),
            showProvider ? findsOneWidget : findsNothing,
          );
          final tooltip = formatMessageMetadataTooltip(message, snapshot);
          expect(tooltip.contains(message.modelName), showModel);
          expect(tooltip.contains(message.provider), showProvider);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        });
      }
    }

    testWidgets('$style: live switches update existing messages', (
      tester,
    ) async {
      await _pumpMessage(tester, _message(), _snapshot(style, false, false));
      await tester.pumpAndSettle();
      final controller = OperitTheme.of(
        tester.element(find.byType(Scaffold).last),
      );
      for (final switches in [
        (true, false),
        (false, true),
        (true, true),
        (false, false),
      ]) {
        controller.setInitialThemePreferenceSnapshot(
          _snapshot(style, switches.$1, switches.$2),
        );
        controller.previewThemeSettings();
        await tester.pumpAndSettle();
        expect(
          find.textContaining('org/model:latest'),
          switches.$1 ? findsOneWidget : findsNothing,
        );
        expect(
          find.textContaining('我的代理: OpenAI'),
          switches.$2 ? findsOneWidget : findsNothing,
        );
        expect(tester.takeException(), isNull);
      }
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  test('missing historical metadata does not invent names or separators', () {
    final snapshot = _snapshot('cursor', true, true);
    expect(
      formatModelProviderLabel(_message(model: '', provider: ''), snapshot),
      '',
    );
    expect(
      formatModelProviderLabel(_message(provider: ''), snapshot),
      'org/model:latest',
    );
    expect(
      formatModelProviderLabel(_message(model: ''), snapshot),
      '我的代理: OpenAI',
    );
  });

  for (final style in ['cursor', 'bubble', 'bubble-wide']) {
    testWidgets('$style: user messages do not show assistant metadata', (
      tester,
    ) async {
      await _pumpMessage(
        tester,
        _message(sender: 'user'),
        _snapshot(style, true, true),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('org/model:latest'), findsNothing);
      expect(find.textContaining('我的代理: OpenAI'), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }
}

ThemePreferenceSnapshot _snapshot(String style, bool model, bool provider) {
  return ThemePreferenceSnapshot.fromJson({
    ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    'chatStyle': style == 'cursor' ? 'cursor' : 'bubble',
    'bubbleWideLayoutEnabled': style == 'bubble-wide',
    'showRoleName': false,
    'showModelName': model,
    'showModelProvider': provider,
    'showMessageTokenStats': false,
    'showMessageTimingStats': false,
    'showMessageTimestamp': false,
  });
}

ChatMessage _message({
  String sender = 'ai',
  String model = 'org/model:latest',
  String provider = '我的代理: OpenAI',
}) => ChatMessage(
  pluginExtensions: const {},
  sender: sender,
  parts: const [],
  timestamp: 1,
  roleName: 'Operit',
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: provider,
  modelName: model,
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  sentAt: 0,
  waitDurationMs: 0,
  outputDurationMs: 0,
  completedAt: 0,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);

Future<void> _pumpMessage(
  WidgetTester tester,
  ChatMessage message,
  ThemePreferenceSnapshot snapshot,
) => tester.pumpWidget(
  OperitTheme(
    initialThemePreferenceSnapshot: snapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: 280,
            child: Builder(
              builder: (context) => buildThemedChatMessage(
                snapshot: OperitTheme.of(context).themePreferenceSnapshot,
                colorScheme: Theme.of(context).colorScheme,
                message: message,
                enableDialogs: false,
                splitMarkdownContent: (_) async => const [],
              ),
            ),
          ),
        ),
      ),
    ),
  ),
);
