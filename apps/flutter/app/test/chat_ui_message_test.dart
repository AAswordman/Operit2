import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/chat/components/MessageEditorDialog.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

/// Verifies editable chat text preserves every semantic assistant part.
void main() {
  test('reconstructs complete assistant markup for memory editing', () {
    final message = _message(
      sender: 'ai',
      parts: const <MessagePart>[
        MessagePart(
          partId: 'status',
          sequence: 4,
          kind: MessagePartKind.status,
          content: 'careful',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{'z': 'last', 'type': 'warning'},
        ),
        MessagePart(
          partId: 'markdown',
          sequence: 0,
          kind: MessagePartKind.markdown,
          content: 'Answer',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{},
        ),
        MessagePart(
          partId: 'thinking',
          sequence: 1,
          kind: MessagePartKind.thinking,
          content: 'reasoning',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{},
        ),
        MessagePart(
          partId: 'tool-call',
          sequence: 2,
          kind: MessagePartKind.toolCall,
          content: '',
          toolCallId: 'call&1',
          toolName: 'read<file',
          attributes: <String, String>{'z': 'last', 'path': 'A & B'},
        ),
        MessagePart(
          partId: 'tool-result',
          sequence: 3,
          kind: MessagePartKind.toolResult,
          content: '<payload>',
          toolCallId: 'call&1',
          toolName: 'read<file',
          attributes: <String, String>{
            'status': 'success',
            'detail': 'A & "B"',
          },
        ),
      ],
    );

    expect(message.displayText, 'Answercareful');
    expect(
      message.editableText,
      'Answer'
      '<think>reasoning</think>'
      '<tool name="read&lt;file" call_id="call&amp;1">'
      '<param name="path">A & B</param>'
      '<param name="z">last</param>'
      '</tool>'
      '<tool_result name="read&lt;file" call_id="call&amp;1" '
      'detail="A &amp; &quot;B&quot;" status="success">'
      '<content><payload></content></tool_result>'
      '<status type="warning" z="last">careful</status>',
    );
  });

  test('keeps user editing text as visible markdown', () {
    final message = _message(
      sender: 'user',
      parts: const <MessagePart>[
        MessagePart(
          partId: 'markdown',
          sequence: 0,
          kind: MessagePartKind.markdown,
          content: 'User text',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{},
        ),
      ],
    );

    expect(message.editableText, 'User text');
  });

  test('keeps summary editing text in display order', () {
    final message = _message(
      sender: 'summary',
      parts: const <MessagePart>[
        MessagePart(
          partId: 'second',
          sequence: 1,
          kind: MessagePartKind.markdown,
          content: 'Second paragraph.',
          toolCallId: null,
          toolName: null,
          attributes: {},
        ),
        MessagePart(
          partId: 'first',
          sequence: 0,
          kind: MessagePartKind.markdown,
          content: 'Summary **memory**.\n\n',
          toolCallId: null,
          toolName: null,
          attributes: {},
        ),
      ],
    );

    expect(message.editableText, 'Summary **memory**.\n\nSecond paragraph.');
  });

  test('allows editing an empty summary', () {
    expect(_message(sender: 'summary', parts: const []).editableText, isEmpty);
  });

  testWidgets('opens and saves the summary memory editor', (tester) async {
    final message = _message(
      sender: 'summary',
      parts: const <MessagePart>[
        MessagePart(
          partId: 'summary',
          sequence: 0,
          kind: MessagePartKind.markdown,
          content: 'Original summary.',
          toolCallId: null,
          toolName: null,
          attributes: {},
        ),
      ],
    );
    String? saved;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (context) => MessageEditorDialog(
                  initialText: message.editableText,
                  showResendButton: message.sender == 'user',
                  onSave: (content) async {
                    saved = content;
                  },
                  onResend: (_) async {
                    fail('Summary cannot be resent');
                  },
                ),
              ),
              child: const Text('Edit summary'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Edit summary'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('Original summary.'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'Updated **summary**.');
    await tester.tap(find.text('更新记忆'));
    await tester.pumpAndSettle();
    expect(saved, 'Updated **summary**.');
    expect(tester.takeException(), isNull);
  });
}

/// Creates a complete UI message fixture with the supplied semantic parts.
ChatUiMessage _message({
  required String sender,
  required List<MessagePart> parts,
}) {
  return ChatMessage(
    pluginExtensions: const {},
    sender: sender,
    parts: parts,
    timestamp: 1,
    roleName: sender,
    selectedVariantIndex: 0,
    variantCount: 1,
    provider: 'test',
    modelName: 'test',
    inputTokens: 0,
    outputTokens: 0,
    cachedInputTokens: 0,
    sentAt: 0,
    outputDurationMs: 0,
    waitDurationMs: 0,
    completedAt: 1,
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
}
