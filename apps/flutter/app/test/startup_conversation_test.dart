import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/main/screens/OperitMainScreen.dart';

void main() {
  test('an empty identity creates the default conversation', () {
    expect(
      resolveStartupConversation(historyIds: const <String>[], currentChatId: null),
      StartupConversationResolution.create,
    );
    expect(
      resolveStartupConversation(
        historyIds: const <String>[],
        currentChatId: '   ',
      ),
      StartupConversationResolution.create,
    );
  });

  test('a real selection is left alone', () {
    expect(
      resolveStartupConversation(
        historyIds: const <String>['chat-a', 'chat-b'],
        currentChatId: 'chat-b',
      ),
      StartupConversationResolution.keep,
    );
  });

  test('a missing selection opens an existing conversation', () {
    expect(
      resolveStartupConversation(
        historyIds: const <String>['chat-a'],
        currentChatId: null,
      ),
      StartupConversationResolution.openExisting,
    );
    expect(
      resolveStartupConversation(
        historyIds: const <String>['chat-a'],
        currentChatId: 'deleted',
      ),
      StartupConversationResolution.openExisting,
    );
  });
}
