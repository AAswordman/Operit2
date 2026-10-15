import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/MessageHeaderMetadata.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/common/ChatAvatarImage.dart';
import 'package:operit2/ui/common/ChatMessageIdentity.dart';
import 'package:operit2/ui/features/settings/model/ProviderLogo.dart';

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
    testWidgets(
      '$style: model-only replies preserve avatar visibility and show the model name',
      (tester) async {
        final model = _message(
          role: '',
          extensions: const {
            'host.chat.identity': {
              'title': 'org/model:latest',
              'avatarUri': 'provider-logo:OPENAI',
            },
          },
        );
        final snapshot = ThemePreferenceSnapshot.fromJson({
          ..._snapshot(style, false, false).toJson(),
          'bubbleShowAvatar': true,
        });
        await _pumpMessage(tester, model, snapshot);
        await tester.pumpAndSettle();
        expect(find.textContaining(model.modelName), findsOneWidget);
        if (style == 'cursor') {
          expect(find.byType(ProviderIdentityLogo), findsNothing);
          expect(find.byType(ChatAvatarImage), findsNothing);
        } else {
          final logo = tester.widget<ProviderIdentityLogo>(
            find.byType(ProviderIdentityLogo),
          );
          expect(logo.providerTypeId, 'OPENAI');
        }
        expect(
          readChatMessageIdentity(model)!.avatarUri,
          'provider-logo:OPENAI',
        );
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );

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

/// Creates preferences for one production renderer and metadata combination.
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

/// Creates an explicit message identity independently of the active provider preferences.
ChatMessage _message({
  String sender = 'ai',
  String model = 'org/model:latest',
  String provider = '我的代理: OpenAI',
  String role = 'Operit',
  Map<String, Object?> extensions = const {},
}) => ChatMessage(
  pluginExtensions: extensions,
  sender: sender,
  parts: const [],
  timestamp: 1,
  roleName: role,
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

/// Mounts the production message renderer under a real theme and localized app.
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
