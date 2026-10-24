import 'dart:async';
import 'dart:convert';
import 'dart:ffi' hide Size;
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/compose/compose_program_buffer.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

typedef _CreateNative = Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef _Create = Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef _ConnectNative = Pointer<Utf8> Function(Pointer<Void>);
typedef _Connect = Pointer<Utf8> Function(Pointer<Void>);

/// Reports nearest-rank quantiles without adding independent phase medians.
Map<String, double> pipelineDistribution(List<double> values) {
  final sorted = [...values]..sort();
  return {
    'p50': sorted[(sorted.length * .5).ceil() - 1],
    'p95': sorted[(sorted.length * .95).ceil() - 1],
  };
}

/// Reads root lifecycle metadata directly from the measured program transaction.
Map<String, Object?> probeRootProps(ToolPkgComposeDslTransaction envelope) {
  final update = ComposeProgramBuffer(envelope.buffer).transaction;
  final source = update.instances.singleWhere(
    (value) => value.id == update.root.instanceId,
  );
  final program = update.programs.singleWhere(
    (value) => value.id == source.programId,
  );

  /// Evaluates metadata operands without projecting the measured layout tree.
  Object? evaluate(Object? raw) {
    final code = raw as List;
    switch (code[0]) {
      case 0:
        return code[1];
      case 1:
        return source.values[code[1] as int];
      case 2:
        return {
          for (var index = 0; index < (code[1] as List).length; index++)
            (code[1] as List)[index] as String: evaluate(
              (code[2] as List)[index],
            ),
        };
      case 3:
        return [for (final member in code[1] as List) evaluate(member)];
      case 4:
        return {
          '__composeRef': [source.id, code[1]],
        };
      case 5:
        return [...evaluate(code[1]) as List, ...evaluate(code[2]) as List];
      default:
        throw StateError('Unknown probe operand');
    }
  }

  var props = (evaluate(program.nodes[update.root.node].props) as Map)
      .cast<String, Object?>();
  for (final patch in source.patches) {
    if (patch.node == update.root.node && patch.field == 'props') {
      props = (patch.value as Map).cast<String, Object?>();
    }
  }
  return props;
}

/// Measures an explicitly supplied native runtime through production CoreLink and Flutter.
void main() {
  const enabled = bool.fromEnvironment('OPERIT_DSL_LOAD_PIPELINE_PROBE');
  const libraryPath = String.fromEnvironment('OPERIT_DSL_PROBE_LIBRARY');
  const currentSdk = bool.fromEnvironment('OPERIT_DSL_PROBE_CURRENT_SDK');
  testWidgets(
    'native DSL loading through retained Flutter first frame',
    (tester) async {
      expect(libraryPath, isNotEmpty);
      tester.view.physicalSize = const Size(1000, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final directory = Directory(
        '../../../target/dsl-performance/current-load',
      ).absolute;
      final runtimeDirectory = directory.createTempSync('runtime-');
      final library = DynamicLibrary.open(libraryPath);
      final create = library.lookupFunction<_CreateNative, _Create>(
        'operit_flutter_bridge_create_with_storage_roots',
      );
      final connect = library.lookupFunction<_ConnectNative, _Connect>(
        'operit_flutter_bridge_ffi_connect',
      );
      final freeString = library
          .lookupFunction<
            Void Function(Pointer<Utf8>),
            void Function(Pointer<Utf8>)
          >('operit_flutter_bridge_free_string');
      final destroy = library
          .lookupFunction<
            Void Function(Pointer<Void>),
            void Function(Pointer<Void>)
          >('operit_flutter_bridge_destroy');
      final runtimePath = '${runtimeDirectory.path}/runtime'.toNativeUtf8();
      final workspacePath = '${runtimeDirectory.path}/workspace'.toNativeUtf8();
      final bootstrapWatch = Stopwatch()..start();
      final handle = create(runtimePath, workspacePath);
      calloc.free(runtimePath);
      calloc.free(workspacePath);
      if (handle == nullptr) {
        final error = library
            .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
              'operit_flutter_bridge_create_error',
            )();
        final message = error.toDartString();
        freeString(error);
        throw StateError(message);
      }
      final bootstrapMs = bootstrapWatch.elapsedMicroseconds / 1000;
      addTearDown(() => destroy(handle));
      final transport = FfiCoreTransport();
      addTearDown(transport.close);
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(
            const MethodChannel('operit/runtime'),

            /// Provides the real library's host-owned FFI function table to the production transport.
            (call) async {
              expect(call.method, 'connectCoreFfi');
              final pointer = connect(handle);
              final descriptor = pointer.toDartString();
              freeString(pointer);
              return descriptor;
            },
          );
      addTearDown(
        () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(
              const MethodChannel('operit/runtime'),
              null,
            ),
      );
      final bridge = ProxyCoreRuntimeBridge(
        coreProxy: FfiCoreProxy(transport: transport),
      );
      final clients = GeneratedCoreProxyClients(bridge);
      final manager = clients.application.packageManager();
      var registryMs = 0.0;
      await tester.runAsync(() async {
        final watch = Stopwatch()..start();
        await manager.getToolPkgPublicApiOwners(
          apiName: 'dsl-performance-probe',
        );
        registryMs = watch.elapsedMicroseconds / 1000;
      });
      final service = clients.servicesComposeDslSessionService;
      final results = <Map<String, Object?>>[];
      for (final rows in [20, 100, 500]) {
        final rawSamples = <Map<String, double>>[];
        double? firstSampleMs;
        var nodes = 0;
        for (var index = 0; index < 10; index++) {
          await tester.pumpWidget(const SizedBox.shrink());
          final watch = Stopwatch()..start();
          final times = <String, double>{};
          final contextKey = 'dsl-pipeline-$rows-$index';
          late String sessionId;
          late CorePushSink commands;
          late StreamSubscription<ToolPkgComposeDslEvent> subscription;
          late ToolPkgComposeDslEvent event;
          late Completer<ToolPkgComposeDslEvent> response;
          await tester.runAsync(() async {
            response = Completer();
            await manager.acquireToolPkgExecutionEngine(
              contextKey: contextKey,
              containerPackageName: 'com.operit.character_cards',
            );
            times['acquireEngineMs'] = watch.elapsedMicroseconds / 1000;
            var previous = watch.elapsedMicroseconds / 1000;
            sessionId = await manager.openComposeDslSession(
              contextKey: contextKey,
              containerPackageName: 'com.operit.character_cards',
            );
            subscription = service
                .updates(sessionId: sessionId)
                .listen(
                  /// Correlates each command with its actual decoded production event.
                  (incoming) {
                    if (response.isCompleted) return;
                    if (incoming.phase == 'error' ||
                        incoming.phase == 'sessionError') {
                      response.completeError(StateError(incoming.error!));
                    } else if (incoming.phase == 'final') {
                      response.complete(incoming);
                    }
                  },
                  onError: (Object error, StackTrace stack) {
                    if (!response.isCompleted) {
                      response.completeError(error, stack);
                    }
                  },
                );
            commands = await bridge.push(
              CorePushRequest(
                requestId: contextKey,
                target: service.objectId,
                methodName: 'submit',
                args: {'sessionId': sessionId},
              ),
            );
            times['openSessionMs'] =
                watch.elapsedMicroseconds / 1000 - previous;
            previous = watch.elapsedMicroseconds / 1000;
            final script = File(
              '${directory.path}/native-selector-$rows.cjs',
            ).readAsStringSync();
            times['readScriptMs'] = watch.elapsedMicroseconds / 1000 - previous;
            previous = watch.elapsedMicroseconds / 1000;
            await commands.add(
              ToolPkgComposeDslCommand(
                requestId: contextKey,
                operation: 'render',
                script: script,
                actionId: null,
                payload: null,
                runtimeOptions: const {},
                envOverrides: const {},
              ).toJson(),
            );
            event = await response.future.timeout(const Duration(seconds: 30));
            times['renderCommandMs'] =
                watch.elapsedMicroseconds / 1000 - previous;
          });
          expect(tester.takeException(), isNull);
          final update = event.update!;
          nodes = ComposeProgramBuffer(
            update.buffer,
          ).transaction.instances.length;
          final previous = watch.elapsedMicroseconds / 1000;
          late VoidCallback dispose;
          final child = buildComposeProgramLayoutForTest(
            update: update,
            hostContext: ComposeDslWebViewHostContext(
              packageName: 'diagnostic',
              routeInstanceId: contextKey,
              executionContextKey: contextKey,

              /// Keeps interaction out of the measured first-frame path.
              dispatchAction: (id, [payload]) async => null,
              runtimeOptionsProvider: () => {},
            ),
            registerDispose: (callback) => dispose = callback,
          );
          times['ingestMs'] = watch.elapsedMicroseconds / 1000 - previous;
          final frameStart = watch.elapsedMicroseconds / 1000;
          await tester.pumpWidget(MaterialApp(home: Scaffold(body: child)));
          await tester.pump();
          times['testFrameMs'] = watch.elapsedMicroseconds / 1000 - frameStart;
          times['totalMs'] = watch.elapsedMicroseconds / 1000;
          watch.stop();
          expect(tester.takeException(), isNull);
          expect(find.text('Diagnostic group 0'), findsOneWidget);
          firstSampleMs ??= times['totalMs'];
          await tester.runAsync(() async {
            response = Completer();
            final onLoad = probeRootProps(update)['onLoad'] as Map;
            await commands.add(
              ToolPkgComposeDslCommand(
                requestId: '$contextKey-stages',
                operation: 'action',
                script: null,
                actionId: onLoad['__actionId'] as String,
                payload: const {'__no_render': true},
                runtimeOptions: const {},
                envOverrides: const {},
              ).toJson(),
            );
            final inspected = await response.future.timeout(
              const Duration(seconds: 30),
            );
            final stages = inspected.actionResult as Map;
            expect(stages['parseCalls'], 0);
            expect(stages['compileCalls'], 0);
            times['rootRenderMs'] = (stages['rootRenderMs'] as num).toDouble();
            times['retainedCommitMs'] = (stages['retainedCommitMs'] as num)
                .toDouble();
            await commands.close();
            await subscription.cancel();
            await manager.releaseToolPkgExecutionEngine(
              contextKey: contextKey,
              containerPackageName: 'com.operit.character_cards',
            );
          });
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
          dispose();
          if (index >= 2) rawSamples.add(times);
          stdout.writeln(
            'dsl_pipeline_sample rows=$rows sample=$index times=${jsonEncode(times)}',
          );
        }
        final summary = <String, Object?>{};
        for (final key in rawSamples.first.keys) {
          summary[key] = pipelineDistribution(
            rawSamples.map((sample) => sample[key]!).toList(),
          );
        }
        results.add({
          'rows': rows,
          'nodes': nodes,
          'firstSampleMs': firstSampleMs,
          'summary': summary,
          'rawSamples': rawSamples,
        });
      }
      final report = {
        'measuredAt': DateTime.now().toUtc().toIso8601String(),
        'library': libraryPath,
        'currentSdkInjected': currentSdk,
        'scope':
            'Native QuickJS + production CoreLink FFI + retained Flutter debug widget first frame; synthetic preloaded selector; excludes business queries, WebView and GPU presentation; native stage timers use 1 ms resolution',
        'sdkInstallation': currentSdk
            ? 'Checked-out SDK installed in the existing DLL; installation is included in render-command time; Rust package cache remains the DLL implementation'
            : 'SDK embedded in the supplied DLL',
        'samples': 8,
        'runtimeBootstrapMs': bootstrapMs,
        'packageRegistryMs': registryMs,
        'runtimeDirectory': runtimeDirectory.path,
        'results': results,
      };
      final output = currentSdk
          ? 'native-flutter-current-sdk.json'
          : 'native-flutter-pipeline.json';
      File(
        '${directory.path}/$output',
      ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(report));
      stdout.writeln(const JsonEncoder.withIndent('  ').convert(report));
    },
    skip: !enabled,
    timeout: const Timeout(Duration(minutes: 5)),
  );
}
