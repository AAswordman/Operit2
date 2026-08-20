import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
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
}

/// Creates a complete UI message fixture with the supplied semantic parts.
ChatUiMessage _message({
  required String sender,
  required List<MessagePart> parts,
}) {
  return ChatMessage(
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
