import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/chat/components/MessageContextMenu.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets('message context menu uses Japanese labels', (tester) async {
    tester.binding.platformDispatcher.localesTestValue = const <Locale>[
      Locale('ja'),
    ];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);

    await tester.pumpWidget(
      OperitTheme(
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: MessageContextMenu(
            index: 0,
            message: _userMessage(),
            onToggleFavoriteMessage: (_, _) async {},
            child: const Text('target message'),
          ),
        ),
      ),
    );

    await tester.longPress(find.text('target message'));
    await tester.pumpAndSettle();

    expect(find.text('メッセージをコピー'), findsOneWidget);
    expect(find.text('編集して再送信'), findsOneWidget);
    expect(find.text('复制消息'), findsNothing);
    expect(find.text('编辑并重发'), findsNothing);
  });
}

ChatUiMessage _userMessage() {
  return ChatUiMessage(
    sender: 'user',
    parts: const <MessagePart>[],
    timestamp: 1,
    roleName: 'user',
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
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
    completedAt: 1,
  );
}
