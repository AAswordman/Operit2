import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

/// Verifies the Core chat input bridge decision decodes to canonical actions.
void main() {
  /// Builds the raw JSON decision exactly as Core serializes it.
  Map<String, Object?> wire(String action, {bool clearInput = false}) {
    return <String, Object?>{
      'action': action,
      'text': null,
      'message': null,
      'clearInput': clearInput,
      'timedOut': false,
      'metadata': <String, Object?>{},
    };
  }

  test('keeps the canonical capitalized consume decision intact', () {
    final decision = ChatInputSubmitDecision.fromJson(
      wire('Consume', clearInput: true),
    );
    expect(decision.action, chatInputActionConsume);
    expect(decision.clearInput, isTrue);
  });

  test('accepts every canonical Core action', () {
    expect(
      ChatInputSubmitDecision.fromJson(wire('Allow')).action,
      chatInputActionAllow,
    );
    expect(
      ChatInputSubmitDecision.fromJson(wire('Block')).action,
      chatInputActionBlock,
    );
    expect(
      ChatInputSubmitDecision.fromJson(wire('Consume')).action,
      chatInputActionConsume,
    );
    expect(
      ChatInputSubmitDecision.fromJson(wire('Replace')).action,
      chatInputActionReplace,
    );
  });

  test('rejects actions outside the canonical Core contract', () {
    for (final action in ['consume', 'Mystery', '']) {
      expect(
        () => ChatInputSubmitDecision.fromJson(wire(action)),
        throwsFormatException,
      );
    }
  });
}
