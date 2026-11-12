import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/ThemedChatMessage.dart';
import 'package:operit2/ui/features/settings/appearance/ChatAppearancePreview.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies user names through the production transcript and appearance preview.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() async {
    const channel = MethodChannel('operit/runtime');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    final config = LocalRuntimeStorageConfig.platformDefault();
    messenger.setMockMethodCallHandler(channel, (call) async {
      switch (call.method) {
        case 'runtimeBootstrapRead':
          return jsonEncode({'ok': true, 'value': jsonEncode(config.toJson())});
        case 'runtimeBootstrapWrite':
          return jsonEncode({'ok': true});
        default:
          throw StateError('Unexpected host method: ${call.method}');
      }
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    await ClientLogger.initialize();
    await RuntimeBootstrapManager.instance.initialize();
    await _rename('  小明  ');
  });

  for (final style in ['cursor', 'bubble', 'bubble-wide']) {
    testWidgets(
      '$style: transcript name follows the switch and profile edits',
      (tester) async {
        await _pump(tester, style, message: _message());
        await tester.pumpAndSettle();
        expect(find.text('小明'), findsOneWidget);
        expect(find.text('AI recipient'), findsNothing);

        await tester.runAsync(() => _rename('小红'));
        await tester.pumpAndSettle();
        expect(find.text('小明'), findsNothing);
        expect(find.text('小红'), findsOneWidget);

        _setShowName(tester, false);
        await tester.pumpAndSettle();
        expect(find.text('小红'), findsNothing);

        _setShowName(tester, true);
        await tester.pumpAndSettle();
        expect(find.text('小红'), findsOneWidget);

        await tester.runAsync(() => _rename(''));
        await tester.pumpAndSettle();
        expect(find.text('小红'), findsNothing);
        expect(find.text('AI recipient'), findsNothing);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );

    testWidgets('$style: appearance preview uses the live profile name', (
      tester,
    ) async {
      await _pump(tester, style);
      await tester.pumpAndSettle();
      expect(find.text('小明'), findsOneWidget);
      await tester.runAsync(() => _rename('小红'));
      await tester.pumpAndSettle();
      expect(find.text('小明'), findsNothing);
      expect(find.text('小红'), findsOneWidget);
      _setShowName(tester, false);
      await tester.pumpAndSettle();
      expect(find.text('小红'), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });

    testWidgets('$style: proxy messages retain their explicit author', (
      tester,
    ) async {
      await _pump(
        tester,
        style,
        message: _message(text: '<proxy_sender name="代发者"/>你好'),
        showName: false,
      );
      await tester.pumpAndSettle();
      expect(find.text('代发者'), findsOneWidget);
      expect(find.text('小明'), findsNothing);
      expect(find.text('AI recipient'), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });

    testWidgets('$style: hidden placeholders do not reveal the user name', (
      tester,
    ) async {
      await _pump(
        tester,
        style,
        message: _message(
          displayMode: ChatMessageDisplayMode.hiddenPlaceholder,
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('小明'), findsNothing);
      expect(find.text('AI recipient'), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }
}

/// Updates the real observable profile without changing the mounted message.
Future<void> _rename(String name) {
  final manager = RuntimeBootstrapManager.instance;
  return manager.renameIdentity(manager.activeIdentity.id, name);
}

/// Changes the theme snapshot while preserving the mounted message state.
void _setShowName(WidgetTester tester, bool value) {
  final controller = OperitTheme.of(tester.element(find.byType(Scaffold)));
  controller.setInitialThemePreferenceSnapshot(
    ThemePreferenceSnapshot.fromJson({
      ...controller.themePreferenceSnapshot.toJson(),
      'showUserName': value,
    }),
  );
  controller.previewThemeSettings();
}

/// Includes recipient metadata to ensure it never supplies the user's name.
ChatMessage _message({
  String text = '你好',
  ChatMessageDisplayMode displayMode = ChatMessageDisplayMode.normal,
}) => ChatMessage(
  pluginExtensions: const {
    'host.chat.identity': {'title': 'AI recipient', 'avatarUri': null},
  },
  sender: 'user',
  parts: [
    MessagePart(
      partId: 'user-text',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: text,
      toolCallId: null,
      toolName: null,
      attributes: const {},
    ),
  ],
  timestamp: 1,
  roleName: 'AI recipient',
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
  displayMode: displayMode,
  isFavorite: false,
  contentStream: null,
);

/// Mounts production widgets in a narrow layout with avatars disabled.
Future<void> _pump(
  WidgetTester tester,
  String style, {
  ChatMessage? message,
  bool showName = true,
}) => tester.pumpWidget(
  OperitTheme(
    initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      'userChatStyle': style == 'cursor' ? 'cursor' : 'bubble',
      'aiChatStyle': style == 'cursor' ? 'cursor' : 'bubble',
      'bubbleWideLayoutEnabled': style == 'bubble-wide',
      'bubbleShowAvatar': false,
      'showUserName': showName,
    }),
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Builder(
        builder: (context) => Scaffold(
          body: SingleChildScrollView(
            child: Center(
              child: SizedBox(
                width: 280,
                child: message == null
                    ? const ChatAppearancePreview(showInputPreview: false)
                    : buildThemedChatMessage(
                        snapshot: OperitTheme.of(
                          context,
                        ).themePreferenceSnapshot,
                        colorScheme: Theme.of(context).colorScheme,
                        message: message,
                        enableDialogs: false,
                      ),
              ),
            ),
          ),
        ),
      ),
    ),
  ),
);
