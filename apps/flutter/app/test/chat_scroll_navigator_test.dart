import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/ChatScrollNavigator.dart';
import 'package:operit2/ui/features/chat/components/ChatScrollRailGeometry.dart';

/// Exercises the production rail geometry and real pointer gesture boundaries.
void main() {
  test('horizontal proximity grows smoothly and saturates beside the rail', () {
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    expect(geometry.proximityAt(0), 0);
    var previous = 0.0;
    for (var x = 8.0; x <= 80; x++) {
      final proximity = geometry.proximityAt(x);
      expect(proximity, inInclusiveRange(previous, 1.0));
      previous = proximity;
    }
    expect(geometry.proximityAt(68), 1);
    expect(geometry.proximityAt(80), 1);
    expect(geometry.proximityAt(88), 1);
  });

  test('the wave saturates before contact and cannot grow farther inward', () {
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    const right =
        ChatScrollRailGeometry.interactionWidth -
        ChatScrollRailGeometry.rightInset;
    const tip = right - ChatScrollRailGeometry.maximumTickWidth;
    expect(geometry.proximityAt(tip - 4), 1);
    for (var x = tip; x <= ChatScrollRailGeometry.interactionWidth; x++) {
      expect(geometry.proximityAt(x), 1);
      expect(
        geometry.tickWidth(
          restingWidth: 10,
          tickY: 250,
          pointerY: 250,
          proximity: geometry.proximityAt(x),
        ),
        24,
      );
    }
  });

  test('the vertical wave is symmetric, bounded and peaks at the pointer', () {
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    double width(double y, double proximity) => geometry.tickWidth(
      restingWidth: 10,
      tickY: y,
      pointerY: 250,
      proximity: proximity,
    );
    expect(width(250, 1), ChatScrollRailGeometry.maximumTickWidth);
    expect(width(250, 0), 10);
    expect(width(220, 1), closeTo(width(280, 1), 0.00001));
    expect(width(250, 1), greaterThan(width(270, 1)));
    expect(width(270, 1), greaterThan(width(310, 1)));
    expect(width(250, 0.5), inExclusiveRange(10, width(250, 1)));
  });

  for (final count in <int>[1, 20, 10000]) {
    for (final height in <double>[61, 120, 500, 1200]) {
      test('all $count messages fit and remain selectable at height $height', () {
        final geometry = ChatScrollRailGeometry(
          entryCount: count,
          viewportHeight: height,
        );
        expect(geometry.top, greaterThanOrEqualTo(0));
        expect(geometry.top + geometry.height, lessThanOrEqualTo(height));
        expect(geometry.tickCount, lessThanOrEqualTo(count));
        expect(geometry.tickCount, lessThanOrEqualTo((height / 3).floor()));
        expect(geometry.entryIndexAt(geometry.hitTop), 0);
        expect(geometry.entryIndexAt(geometry.hitTop + geometry.hitHeight), count - 1);
        for (var index = 0; index < count; index++) {
          expect(geometry.entryIndexAt(geometry.entryCenter(index)), index);
        }
        var end = 0;
        for (var tick = 0; tick < geometry.tickCount; tick++) {
          expect(geometry.bucketStart(tick), end);
          end = geometry.bucketEnd(tick);
          expect(end, greaterThan(geometry.bucketStart(tick)));
        }
        expect(end, count);
      });
    }
  }

  testWidgets('scroll boundaries highlight endpoints instead of the viewport center', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    await _pumpNavigator(
      tester,
      jumps: <int>[],
      scrollController: scroll,
      messageAnchors: <int, ChatScrollMessageAnchor>{
        for (var index = 0; index < 20; index++)
          index: ChatScrollMessageAnchor(
            timestamp: index,
            index: index,
            key: GlobalKey(),
            absoluteTopPx: index * 100.0,
            heightPx: 100,
          ),
      },
    );
    expect(_activeRailIndex(tester), 0);
    scroll.jumpTo(500);
    await tester.pump();
    expect(_activeRailIndex(tester), 7);
    scroll.jumpTo(scroll.position.maxScrollExtent);
    await tester.pump();
    expect(_activeRailIndex(tester), 19);
    scroll.jumpTo(scroll.position.minScrollExtent);
    await tester.pump();
    expect(_activeRailIndex(tester), 0);
  });

  testWidgets('boundary highlights do not depend on collected message anchors', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    await _pumpNavigator(tester, jumps: <int>[], scrollController: scroll);
    expect(_activeRailIndex(tester), 0);
    scroll.jumpTo(scroll.position.maxScrollExtent);
    await tester.pump();
    expect(_activeRailIndex(tester), 19);
  });

  testWidgets('a transcript without scroll range keeps its latest message active', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    await _pumpNavigator(
      tester,
      jumps: <int>[],
      scrollController: scroll,
      contentHeight: 100,
    );
    expect(scroll.position.minScrollExtent, scroll.position.maxScrollExtent);
    expect(_activeRailIndex(tester), 19);
  });

  testWidgets('mouse contact previews immediately without jumping or rebuilding per move', (
    tester,
  ) async {
    final jumps = <int>[];
    await _pumpNavigator(tester, jumps: jumps);
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    final paint = find.byKey(const ValueKey<String>('chat-scroll-rail-paint'));
    final rect = tester.getRect(paint);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(Offset(rect.right - 8, rect.top + geometry.entryCenter(7)));
    await tester.pump();
    expect(find.text('message 7'), findsOneWidget);
    expect(find.text('点击跳转'), findsNothing);
    expect(jumps, isEmpty);
    final painter = tester.widget<CustomPaint>(paint).painter;
    await mouse.moveBy(const Offset(-1, 1));
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.widget<CustomPaint>(paint).painter, same(painter));
    expect(jumps, isEmpty);
    await mouse.moveTo(Offset.zero);
    await tester.pumpAndSettle();
    expect(find.text('message 7'), findsNothing);
    await mouse.removePointer();
  });

  testWidgets('touch down previews and a completed tap jumps exactly once', (
    tester,
  ) async {
    final jumps = <int>[];
    await _pumpNavigator(tester, jumps: jumps);
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    final rect = tester.getRect(find.byKey(const ValueKey<String>('chat-scroll-rail-paint')));
    final touch = await tester.startGesture(
      Offset(rect.right - 12, rect.top + geometry.entryCenter(3)),
    );
    await tester.pump();
    expect(find.text('message 3'), findsOneWidget);
    expect(jumps, isEmpty);
    await touch.up();
    await tester.pumpAndSettle();
    expect(jumps, <int>[3]);
    expect(find.text('message 3'), findsNothing);
  });

  testWidgets('touch scrubbing only previews instead of repeatedly jumping', (
    tester,
  ) async {
    final jumps = <int>[];
    await _pumpNavigator(tester, jumps: jumps);
    const geometry = ChatScrollRailGeometry(entryCount: 20, viewportHeight: 500);
    final rect = tester.getRect(find.byKey(const ValueKey<String>('chat-scroll-rail-paint')));
    final touch = await tester.startGesture(
      Offset(rect.right - 12, rect.top + geometry.entryCenter(2)),
    );
    await tester.pump();
    await touch.moveTo(Offset(rect.right - 12, rect.top + geometry.entryCenter(10)));
    await tester.pump();
    expect(find.text('message 10'), findsOneWidget);
    expect(jumps, isEmpty);
    await touch.up();
    await tester.pumpAndSettle();
    expect(jumps, isEmpty);
  });

  testWidgets('the approach area does not consume transcript taps', (
    tester,
  ) async {
    final jumps = <int>[];
    var transcriptTaps = 0;
    await _pumpNavigator(tester, jumps: jumps, onTranscriptTap: () => transcriptTaps++);
    final rect = tester.getRect(find.byKey(const ValueKey<String>('chat-scroll-rail-paint')));
    await tester.tapAt(Offset(rect.left + 12, rect.center.dy));
    await tester.pumpAndSettle();
    expect(transcriptTaps, 1);
    expect(jumps, isEmpty);
  });

  testWidgets('dense history paints one layer and still selects both endpoints', (
    tester,
  ) async {
    final jumps = <int>[];
    await _pumpNavigator(tester, count: 5000, jumps: jumps);
    final target = find.byKey(const ValueKey<String>('chat-scroll-rail-hit-target'));
    final rect = tester.getRect(target);
    expect(tester.getSize(target).width, greaterThanOrEqualTo(44));
    expect(
      find.descendant(of: find.byType(ChatScrollNavigator), matching: find.byType(GestureDetector)),
      findsOneWidget,
    );
    await tester.tapAt(Offset(rect.center.dx, rect.top + 1));
    await tester.pumpAndSettle();
    await tester.tapAt(Offset(rect.center.dx, rect.bottom - 1));
    await tester.pumpAndSettle();
    expect(jumps, <int>[0, 4999]);
    expect(tester.takeException(), isNull);
  });
}

/// Reads the production painter's active bucket without involving hover state.
int? _activeRailIndex(WidgetTester tester) {
  final dynamic painter = tester.widget<CustomPaint>(
    find.byKey(const ValueKey<String>('chat-scroll-rail-paint')),
  ).painter;
  return painter.activeIndex as int?;
}

/// Mounts the real navigator with in-memory previews and no native runtime.
Future<void> _pumpNavigator(
  WidgetTester tester, {
  int count = 20,
  required List<int> jumps,
  VoidCallback? onTranscriptTap,
  ScrollController? scrollController,
  double contentHeight = 2000,
  Map<int, ChatScrollMessageAnchor> messageAnchors =
      const <int, ChatScrollMessageAnchor>{},
}) async {
  final messages = List<ChatMessage>.generate(count, _message);
  final scroll = scrollController ?? ScrollController();
  if (scrollController == null) addTearDown(scroll.dispose);
  await tester.pumpWidget(MaterialApp(
    locale: const Locale('zh'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: SizedBox(
        width: 360,
        height: 500,
        child: Stack(
          fit: StackFit.expand,
          children: <Widget>[
            if (scrollController != null)
              SingleChildScrollView(
                controller: scroll,
                child: SizedBox(height: contentHeight),
              )
            else
              GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onTranscriptTap,
              child: const SizedBox.expand(),
            ),
            AnimatedBuilder(
              animation: scroll,
              builder: (context, _) => ChatScrollNavigator(
              messages: messages,
              currentChatId: 'rail-test',
              scrollController: scroll,
              messageAnchors: messageAnchors,
              viewportHeight: 500,
              autoScrollToBottom: true,
              hasNewerDisplayHistory: false,
              loadLocatorEntries: (_, _) async => <ChatMessageLocatorPreview>[
                for (final message in messages)
                  ChatMessageLocatorPreview(
                    messageIndex: message.timestamp,
                    timestamp: message.timestamp,
                    sender: message.sender,
                    previewContent: message.displayText,
                    contentLength: message.displayText.length,
                    displayMode: 'NORMAL',
                    isFavorite: false,
                  ),
              ],
              onRequestLatestMessages: () async {},
              onAutoScrollToBottomChanged: (_) {},
              onJumpToMessageTimestamp: jumps.add,
              onJumpToMessage: jumps.add,
              onToggleFavoriteMessage: (_, _) async {},
              onRequestScrollToBottom: () {},
              showNavigatorChip: true,
              onNavigatorChipHidden: () {},
              ),
            ),
          ],
        ),
      ),
    ),
  ));
  await tester.pumpAndSettle();
}

/// Creates one ordinary transcript message for locator geometry and previews.
ChatMessage _message(int index) => ChatMessage(
  pluginExtensions: const {},
  sender: index.isEven ? 'user' : 'ai',
  parts: <MessagePart>[
    MessagePart(
      partId: '$index',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: 'message $index',
      toolCallId: null,
      toolName: null,
      attributes: const <String, String>{},
    ),
  ],
  timestamp: index,
  roleName: '',
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: '',
  modelName: '',
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  sentAt: 0,
  waitDurationMs: 0,
  outputDurationMs: 0,
  completedAt: 0,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);
