import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/ChatMessageExtent.dart';

/// Verifies persistent row distances and asynchronous layout reservations.
void main() {
  test('keeps measured heights independent from changing row estimates', () {
    final extent = ChatMessageExtent();
    addTearDown(extent.dispose);
    expect(extent.estimateHeight(600, 120), 120);
    expect(extent.estimateHeight(600, 20), 20);
    extent.recordHeight(600, 340);
    expect(extent.estimateHeight(600, 20), 340);
    extent.recordHeight(600, 80);
    expect(extent.estimateHeight(600, 340), 80);
  });

  test('does not count an asynchronous placeholder as measured content', () {
    final extent = ChatMessageExtent();
    addTearDown(extent.dispose);
    final source = Object();
    extent.updatePendingContent(source, isPending: true);
    extent.recordHeight(600, 20);
    expect(extent.estimateHeight(600, 400), 400);
    extent.updatePendingContent(source, isPending: false);
    extent.recordHeight(600, 360);
    expect(extent.estimateHeight(600, 20), 360);
  });

  test('updates cached separator distance when the last row changes', () {
    final extent = ChatMessageExtent();
    addTearDown(extent.dispose);
    extent.recordHeight(600, 340);
    extent.updateTrailingSpacing(8);
    expect(extent.estimateHeight(600, 120), 348);
    extent.updateTrailingSpacing(8);
    expect(extent.estimateHeight(600, 120), 348);
    extent.updateTrailingSpacing(0);
    expect(extent.estimateHeight(600, 120), 340);
  });

  test('invalidates measurements when the row width changes', () {
    final extent = ChatMessageExtent();
    addTearDown(extent.dispose);
    extent.recordHeight(600, 340);
    expect(extent.reservedHeight(400), 0);
    expect(extent.estimateHeight(400, 180), 180);
    extent.recordHeight(400, 460);
    expect(extent.estimateHeight(400, 180), 460);
  });

  test(
    'tracks independent pending descendants without releasing space early',
    () {
      final extent = ChatMessageExtent();
      addTearDown(extent.dispose);
      final first = Object();
      final second = Object();
      var notifications = 0;
      extent.addListener(() => notifications++);
      extent.recordHeight(600, 340);
      extent.updatePendingContent(first, isPending: true);
      extent.updatePendingContent(second, isPending: true);
      extent.recordHeight(600, 20);
      expect(extent.reservedHeight(600), 340);
      extent.updatePendingContent(first, isPending: false);
      expect(extent.hasPendingContent, isTrue);
      extent.recordHeight(600, 40);
      expect(extent.reservedHeight(600), 340);
      extent.updatePendingContent(second, isPending: false);
      expect(extent.hasPendingContent, isFalse);
      extent.recordHeight(600, 40);
      expect(extent.reservedHeight(600), 40);
      expect(notifications, 2);
    },
  );

  testWidgets('reserves a remounted row until its real content is ready', (
    tester,
  ) async {
    final extent = ChatMessageExtent();
    addTearDown(extent.dispose);
    final source = Object();

    /// Recreates the content subtree while retaining only its height record.
    Widget row(
      int generation, {
      required double height,
      required bool pending,
    }) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            child: observeChatMessageExtent(
              key: ValueKey(generation),
              extent: extent,
              child: _PendingExtentContent(
                source: source,
                height: height,
                pending: pending,
              ),
            ),
          ),
        ),
      );
    }

    await tester.pumpWidget(row(0, height: 320, pending: false));
    expect(tester.getSize(find.byType(ChatMessageExtentBox)).height, 320);
    await tester.pumpWidget(row(1, height: 20, pending: true));
    expect(tester.getSize(find.byType(ChatMessageExtentBox)).height, 320);
    expect(extent.reservedHeight(400), 320);
    await tester.pumpWidget(row(1, height: 180, pending: false));
    expect(tester.getSize(find.byType(ChatMessageExtentBox)).height, 180);
    expect(extent.reservedHeight(400), 180);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'uses measured tail distances instead of the visible row average',
    (tester) async {
      final extents = List.generate(30, (_) => ChatMessageExtent());
      final scroll = ScrollController();
      addTearDown(scroll.dispose);
      addTearDown(() {
        for (final extent in extents) {
          extent.dispose();
        }
      });
      final heights = List.generate(
        30,
        (index) => index % 3 == 0 ? 420.0 : 40.0,
      );
      for (var index = 0; index < extents.length; index++) {
        extents[index].recordHeight(800, heights[index]);
      }
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: scroll,
            physics: const ClampingScrollPhysics(),
            slivers: [
              ChatMessageSliverList(
                extents: extents,
                preserveViewport: () => true,
                itemBuilder: (context, index) => observeChatMessageExtent(
                  key: ValueKey(index),
                  extent: extents[index],
                  child: SizedBox(height: heights[index]),
                ),
              ),
            ],
          ),
        ),
      );
      final expectedBottom =
          heights.reduce((sum, height) => sum + height) - 600;
      expect(scroll.position.maxScrollExtent, expectedBottom);
      for (final offset in [600.0, 1600.0, 3400.0, 1200.0, 0.0]) {
        scroll.jumpTo(offset);
        await tester.pump();
        expect(scroll.position.maxScrollExtent, expectedBottom);
      }
      expect(find.byType(ChatMessageExtentBox).evaluate().length, lessThan(15));
      await tester.pumpWidget(const SizedBox());
    },
  );
  for (final direction in GrowthDirection.values) {
    testWidgets('preserves the active drag across height changes in $direction', (
      tester,
    ) async {
      const centerKey = ValueKey<String>('extent-center');
      final extents = List.generate(12, (_) => ChatMessageExtent());
      final heights = ValueNotifier<List<double>>(List.filled(12, 100));
      final scroll = ScrollController();
      addTearDown(heights.dispose);
      addTearDown(scroll.dispose);
      addTearDown(() {
        for (final extent in extents) {
          extent.dispose();
        }
      });
      for (final extent in extents) {
        extent.recordHeight(800, 100);
      }

      /// Rebuilds row sizes without replacing their measurements or scroll owner.
      Widget measuredList(Key key, List<double> sizes) => ChatMessageSliverList(
        key: key,
        extents: extents,
        preserveViewport: () => true,
        itemBuilder: (context, index) => observeChatMessageExtent(
          key: ValueKey<int>(index),
          extent: extents[index],
          child: SizedBox(height: sizes[index]),
        ),
      );

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: ValueListenableBuilder<List<double>>(
            valueListenable: heights,
            builder: (context, sizes, child) => CustomScrollView(
              controller: scroll,
              center: centerKey,
              physics: const ClampingScrollPhysics(),
              slivers: direction == GrowthDirection.forward
                  ? [measuredList(centerKey, sizes)]
                  : [
                      measuredList(
                        const ValueKey<String>('extent-history'),
                        sizes,
                      ),
                      const SliverToBoxAdapter(key: centerKey),
                    ],
            ),
          ),
        ),
      );
      scroll.jumpTo(direction == GrowthDirection.forward ? 260 : -730);
      await tester.pump();
      final target = find.byKey(const ValueKey<int>(2));
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(CustomScrollView)),
      );
      await gesture.moveBy(const Offset(0, 50));
      await tester.pump();
      await gesture.moveBy(const Offset(0, 50));
      await tester.pump();
      final targetTop = tester.getTopLeft(target).dy;
      final dragActivity = scroll.position.activity;
      expect(dragActivity, isA<DragScrollActivity>());
      for (final firstHeight in [160.0, 60.0]) {
        heights.value = [firstHeight, ...heights.value.skip(1)];
        await tester.pump();
        expect(tester.getTopLeft(target).dy, closeTo(targetTop, 1));
        expect(scroll.position.activity, same(dragActivity));
      }
      await gesture.moveBy(const Offset(0, 50));
      await tester.pump();
      final dragOffset = scroll.offset;
      await gesture.moveBy(const Offset(0, 50));
      await tester.pump();
      expect(scroll.offset, closeTo(dragOffset - 50, 1));
      await gesture.up();
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    });
  }
}

/// Emits the same preparation lifecycle as a Markdown surface without parsing.
class _PendingExtentContent extends StatelessWidget {
  /// Configures one asynchronous descendant's layout state.
  const _PendingExtentContent({
    required this.source,
    required this.height,
    required this.pending,
  });

  final Object source;
  final double height;
  final bool pending;

  /// Announces preparation before the containing row performs its layout.
  @override
  Widget build(BuildContext context) {
    MarkdownLayoutNotification(
      source: source,
      isPending: pending,
    ).dispatch(context);
    return SizedBox(height: height);
  }
}
