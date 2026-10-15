import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/compose_dsl/lazy_viewport.dart';

/// Builds a real lazy list with item sizes that cannot be inferred from DSL props.
Widget _list(
  ScrollController controller,
  List<Size> sizes, {
  bool reverse = false,
}) => ComposeDslLazyListView(
  controller: controller,
  scrollDirection: Axis.horizontal,
  reverse: reverse,
  shrinkWrap: false,
  childrenDelegate: SliverChildBuilderDelegate(
    (context, index) => ComposeDslLazyListItem(
      key: ValueKey(index),
      axis: Axis.horizontal,
      alignment: Alignment.topLeft,
      child: SizedBox.fromSize(size: sizes[index], child: Text('Item $index')),
    ),
    childCount: sizes.length,
  ),
);

/// Places a horizontal viewport in a form-like column with no height constraint.
Widget _harness(Widget child) => MaterialApp(
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: 180,
        child: Column(mainAxisSize: MainAxisSize.min, children: [child]),
      ),
    ),
  ),
);

/// Verifies natural viewport geometry without depending on runtime or theme modules.
void main() {
  for (final axis in Axis.values) {
    for (final reverse in [false, true]) {
      for (final shrinkWrap in [false, true]) {
        testWidgets(
          'resizes retained $axis items with reverse=$reverse, shrinkWrap=$shrinkWrap',
          (tester) async {
            final controller = ScrollController();
            addTearDown(controller.dispose);
            // Reuse the widget so a rebuild cannot hide missing render invalidation.
            final list = ComposeDslLazyListView(
              controller: controller,
              scrollDirection: axis,
              reverse: reverse,
              shrinkWrap: shrinkWrap,
              childrenDelegate: SliverChildBuilderDelegate(
                (context, index) => ComposeDslLazyListItem(
                  key: ValueKey(index),
                  axis: axis,
                  alignment: Alignment.topLeft,
                  child: LayoutBuilder(
                    builder: (context, constraints) => SizedBox(
                      width: axis == Axis.vertical
                          ? constraints.maxWidth / 2
                          : 100,
                      height: axis == Axis.horizontal
                          ? constraints.maxHeight / 2
                          : 100,
                      child: _RememberingItem(index: index),
                    ),
                  ),
                ),
                childCount: 20,
              ),
            );
            for (final crossExtent in [300.0, 500.0, 180.0, 400.0]) {
              await tester.pumpWidget(
                MaterialApp(
                  home: Scaffold(
                    body: Align(
                      alignment: Alignment.topLeft,
                      child: SizedBox(
                        width: axis == Axis.vertical ? crossExtent : 390,
                        height: axis == Axis.horizontal ? crossExtent : 390,
                        child: list,
                      ),
                    ),
                  ),
                ),
              );
              await tester.pumpAndSettle();
              expect(tester.takeException(), isNull);
              if (crossExtent == 300) {
                await tester.tap(find.text('Count 0: 0'));
                await tester.pump();
              }
              final item = find.widgetWithText(TextButton, 'Count 0: 1');
              expect(item, findsOneWidget);
              final size = tester.getSize(item);
              expect(
                axis == Axis.vertical ? size.width : size.height,
                crossExtent / 2,
              );
              expect(controller.offset, 0);
              expect(controller.position.viewportDimension, 390);
            }
          },
        );
      }
    }
  }

  testWidgets('retains a horizontal list across bounded and natural sizing', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final list = _list(controller, [const Size(40, 30)]);
    for (final height in [90.0, null, 40.0, null]) {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 180,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [SizedBox(height: height, child: list)],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        tester.getSize(find.byType(ComposeDslLazyListView)),
        Size(180, height ?? 30),
      );
      expect(tester.getSize(find.text('Item 0')).height, 30);
    }
  });

  testWidgets('measures real children and updates the same controller', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    for (final height in [42.0, 80.0, 24.0]) {
      await tester.pumpWidget(
        _harness(_list(controller, [const Size(40, 18), Size(40, height)])),
      );
      expect(tester.takeException(), isNull);
      expect(
        tester.getSize(find.byType(ComposeDslLazyListView)),
        Size(180, height),
      );
      expect(controller.position.viewportDimension, 180);
      expect(tester.getSize(find.text('Item 0')).height, 18);
    }
  });

  testWidgets('fills a finite cross axis like Flutter ListView', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 180,
            height: 90,
            child: _list(controller, [const Size(40, 30)]),
          ),
        ),
      ),
    );
    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byType(ComposeDslLazyListView)),
      const Size(180, 90),
    );
    expect(tester.getTopLeft(find.text('Item 0')).dy, 0);
  });

  for (final reverse in [false, true]) {
    testWidgets('preserves lazy scrolling with reverse=$reverse', (
      tester,
    ) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final sizes = List.generate(
        200,
        (index) => Size(80, index == 199 ? 42 : 20),
      );
      await tester.pumpWidget(
        _harness(_list(controller, sizes, reverse: reverse)),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Item 0'), findsOneWidget);
      expect(find.text('Item 199'), findsNothing);
      expect(find.byType(Text).evaluate().length, lessThan(15));
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 20);
      controller.jumpTo(controller.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Item 199'), findsOneWidget);
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 42);
      controller.jumpTo(0);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 20);
    });
  }

  testWidgets(
    'keeps offscreen item state without retaining its measured height',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final list = ComposeDslLazyListView(
        controller: controller,
        scrollDirection: Axis.horizontal,
        reverse: false,
        shrinkWrap: false,
        childrenDelegate: SliverChildBuilderDelegate(
          (context, index) => ComposeDslLazyListItem(
            key: ValueKey(index),
            axis: Axis.horizontal,
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 100,
              height: index == 0 ? 90 : 40,
              child: _RememberingItem(index: index),
            ),
          ),
          childCount: 200,
        ),
      );
      await tester.pumpWidget(_harness(list));
      await tester.tap(find.text('Count 0: 0'));
      await tester.pump();
      expect(find.text('Count 0: 1'), findsOneWidget);
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 90);
      controller.jumpTo(controller.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 40);
      controller.jumpTo(0);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Count 0: 1'), findsOneWidget);
      expect(tester.getSize(find.byType(ComposeDslLazyListView)).height, 90);
    },
  );
}

/// Supplies a stateful, retained item to exercise the sliver keep-alive protocol.
class _RememberingItem extends StatefulWidget {
  /// Creates the counter belonging to one stable list item.
  const _RememberingItem({required this.index});

  final int index;

  /// Creates the state that must survive scrolling out of the viewport.
  @override
  State<_RememberingItem> createState() => _RememberingItemState();
}

class _RememberingItemState extends State<_RememberingItem>
    with AutomaticKeepAliveClientMixin {
  int _count = 0;

  /// Requests retention while its render box is outside the active cache.
  @override
  bool get wantKeepAlive => true;

  /// Verifies hit testing through the measured item and updates retained state.
  @override
  Widget build(BuildContext context) {
    super.build(context);
    return TextButton(
      onPressed: () => setState(() => _count++),
      child: Text('Count ${widget.index}: $_count'),
    );
  }
}
