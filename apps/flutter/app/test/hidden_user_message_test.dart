import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/MessageContextMenu.dart';
import 'package:operit2/ui/features/chat/components/attachments/ChatMessageImageAttachments.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/components/style/common/HiddenUserMessagePlaceholder.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/CursorStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

const _source =
    'Automatic instruction\n'
    '<attachment id="secret" filename="private.txt" type="text/plain">'
    'Private context</attachment>';

/// Verifies hidden sources stay stored while transcript interactions stay explicit.
void main() {
  for (final style in ['cursor', 'bubble']) {
    for (final enableDialogs in [true, false]) {
      testWidgets('$style hides source and respects dialogs=$enableDialogs', (
        tester,
      ) async {
        final message = _message(_source, hidden: true);
        await tester.pumpWidget(
          MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(
              body: style == 'cursor'
                  ? CursorStyleChatMessage(
                      message: message,
                      enableDialogs: enableDialogs,
                    )
                  : BubbleStyleChatMessage(
                      message: message,
                      userMessageColor: Colors.blue,
                      aiMessageColor: Colors.white,
                      userTextColor: Colors.black,
                      aiTextColor: Colors.black,
                      systemMessageColor: Colors.grey,
                      systemTextColor: Colors.black,
                      enableDialogs: enableDialogs,
                    ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final placeholder = find.byType(HiddenUserMessagePlaceholder);
        expect(placeholder, findsOneWidget);
        expect(find.textContaining('Automatic instruction'), findsNothing);
        expect(find.byType(ChatMessageAttachments), findsNothing);
        expect(message.displayText, _source);

        await tester.tap(find.text('AUTO'));
        await tester.pumpAndSettle();
        expect(
          find.byType(AlertDialog),
          enableDialogs ? findsOneWidget : findsNothing,
        );
        expect(
          find.text(_source),
          enableDialogs ? findsOneWidget : findsNothing,
        );
        if (enableDialogs) {
          await tester.tap(find.text('Close'));
          await tester.pumpAndSettle();
          expect(find.text(_source), findsNothing);
        }
        expect(message.displayText, _source);
        expect(tester.takeException(), isNull);
      });
    }
  }

  test(
    'multi-message copy skips hidden user turns and keeps visible order',
    () {
      expect(
        buildMessagesCopyText([
          _message('Visible user'),
          _message(_source, hidden: true),
          _message('Visible answer', sender: 'ai'),
        ]),
        'Visible user\n\nVisible answer',
      );
      expect(buildMessagesCopyText([_message(_source, hidden: true)]), isEmpty);
      expect(buildMessagesCopyText([_message('Normal user')]), 'Normal user');
    },
  );
}

/// Creates a complete Core message without starting a runtime connection.
ChatMessage _message(
  String source, {
  String sender = 'user',
  bool hidden = false,
}) => ChatMessage(
  pluginExtensions: const {},
  sender: sender,
  parts: [
    MessagePart(
      partId: 'text',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: source,
      toolCallId: null,
      toolName: null,
      attributes: const {},
    ),
  ],
  timestamp: 1,
  roleName: sender,
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
  completedAt: 1,
  displayMode: hidden
      ? ChatMessageDisplayMode.hiddenPlaceholder
      : ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);
