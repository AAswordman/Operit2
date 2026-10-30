import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

List<Map<String, Object?>> flatten(Map<String, Object?> snapshot) {
  final result = <Map<String, Object?>>[];
  void visit(Map<String, Object?> node) {
    result.add(node);
    for (final child in node['children'] as List? ?? []) {
      visit(child as Map<String, Object?>);
    }
  }

  for (final node in (snapshot['document'] as Map)['scenes'] as List) {
    visit(node as Map<String, Object?>);
  }
  return result;
}

String target(Map<String, Object?> node) =>
    (node['source'] as Map)['runtimeTarget'] as String;

Future<Map<String, Object?>> snapshot(
  WidgetTester tester,
  OperitDesignRuntime runtime,
) async {
  await tester.pump();
  await tester.pump();
  return (await tester.runAsync(runtime.snapshot))!;
}

void main() {
  testWidgets(
    'runtime events navigate, edit real input and preserve controller and design across rebuilds',
    (tester) async {
      final boundary = GlobalKey();
      final controller = TextEditingController();
      addTearDown(controller.dispose);
      int clicks = 0;
      String changed = '';
      late StateSetter rebuild;
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: MaterialApp(
            home: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Scaffold(
                  body: Column(
                    children: [
                      OperitDesignSource(
                        source: const {'designId': 'persistent-label'},
                        child: Text('Count $clicks'),
                      ),
                      TextField(
                        controller: controller,
                        inputFormatters: [LengthLimitingTextInputFormatter(5)],
                        onChanged: (value) => changed = value,
                      ),
                      TextButton(
                        onPressed: () => setState(() => clicks++),
                        child: const Text('Increment'),
                      ),
                      TextButton(
                        onPressed: () => Navigator.of(context).push(
                          MaterialPageRoute<void>(
                            builder: (_) => Scaffold(
                              body: TextButton(
                                onPressed: () => Navigator.of(context).pop(),
                                child: const Text('Return'),
                              ),
                            ),
                          ),
                        ),
                        child: const Text('Open page'),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final runtime = OperitDesignRuntime(
        root: () =>
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary,
      );
      addTearDown(runtime.dispose);
      var all = flatten(await snapshot(tester, runtime));
      final label = all.singleWhere((n) => n['text'] == 'Count 0');
      final labelId = target(label);
      runtime.edit(labelId, {'fontSize': 26, 'fill': '#FF2200FF'});
      runtime.dispatch({
        'type': 'tap',
        'target': target(all.singleWhere((n) => n['text'] == 'Increment')),
      });
      await tester.pumpAndSettle();
      expect(clicks, 1);
      all = flatten(await snapshot(tester, runtime));
      var count = all.singleWhere((n) => n['text'] == 'Count 1');
      expect(count['fontSize'], 26);
      expect(count['fill'], '#FF2200FF');
      final input = all.singleWhere(
        (n) => (n['source'] as Map)['runtimeInput'] == true,
      );
      runtime.dispatch({
        'type': 'input',
        'target': target(input),
        'text': 'abcdef',
      });
      await tester.pumpAndSettle();
      expect(controller.text, 'abcde');
      expect(changed, 'abcde');
      all = flatten(await snapshot(tester, runtime));
      final openId = target(all.singleWhere((n) => n['text'] == 'Open page'));
      runtime.dispatch({'type': 'tap', 'target': openId});
      await tester.pumpAndSettle();
      all = flatten(await snapshot(tester, runtime));
      expect(all.any((n) => n['text'] == 'Return'), isTrue);
      expect(
        () => runtime.dispatch({'type': 'tap', 'target': labelId}),
        throwsStateError,
      );
      runtime.dispatch({
        'type': 'tap',
        'target': target(all.singleWhere((n) => n['text'] == 'Return')),
      });
      await tester.pumpAndSettle();
      rebuild(() => clicks++);
      all = flatten(await snapshot(tester, runtime));
      count = all.singleWhere((n) => n['text'] == 'Count 2');
      expect(count['fontSize'], 26);
      expect(controller.text, 'abcde');
      runtime.clear(target(count));
      all = flatten(await snapshot(tester, runtime));
      count = all.singleWhere((n) => n['text'] == 'Count 2');
      expect(count['fontSize'], isNot(26));
      expect(count['fill'], isNot('#FF2200FF'));
      expect(tester.takeException(), isNull);
    },
    skip: !operitDesignBridgeEnabled,
  );

  testWidgets(
    'explicit source edits survive remount; constraints change real layout and undo restores it',
    (tester) async {
      final boundary = GlobalKey();
      bool visible = true;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: RepaintBoundary(
            key: boundary,
            child: Scaffold(
              body: StatefulBuilder(
                builder: (context, setState) {
                  rebuild = setState;
                  return visible
                      ? const OperitDesignSource(
                          source: {'designId': 'remount-card'},
                          child: SizedBox(
                            width: 300,
                            height: 90,
                            child: DecoratedBox(
                              decoration: BoxDecoration(color: Colors.blue),
                              child: Padding(
                                padding: EdgeInsets.all(8),
                                child: Text('Source label'),
                              ),
                            ),
                          ),
                        )
                      : const SizedBox.shrink();
                },
              ),
            ),
          ),
        ),
      );
      final runtime = OperitDesignRuntime(
        root: () =>
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary,
      );
      addTearDown(runtime.dispose);
      var all = flatten(await snapshot(tester, runtime));
      final sized = all.singleWhere(
        (n) => n['name'] == 'SizedBox' && n['width'] == 300,
      );
      final label = all.singleWhere((n) => n['text'] == 'Source label');
      runtime.edit(target(sized), {'width': 180});
      runtime.edit(target(label), {
        'text': 'Changed in design',
        'fontSize': 22,
      });
      all = flatten(await snapshot(tester, runtime));
      expect(
        all.singleWhere(
          (n) => n['name'] == 'SizedBox' && n['height'] == 90,
        )['width'],
        180,
      );
      expect(all.any((n) => n['text'] == 'Changed in design'), isTrue);
      rebuild(() => visible = false);
      await tester.pump();
      rebuild(() => visible = true);
      all = flatten(await snapshot(tester, runtime));
      expect(all.any((n) => n['text'] == 'Changed in design'), isTrue);
      expect(
        all.singleWhere(
          (n) => n['name'] == 'SizedBox' && n['height'] == 90,
        )['width'],
        180,
      );
      final remounted = all.singleWhere(
        (n) => n['text'] == 'Changed in design',
      );
      expect((remounted['source'] as Map)['runtimePersistence'], 'source');
      expect(target(remounted), isNot(target(label)));
      expect(
        () => runtime.edit(target(remounted), {'fontSize': double.nan}),
        throwsArgumentError,
      );
      expect(
        () => runtime.edit(target(remounted), {'width': 100}),
        throwsArgumentError,
      );
      expect(
        () => runtime.dispatch({
          'type': 'tap',
          'target': target(remounted),
          'x': -1,
        }),
        throwsArgumentError,
      );
      runtime.clear();
      all = flatten(await snapshot(tester, runtime));
      expect(all.any((n) => n['text'] == 'Source label'), isTrue);
      expect(
        all.singleWhere(
          (n) => n['name'] == 'SizedBox' && n['height'] == 90,
        )['width'],
        300,
      );
      expect(tester.takeException(), isNull);
    },
    skip: !operitDesignBridgeEnabled,
  );
}
