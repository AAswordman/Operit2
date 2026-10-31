import 'support/markdown_event_fixture.dart';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/ChatResponsePresentation.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleAiMessageComposable.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises generic native response delivery through the actual production message renderer.
void main() {
  for (final style in ['cursor', 'bubble']) {
    testWidgets(
      '$style renders independent bubbles without duplicating canonical output',
      (tester) async {
        await pumpMessage(tester, message(['第一句。', '第二句。']), style);
        await tester.pumpAndSettle();
        expect(find.text('第一句。', findRichText: true), findsOneWidget);
        expect(find.text('第二句。', findRichText: true), findsOneWidget);
        expect(find.textContaining('canonical'), findsNothing);
        if (style == 'bubble') {
          expect(find.byType(BubbleAiMessageComposable), findsNWidgets(2));
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
  for (final style in ['cursor', 'bubble']) {
    for (final showThinking in [false, true]) {
      testWidgets(
        '$style only creates a thinking bubble when thinking is visible ($showThinking)',
        (tester) async {
          final thinking = part(
            'private reasoning',
            kind: MessagePartKind.thinking,
          );
          final original = message(['第一句。', '第二句。'], structural: [thinking]);
          await pumpMessage(
            tester,
            original,
            style,
            showThinking: showThinking,
          );
          await tester.pumpAndSettle();
          expect(
            find.byKey(const ValueKey('response-structure')),
            showThinking ? findsOneWidget : findsNothing,
          );
          expect(find.text('第一句。', findRichText: true), findsOneWidget);
          expect(find.text('第二句。', findRichText: true), findsOneWidget);
          if (style == 'bubble') {
            expect(
              find.byType(BubbleAiMessageComposable),
              findsNWidgets(showThinking ? 3 : 2),
            );
          }
          expect(original.parts.first, same(thinking));
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
  testWidgets(
    'hidden thinking does not create a bubble before the first sentence',
    (tester) async {
      await pumpMessage(
        tester,
        message(
          [],
          pending: true,
          structural: [
            part('private reasoning', kind: MessagePartKind.thinking),
          ],
        ),
        'bubble',
        showThinking: false,
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('response-structure')), findsNothing);
      expect(find.byType(BubbleAiMessageComposable), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
  testWidgets('hiding thinking preserves visible status content', (
    tester,
  ) async {
    await pumpMessage(
      tester,
      message(
        ['第一句。'],
        structural: [
          part('private reasoning', kind: MessagePartKind.thinking),
          part('visible status', kind: MessagePartKind.status),
        ],
      ),
      'bubble',
      showThinking: false,
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('response-structure')), findsOneWidget);
    expect(find.text('visible status', findRichText: true), findsOneWidget);
    expect(find.byType(BubbleAiMessageComposable), findsNWidgets(2));
    final structure = tester.widget<BubbleAiMessageComposable>(
      find.byType(BubbleAiMessageComposable).first,
    );
    expect(structure.message.parts.map((part) => part.kind), [
      MessagePartKind.status,
    ]);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets(
    'streamed delivery retains the first bubble and does not attach the original full-response stream',
    (tester) async {
      final source = StreamController<MarkdownStreamEvent>.broadcast();
      final thinking = part(
        'private reasoning',
        kind: MessagePartKind.thinking,
      );
      await pumpMessage(
        tester,
        message(
          ['第一句。'],
          pending: true,
          stream: source.stream,
          structural: [thinking],
        ),
        'bubble',
        showThinking: false,
      );
      await tester.pumpAndSettle();
      final first = tester.state(find.byType(BubbleAiMessageComposable).first);
      expect(source.hasListener, false);
      await pumpMessage(
        tester,
        message(['第一句。', '第二句。'], structural: [thinking]),
        'bubble',
        showThinking: false,
      );
      await tester.pumpAndSettle();
      expect(
        identical(
          first,
          tester.state(find.byType(BubbleAiMessageComposable).first),
        ),
        true,
      );
      expect(find.byType(BubbleAiMessageComposable), findsNWidgets(2));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      await source.close();
    },
  );
  test(
    'saved native bubbles preserve original response history and account metrics once',
    () {
      final original = message(['第一句。', '第二句。']);
      final presentation = readChatResponsePresentation(original)!;
      final first = responseBubbleMessage(
        original,
        presentation.bubbles.first,
        includeMetrics: false,
      );
      final last = responseBubbleMessage(
        original,
        presentation.bubbles.last,
        includeMetrics: true,
      );
      expect(
        original.parts.single.content,
        'canonical model response with <emotion>happy</emotion>',
      );
      expect(first.outputTokens, 0);
      expect(last.outputTokens, original.outputTokens);
      expect(first.timestamp, original.timestamp);
      expect(
        first.pluginExtensions.containsKey(responsePresentationExtension),
        false,
      );
    },
  );
}

/// Creates a semantic part for canonical or presented response content.
MessagePart part(
  String content, {
  MessagePartKind kind = MessagePartKind.markdown,
}) => MessagePart(
  partId: content,
  sequence: 0,
  kind: kind,
  content: content,
  toolCallId: null,
  toolName: null,
  attributes: const {},
);

/// Supplies real canonical content separately from already delivered presentation bubbles.
ChatMessage message(
  List<String> bubbles, {
  bool pending = false,
  List<MessagePart> structural = const [],
  Stream<MarkdownStreamEvent>? stream,
}) => ChatMessage(
  sender: 'ai',
  parts: [
    ...structural,
    part('canonical model response with <emotion>happy</emotion>'),
  ],
  timestamp: 10,
  roleName: 'Alice',
  pluginExtensions: {
    responsePresentationExtension: {
      'bubbles': bubbles.map((text) => [part(text).toJson()]).toList(),
      'pending': pending,
    },
  },
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: 'Provider',
  modelName: 'Model',
  inputTokens: 5,
  outputTokens: 10,
  cachedInputTokens: 0,
  sentAt: 0,
  outputDurationMs: 0,
  waitDurationMs: 0,
  completedAt: pending ? 0 : 100,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: stream,
);

/// Mounts the production renderer with actual theme and localization dependencies.
Future<void> pumpMessage(
  WidgetTester tester,
  ChatMessage message,
  String style, {
  bool showThinking = true,
}) async {
  final snapshot = ThemePreferenceSnapshot.fromJson({
    ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    'userChatStyle': style,
    'aiChatStyle': style,
    'showThinkingProcess': showThinking,
    'showRoleName': false,
    'showModelName': false,
    'showModelProvider': false,
    'showMessageTokenStats': false,
    'showMessageTimingStats': false,
    'showMessageTimestamp': false,
  });
  await tester.pumpWidget(
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
          body: Builder(
            builder: (context) => buildThemedChatMessage(
              snapshot: snapshot,
              colorScheme: Theme.of(context).colorScheme,
              message: message,
              enableDialogs: false,
              splitMarkdownContent: splitMarkdownEventFixture,
            ),
          ),
        ),
      ),
    ),
  );
}
