import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/MessageHeaderMetadata.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Keeps complete response metadata visible on narrow screens and at large fonts.
void main() {
  test('prepared text does not invent token, timing or speed measurements', () {
    final message = metadataMessage();
    final snapshot = ThemePreferenceSnapshot.fromJson({
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      'showMessageTokenStats': true,
      'showMessageTimingStats': true,
      'showMessageTimestamp': false,
    });
    expect(formatCompactTokenStats(message), isEmpty);
    expect(formatCompactTimingStats(message), isEmpty);
    expect(formatTokenSpeed(message), isEmpty);
    expect(formatMessageStatsText(message, snapshot), isEmpty);
    expect(formatMessageMetadataTooltip(message, snapshot), isEmpty);
  });
  test(
    'scheduled replies display delivery time while retaining preparation measurements',
    () {
      final due = DateTime(2026, 11, 20, 12).millisecondsSinceEpoch;
      final message = metadataMessage(timestamp: due, scheduled: true);
      expect(
        formatSmartMessageTimestamp(message, now: DateTime(2026, 11, 20, 13)),
        '12:00',
      );
      final snapshot = ThemePreferenceSnapshot.fromJson({
        ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
        'showMessageTimestamp': true,
      });
      expect(
        formatMessageMetadataTooltip(message, snapshot),
        contains('2026-11-20 12:00:00'),
      );
    },
  );
  for (final style in ['cursor', 'bubble', 'bubble-wide']) {
    for (final width in [280.0, 380.0]) {
      for (final textScale in [1.0, 1.6]) {
        testWidgets('$style wraps metadata at $width, scale=$textScale', (
          tester,
        ) async {
          final snapshot = ThemePreferenceSnapshot.fromJson({
            ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
            'chatStyle': style == 'cursor' ? 'cursor' : 'bubble',
            'bubbleWideLayoutEnabled': style == 'bubble-wide',
            'showRoleName': true,
            'showModelName': true,
            'showModelProvider': true,
            'showMessageTokenStats': true,
            'showMessageTimingStats': true,
            'showMessageTimestamp': true,
          });
          final timestamp = DateTime(
            2026,
            10,
            7,
            14,
            20,
          ).millisecondsSinceEpoch;
          final message = ChatMessage(
            pluginExtensions: const {},
            sender: 'ai',
            parts: const [],
            timestamp: timestamp,
            roleName: '回复',
            selectedVariantIndex: 0,
            variantCount: 1,
            provider: 'OpenAI',
            modelName: 'gpt-6.1-sol',
            inputTokens: 29200,
            outputTokens: 251,
            cachedInputTokens: 14100,
            sentAt: timestamp,
            waitDurationMs: 8000,
            outputDurationMs: 10800,
            completedAt: timestamp + 18800,
            displayMode: ChatMessageDisplayMode.normal,
            isFavorite: false,
            contentStream: null,
          );
          await tester.pumpWidget(
            OperitTheme(
              initialThemePreferenceSnapshot: snapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Builder(
                builder: (context) => MaterialApp(
                  locale: const Locale('zh'),
                  localizationsDelegates:
                      AppLocalizations.localizationsDelegates,
                  supportedLocales: AppLocalizations.supportedLocales,
                  home: Scaffold(
                    body: SingleChildScrollView(
                      child: Center(
                        child: SizedBox(
                          width: width,
                          child: Builder(
                            builder: (context) => MediaQuery(
                              data: MediaQuery.of(context).copyWith(
                                textScaler: TextScaler.linear(textScale),
                              ),
                              child: buildThemedChatMessage(
                                snapshot: snapshot,
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
                ),
              ),
            ),
          );
          await tester.pumpAndSettle();

          final modelLabel = find.textContaining('gpt-6.1-sol · OpenAI');
          expect(modelLabel, findsOneWidget);
          final modelText = tester.widget<Text>(modelLabel);
          expect(modelText.maxLines, isNull);
          expect(modelText.overflow, isNot(TextOverflow.ellipsis));
          expect(modelText.softWrap, isTrue);
          expect(
            tester.renderObject<RenderParagraph>(modelLabel).didExceedMaxLines,
            isFalse,
          );

          final metadata = find.byWidgetPredicate(
            (widget) =>
                widget is Text && (widget.data?.contains('↑29.2k') ?? false),
          );
          expect(metadata, findsOneWidget);
          final text = tester.widget<Text>(metadata);
          final l10n = AppLocalizations.of(tester.element(metadata))!;
          expect(
            text.data,
            contains(formatMessageStatsText(message, snapshot, l10n: l10n)),
          );
          expect(text.data, contains('缓存 14.1k 48%'));
          expect(text.data, contains('23.2 t/s'));
          expect(text.data, contains('◷ 8.0s+10.8s'));
          expect(text.maxLines, isNull);
          expect(text.overflow, isNot(TextOverflow.ellipsis));
          expect(text.softWrap, isTrue);
          final paragraph = tester.renderObject<RenderParagraph>(metadata);
          expect(paragraph.didExceedMaxLines, isFalse);
          final boxes = paragraph.getBoxesForSelection(
            TextSelection(baseOffset: 0, extentOffset: text.data!.length),
          );
          expect(boxes.map((box) => box.top).toSet().length, greaterThan(1));
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        });
      }
    }
  }
}

/// Creates a completed text-only message with no provider measurements.
ChatMessage metadataMessage({int timestamp = 2000, bool scheduled = false}) {
  return ChatMessage(
    sender: 'ai',
    parts: const [],
    timestamp: timestamp,
    roleName: '',
    pluginExtensions: scheduled
        ? {
            'host.chat.scheduled': {'preparedAt': 1000},
          }
        : const {},
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
    completedAt: 1000,
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
}
