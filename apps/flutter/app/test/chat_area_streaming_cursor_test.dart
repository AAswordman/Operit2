import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/features/chat/components/ChatArea.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/InputProcessingStatusLane.dart';
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
              splitMarkdownContent: _splitMarkdownContent,
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

  testWidgets('interpolates the cursor position across a stream line break', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    addTearDown(() async {
      await streamController.close();
    });

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 80,
            child: StreamMarkdownRenderer(
              content: '',
              isStreaming: true,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              contentStream: streamController.stream,
              splitMarkdownContent: _splitMarkdownContent,
            ),
          ),
        ),
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('ab\ncd'));
    await tester.pump(const Duration(milliseconds: 200));

    final cursorFinder = find.byType(StreamingCursor);
    final initialPosition = tester.getTopLeft(cursorFinder);
    await tester.pump(const Duration(milliseconds: 100));
    final intermediatePosition = tester.getTopLeft(cursorFinder);
    await tester.pump(const Duration(milliseconds: 100));
    final finalPosition = tester.getTopLeft(cursorFinder);

    expect(intermediatePosition.dy, greaterThan(initialPosition.dy));
    expect(intermediatePosition.dy, lessThan(finalPosition.dy));
  });

  testWidgets('builds only the visible portion of a long transcript', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    final messages = List<ChatUiMessage>.generate(
      80,
      (index) => _aiMessage(
        parts: <MessagePart>[
          MessagePart(
            partId: 'part-$index',
            sequence: 0,
            kind: MessagePartKind.markdown,
            content: 'Response $index',
            toolCallId: null,
            toolName: null,
            attributes: const <String, String>{},
          ),
        ],
        timestamp: index + 1,
        completedAt: 1,
      ),
    );
    addTearDown(() {
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });

    await tester.pumpWidget(
      _chatArea(
        messages: messages,
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    await tester.pump();

    expect(
      find.byType(CursorStyleChatMessage).evaluate().length,
      lessThan(messages.length),
    );
  });

  testWidgets('reserves transcript padding for the floating status lane', (
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
          parts: <MessagePart>[
            MessagePart(
              partId: 'part',
              sequence: 0,
              kind: MessagePartKind.markdown,
              content: 'Response',
              toolCallId: null,
              toolName: null,
              attributes: const <String, String>{},
            ),
          ],
          completedAt: 1,
        ),
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        bottomContentInset: inputProcessingStatusLaneHeight,
      ),
    );
    await tester.pump();

    final listView = tester.widget<ListView>(find.byType(ListView));
    expect(listView.padding, const EdgeInsets.fromLTRB(16, 16, 16, 48));
  });

  testWidgets('does not render a trailing stream newline as a paragraph gap', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    addTearDown(() async {
      await streamController.close();
    });

    await tester.pumpWidget(
      _streamingStructuredRendererHarness(
        parts: const <MessagePart>[],
        contentStream: streamController.stream,
        isStreaming: true,
        streamState: StreamMarkdownRendererState(),
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('final line\n'));
    await tester.pump(const Duration(milliseconds: 250));

    expect(
      find.byKey(const ValueKey<String>('markdown-paragraph-break')),
      findsNothing,
    );
  });

  testWidgets('keeps live output when Flow rebuilds the stream wrapper', (
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
      ..add(_markdownBlockChunk('before flow rebuild'))
      ..add(_markdownCompleted());
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('before flow rebuild'), findsWidgets);

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[],
          stream: streamController.stream.map((event) => event),
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    await tester.pump();

    expect(find.textContaining('before flow rebuild'), findsWidgets);
  });

  testWidgets(
    'renders generated chat flow stream through route snapshot churn',
    (tester) async {
      final bridge = _ScriptedGeneratedChatBridge();
      final rendererState = StreamMarkdownRendererState();

      await tester.pumpWidget(
        _GeneratedFlowRendererHarness(
          bridge: bridge,
          rendererState: rendererState,
        ),
      );
      await tester.pump();

      bridge.emitChatSnapshot(<Object?>[
        _chatMessageLinkValue(contentStream: null),
      ]);
      await tester.pump();

      bridge.emitChatDelta(<String, Object?>{
        r'$coreDelta': <Object?>[
          <String, Object?>{
            'op': 'set',
            'path': <Object?>[0, 'contentStream'],
            'value': _streamDescriptorLinkValue('stream-ai'),
          },
        ],
      });
      await tester.pump();

      expect(bridge.openedStreamIds, <String>['stream-ai']);

      bridge
        ..emitMarkdownEvent('stream-ai', _markdownBlockStart())
        ..emitMarkdownEvent('stream-ai', _markdownBlockChunk('before switch '));
      await tester.pump(const Duration(milliseconds: 250));

      expect(_renderedMarkdownText(rendererState), contains('before switch'));

      bridge.emitChatSnapshot(<Object?>[
        _chatMessageLinkValue(
          contentStream: _streamDescriptorLinkValue('stream-ai'),
        ),
      ]);
      await tester.pump();

      expect(_renderedMarkdownText(rendererState), contains('before switch'));

      bridge.emitMarkdownEvent(
        'stream-ai',
        _markdownBlockChunk('after switch'),
      );
      await tester.pump(const Duration(milliseconds: 250));

      expect(
        _renderedMarkdownText(rendererState),
        contains('before switch after switch'),
      );

      bridge.completeMarkdownStream('stream-ai');
      await tester.pump();

      bridge.emitChatSnapshot(<Object?>[
        _chatMessageLinkValue(
          contentStream: null,
          parts: <Object?>[_messagePartLinkValue('before switch after switch')],
          completedAt: 1,
        ),
      ]);
      await _pumpRenderBoundary(tester);

      expect(
        _renderedMarkdownText(rendererState),
        contains('before switch after switch'),
      );
      expect(bridge.openedStreamIds, <String>['stream-ai']);
    },
  );

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

  testWidgets(
    'keeps receiving live chunks when message parts arrive before stream end',
    (tester) async {
      final streamController = StreamController<MarkdownStreamEvent>();
      final rendererState = StreamMarkdownRendererState();
      addTearDown(() async {
        await streamController.close();
      });

      await tester.pumpWidget(
        _streamingStructuredRendererHarness(
          parts: const <MessagePart>[],
          contentStream: streamController.stream,
          isStreaming: true,
          streamState: rendererState,
        ),
      );
      streamController
        ..add(_markdownBlockStart())
        ..add(_markdownBlockChunk('before switch '));
      await tester.pump(const Duration(milliseconds: 250));

      await tester.pumpWidget(
        _streamingStructuredRendererHarness(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'part-0',
              sequence: 0,
              kind: MessagePartKind.markdown,
              content: 'persisted partial',
              toolCallId: null,
              toolName: null,
              attributes: <String, String>{},
            ),
          ],
          contentStream: null,
          isStreaming: false,
          streamState: rendererState,
        ),
      );
      await _pumpRenderBoundary(tester);

      expect(
        find.byKey(const ValueKey<String>('live-markdown')),
        findsOneWidget,
      );

      streamController.add(_markdownBlockChunk('after switch'));
      await tester.pump(const Duration(milliseconds: 250));

      expect(
        _renderedMarkdownText(rendererState),
        contains('before switch after switch'),
      );

      streamController.add(_markdownCompleted());
      await tester.pump(const Duration(milliseconds: 250));

      await tester.pumpWidget(
        _streamingStructuredRendererHarness(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'part-0',
              sequence: 0,
              kind: MessagePartKind.markdown,
              content: 'before switch after switch',
              toolCallId: null,
              toolName: null,
              attributes: <String, String>{},
            ),
          ],
          contentStream: null,
          isStreaming: false,
          streamState: rendererState,
        ),
      );
      await _pumpRenderBoundary(tester);

      expect(find.byKey(const ValueKey<String>('live-markdown')), findsNothing);
      expect(find.textContaining('before switch after switch'), findsOneWidget);
    },
  );

  testWidgets('does not release retained live output to empty message parts', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    final rendererState = StreamMarkdownRendererState();
    addTearDown(() async {
      await streamController.close();
    });

    await tester.pumpWidget(
      _streamingStructuredRendererHarness(
        parts: const <MessagePart>[],
        contentStream: streamController.stream,
        isStreaming: true,
        streamState: rendererState,
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('before switch '));
    await tester.pump(const Duration(milliseconds: 250));

    await tester.pumpWidget(
      _streamingStructuredRendererHarness(
        parts: const <MessagePart>[],
        contentStream: null,
        isStreaming: false,
        streamState: rendererState,
      ),
    );
    await _pumpRenderBoundary(tester);

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsOneWidget);

    streamController.add(_markdownCompleted());
    await tester.pump(const Duration(milliseconds: 250));

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsOneWidget);
    expect(_renderedMarkdownText(rendererState), contains('before switch'));

    await tester.pumpWidget(
      _streamingStructuredRendererHarness(
        parts: const <MessagePart>[
          MessagePart(
            partId: 'part-0',
            sequence: 0,
            kind: MessagePartKind.markdown,
            content: 'before switch',
            toolCallId: null,
            toolName: null,
            attributes: <String, String>{},
          ),
        ],
        contentStream: null,
        isStreaming: false,
        streamState: rendererState,
      ),
    );
    await _pumpRenderBoundary(tester);

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsNothing);
    expect(find.textContaining('before switch'), findsOneWidget);
  });

  testWidgets('hands off a completed live stream to final Markdown parts', (
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
      ..add(_markdownBlockChunk('live answer'))
      ..add(_markdownCompleted());
    await tester.pump(const Duration(milliseconds: 250));

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsOneWidget);

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
          parts: const <MessagePart>[
            MessagePart(
              partId: 'part-0',
              sequence: 0,
              kind: MessagePartKind.markdown,
              content: 'final persisted answer',
              toolCallId: null,
              toolName: null,
              attributes: <String, String>{},
            ),
          ],
          completedAt: 1,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );

    expect(find.textContaining('live answer'), findsWidgets);
    expect(
      find.byKey(
        const ValueKey<String>('structured-parts'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );

    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey<String>('live-markdown')), findsNothing);
    expect(find.textContaining('final persisted answer'), findsOneWidget);
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

/// Pumps the renderer through stream delivery, throttled flush, and readiness frames.
Future<void> _pumpRenderBoundary(WidgetTester tester) async {
  for (var index = 0; index < 3; index += 1) {
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
  }
  await tester.pump();
}

/// Hosts a generated chat flow subscription and renders the latest AI message.
class _GeneratedFlowRendererHarness extends StatefulWidget {
  const _GeneratedFlowRendererHarness({
    required this.bridge,
    required this.rendererState,
  });

  final _ScriptedGeneratedChatBridge bridge;
  final StreamMarkdownRendererState rendererState;

  @override
  State<_GeneratedFlowRendererHarness> createState() =>
      _GeneratedFlowRendererHarnessState();
}

/// Drives the structured renderer from generated chat flow snapshots.
class _GeneratedFlowRendererHarnessState
    extends State<_GeneratedFlowRendererHarness> {
  StreamSubscription<List<ChatUiMessage>>? _messagesSubscription;
  List<ChatUiMessage> _messages = const <ChatUiMessage>[];

  @override
  void initState() {
    super.initState();
    final chat = GeneratedCoreProxyClients(widget.bridge).chatRuntimeHolderMain;
    _messagesSubscription = chat.chatMessagesFlow(chatId: 'chat').listen((
      messages,
    ) {
      setState(() {
        _messages = messages;
      });
    });
  }

  @override
  void dispose() {
    unawaited(_messagesSubscription?.cancel());
    widget.bridge.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final message = _messages.isEmpty ? null : _messages.last;
    return MaterialApp(
      home: Scaffold(
        body: message == null
            ? const SizedBox.shrink()
            : StreamingStructuredMessageRenderer(
                parts: message.parts,
                contentStream: message.contentStream,
                isStreaming: message.contentStream != null,
                textColor: Colors.black,
                backgroundColor: Colors.white,
                showThinkingProcess: true,
                nodeGrouper: const ThinkToolsXmlNodeGrouper(
                  showThinkingProcess: true,
                ),
                streamState: widget.rendererState,
                rendererId: 'generated-flow-${message.timestamp}',
                splitMarkdownContent: _splitMarkdownContent,
              ),
      ),
    );
  }
}

/// Emits generated proxy events in the same order as routed chat streaming.
class _ScriptedGeneratedChatBridge extends OperitRuntimeBridge {
  final StreamController<CoreEvent> _chatEvents =
      StreamController<CoreEvent>.broadcast(sync: true);
  final Map<String, StreamController<CoreEvent>> _streamEventsById =
      <String, StreamController<CoreEvent>>{};
  final Map<String, CoreWatchRequest> _streamRequestsById =
      <String, CoreWatchRequest>{};
  final List<String> openedStreamIds = <String>[];
  CoreWatchRequest? _chatWatchRequest;

  /// Releases all test-owned stream controllers.
  void dispose() {
    unawaited(_chatEvents.close());
    for (final controller in _streamEventsById.values) {
      unawaited(controller.close());
    }
  }

  /// Rejects encoded calls because this harness only exercises watch streams.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    throw UnimplementedError();
  }

  /// Rejects client-owned streams because this harness only exercises watches.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnimplementedError();
  }

  /// Rejects one-shot snapshots because this harness only exercises streams.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnimplementedError();
  }

  /// Opens the generated chat flow or one embedded Markdown stream.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    switch (request.propertyName) {
      case 'chatMessagesFlow':
        _chatWatchRequest = request;
        return _chatEvents.stream;
      case 'openCoreStream':
        final streamId = _streamIdFromRequest(request);
        final controller = StreamController<CoreEvent>.broadcast(sync: true);
        _streamEventsById[streamId] = controller;
        _streamRequestsById[streamId] = request;
        openedStreamIds.add(streamId);
        return controller.stream;
      default:
        throw StateError('Unexpected watch property ${request.propertyName}');
    }
  }

  /// Emits a complete chatMessagesFlow snapshot through the generated decoder.
  void emitChatSnapshot(List<Object?> messages) {
    _chatEvents.add(_chatEvent('Snapshot', messages));
  }

  /// Emits one chatMessagesFlow delta through the generated decoder.
  void emitChatDelta(Object? delta) {
    _chatEvents.add(_chatEvent('Delta', delta));
  }

  /// Emits one Markdown stream event through an opened embedded stream.
  void emitMarkdownEvent(String streamId, MarkdownStreamEvent event) {
    final controller = _requiredStreamController(streamId);
    controller.add(_streamEvent(streamId, 'Changed', event.toJson()));
  }

  /// Completes one opened embedded Markdown stream.
  void completeMarkdownStream(String streamId) {
    final controller = _requiredStreamController(streamId);
    controller.add(_streamEvent(streamId, 'Completed', null));
    unawaited(controller.close());
  }

  /// Returns the generated chat flow event for the active subscription.
  CoreEvent _chatEvent(String kind, Object? value) {
    final request = _chatWatchRequest;
    if (request == null) {
      throw StateError('chatMessagesFlow was not opened');
    }
    return _rawGeneratedEvent(request, kind, value);
  }

  /// Returns the generated embedded stream event for one opened stream.
  CoreEvent _streamEvent(String streamId, String kind, Object? value) {
    final request = _streamRequestsById[streamId];
    if (request == null) {
      throw StateError('Embedded stream was not opened: $streamId');
    }
    return _rawGeneratedEvent(request, kind, value);
  }

  /// Returns the controller backing one opened embedded stream.
  StreamController<CoreEvent> _requiredStreamController(String streamId) {
    final controller = _streamEventsById[streamId];
    if (controller == null) {
      throw StateError('Embedded stream controller missing: $streamId');
    }
    return controller;
  }
}

/// Builds a direct structured renderer harness with a caller-owned stream state.
Widget _streamingStructuredRendererHarness({
  required List<MessagePart> parts,
  required Stream<Object>? contentStream,
  required bool isStreaming,
  required StreamMarkdownRendererState streamState,
}) {
  return MaterialApp(
    home: Scaffold(
      body: StreamingStructuredMessageRenderer(
        parts: parts,
        contentStream: contentStream,
        isStreaming: isStreaming,
        textColor: Colors.black,
        backgroundColor: Colors.white,
        showThinkingProcess: true,
        nodeGrouper: const ThinkToolsXmlNodeGrouper(showThinkingProcess: true),
        streamState: streamState,
        rendererId: 'direct-generated-flow',
        splitMarkdownContent: _splitMarkdownContent,
      ),
    ),
  );
}

/// Builds a minimal themed transcript around one active AI message.
Widget _chatArea({
  ChatUiMessage? message,
  List<ChatUiMessage>? messages,
  required ScrollController scrollController,
  required ValueNotifier<bool> autoScrollToBottom,
  bool isLoading = true,
  double bottomContentInset = 0,
}) {
  return OperitTheme(
    initialThemePreferenceSnapshot:
        UserPreferencesManager.defaultThemePreferenceSnapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: Scaffold(
      body: ChatArea(
        messages:
            messages ??
            (message == null
                ? const <ChatUiMessage>[]
                : <ChatUiMessage>[message]),
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
        onRevealMessageForLocator: (timestamp) async => false,
        onAutoScrollToBottomChanged: (_) {},
        onLoadOlderDisplayWindow: () async {},
        onLoadNewerDisplayWindow: () async {},
        onShowLatestDisplayWindow: () async {},
        onToggleFavoriteMessage: (timestamp, isFavorite) async {},
        onDeleteMessage: (timestamp) async {},
        onDeleteMessagesFrom: (timestamp) async => true,
        onDeleteMessageVariant: (timestamp, variantIndex) async {},
        onSelectMessageVariant: (timestamp, selectedVariantIndex) async {},
        onRollbackToMessage: (_) {},
        onSelectMessageToEdit: (message) {},
        onRegenerateMessage: (timestamp) async {},
        onInsertSummary: (_) {},
        onCreateBranch: (timestamp) async {},
        onReplyToMessage: (_) {},
        onPlayVoice: (message) async {},
        onToggleMultiSelectMode: (_) {},
        onToggleMessageSelection: (_) {},
        onRefreshRequested: () async {},
        bottomContentInset: bottomContentInset,
        splitMarkdownContent: _splitMarkdownContent,
      ),
    ),
  );
}

/// Reads the stream id from one generated openCoreStream request.
String _streamIdFromRequest(CoreWatchRequest request) {
  final args = request.args;
  if (args is! Map) {
    throw StateError('openCoreStream args must be a map');
  }
  final streamId = args['streamId'];
  if (streamId is! String || streamId.isEmpty) {
    throw StateError('openCoreStream args must include streamId');
  }
  return streamId;
}

/// Creates one raw generated watch event with encoded Link payload bytes.
CoreEvent _rawGeneratedEvent(
  CoreWatchRequest request,
  String kind,
  Object? value,
) {
  return CoreEvent.raw(
    requestId: request.requestId,
    targetObjectId: request.targetObjectId,
    propertyName: request.propertyName,
    kind: kind,
    valueBytes: encodeCoreLink(value),
    decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
  );
}

/// Builds a complete ChatMessage Link map for generated proxy decoding.
Map<String, Object?> _chatMessageLinkValue({
  required Object? contentStream,
  List<Object?> parts = const <Object?>[],
  int completedAt = 0,
}) {
  return <String, Object?>{
    'sender': 'ai',
    'parts': parts,
    'timestamp': 1,
    'roleName': 'assistant',
    'selectedVariantIndex': 0,
    'variantCount': 1,
    'provider': 'test',
    'modelName': 'test',
    'inputTokens': 0,
    'outputTokens': 0,
    'cachedInputTokens': 0,
    'sentAt': 0,
    'outputDurationMs': 0,
    'waitDurationMs': 0,
    'completedAt': completedAt,
    'displayMode': 'NORMAL',
    'isFavorite': false,
    'contentStream': contentStream,
  };
}

/// Builds a Markdown MessagePart Link map for generated proxy decoding.
Map<String, Object?> _messagePartLinkValue(String content) {
  return <String, Object?>{
    'partId': 'part-0',
    'sequence': 0,
    'kind': 'markdown',
    'content': content,
    'toolCallId': null,
    'toolName': null,
    'attributes': <String, Object?>{},
  };
}

/// Builds one embedded stream descriptor in the standard Link wire shape.
Map<String, Object?> _streamDescriptorLinkValue(String streamId) {
  return <String, Object?>{
    r'$coreStream': <String, Object?>{
      'streamId': streamId,
      'targetObjectId': 4294967295,
      'propertyName': 'openCoreStream',
      'args': <String, Object?>{'streamId': streamId},
    },
  };
}

/// Returns visible text currently represented by the live Markdown node state.
String _renderedMarkdownText(StreamMarkdownRendererState state) {
  final buffer = StringBuffer();
  for (final node in state.renderNodes) {
    _appendRenderedMarkdownNodeText(buffer, node);
  }
  return buffer.toString();
}

/// Appends one Markdown node and its children to a plain text buffer.
void _appendRenderedMarkdownNodeText(
  StringBuffer buffer,
  MarkdownNodeStable node,
) {
  buffer.write(node.content);
  for (final child in node.children) {
    _appendRenderedMarkdownNodeText(buffer, child);
  }
}

/// Produces the same event boundary used by Core for one static Markdown block.
Future<List<MarkdownStreamEvent>> _splitMarkdownContent(String content) async {
  return <MarkdownStreamEvent>[
    _markdownBlockStart(),
    _markdownBlockChunk(content),
    _markdownCompleted(),
  ];
}

/// Creates the active AI message used to verify transcript cursor ownership.
ChatUiMessage _aiMessage({
  required List<MessagePart> parts,
  Stream<MarkdownStreamEvent>? stream,
  int timestamp = 1,
  int completedAt = 0,
}) {
  return ChatMessage(
    sender: 'ai',
    parts: parts,
    timestamp: timestamp,
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
