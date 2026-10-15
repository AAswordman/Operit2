import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

core.ToolPkgComposeDslNodeRecord record(
  String id,
  String type,
  Map<String, Object?> props, [
  List<String> children = const [],
]) => core.ToolPkgComposeDslNodeRecord(
  id: id,
  nodeType: type,
  props: props,
  children: children,
  slots: const {},
);

void main() {
  testWidgets('retained Box updates cached hit order and child alignment', (
    tester,
  ) async {
    final actions = <String>[];
    late void Function(core.ToolPkgComposeDslNodeUpdate) apply;
    late VoidCallback dispose;
    var revision = 1;
    core.ToolPkgComposeDslNodeRecord tile(String id, int z, [String? align]) =>
        record(
          id,
          'Box',
          {
            'width': 80,
            'height': 80,
            'zIndex': z,
            'align': ?align,
            'background': '#dddddd',
            'modifier': {
              '__modifierOps': [
                {
                  'name': 'clickable',
                  'args': [
                    {'__actionId': id},
                  ],
                },
              ],
            },
          },
          ['$id-label'],
        );
    final root = record(
      'root',
      'Box',
      {'width': 200, 'height': 200},
      ['a', 'b'],
    );
    final child = buildRetainedComposeDslLayoutForTest(
      update: core.ToolPkgComposeDslNodeUpdate(
        reset: true,
        revision: revision,
        rootId: 'root',
        upserts: [
          root,
          tile('a', 0),
          tile('b', 1),
          record('a-label', 'Text', {'text': 'A'}),
          record('b-label', 'Text', {'text': 'B'}),
        ],
        removed: const [],
      ),
      hostContext: ComposeDslWebViewHostContext(
        packageName: 'layout-test',
        routeInstanceId: 'page',
        executionContextKey: 'page',
        dispatchAction: (id, [payload]) async {
          actions.add(id);
          return null;
        },
        runtimeOptionsProvider: () => {},
      ),
      registerApply: (callback) => apply = callback,
      registerDispose: (callback) => dispose = callback,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(alignment: Alignment.topLeft, child: child),
        ),
      ),
    );
    final origin = tester.getTopLeft(find.text('A'));
    final overlapping = origin + const Offset(30, 30);
    await tester.tapAt(overlapping);
    expect(actions, ['b']);
    for (final z in [2, -1, 3]) {
      apply(
        core.ToolPkgComposeDslNodeUpdate(
          reset: false,
          revision: ++revision,
          rootId: 'root',
          upserts: [tile('a', z)],
          removed: const [],
        ),
      );
      await tester.pump();
      await tester.tapAt(overlapping);
      expect(actions.last, z > 1 ? 'a' : 'b');
    }
    apply(
      core.ToolPkgComposeDslNodeUpdate(
        reset: false,
        revision: ++revision,
        rootId: 'root',
        upserts: [tile('a', 3, 'bottomEnd')],
        removed: const [],
      ),
    );
    await tester.pump();
    expect(tester.getTopLeft(find.text('A')) - origin, const Offset(120, 120));
    await tester.tapAt(overlapping);
    expect(actions.last, 'b');
    await tester.tapAt(origin + const Offset(150, 150));
    expect(actions.last, 'a');
    apply(
      core.ToolPkgComposeDslNodeUpdate(
        reset: false,
        revision: ++revision,
        rootId: 'root',
        upserts: [
          record('root', 'Box', root.props, ['a']),
        ],
        removed: const ['b', 'b-label'],
      ),
    );
    await tester.pump();
    expect(find.text('B'), findsNothing);
    await tester.tapAt(origin + const Offset(150, 150));
    expect(actions.last, 'a');
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
    dispose();
  });
}
