import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

const _mixed =
    '# **标题**\n\n'
    '这是中文与 English 混合的正文，包含 **粗体 *斜体***、`code`、~~删除线~~ '
    '和 [链接](https://example.com/a_(b))。\n\n'
    '- **列表项目**\n  - 第二层项目\n3) 有序项目\n\n'
    '> **引用**\n> - 引用中的列表\n\n'
    '```rust\nlet value = 42;\n```\n\n'
    '| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |\n\n';

String _source(int bytes) {
  final pattern = utf8.encode(_mixed);
  final result = <int>[];
  while (result.length <= bytes) {
    result.addAll(pattern);
  }
  var end = bytes;
  while ((result[end] & 0xc0) == 0x80) {
    end--;
  }
  return utf8.decode(result.sublist(0, end));
}

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'production Markdown FFI and node pipeline',
    (tester) async {
      const path = String.fromEnvironment('OPERIT_MARKDOWN_BENCH_LIBRARY');
      expect(
        path,
        isNotEmpty,
        reason: 'Build the release native fixture first',
      );
      final library = DynamicLibrary.open(path);
      final connect = library
          .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
            'markdown_bench_connect',
          );
      final free = library
          .lookupFunction<
            Void Function(Pointer<Utf8>),
            void Function(Pointer<Utf8>)
          >('markdown_bench_free_descriptor');
      binding.defaultBinaryMessenger.setMockMethodCallHandler(
        const MethodChannel('operit/runtime'),
        (call) async {
          expect(call.method, 'connectCoreFfi');
          final pointer = connect();
          try {
            return pointer.toDartString();
          } finally {
            free(pointer);
          }
        },
      );
      final transport = FfiCoreTransport();
      final bridge = ProxyCoreRuntimeBridge(
        coreProxy: FfiCoreProxy(transport: transport),
      );
      addTearDown(() async {
        await transport.close();
        binding.defaultBinaryMessenger.setMockMethodCallHandler(
          const MethodChannel('operit/runtime'),
          null,
        );
      });
      var sequence = 0;
      Stream<MarkdownStreamEvent> open(
        String source,
        int chars, {
        bool instrument = false,
      }) {
        return bridge.openEmbeddedCoreStream<MarkdownStreamEvent>(
          'markdown-benchmark-${sequence++}',
          'markdown-benchmark',
          'contentStream',
          {'source': source, 'chunkChars': chars, 'instrument': instrument},
          MarkdownStreamEvent.fromMessagePack,
        );
      }

      Future<Map<String, Object?>> metrics() async {
        return Map<String, Object?>.from(
          await bridge.call(
                CoreCallRequest(
                  requestId: 'metrics-${sequence++}',
                  target: 'markdown-benchmark',
                  methodName: 'metrics',
                  args: const {},
                ),
              )
              as Map,
        );
      }

      Future<Map<String, Object?>> consume(
        String source,
        int chars, {
        bool instrument = false,
      }) async {
        final state = StreamMarkdownRendererState();
        var events = 0;
        final watch = Stopwatch()..start();
        await for (final event in open(source, chars, instrument: instrument)) {
          state.applyEvent(event);
          events++;
        }
        // Transport Completed is consumed by the production embedded stream.
        // The renderer completes its builder from the stream's onDone callback.
        state.eventBuilder.complete();
        final nodes = state.eventBuilder.toStableNodes(isStreaming: false);
        watch.stop();
        final elapsed = watch.elapsedMicroseconds / 1000;
        // Assertions and metrics requests are outside the timed region.
        expect(state.collectedContent.toString(), source);
        expect(nodes, isNotEmpty);
        final native = await metrics();
        expect(
          events,
          (native['events'] as int) - 1,
          reason: 'Only root transport Completed is consumed by the bridge',
        );
        final result = <String, Object?>{
          'wallMs': elapsed,
          'events': events,
          'rootNodes': nodes.length,
          'native': native,
        };
        state.reset();
        return result;
      }

      final reports = <Map<String, Object?>>[];
      await tester.runAsync(() => consume(_mixed, 128));
      for (final (bytes, chars) in [
        (10 * 1024, 128),
        (100 * 1024, 128),
        (1024 * 1024, 128),
        (100 * 1024, 1),
        (100 * 1024, 32),
        (100 * 1024, 1024),
      ]) {
        final source = _source(bytes);
        final samples = <double>[];
        await tester.runAsync(() => consume(source, chars));
        Map<String, Object?>? last;
        for (var i = 0; i < 5; i++) {
          last = await tester.runAsync(() => consume(source, chars));
          samples.add(last!['wallMs'] as double);
        }
        samples.sort();
        // Separate instrumented pass: per-event clocks do not affect headlines.
        // Native and Dart execute concurrently, so stage times must not be summed.
        final diagnostic = await tester.runAsync(
          () => consume(source, chars, instrument: true),
        );
        reports.add({
          'bytes': utf8.encode(source).length,
          'chunkChars': chars,
          'medianWallMs': samples[samples.length ~/ 2],
          'samplesMs': samples,
          'events': last!['events'],
          'rootNodes': last['rootNodes'],
          'diagnostic': diagnostic,
        });
        debugPrint('MARKDOWN_PIPELINE_SAMPLE ${jsonEncode(reports.last)}');
      }

      // Exercise the actual widget and nested quote/table consumers as a separate
      // correctness check. This does not measure painting or frame rate.
      final done = Completer<void>();
      final state = StreamMarkdownRendererState();
      final stream = open(_mixed, 32);
      Future<List<MarkdownStreamEvent>> split(String source) =>
          GeneratedCoreProxyClients(
            bridge,
          ).chatRuntimeHolderMain.splitMarkdownContent(content: source);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(
              body: StreamMarkdownRenderer(
                content: '',
                isStreaming: true,
                contentStream: stream,
                state: state,
                textColor: Colors.black,
                backgroundColor: Colors.white,
                splitMarkdownContent: split,
                onStreamDone: () {
                  if (!done.isCompleted) done.complete();
                },
                onStreamError: () {
                  if (!done.isCompleted) {
                    done.completeError(StateError('Render stream failed'));
                  }
                },
              ),
            ),
          ),
        ),
      );
      await tester.runAsync(
        () => done.future.timeout(const Duration(seconds: 30)),
      );
      await tester.pumpAndSettle();
      expect(state.collectedContent.toString(), _mixed);
      expect(tester.takeException(), isNull);
      expect(find.textContaining('粗体', findRichText: true), findsWidgets);
      await tester.pumpWidget(const SizedBox());
      binding.reportData = {
        'mode': kProfileMode ? 'profile' : (kReleaseMode ? 'release' : 'debug'),
        'nativeMode': 'release',
        'platform': Platform.operatingSystem,
        'scope':
            'fixed native chunk source -> production CoreValue/BridgeStreams/'
            'NativeFrame/DartPort/FfiCoreTransport/FfiCoreProxy/embedded stream/'
            'typed MessagePack decoder -> root node builder + final snapshot',
        'excludes': [
          'model/network',
          'root node snapshot refresh during streaming',
          'nested widget consumers and paint (checked separately, not timed)',
        ],
        'samplesPerCase': 5,
        'samples': reports,
      };
      debugPrint('MARKDOWN_PIPELINE_RESULT ${jsonEncode(binding.reportData)}');
    },
    timeout: const Timeout(Duration(minutes: 10)),
  );
}
