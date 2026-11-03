import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/interactions/ChainedScrollController.dart';

const _innerKey = ValueKey('inner');

Future<void> _mount(
  WidgetTester tester, {
  required ScrollController outer,
  required ScrollController inner,
  ScrollController? middle,
  bool reverseOuter = false,
  bool scoped = false,
  double innerHeight = 600,
  ScrollPhysics? physics,
}) async {
  Widget content = SizedBox(
    height: 160,
    child: SingleChildScrollView(
      key: _innerKey,
      controller: inner,
      physics: physics,
      child: SizedBox(
        height: innerHeight,
        child: const Text('Message content'),
      ),
    ),
  );
  if (middle != null) {
    content = SizedBox(
      height: 220,
      child: SingleChildScrollView(
        controller: middle,
        child: Column(children: [const SizedBox(height: 500), content]),
      ),
    );
  }
  await tester.pumpWidget(
    MaterialApp(
      theme: scoped ? ThemeData(platform: TargetPlatform.macOS) : null,
      home: Scaffold(
        body: Builder(
          builder: (context) {
            final viewport = Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 400,
                height: 300,
                child: SingleChildScrollView(
                  controller: outer,
                  reverse: reverseOuter,
                  physics: const ClampingScrollPhysics(),
                  child: Column(
                    children: reverseOuter
                        ? [const SizedBox(height: 1000), content]
                        : [content, const SizedBox(height: 1000)],
                  ),
                ),
              ),
            );
            return scoped ? ChainedScrollScope(child: viewport) : viewport;
          },
        ),
      ),
    ),
  );
}

Future<void> _wheel(WidgetTester tester, double delta) async {
  await tester.sendEventToBinding(
    PointerScrollEvent(
      position: tester.getCenter(find.byKey(_innerKey)),
      scrollDelta: Offset(0, delta),
    ),
  );
  await tester.pump();
}

void main() {
  late ScrollController outer;
  late ChainedScrollController inner;

  setUp(() {
    outer = ScrollController();
    inner = ChainedScrollController();
  });

  tearDown(() {
    outer.dispose();
    inner.dispose();
  });

  testWidgets(
    'wheel stays inside until the inner edge, then passes remainder',
    (tester) async {
      await _mount(tester, outer: outer, inner: inner);
      await _wheel(tester, 60);
      expect(inner.offset, 60);
      expect(outer.offset, 0);

      inner.jumpTo(inner.position.maxScrollExtent - 20);
      await _wheel(tester, 80);
      expect(inner.offset, inner.position.maxScrollExtent);
      expect(outer.offset, 60);

      await _wheel(tester, 30);
      expect(outer.offset, 90);
    },
  );

  testWidgets('upward remainder reaches parent and reversing scrolls inner', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    outer.jumpTo(80);
    inner.jumpTo(15);
    await _wheel(tester, -50);
    expect(inner.offset, 0);
    expect(outer.offset, 45);
    await _wheel(tester, 30);
    expect(inner.offset, 30);
    expect(outer.offset, 45);
  });

  testWidgets('one continuous touch drag crosses the edge without lifting', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(inner.position.maxScrollExtent - 20);
    final gesture = await tester.startGesture(
      tester.getCenter(find.byKey(_innerKey)),
    );
    await gesture.moveBy(const Offset(0, -20));
    await gesture.moveBy(const Offset(0, -70));
    await tester.pump();
    expect(inner.offset, inner.position.maxScrollExtent);
    expect(outer.offset, greaterThan(0));
    final previousOuter = outer.offset;
    await gesture.moveBy(const Offset(0, -30));
    await tester.pump();
    expect(outer.offset, closeTo(previousOuter + 30, 0.01));
    await gesture.cancel();
    await tester.pumpAndSettle();
  });

  testWidgets('trackpad pan continues in parent after inner reaches edge', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(inner.position.maxScrollExtent - 10);
    final gesture = await tester.createGesture(
      kind: PointerDeviceKind.trackpad,
    );
    final point = tester.getCenter(find.byKey(_innerKey));
    await gesture.panZoomStart(point);
    await gesture.panZoomUpdate(point, pan: const Offset(0, -20));
    await gesture.panZoomUpdate(point, pan: const Offset(0, -80));
    await tester.pump();
    expect(inner.offset, inner.position.maxScrollExtent);
    expect(outer.offset, greaterThan(0));
    await gesture.panZoomEnd();
    await tester.pumpAndSettle();
  });

  testWidgets('remainder crosses multiple capped levels exactly once', (
    tester,
  ) async {
    final middle = ChainedScrollController();
    addTearDown(middle.dispose);
    await _mount(tester, outer: outer, inner: inner, middle: middle);
    middle.jumpTo(middle.position.maxScrollExtent - 15);
    inner.jumpTo(inner.position.maxScrollExtent - 10);
    await _wheel(tester, 80);
    expect(inner.offset, inner.position.maxScrollExtent);
    expect(middle.offset, middle.position.maxScrollExtent);
    expect(outer.offset, 55);
  });

  testWidgets(
    'scope preserves each remainder across three ordinary controllers',
    (tester) async {
      final plainInner = ScrollController();
      final middle = ScrollController();
      addTearDown(plainInner.dispose);
      addTearDown(middle.dispose);
      await _mount(
        tester,
        outer: outer,
        inner: plainInner,
        middle: middle,
        scoped: true,
      );
      middle.jumpTo(middle.position.maxScrollExtent - 15);
      plainInner.jumpTo(plainInner.position.maxScrollExtent - 10);
      await _wheel(tester, 80);
      expect(plainInner.offset, plainInner.position.maxScrollExtent);
      expect(middle.offset, middle.position.maxScrollExtent);
      expect(outer.offset, 55);
    },
  );

  testWidgets('scope handles a continuous three-level trackpad gesture', (
    tester,
  ) async {
    final plainInner = ScrollController();
    final middle = ScrollController();
    addTearDown(plainInner.dispose);
    addTearDown(middle.dispose);
    await _mount(
      tester,
      outer: outer,
      inner: plainInner,
      middle: middle,
      scoped: true,
    );
    middle.jumpTo(middle.position.maxScrollExtent - 15);
    plainInner.jumpTo(plainInner.position.maxScrollExtent - 10);
    final gesture = await tester.createGesture(
      kind: PointerDeviceKind.trackpad,
    );
    final point = tester.getCenter(find.byKey(_innerKey));
    await gesture.panZoomStart(point);
    await gesture.panZoomUpdate(point, pan: const Offset(0, -20));
    await gesture.panZoomUpdate(point, pan: const Offset(0, -100));
    await tester.pump();
    expect(plainInner.offset, plainInner.position.maxScrollExtent);
    expect(middle.offset, middle.position.maxScrollExtent);
    expect(outer.offset, greaterThan(0));
    final before = outer.offset;
    await gesture.panZoomUpdate(point, pan: const Offset(0, -130));
    await tester.pump();
    expect(outer.offset, closeTo(before + 30, 0.01));
    await gesture.panZoomEnd();
    await tester.pumpAndSettle();
  });

  testWidgets('scope respects a reversed parent for ordinary controllers', (
    tester,
  ) async {
    final plainInner = ScrollController();
    addTearDown(plainInner.dispose);
    await _mount(
      tester,
      outer: outer,
      inner: plainInner,
      scoped: true,
      reverseOuter: true,
    );
    plainInner.jumpTo(10);
    await _wheel(tester, -50);
    expect(plainInner.offset, 0);
    expect(outer.offset, 40);
  });

  testWidgets('scope leaves programmatic inner animation inside its viewport', (
    tester,
  ) async {
    final plainInner = ScrollController();
    addTearDown(plainInner.dispose);
    await _mount(tester, outer: outer, inner: plainInner, scoped: true);
    final animation = plainInner.animateTo(
      plainInner.position.maxScrollExtent + 100,
      duration: const Duration(milliseconds: 100),
      curve: Curves.linear,
    );
    await tester.pumpAndSettle();
    await animation;
    expect(outer.offset, 0);
  });

  testWidgets(
    'scope forwards momentum when an ordinary inner fling hits its edge',
    (tester) async {
      final plainInner = ScrollController();
      addTearDown(plainInner.dispose);
      await _mount(tester, outer: outer, inner: plainInner, scoped: true);
      plainInner.jumpTo(plainInner.position.maxScrollExtent - 100);
      await tester.fling(find.byKey(_innerKey), const Offset(0, -50), 800);
      expect(outer.offset, 0);
      await tester.pumpAndSettle();
      expect(plainInner.offset, plainInner.position.maxScrollExtent);
      expect(outer.offset, greaterThan(0));
    },
  );

  testWidgets('scope continues a fling through three ordinary viewports', (
    tester,
  ) async {
    final plainInner = ScrollController();
    final middle = ScrollController();
    addTearDown(plainInner.dispose);
    addTearDown(middle.dispose);
    await _mount(
      tester,
      outer: outer,
      inner: plainInner,
      middle: middle,
      scoped: true,
    );
    middle.jumpTo(middle.position.maxScrollExtent - 80);
    plainInner.jumpTo(plainInner.position.maxScrollExtent - 100);
    await tester.flingFrom(
      tester.getTopLeft(find.byKey(_innerKey)) + const Offset(100, 30),
      const Offset(0, -50),
      1200,
    );
    await tester.pumpAndSettle();
    expect(plainInner.offset, plainInner.position.maxScrollExtent);
    expect(middle.offset, middle.position.maxScrollExtent);
    expect(outer.offset, greaterThan(50));
  });

  testWidgets('handoff respects a reversed parent axis', (tester) async {
    await _mount(tester, outer: outer, inner: inner, reverseOuter: true);
    inner.jumpTo(10);
    await _wheel(tester, -50);
    expect(inner.offset, 0);
    expect(outer.offset, 40);
  });

  testWidgets('short inner content allows parent wheel scrolling', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner, innerHeight: 40);
    await _wheel(tester, 50);
    expect(inner.offset, 0);
    expect(outer.offset, 50);
  });

  testWidgets('programmatic positioning never moves the parent', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(inner.position.maxScrollExtent);
    final animation = inner.animateTo(
      0,
      duration: const Duration(milliseconds: 100),
      curve: Curves.linear,
    );
    await tester.pumpAndSettle();
    await animation;
    expect(outer.offset, 0);
  });

  testWidgets('new inner wheel input stops an earlier parent fling', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(100);
    (outer.position as ScrollPositionWithSingleContext).goBallistic(800);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 30));
    final before = outer.offset;
    expect(before, greaterThan(0));
    await _wheel(tester, -10);
    expect(inner.offset, 90);
    await tester.pump(const Duration(milliseconds: 100));
    expect(outer.offset, before);
  });

  testWidgets('a fling at the inner edge keeps moving the parent', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(inner.position.maxScrollExtent);
    await tester.fling(find.byKey(_innerKey), const Offset(0, -60), 800);
    final afterRelease = outer.offset;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(outer.offset, greaterThan(afterRelease));
    expect(inner.offset, inner.position.maxScrollExtent);
    await tester.pumpAndSettle();
  });

  testWidgets('a fling reaching the inner edge transfers its momentum', (
    tester,
  ) async {
    await _mount(tester, outer: outer, inner: inner);
    inner.jumpTo(inner.position.maxScrollExtent - 100);
    await tester.fling(find.byKey(_innerKey), const Offset(0, -50), 800);
    expect(outer.offset, 0);
    await tester.pumpAndSettle();
    expect(inner.offset, inner.position.maxScrollExtent);
    expect(outer.offset, greaterThan(0));
  });

  testWidgets(
    'handoff clamps inner content even with bouncing platform physics',
    (tester) async {
      await _mount(
        tester,
        outer: outer,
        inner: inner,
        physics: const BouncingScrollPhysics(),
      );
      inner.jumpTo(inner.position.maxScrollExtent - 10);
      final gesture = await tester.startGesture(
        tester.getCenter(find.byKey(_innerKey)),
      );
      await gesture.moveBy(const Offset(0, -20));
      await gesture.moveBy(const Offset(0, -70));
      await tester.pump();
      expect(inner.offset, inner.position.maxScrollExtent);
      expect(outer.offset, greaterThan(0));
      await gesture.cancel();
      await tester.pumpAndSettle();
    },
  );
}
