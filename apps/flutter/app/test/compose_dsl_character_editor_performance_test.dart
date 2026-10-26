import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Reads exact production compiler transactions prepared by the Node editor benchmark.
core.ToolPkgComposeDslTransaction transaction(int length, int tagCount, String name) =>
    core.ToolPkgComposeDslTransaction(buffer: File(
      '../../../target/dsl-performance/character-editor/$length-$tagCount-$name.cpb',
    ).readAsBytesSync());

/// Summarizes warm test-frame wall times while excluding fixture IO.
Map<String, double> distribution(List<double> samples) {
  final sorted = [...samples]..sort();
  return {
    'p50': sorted[(sorted.length * .5).ceil() - 1],
    'p95': sorted[(sorted.length * .95).ceil() - 1],
  };
}

/// Measures real editor mount and retained input updates in Flutter's rendering pipeline.
void main() {
  const enabled = bool.fromEnvironment('OPERIT_CHARACTER_EDITOR_PROBE');
  testWidgets('production character editor input performance', (tester) async {
    tester.view.physicalSize = const Size(1100, 950);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final reports = <Map<String, Object?>>[];
    for (final (length, tagCount) in [(100, 2), (10000, 2), (100000, 2), (100, 100), (100, 500)]) {
      final initial = transaction(length, tagCount, 'initial');
      final updates = List.generate(25, (index) => transaction(length, tagCount, 'input-$index'));
      late VoidCallback dispose;
      late void Function(core.ToolPkgComposeDslTransaction) apply;
      var rebuilds = 0;
      debugOnRebuildDirtyWidget = (element, builtOnce) { rebuilds++; };
      addTearDown(() => debugOnRebuildDirtyWidget = null);
      final watch = Stopwatch()..start();
      final child = buildComposeProgramLayoutForTest(
        update: initial,
        hostContext: ComposeDslWebViewHostContext(
          packageName: 'com.operit.character_cards',
          routeInstanceId: 'editor-probe',
          executionContextKey: 'editor-probe',
          dispatchAction: (id, [payload]) async => null,
          runtimeOptionsProvider: () => {},
        ),
        registerApply: (callback) => apply = callback,
        registerDispose: (callback) => dispose = callback,
      );
      await tester.pumpWidget(MaterialApp(home: Scaffold(body: child)));
      await tester.pumpAndSettle();
      final mountMs = watch.elapsedMicroseconds / 1000;
      expect(tester.takeException(), isNull);
      final editor = tester.state(find.byType(EditableText).first);
      final samples = <double>[];
      final counts = <int>[];
      for (var index = 0; index < updates.length; index++) {
        rebuilds = 0;
        watch.reset();
        apply(updates[index]);
        await tester.pump();
        await tester.pump();
        final ms = watch.elapsedMicroseconds / 1000;
        expect(tester.takeException(), isNull);
        expect(tester.state(find.byType(EditableText).first), same(editor));
        expect(find.text('编辑角色名称$index'), findsOneWidget);
        if (index >= 5) { samples.add(ms); counts.add(rebuilds); }
      }
      final contentSamples = <double>[];
      for (var index = 0; index < 15; index++) {
        final update = transaction(length, tagCount, 'content-$index');
        watch.reset();
        apply(update);
        await tester.pump();
        await tester.pump();
        final ms = watch.elapsedMicroseconds / 1000;
        expect(tester.takeException(), isNull);
        if (index >= 5) contentSamples.add(ms);
      }
      final tabs = <Map<String, Object?>>[];
      for (final tab in [1, 2, 0]) {
        final update = transaction(length, tagCount, 'tab-$tab');
        watch.reset();
        apply(update);
        await tester.pumpAndSettle();
        final ms = watch.elapsedMicroseconds / 1000;
        expect(tester.takeException(), isNull);
        tabs.add({'tab': tab, 'ms': ms});
      }
      reports.add({ 'length': length, 'tagCount': tagCount, 'mountMs': mountMs, 'inputMs': distribution(samples), 'contentMs': distribution(contentSamples), 'tabs': tabs, 'rebuiltWidgets': counts });
      debugOnRebuildDirtyWidget = null;
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      dispose();
    }
    final report = { 'scope': 'Flutter debug widget test; real program store/build/layout/paint, excludes JS/CoreLink/GPU presentation', 'reports': reports };
    File('../../../target/dsl-performance/character-editor/flutter.json').writeAsStringSync(const JsonEncoder.withIndent('  ').convert(report));
    stdout.writeln(const JsonEncoder.withIndent('  ').convert(report));
  }, skip: !enabled);
}

