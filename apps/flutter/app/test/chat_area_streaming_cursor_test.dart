import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/features/chat/components/ChatArea.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/CursorStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets(
    'rebuilds Markdown nodes when a routed stream starts a snapshot',
    (tester) async {
      final streamController = StreamController<MarkdownStreamEvent>();
      final rendererState = StreamMarkdownRendererState();
      addTearDown(() async {
        await streamController.close();
      });

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StreamMarkdownRenderer(
              content: '',
              isStreaming: true,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              contentStream: streamController.stream,
              state: rendererState,
            ),
          ),
        ),
      );

      streamController
        ..add(_markdownBlockStart())
        ..add(_markdownBlockChunk('old segment'));
      await tester.pump(const Duration(milliseconds: 250));

      streamController
        ..add(_markdownReset())
        ..add(_markdownBlockStart(blockId: 7))
        ..add(_markdownInlineStart(blockId: 7, inlineId: 1))
        ..add(
          _markdownInlineChunk(blockId: 7, inlineId: 1, value: 'new segment'),
        );
      await tester.pump(const Duration(milliseconds: 250));

      expect(tester.takeException(), isNull);
      expect(rendererState.nodes, hasLength(1));
      expect(rendererState.nodes.single.children, hasLength(1));
      expect(
        rendererState.nodes.single.children.single.content.toString(),
        'new segment',
      );
    },
  );

  testWidgets('does not add a standalone cursor after the AI stream attaches', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    addTearDown(() async {
      await streamController.close();
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[],
          stream: streamController.stream,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );

    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('hello'));
    await tester.pump(const Duration(milliseconds: 250));

    expect(find.byType(StreamingCursor), findsOneWidget);
  });

  testWidgets('does not treat a tool-only AI message as empty', (tester) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    addTearDown(() {
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'tool-1',
              sequence: 0,
              kind: MessagePartKind.toolCall,
              content: '',
              toolCallId: 'call-1',
              toolName: 'read_file',
              attributes: <String, String>{'path': 'README.md'},
            ),
          ],
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );

    expect(find.byType(StreamingCursor), findsNothing);
  });

  testWidgets('does not rebuild a completed AI row when loading changes', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    final message = _aiMessage(
      parts: const <MessagePart>[
        MessagePart(
          partId: 'part-0',
          sequence: 0,
          kind: MessagePartKind.markdown,
          content: 'previous answer',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{},
        ),
      ],
      completedAt: 1,
    );
    addTearDown(() {
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        message: message,
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    final previousMessageWidget = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage),
    );

    await tester.pumpWidget(
      _chatArea(
        message: message,
        isLoading: true,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    final currentMessageWidget = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage),
    );

    expect(currentMessageWidget.isStreaming, isFalse);
    expect(identical(currentMessageWidget, previousMessageWidget), isTrue);

    await tester.pumpWidget(
      _chatArea(
        message: message,
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    final settledMessageWidget = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage),
    );

    expect(identical(settledMessageWidget, previousMessageWidget), isTrue);
  });

  testWidgets('retains live output until final message parts are ready', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    addTearDown(() async {
      await streamController.close();
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[],
          stream: streamController.stream,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('final answer'))
      ..add(_markdownCompleted());
    await tester.pump(const Duration(milliseconds: 250));
    final liveRenderer = tester.element(
      find.byType(StreamingStructuredMessageRenderer),
    );

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'part-0',
              sequence: 0,
              kind: MessagePartKind.toolCall,
              content: '',
              toolCallId: 'call-1',
              toolName: 'read_file',
              attributes: <String, String>{'path': 'README.md'},
            ),
          ],
          completedAt: 1,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );

    expect(
      identical(
        tester.element(find.byType(StreamingStructuredMessageRenderer)),
        liveRenderer,
      ),
      isTrue,
    );
    expect(find.byKey(const ValueKey<String>('live-markdown')), findsOneWidget);

    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsNothing);
    expect(
      find.byKey(const ValueKey<String>('structured-parts')),
      findsOneWidget,
    );
    expect(find.byType(CompactToolDisplay), findsOneWidget);
  });

  testWidgets('hides the scroll navigator after the scroll view detaches', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    addTearDown(() {
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'part-0',
              sequence: 0,
              kind: MessagePartKind.markdown,
              content: 'response',
              toolCallId: null,
              toolName: null,
              attributes: <String, String>{},
            ),
          ],
          completedAt: 1,
        ),
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );

    final scrollableContext = tester.element(find.byType(Scrollable).first);
    final metrics = FixedScrollMetrics(
      minScrollExtent: 0,
      maxScrollExtent: 100,
      pixels: 20,
      viewportDimension: 100,
      axisDirection: AxisDirection.down,
      devicePixelRatio: 1,
    );
    UserScrollNotification(
      metrics: metrics,
      context: scrollableContext,
      direction: ScrollDirection.forward,
    ).dispatch(scrollableContext);
    UserScrollNotification(
      metrics: metrics,
      context: scrollableContext,
      direction: ScrollDirection.idle,
    ).dispatch(scrollableContext);

    await tester.pumpWidget(
      _chatArea(
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    expect(scrollController.hasClients, isFalse);

    await tester.pump(const Duration(milliseconds: 1201));

    expect(tester.takeException(), isNull);
  });
}

/// Builds a minimal themed transcript around one active AI message.
Widget _chatArea({
  ChatUiMessage? message,
  required ScrollController scrollController,
  required ValueNotifier<bool> autoScrollToBottom,
  bool isLoading = true,
}) {
  return OperitTheme(
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: Scaffold(
      body: ChatArea(
        messages: message == null
            ? const <ChatUiMessage>[]
            : <ChatUiMessage>[message],
        isLoading: isLoading,
        errorMessage: null,
        scrollController: scrollController,
        currentChatId: 'chat',
        currentCharacterCardAvatarUri: null,
        autoScrollToBottomListenable: autoScrollToBottom,
        hasOlderDisplayHistory: false,
        hasNewerDisplayHistory: false,
        isLoadingDisplayWindow: false,
        loadLocatorEntries: (chatId, query) async => const [],
        onAutoScrollToBottomChanged: (_) {},
        onLoadOlderDisplayWindow: () async {},
        onLoadNewerDisplayWindow: () async {},
        onShowLatestDisplayWindow: () async {},
        onToggleFavoriteMessage: (timestamp, isFavorite) async {},
        onDeleteMessage: (index) async {},
        onDeleteMessagesFrom: (index) async => true,
        onDeleteMessageVariant: (timestamp, variantIndex) async {},
        onRollbackToMessage: (_) {},
        onSelectMessageToEdit: (index, message) {},
        onRegenerateMessage: (index) async {},
        onInsertSummary: (_) {},
        onCreateBranch: (timestamp) async {},
        onReplyToMessage: (_) {},
        onPlayVoice: (message) async {},
        onToggleMultiSelectMode: (_) {},
        onToggleMessageSelection: (_) {},
        onRefreshRequested: () async {},
      ),
    ),
  );
}

/// Creates the active AI message used to verify transcript cursor ownership.
ChatUiMessage _aiMessage({
  required List<MessagePart> parts,
  Stream<MarkdownStreamEvent>? stream,
  int completedAt = 0,
}) {
  return ChatMessage(
    sender: 'ai',
    parts: parts,
    timestamp: 1,
    roleName: 'assistant',
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
    completedAt: completedAt,
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: stream,
  );
}

/// Creates a plain-text Markdown block start event for the live renderer.
MarkdownStreamEvent _markdownBlockStart({int blockId = 1}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockStart',
    value: null,
    id: null,
    blockId: blockId,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}

/// Creates the boundary event for one complete Markdown stream snapshot.
MarkdownStreamEvent _markdownReset() {
  return const MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'reset',
    value: null,
    id: null,
    blockId: null,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}

/// Creates a plain-text Markdown inline start event for the live renderer.
MarkdownStreamEvent _markdownInlineStart({
  required int blockId,
  required int inlineId,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownInlineStart',
    value: null,
    id: null,
    blockId: blockId,
    inlineId: inlineId,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}

/// Creates a plain-text Markdown inline content event for the live renderer.
MarkdownStreamEvent _markdownInlineChunk({
  required int blockId,
  required int inlineId,
  required String value,
}) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownInlineChunk',
    value: value,
    id: null,
    blockId: blockId,
    inlineId: inlineId,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}

/// Creates a plain-text Markdown block content event for the live renderer.
MarkdownStreamEvent _markdownBlockChunk(String value) {
  return MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'markdownBlockChunk',
    value: value,
    id: null,
    blockId: 1,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}

/// Creates the root completion event that closes one live Markdown response.
MarkdownStreamEvent _markdownCompleted() {
  return const MarkdownStreamEvent(
    chatId: 'chat',
    eventType: 'completed',
    value: null,
    id: null,
    blockId: null,
    inlineId: null,
    parentBlockId: null,
    nodeType: null,
    headerLevel: null,
  );
}
