import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:operit2/core/compose/compose_program_buffer.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Summarizes measured test-frame wall times without counting fixture decoding.
Map<String, double> distribution(List<double> values) {
  final sorted = [...values]..sort();
  return {
    'p50': sorted[(sorted.length * .5).ceil() - 1],
    'p95': sorted[(sorted.length * .95).ceil() - 1],
  };
}

/// Measures real program-node ingestion and Flutter's build/layout/paint test pipeline.
void main() {
  const enabled = bool.fromEnvironment('OPERIT_DSL_FIRST_FRAME_PROBE');
  testWidgets('production selector program first frame probe', (tester) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final results = <Map<String, Object?>>[];
    for (final rows in [20, 100, 500]) {
      final update = ToolPkgComposeDslTransaction(
        buffer: File(
          '../../../target/dsl-performance/first-render-$rows.cpb',
        ).readAsBytesSync(),
      );
      final ingest = <double>[], frame = <double>[], total = <double>[];
      double? first;
      for (var sample = 0; sample < 18; sample++) {
        await tester.pumpWidget(const SizedBox.shrink());
        late VoidCallback dispose;
        final watch = Stopwatch()..start();
        final child = buildComposeProgramLayoutForTest(
          update: update,
          hostContext: ComposeDslWebViewHostContext(
            packageName: 'diagnostic',
            routeInstanceId: 'diagnostic',
            executionContextKey: 'diagnostic',
            dispatchAction: (id, [payload]) async => null,
            runtimeOptionsProvider: () => {},
          ),
          registerDispose: (callback) => dispose = callback,
        );
        final ingestMs = watch.elapsedMicroseconds / 1000;
        await tester.pumpWidget(MaterialApp(home: Scaffold(body: child)));
        // The production Dialog inserts its route in a post-frame callback.
        await tester.pump();
        final totalMs = watch.elapsedMicroseconds / 1000;
        watch.stop();
        expect(tester.takeException(), isNull);
        expect(find.text('Diagnostic group 0'), findsOneWidget);
        first ??= totalMs;
        if (sample >= 3) {
          ingest.add(ingestMs);
          frame.add(totalMs - ingestMs);
          total.add(totalMs);
        }
        await tester.pumpWidget(const SizedBox.shrink());
        dispose();
      }
      results.add({
        'rows': rows,
        'instances': ComposeProgramBuffer(
          update.buffer,
        ).transaction.instances.length,
        'firstSampleMs': first,
        'ingestMs': distribution(ingest),
        'testFrameMs': distribution(frame),
        'totalMs': distribution(total),
      });
    }
    final report = {
      'measuredAt': DateTime.now().toUtc().toIso8601String(),
      'scope':
          'Flutter debug widget test, production program store + renderer; no GPU presentation, CoreLink or JS',
      'samples': 15,
      'results': results,
    };
    File(
      '../../../target/dsl-performance/first-render-flutter.json',
    ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(report));
    stdout.writeln(const JsonEncoder.withIndent('  ').convert(report));
  }, skip: !enabled);
}
