import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/OperitDialog.dart';

const _lastItemKey = ValueKey('last-dialog-item');

Widget _content() => Column(
  mainAxisSize: MainAxisSize.min,
  children: <Widget>[
    for (var index = 0; index < 12; index++)
      SizedBox(
        height: 48,
        child: Text('Item $index', key: index == 11 ? _lastItemKey : null),
      ),
  ],
);

Widget _app(Widget content, {double? maxHeight}) => MaterialApp(
  home: Scaffold(
    body: OperitDialogScaffold(
      title: 'Dialog title',
      maxHeight: maxHeight,
      actions: <Widget>[
        TextButton(onPressed: () {}, child: const Text('Cancel')),
        FilledButton(onPressed: () {}, child: const Text('Save')),
      ],
      child: content,
    ),
  ),
);

void main() {
  for (final alreadyScrollable in <bool>[false, true]) {
    testWidgets('wrap-content dialog scrolls in a short window '
        '(existing scroll view: $alreadyScrollable)', (tester) async {
      await tester.binding.setSurfaceSize(const Size(800, 420));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      var content = _content();
      if (alreadyScrollable) {
        content = SingleChildScrollView(child: content);
      }

      await tester.pumpWidget(_app(content));
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(find.text('Dialog title').hitTestable(), findsOneWidget);
      expect(find.text('Save').hitTestable(), findsOneWidget);
      await tester.drag(
        find.byType(SingleChildScrollView).first,
        const Offset(0, -600),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(_lastItemKey).hitTestable(), findsOneWidget);
      expect(find.text('Save').hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('wrap-content dialog adapts when the keyboard opens', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    addTearDown(tester.view.resetViewInsets);
    await tester.pumpWidget(_app(_content()));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    tester.view.viewInsets = FakeViewPadding(
      bottom: 400 * tester.view.devicePixelRatio,
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Save').hitTestable(), findsOneWidget);
    await tester.drag(
      find.byType(SingleChildScrollView).first,
      const Offset(0, -600),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(_lastItemKey).hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('small content keeps its natural dialog height', (tester) async {
    await tester.binding.setSurfaceSize(const Size(800, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(_app(const SizedBox(height: 40)));
    await tester.pumpAndSettle();

    final material = find.descendant(
      of: find.byType(Dialog),
      matching: find.byType(Material),
    );
    expect(tester.getSize(material.last).height, lessThan(300));
    expect(tester.takeException(), isNull);
  });

  testWidgets('bounded dialog keeps expanded content constrained', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      _app(
        Column(
          children: <Widget>[
            const Text('Fixed header'),
            Expanded(child: ListView(children: <Widget>[_content()])),
          ],
        ),
        maxHeight: 500,
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Save').hitTestable(), findsOneWidget);
    expect(find.byType(SingleChildScrollView), findsNothing);
  });
}
