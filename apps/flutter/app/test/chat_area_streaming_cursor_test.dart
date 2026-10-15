import 'support/markdown_event_fixture.dart';

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/gestures.dart';
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
import 'package:operit2/ui/features/chat/components/ChatMessageExtent.dart';
import 'package:operit2/ui/features/chat/components/ChatScrollNavigator.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/InputProcessingStatusLane.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/CursorStyleChatMessage.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies transcript rendering and scroll ownership across asynchronous updates.
void main() {
  testWidgets(
    'keeps measured scroll extent stable during a manual upward drag',
    (tester) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(false);
      addTearDown(scrollController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      final messages = List.generate(
        18,
        (index) => _heightTestMessage(index, lines: index % 4 == 0 ? 20 : 1),
      );
      await tester.pumpWidget(
        _chatArea(
          messages: messages,
          isLoading: false,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
        ),
      );
      await tester.pumpAndSettle();
      for (var step = 0; step < 100; step++) {
        final position = scrollController.position;
        if (position.extentAfter < 1) {
          break;
        }
        scrollController.jumpTo(
          (position.pixels + 120).clamp(0, position.maxScrollExtent),
        );
        await tester.pumpAndSettle();
      }
      expect(scrollController.position.extentAfter, lessThan(1));
      final measuredBottom = scrollController.position.maxScrollExtent;
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(CustomScrollView)),
      );
      for (var step = 0; step < 18; step++) {
        await gesture.moveBy(const Offset(0, 60));
        await tester.pump();
        expect(
          scrollController.position.maxScrollExtent,
          closeTo(measuredBottom, 1),
          reason: 'Measured rows must not be replaced by a visible-row average',
        );
      }
      await gesture.up();
      await tester.pump(const Duration(milliseconds: 1600));
      expect(autoScrollToBottom.value, isFalse);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'keeps the manual viewport fixed while an earlier row changes height',
    (tester) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(false);
      final changedContent = Completer<List<MarkdownStreamEvent>>();
      addTearDown(scrollController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      final messages = List.generate(
        12,
        (index) => _heightTestMessage(100 + index, lines: 5),
      );

      /// Hosts the same transcript while delaying the edited first row's parsing.
      Widget transcript(List<ChatUiMessage> rows) => _chatArea(
        messages: rows,
        isLoading: false,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        splitMarkdownContent: (content) => content == 'height-test-edited'
            ? changedContent.future
            : _splitMarkdownContent(content),
      );

      await tester.pumpWidget(transcript(messages));
      await tester.pumpAndSettle();
      final target = find.byWidgetPredicate(
        (widget) =>
            widget is CursorStyleChatMessage &&
            widget.message.timestamp == messages[1].timestamp,
      );
      final viewportTop = tester.getTopLeft(find.byType(CustomScrollView)).dy;
      scrollController.jumpTo(tester.getTopLeft(target).dy - viewportTop - 40);
      await tester.pumpAndSettle();
      final targetTop = tester.getTopLeft(target).dy;
      final previousOffset = scrollController.offset;
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(CustomScrollView)),
      );
      final edited = _heightTestMessageWithContent(100, 'height-test-edited');
      await tester.pumpWidget(transcript([edited, ...messages.skip(1)]));
      await tester.pump();
      expect(tester.getTopLeft(target).dy, closeTo(targetTop, 1));
      expect(scrollController.offset, closeTo(previousOffset, 1));
      changedContent.complete([
        _markdownBlockStart(),
        _markdownBlockChunk(List.filled(24, 'Expanded earlier row').join('\n')),
        _markdownCompleted(),
      ]);
      await tester.pump();
      await tester.pump();
      expect(tester.getTopLeft(target).dy, closeTo(targetTop, 1));
      expect(scrollController.offset, greaterThan(previousOffset));
      await gesture.moveBy(const Offset(0, 60));
      await tester.pump();
      final dragOffset = scrollController.offset;
      await gesture.moveBy(const Offset(0, 60));
      await tester.pump();
      expect(scrollController.offset, closeTo(dragOffset - 60, 1));
      await gesture.up();
      await tester.pump(const Duration(milliseconds: 1600));
      expect(autoScrollToBottom.value, isFalse);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('retains row distance across display-window changes', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    final pending = Completer<List<MarkdownStreamEvent>>();
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);

    /// Renders one window without changing the chat's stable identity.
    Widget window(ChatUiMessage message) => _chatArea(
      message: message,
      isLoading: false,
      scrollController: scrollController,
      autoScrollToBottom: autoScrollToBottom,
      splitMarkdownContent: (content) => content == 'window-height-delayed'
          ? pending.future
          : _splitMarkdownContent(content),
    );

    await tester.pumpWidget(window(_heightTestMessage(200, lines: 15)));
    await tester.pumpAndSettle();
    final originalExtent = tester
        .widget<ChatMessageExtentBox>(find.byType(ChatMessageExtentBox))
        .extent;
    final originalHeight = tester
        .getSize(find.byType(ChatMessageExtentBox))
        .height;
    await tester.pumpWidget(window(_heightTestMessage(201)));
    await tester.pumpAndSettle();
    await tester.pumpWidget(
      window(_heightTestMessageWithContent(200, 'window-height-delayed')),
    );
    expect(
      tester
          .widget<ChatMessageExtentBox>(find.byType(ChatMessageExtentBox))
          .extent,
      same(originalExtent),
    );
    expect(
      tester.getSize(find.byType(ChatMessageExtentBox)).height,
      originalHeight,
    );
    pending.complete([
      _markdownBlockStart(),
      _markdownBlockChunk('Shortened window message'),
      _markdownCompleted(),
    ]);
    await tester.pumpAndSettle();
    expect(
      tester.getSize(find.byType(ChatMessageExtentBox)).height,
      lessThan(originalHeight),
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('keeps following disabled when scrolling away from the bottom', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(true);
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(parts: const []),
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
      ),
    );
    await tester.pumpAndSettle();
    final scrollableContext = tester.element(find.byType(Scrollable).first);
    final metrics = scrollController.position.copyWith(
      pixels: scrollController.position.maxScrollExtent - 1,
    );
    UserScrollNotification(
      metrics: metrics,
      context: scrollableContext,
      direction: ScrollDirection.forward,
    ).dispatch(scrollableContext);
    ScrollUpdateNotification(
      metrics: metrics,
      context: scrollableContext,
      scrollDelta: -1,
    ).dispatch(scrollableContext);
    expect(autoScrollToBottom.value, isFalse);
    UserScrollNotification(
      metrics: metrics,
      context: scrollableContext,
      direction: ScrollDirection.idle,
    ).dispatch(scrollableContext);
    expect(autoScrollToBottom.value, isFalse);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'does not allow the chat transcript to overscroll at the bottom',
    (tester) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(false);
      addTearDown(scrollController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      await tester.pumpWidget(
        _chatArea(
          message: _aiMessage(parts: const []),
          isLoading: false,
          bottomContentInset: 1600,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
        ),
      );
      await tester.pumpAndSettle();

      expect(scrollController.position.physics, isA<ClampingScrollPhysics>());
      final bottomOffset = scrollController.position.maxScrollExtent;
      scrollController.jumpTo(bottomOffset);
      await tester.drag(find.byType(CustomScrollView), const Offset(0, -200));

      expect(scrollController.offset, bottomOffset);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('ignores nested scroll updates at their own bottom', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(parts: const []),
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
      ),
    );
    await tester.pumpAndSettle();
    final scrollableContext = tester.element(find.byType(Scrollable).first);
    ScrollUpdateNotification(
      metrics: FixedScrollMetrics(
        minScrollExtent: 0,
        maxScrollExtent: 100,
        pixels: 100,
        viewportDimension: 100,
        axisDirection: AxisDirection.down,
        devicePixelRatio: 1,
      ),
      context: scrollableContext,
      depth: 1,
      scrollDelta: 10,
    ).dispatch(scrollableContext);
    expect(autoScrollToBottom.value, isFalse);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('does not enable following after a programmatic scroll', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(parts: const []),
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
      ),
    );
    await tester.pumpAndSettle();
    scrollController.jumpTo(scrollController.position.maxScrollExtent);
    await tester.pumpAndSettle();
    expect(autoScrollToBottom.value, isFalse);
    await tester.pumpWidget(const SizedBox());
  });

  for (final window in [
    (hasNewer: false, loading: false, follows: true),
    (hasNewer: true, loading: false, follows: false),
    (hasNewer: false, loading: true, follows: false),
  ]) {
    testWidgets('follows user scrolling to the bottom only for $window', (
      tester,
    ) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(false);
      addTearDown(scrollController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      await tester.pumpWidget(
        _chatArea(
          message: _aiMessage(parts: const []),
          isLoading: false,
          hasNewerDisplayHistory: window.hasNewer,
          isLoadingDisplayWindow: window.loading,
          bottomContentInset: 1600,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
          onAutoScrollToBottomChanged: (value) =>
              autoScrollToBottom.value = value,
        ),
      );
      await tester.pumpAndSettle();
      scrollController.jumpTo(scrollController.position.maxScrollExtent - 60);
      await tester.pump();
      await tester.drag(find.byType(CustomScrollView), const Offset(0, -200));
      await tester.pumpAndSettle();
      expect(autoScrollToBottom.value, window.follows);
      await tester.pumpWidget(const SizedBox());
    });
  }

  testWidgets('cancels locator correction when the user takes over scrolling', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    await tester.pumpWidget(
      _chatArea(
        messages: [
          _aiMessage(parts: const [], timestamp: 10001),
          _aiMessage(parts: const [], timestamp: 10002),
        ],
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
      ),
    );
    await tester.pumpAndSettle();
    tester
        .widget<ChatScrollNavigator>(find.byType(ChatScrollNavigator))
        .onJumpToMessage(0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 270));
    await tester.pump();
    await tester.drag(find.byType(CustomScrollView), const Offset(0, -200));
    await tester.pumpAndSettle();
    expect(scrollController.offset, greaterThan(100));
    await tester.pumpWidget(const SizedBox());
  });

  for (final messageCount in [1, 80, 500]) {
    testWidgets(
      'paints the switched chat at its bottom with $messageCount rows',
      (tester) async {
        final scrollController = ScrollController();
        final autoScrollToBottom = ValueNotifier<bool>(true);
        final paints = <(double, double)>[];

        /// Observes the active viewport even while an old keyed viewport unmounts.
        Widget observePaint(Widget child) => CustomPaint(
          foregroundPainter: _ScrollPositionPaintProbe(() {
            final position = tester
                .state<ScrollableState>(find.byType(Scrollable).first)
                .position;
            paints.add((position.pixels, position.maxScrollExtent));
          }),
          child: child,
        );
        addTearDown(scrollController.dispose);
        addTearDown(autoScrollToBottom.dispose);
        await tester.pumpWidget(
          _chatArea(
            messages: List.generate(
              40,
              (index) => _aiMessage(parts: const [], timestamp: 41000 + index),
            ),
            isLoading: false,
            bottomContentInset: 1600,
            bodyBuilder: observePaint,
            scrollController: scrollController,
            autoScrollToBottom: autoScrollToBottom,
            onAutoScrollToBottomChanged: (value) =>
                autoScrollToBottom.value = value,
          ),
        );
        await tester.pumpAndSettle();
        // Switch from a locator's coordinate system, not just a fresh viewport.
        tester
            .widget<ChatScrollNavigator>(find.byType(ChatScrollNavigator))
            .onJumpToMessage(20);
        await tester.pumpAndSettle();
        expect(autoScrollToBottom.value, isFalse);
        autoScrollToBottom.value = true;
        paints.clear();
        final loaded = Completer<List<MarkdownStreamEvent>>();
        final messages = List.generate(
          messageCount,
          (index) => _aiMessage(
            timestamp: 42000 + index,
            completedAt: 1,
            parts: [
              MessagePart(
                partId: 'switch-$index',
                sequence: 0,
                kind: MessagePartKind.markdown,
                content: 'switch-$messageCount-$index',
                toolCallId: null,
                toolName: null,
                attributes: const {},
              ),
            ],
          ),
        );
        await tester.pumpWidget(
          _chatArea(
            currentChatId: 'switched-chat',
            messages: messages,
            isLoading: false,
            bottomContentInset: 80,
            scrollController: scrollController,
            autoScrollToBottom: autoScrollToBottom,
            splitMarkdownContent: (_) => loaded.future,
            bodyBuilder: observePaint,
          ),
        );
        expect(paints, isNotEmpty);
        for (final (pixels, bottom) in paints) {
          expect(
            pixels,
            closeTo(bottom, 0.01),
            reason: 'The first painted frame',
          );
        }
        paints.clear();
        loaded.complete([
          _markdownBlockStart(),
          _markdownBlockChunk(
            List.filled(40, 'Delayed switched content').join('\n'),
          ),
          _markdownCompleted(),
        ]);
        for (var frame = 0; frame < 40; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
        }
        expect(paints, isNotEmpty);
        for (final (pixels, bottom) in paints) {
          expect(
            pixels,
            closeTo(bottom, 0.01),
            reason: 'Async content must align before paint',
          );
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        expect(scrollController.hasClients, isFalse);
      },
    );
  }

  testWidgets(
    'initially aligns a switched live chat without snapping later growth',
    (tester) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(true);
      final streamController = StreamController<MarkdownStreamEvent>();
      final paints = <(double, double)>[];
      addTearDown(scrollController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      addTearDown(streamController.close);

      /// Observes the active viewport before post-frame callbacks can move it.
      Widget observePaint(Widget child) => CustomPaint(
        foregroundPainter: _ScrollPositionPaintProbe(() {
          final position = tester
              .state<ScrollableState>(find.byType(Scrollable).first)
              .position;
          paints.add((position.pixels, position.maxScrollExtent));
        }),
        child: child,
      );
      await tester.pumpWidget(
        _chatArea(
          message: _aiMessage(parts: const [], timestamp: 43000),
          isLoading: false,
          bottomContentInset: 80,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
          bodyBuilder: observePaint,
        ),
      );
      await tester.pumpAndSettle();
      paints.clear();
      await tester.pumpWidget(
        _chatArea(
          currentChatId: 'live-switched-chat',
          messages: [
            ...List.generate(
              80,
              (index) => _aiMessage(parts: const [], timestamp: 44000 + index),
            ),
            _aiMessage(
              parts: const [],
              timestamp: 44080,
              stream: streamController.stream,
            ),
          ],
          isLoading: false,
          bottomContentInset: 80,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
          bodyBuilder: observePaint,
        ),
      );
      expect(paints, isNotEmpty);
      for (final (pixels, bottom) in paints) {
        expect(pixels, closeTo(bottom, 0.01));
      }
      await tester.pump();
      final initialOffset = scrollController.offset;
      streamController
        ..add(_markdownBlockStart())
        ..add(
          _markdownBlockChunk(
            List.filled(80, 'Continued live output').join('\n'),
          ),
        );
      await _pumpRenderBoundary(tester);
      expect(
        scrollController.position.maxScrollExtent,
        greaterThan(initialOffset),
      );
      expect(scrollController.position.extentAfter, greaterThan(1));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'transfers the layout position when its external controller changes',
    (tester) async {
      final oldController = ScrollController();
      final newController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(true);
      addTearDown(oldController.dispose);
      addTearDown(newController.dispose);
      addTearDown(autoScrollToBottom.dispose);
      final message = _aiMessage(parts: const [], timestamp: 45000);
      await tester.pumpWidget(
        _chatArea(
          message: message,
          isLoading: false,
          bottomContentInset: 1600,
          scrollController: oldController,
          autoScrollToBottom: autoScrollToBottom,
        ),
      );
      await tester.pumpAndSettle();
      final position = oldController.position;
      await tester.pumpWidget(
        _chatArea(
          message: message,
          isLoading: false,
          bottomContentInset: 1600,
          scrollController: newController,
          autoScrollToBottom: autoScrollToBottom,
        ),
      );
      await tester.pumpAndSettle();
      expect(oldController.hasClients, isFalse);
      expect(newController.position, same(position));
      autoScrollToBottom.value = false;
      newController.jumpTo(120);
      await tester.pumpAndSettle();
      expect(newController.offset, 120);
      await tester.pumpWidget(const SizedBox());
      expect(newController.hasClients, isFalse);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('keeps the locator target fixed while older Markdown loads', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    final pending = <String, Completer<List<MarkdownStreamEvent>>>{};
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    final messages = List.generate(
      24,
      (index) => _aiMessage(
        timestamp: 20000 + index,
        completedAt: 1,
        parts: [
          MessagePart(
            partId: 'delayed-$index',
            sequence: 0,
            kind: MessagePartKind.markdown,
            content: 'locator-delayed-$index',
            toolCallId: null,
            toolName: null,
            attributes: const {},
          ),
        ],
      ),
    );
    await tester.pumpWidget(
      _chatArea(
        messages: messages,
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
        splitMarkdownContent: (content) => pending
            .putIfAbsent(content, () => Completer<List<MarkdownStreamEvent>>())
            .future,
      ),
    );
    await tester.pumpAndSettle();
    tester
        .widget<ChatScrollNavigator>(find.byType(ChatScrollNavigator))
        .onJumpToMessage(12);
    await tester.pumpAndSettle();
    final target = find.byWidgetPredicate(
      (widget) =>
          widget is CursorStyleChatMessage && widget.message.timestamp == 20012,
    );
    final targetTop = tester.getTopLeft(target).dy;
    final viewportTop = tester.getTopLeft(find.byType(CustomScrollView)).dy;
    expect(targetTop, closeTo(viewportTop, 1));
    expect(pending['locator-delayed-11'], isNotNull);
    var scrollUpdates = 0;
    scrollController.addListener(() => scrollUpdates++);
    for (final index in [11, 10, 9]) {
      // Resolve independent loads well beyond the old locator settle interval.
      await tester.pump(const Duration(seconds: 1));
      pending['locator-delayed-$index']!.complete([
        _markdownBlockStart(),
        _markdownBlockChunk(
          List.filled(60, 'Expanded older message').join('\n'),
        ),
        _markdownCompleted(),
      ]);
      for (var frame = 0; frame < 24; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        expect(tester.getTopLeft(target).dy, closeTo(targetTop, 1));
      }
    }
    expect(scrollUpdates, 0);
    expect(autoScrollToBottom.value, isFalse);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('locates an unbuilt message without walking intermediate rows', (
    tester,
  ) async {
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    addTearDown(scrollController.dispose);
    addTearDown(autoScrollToBottom.dispose);
    await tester.pumpWidget(
      _chatArea(
        messages: List.generate(
          500,
          (index) => _aiMessage(parts: const [], timestamp: 30000 + index),
        ),
        isLoading: false,
        bottomContentInset: 1600,
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        onAutoScrollToBottomChanged: (value) =>
            autoScrollToBottom.value = value,
      ),
    );
    await tester.pumpAndSettle();
    final target = find.byWidgetPredicate(
      (widget) =>
          widget is CursorStyleChatMessage && widget.message.timestamp == 30400,
    );
    expect(target, findsNothing);
    tester
        .widget<ChatScrollNavigator>(find.byType(ChatScrollNavigator))
        .onJumpToMessage(400);
    await tester.pumpAndSettle();
    expect(target, findsOneWidget);
    expect(
      tester.getTopLeft(target).dy,
      closeTo(tester.getTopLeft(find.byType(CustomScrollView)).dy, 1),
    );
    expect(find.byType(CursorStyleChatMessage).evaluate().length, lessThan(50));
    await tester.drag(find.byType(CustomScrollView), const Offset(0, 180));
    await tester.pumpAndSettle();
    final userOffset = scrollController.offset;
    expect(userOffset, lessThan(0));
    await tester.pump(const Duration(seconds: 2));
    expect(scrollController.offset, userOffset);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

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

  testWidgets('keeps live AI row mounted when persisted parts update', (
    tester,
  ) async {
    final streamController = StreamController<MarkdownStreamEvent>();
    final contentStream = streamController.stream;
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
          stream: contentStream,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('before snapshot'));
    await tester.pump(const Duration(milliseconds: 250));
    final previousMessageWidget = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage),
    );

    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(
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
          stream: contentStream,
        ),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
      ),
    );
    await tester.pump();
    final currentMessageWidget = tester.widget<CursorStyleChatMessage>(
      find.byType(CursorStyleChatMessage),
    );

    expect(tester.takeException(), isNull);
    expect(identical(currentMessageWidget, previousMessageWidget), isTrue);

    streamController.add(_markdownBlockChunk(' after snapshot'));
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pump(const Duration(milliseconds: 250));

    expect(find.textContaining('before snapshot after snapshot'), findsWidgets);
  });

  testWidgets('retains the list delegate and live stream during width changes', (
    tester,
  ) async {
    final size = ValueNotifier<Size>(const Size(744, 500));
    final scrollController = ScrollController();
    final autoScrollToBottom = ValueNotifier<bool>(false);
    var listens = 0;
    var cancels = 0;
    final stream = StreamController<MarkdownStreamEvent>.broadcast(
      onListen: () => listens++,
      onCancel: () => cancels++,
    );
    addTearDown(() async {
      await stream.close();
      size.dispose();
      scrollController.dispose();
      autoScrollToBottom.dispose();
    });
    await tester.pumpWidget(
      _chatArea(
        message: _aiMessage(parts: const [], stream: stream.stream),
        scrollController: scrollController,
        autoScrollToBottom: autoScrollToBottom,
        bodyBuilder: (child) => Align(
          alignment: Alignment.topLeft,
          child: ValueListenableBuilder<Size>(
            valueListenable: size,
            child: child,
            builder: (context, value, child) => SizedBox(
              width: value.width,
              height: value.height,
              child: child,
            ),
          ),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 250));
    stream
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('before sidebar'));
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('before sidebar'), findsWidgets);
    final list = tester.widget<CustomScrollView>(find.byType(CustomScrollView));
    final position = scrollController.position;
    final rendererState = tester.state(find.byType(StreamMarkdownRenderer));
    final baselineListens = listens;
    expect(baselineListens, greaterThan(0));
    for (var cycle = 0; cycle < 3; cycle++) {
      for (final width in <double>[
        700,
        640,
        600,
        580,
        520,
        580,
        600,
        640,
        744,
      ]) {
        size.value = Size(width, 500);
        await tester.pump(const Duration(milliseconds: 16));
        expect(
          tester.widget<CustomScrollView>(find.byType(CustomScrollView)),
          same(list),
        );
        expect(scrollController.position, same(position));
        expect(
          tester.state(find.byType(StreamMarkdownRenderer)),
          same(rendererState),
        );
        expect(listens, baselineListens);
        expect(cancels, 0);
      }
    }
    // Height changes must still update the navigator without replacing the list.
    size.value = const Size(744, 400);
    await tester.pump(const Duration(milliseconds: 200));
    await tester.pump(const Duration(milliseconds: 200));
    expect(
      tester.widget<CustomScrollView>(find.byType(CustomScrollView)),
      same(list),
    );
    expect(
      tester
          .widget<ChatScrollNavigator>(find.byType(ChatScrollNavigator))
          .viewportHeight,
      400,
    );
    stream.add(_markdownBlockChunk(' after sidebar'));
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.textContaining('before sidebar after sidebar'), findsWidgets);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
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
    expect(intermediatePosition.dy, lessThanOrEqualTo(finalPosition.dy));
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

    final listView = tester.widget<CustomScrollView>(
      find.byType(CustomScrollView),
    );
    expect(
      (listView.slivers.last as SliverPadding).padding,
      const EdgeInsets.fromLTRB(
        16,
        16,
        16,
        16 + inputProcessingStatusLaneHeight,
      ),
    );
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
        streamState: StreamMarkdownRendererState(),
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(_markdownBlockChunk('final line\n'));
    await tester.pump(const Duration(milliseconds: 250));

    expect(
      find.byKey(const ValueKey<String>('markdown-paragraph-break-0')),
      findsNothing,
    );
  });

  /// Verifies that separated Markdown paragraphs do not create duplicate keys.
  testWidgets('renders multiple Markdown paragraph gaps without key errors', (
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
        streamState: StreamMarkdownRendererState(),
      ),
    );
    streamController
      ..add(_markdownBlockStart())
      ..add(
        _markdownBlockChunk(
          'first paragraph\n\nsecond paragraph\n\nthird paragraph',
        ),
      );
    await tester.pump(const Duration(milliseconds: 250));

    expect(tester.takeException(), isNull);
    expect(find.textContaining('first paragraph'), findsWidgets);
    expect(find.textContaining('second paragraph'), findsWidgets);
    expect(find.textContaining('third paragraph'), findsWidgets);
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
    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );

    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
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
          streamState: rendererState,
        ),
      );
      await _pumpRenderBoundary(tester);

      expect(
        find.byKey(const ValueKey<String>('assistant-markdown-surface')),
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
          streamState: rendererState,
        ),
      );
      await _pumpRenderBoundary(tester);

      expect(
        find.byKey(const ValueKey<String>('assistant-markdown-surface')),
        findsOneWidget,
      );
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
        streamState: rendererState,
      ),
    );
    await _pumpRenderBoundary(tester);

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );

    streamController.add(_markdownCompleted());
    await tester.pump(const Duration(milliseconds: 250));

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );
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
        streamState: rendererState,
      ),
    );
    await _pumpRenderBoundary(tester);

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );
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

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );

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
        const ValueKey<String>('assistant-markdown-surface'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );

    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey<String>('assistant-markdown-surface')),
      findsOneWidget,
    );
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

  for (final kind in [PointerDeviceKind.touch, PointerDeviceKind.trackpad]) {
    testWidgets('pauses live bottom follow while a $kind drag is active', (
      tester,
    ) async {
      final scrollController = ScrollController();
      final autoScrollToBottom = ValueNotifier<bool>(true);
      addTearDown(() {
        scrollController.dispose();
        autoScrollToBottom.dispose();
      });

      Widget chatArea(String content, {double bottomContentInset = 0}) {
        return _chatArea(
          message: _aiMessage(
            parts: <MessagePart>[
              MessagePart(
                partId: 'part-0',
                sequence: 0,
                kind: MessagePartKind.markdown,
                content: content,
                toolCallId: null,
                toolName: null,
                attributes: <String, String>{},
              ),
            ],
            stream: const Stream.empty(),
          ),
          isLoading: false,
          scrollController: scrollController,
          autoScrollToBottom: autoScrollToBottom,
          onAutoScrollToBottomChanged: (value) =>
              autoScrollToBottom.value = value,
          bottomContentInset: bottomContentInset,
        );
      }

      await tester.pumpWidget(chatArea('start', bottomContentInset: 800));
      await tester.pump();

      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(Scrollable).first),
        kind: kind,
      );
      await tester.pumpWidget(chatArea('updated', bottomContentInset: 1600));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 120));

      final maxScrollExtent = scrollController.position.maxScrollExtent;
      final pausedOffset = scrollController.offset;
      expect(maxScrollExtent - pausedOffset, greaterThan(80));
      await tester.pump(const Duration(milliseconds: 120));
      expect(scrollController.offset, pausedOffset);

      // Contact alone must not turn off following, but the same gesture must
      // take over immediately once the user pans, without lifting and retrying.
      expect(autoScrollToBottom.value, isTrue);
      await gesture.moveBy(const Offset(0, -120));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -120));
      await tester.pump();
      expect(scrollController.offset, greaterThan(pausedOffset));
      expect(autoScrollToBottom.value, isFalse);
      await gesture.up();
      await tester.pumpWidget(const SizedBox());
    });
  }
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
  required StreamMarkdownRendererState streamState,
}) {
  return MaterialApp(
    home: Scaffold(
      body: StreamingStructuredMessageRenderer(
        parts: parts,
        contentStream: contentStream,
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
  String currentChatId = 'chat',
  ChatUiMessage? message,
  List<ChatUiMessage>? messages,
  required ScrollController scrollController,
  required ValueNotifier<bool> autoScrollToBottom,
  bool isLoading = true,
  bool hasNewerDisplayHistory = false,
  bool isLoadingDisplayWindow = false,
  double bottomContentInset = 0,
  ValueChanged<bool>? onAutoScrollToBottomChanged,
  Widget Function(Widget)? bodyBuilder,
  MarkdownContentSplitter splitMarkdownContent = _splitMarkdownContent,
  RevealMessageForLocator? onRevealMessageForLocator,
}) {
  final bridge = _ScriptedGeneratedChatBridge();
  final clients = GeneratedCoreProxyClients(bridge);
  addTearDown(bridge.dispose);
  final chatArea = ChatArea(
    messages:
        messages ??
        (message == null ? const <ChatUiMessage>[] : <ChatUiMessage>[message]),
    isLoading: isLoading,
    errorMessage: null,
    scrollController: scrollController,
    currentChatId: currentChatId,
    identityAvatarUri: null,
    clients: clients,
    packageManager: clients.application.packageManager(),
    autoScrollToBottomListenable: autoScrollToBottom,
    hasOlderDisplayHistory: false,
    hasNewerDisplayHistory: hasNewerDisplayHistory,
    isLoadingDisplayWindow: isLoadingDisplayWindow,
    loadLocatorEntries: (chatId, query) async => const [],
    onRevealMessageForLocator:
        onRevealMessageForLocator ?? (timestamp) async => false,
    onAutoScrollToBottomChanged: onAutoScrollToBottomChanged ?? (_) {},
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
    splitMarkdownContent: splitMarkdownContent,
  );
  return OperitTheme(
    initialThemePreferenceSnapshot:
        UserPreferencesManager.defaultThemePreferenceSnapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: Scaffold(body: bodyBuilder?.call(chatArea) ?? chatArea),
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
    target: request.target,
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
    'pluginExtensions': <String, Object?>{},
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
      'target': r'$core.stream.open',
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
Future<List<MarkdownStreamEvent>> _splitMarkdownContent(String content) =>
    splitMarkdownEventFixture(content);

/// Creates a completed row with distinct content for height-cache regressions.
ChatUiMessage _heightTestMessage(int index, {int lines = 1}) {
  return _heightTestMessageWithContent(
    index,
    List.generate(lines, (line) => 'Height row $index line $line').join('\n'),
  );
}

/// Creates a completed height-test row with an explicit Markdown payload.
ChatUiMessage _heightTestMessageWithContent(int index, String content) {
  return _aiMessage(
    timestamp: 80000 + index,
    completedAt: 1,
    parts: [
      MessagePart(
        partId: 'height-part-$index',
        sequence: 0,
        kind: MessagePartKind.markdown,
        content: content,
        toolCallId: null,
        toolName: null,
        attributes: const {},
      ),
    ],
  );
}

/// Creates the active AI message used to verify transcript cursor ownership.
ChatUiMessage _aiMessage({
  required List<MessagePart> parts,
  Stream<MarkdownStreamEvent>? stream,
  int timestamp = 1,
  int completedAt = 0,
}) {
  return ChatMessage(
    pluginExtensions: const {},
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
    xml: null,
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
    xml: null,
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
    xml: null,
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
    xml: null,
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
    xml: null,
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
    xml: null,
  );
}

/// Samples the real paint phase rather than post-frame controller corrections.
class _ScrollPositionPaintProbe extends CustomPainter {
  /// Records the observed position whenever the transcript is painted.
  _ScrollPositionPaintProbe(this.onPaint);

  final VoidCallback onPaint;

  /// Observes the frame after layout has finalized the viewport position.
  @override
  void paint(Canvas canvas, Size size) => onPaint();

  /// Samples every replacement of the probe.
  @override
  bool shouldRepaint(_ScrollPositionPaintProbe oldDelegate) => true;
}
