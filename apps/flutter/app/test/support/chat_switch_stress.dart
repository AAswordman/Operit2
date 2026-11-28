import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/ChatArea.dart';
import 'package:operit2/ui/features/chat/components/ChatMessageExtent.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

const _paragraph =
    '这是超长会话压力测试正文，包含中文与 English、**加粗文字**、'
    '`inline code` 和普通段落。用于检查历史总量是否拖慢当前窗口。\n\n';

/// Creates deterministic Markdown with distinct message identities and bounded paragraphs.
String _content(int chars, String identity) {
  final text = StringBuffer('Message $identity\n\n');
  while (text.length < chars) {
    text.write(_paragraph);
  }
  return '${text.toString().substring(0, chars)}\n\nEND-$identity';
}

/// Produces the canonical archive schema consumed by the production importer.
Map<String, Object?> _archivedMessage(int index, String content) => {
  'baseMessage': {
    'sender': index.isEven ? 'user' : 'ai',
    'parts': [
      {
        'partId': 'body',
        'sequence': 0,
        'kind': 'markdown',
        'content': content,
        'toolCallId': null,
        'toolName': null,
        'attributes': <String, String>{},
      },
    ],
    'timestamp': 1700000000000 + index,
    'roleName': '',
    'pluginExtensions': <String, Object?>{},
    'selectedVariantIndex': 0,
    'variantCount': 1,
    'provider': '',
    'modelName': '',
    'inputTokens': 0,
    'outputTokens': 0,
    'cachedInputTokens': 0,
    'sentAt': 0,
    'outputDurationMs': 0,
    'waitDurationMs': 0,
    'completedAt': 1700000000000 + index,
    'displayMode': 'NORMAL',
    'isFavorite': false,
    'contentStream': null,
  },
  'variants': <Object?>[],
};

/// Owns only benchmark storage and connects the production native runtime and transport.
class _NativeRuntime {
  _NativeRuntime(
    this.library,
    this.handle,
    this.transport,
    this.clients,
    this.root,
  );
  final DynamicLibrary library;
  final Pointer<Void> handle;
  final FfiCoreTransport transport;
  final GeneratedCoreProxyClients clients;
  final Directory root;

  /// Initializes a fresh database without consulting the user's application storage.
  static Future<_NativeRuntime> open(String libraryPath) async {
    final root = await Directory(
      'build/chat_switch_stress',
    ).absolute.create(recursive: true);
    final run = await root.createTemp('run-');
    final library = DynamicLibrary.open(libraryPath);
    final create = library
        .lookupFunction<
          Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>),
          Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>)
        >('operit_flutter_bridge_create_with_storage_roots');
    final runtime = '${run.path}/runtime'.toNativeUtf8();
    final workspace = '${run.path}/workspace'.toNativeUtf8();
    late final Pointer<Void> handle;
    try {
      handle = create(runtime, workspace);
    } finally {
      malloc.free(runtime);
      malloc.free(workspace);
    }
    final free = library
        .lookupFunction<
          Void Function(Pointer<Utf8>),
          void Function(Pointer<Utf8>)
        >('operit_flutter_bridge_free_string');
    if (handle == nullptr) {
      final error = library
          .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
            'operit_flutter_bridge_create_error',
          )();
      final message = error.toDartString();
      free(error);
      throw StateError(message);
    }
    final connect = library
        .lookupFunction<
          Pointer<Utf8> Function(Pointer<Void>),
          Pointer<Utf8> Function(Pointer<Void>)
        >('operit_flutter_bridge_ffi_connect');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), (
          call,
        ) async {
          if (call.method == 'connectCoreFfi') {
            final descriptor = connect(handle);
            try {
              return descriptor.toDartString();
            } finally {
              free(descriptor);
            }
          }
          throw StateError('Unexpected benchmark host call: ${call.method}');
        });
    final transport = FfiCoreTransport();
    final clients = GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(coreProxy: FfiCoreProxy(transport: transport)),
    );
    await ClientLogger.initialize();
    return _NativeRuntime(library, handle, transport, clients, run);
  }

  /// Imports test records through the actual archive service outside measured regions.
  Future<Map<String, Object?>> seed(
    String id,
    int count,
    int chars,
    int tailChars,
  ) async {
    var totalChars = 0;
    final messages = List.generate(count, (index) {
      final content = _content(
        index == count - 1 ? tailChars : chars,
        '$id-$index',
      );
      totalChars += content.length;
      return _archivedMessage(index, content);
    });
    final archive = jsonEncode({
      'archiveType': 'operit_chat_archive',
      'formatVersion': 5,
      'exportedAt': 1700000000000,
      'workspaces': <Object?>[],
      'chats': [
        {
          'id': id,
          'title': 'Stress $id',
          'group': null,
          'pluginExtensions': <String, Object?>{},
          'messages': messages,
          'createdAt': '2026-11-28T00:00:00',
          'updatedAt': '2026-11-28T00:00:00',
          'inputTokens': 0,
          'outputTokens': 0,
          'currentWindowSize': 0,
          'displayOrder': 0,
          'workspaceId': null,
          'parentChatId': null,
          'locked': false,
          'pinned': false,
        },
      ],
    });
    final result = await clients.chatRuntimeHolderMain
        .importChatHistoriesFromJson(jsonString: archive);
    debugPrint('CHAT_STRESS_SEEDED $id ${result.toJson()}');
    return {
      'id': id,
      'messages': count,
      'messageChars': chars,
      'tailChars': tailChars,
      'totalChars': totalChars,
      'archiveBytes': utf8.encode(archive).length,
    };
  }

  /// Closes transport ownership before releasing the runtime's original host reference.
  Future<void> close() async {
    await transport.close();
    library.lookupFunction<
      Void Function(Pointer<Void>),
      void Function(Pointer<Void>)
    >('operit_flutter_bridge_destroy')(handle);
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), null);
  }
}

/// Runs the same Core/FFI/widget workload from either the headless or profile runner.
Future<Map<String, Object?>> runChatSwitchStress(
  WidgetTester tester,
  String libraryPath,
) async {
  final native = (await tester.runAsync(
    () => _NativeRuntime.open(libraryPath),
  ))!;
  final data = ValueNotifier<(String, List<ChatMessage>)>(('empty', []));
  final scroll = ScrollController();
  final autoScroll = ValueNotifier(true);
  final pending = <Object>{};
  final parseSamples = <Map<String, Object?>>[];
  final reports = <Map<String, Object?>>[];
  final frameTimings = <FrameTiming>[];

  /// Collects engine build and raster durations separately from total asynchronous wall time.
  void collectTimings(List<FrameTiming> timings) =>
      frameTimings.addAll(timings);
  SchedulerBinding.instance.addTimingsCallback(collectTimings);
  final clients = native.clients;
  Uri? serviceUri;
  if (const bool.fromEnvironment('OPERIT_CHAT_STRESS_CPU_PROFILE')) {
    serviceUri = (await tester.runAsync(
      () => developer.Service.controlWebServer(enable: true),
    ))!.serverUri;
  }
  final cases = [
    ('small', 20, 2000, 2000),
    ('history-200', 200, 2000, 2000),
    ('history-2000', 2000, 2000, 2000),
    ('history-10000', 10000, 2000, 2000),
    ('dense-history', 2000, 20000, 20000),
    ('tail-100k', 2000, 2000, 100000),
    ('tail-500k', 2000, 2000, 500000),
  ];

  /// Measures the real native parser and generated response decoder for each mounted surface.
  Future<List<MarkdownStreamEvent>> split(String source) async {
    final timer = Stopwatch()..start();
    final events = await clients.chatRuntimeHolderMain.splitMarkdownContent(
      content: source,
    );
    parseSamples.add({
      'chars': source.length,
      'events': events.length,
      'wallMs': timer.elapsedMicroseconds / 1000,
    });
    return events;
  }

  try {
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
            body: NotificationListener<MarkdownLayoutNotification>(
              onNotification: (event) {
                if (event.isPending) {
                  pending.add(event.source);
                } else {
                  pending.remove(event.source);
                }
                return false;
              },
              child: ValueListenableBuilder<(String, List<ChatMessage>)>(
                valueListenable: data,
                builder: (context, value, _) => ChatArea(
                  messages: value.$2,
                  currentChatId: value.$1,
                  isLoading: false,
                  errorMessage: null,
                  scrollController: scroll,
                  identityAvatarUri: null,
                  clients: clients,
                  packageManager: clients.application.packageManager(),
                  autoScrollToBottomListenable: autoScroll,
                  hasOlderDisplayHistory: true,
                  hasNewerDisplayHistory: false,
                  isLoadingDisplayWindow: false,
                  loadLocatorEntries: (_, _) async => [],
                  onRevealMessageForLocator: (_) async => false,
                  onAutoScrollToBottomChanged: (value) =>
                      autoScroll.value = value,
                  onLoadOlderDisplayWindow: () async {},
                  onLoadNewerDisplayWindow: () async {},
                  onShowLatestDisplayWindow: () async {},
                  onToggleFavoriteMessage: (_, _) async {},
                  onDeleteMessage: (_) async {},
                  onDeleteMessagesFrom: (_) async => true,
                  onDeleteMessageVariant: (_, _) async {},
                  onSelectMessageVariant: (_, _) async {},
                  onRollbackToMessage: (_) {},
                  onSelectMessageToEdit: (_) {},
                  onRegenerateMessage: (_) async {},
                  onInsertSummary: (_) {},
                  onCreateBranch: (_) async {},
                  onReplyToMessage: (_) {},
                  onPlayVoice: (_) async {},
                  onToggleMultiSelectMode: (_) {},
                  onToggleMessageSelection: (_) {},
                  onRefreshRequested: () async {},
                  bottomContentInset: 0,
                  splitMarkdownContent: split,
                ),
              ),
            ),
          ),
        ),
      ),
    );
    for (final (id, count, chars, tailChars) in cases) {
      const filter = String.fromEnvironment('OPERIT_CHAT_STRESS_CASE');
      if (filter.isNotEmpty && filter != id) continue;
      final fixture = (await tester.runAsync(
        () => native.seed(id, count, chars, tailChars),
      ))!;
      final samples = <Map<String, Object?>>[];
      for (var repeat = 0; repeat < 3; repeat++) {
        data.value = ('empty', []);
        await tester.pump();
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 50)),
        );
        pending.clear();
        parseSamples.clear();
        frameTimings.clear();
        final timer = Stopwatch()..start();
        final profileStart = developer.Timeline.now;
        late double commandMs;
        late double receivedMs;
        late List<ChatMessage> messages;
        StreamSubscription<List<ChatMessage>>? windowSubscription;
        List<ChatMessage>? receivedWindow;
        Object? receiveError;
        await tester.runAsync(() async {
          stdout.writeln('CHAT_STRESS_OPEN $id repeat=$repeat');
          await clients.chatRuntimeHolderMain
              .switchChat(chatId: id)
              .timeout(const Duration(seconds: 30));
          commandMs = timer.elapsedMicroseconds / 1000;
          stdout.writeln('CHAT_STRESS_SWITCHED $id $commandMs');
          windowSubscription = clients.chatRuntimeHolderMain
              .chatMessagesFlow(chatId: id)
              .listen(
                (value) {
                  stdout.writeln(
                    'CHAT_STRESS_WINDOW $id count=${value.length}',
                  );
                  if (value.isNotEmpty && receivedWindow == null) {
                    receivedWindow = value;
                    receivedMs = timer.elapsedMicroseconds / 1000;
                  }
                },
                onError: (Object error, StackTrace stack) {
                  receiveError = error;
                },
              );
        });
        while (receivedWindow == null) {
          await tester.pump();
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 2)),
          );
          if (receiveError != null) throw receiveError!;
          if (timer.elapsed > const Duration(seconds: 30)) {
            throw TimeoutException('Message window: $id');
          }
        }
        messages = receivedWindow!;
        stdout.writeln('CHAT_STRESS_RECEIVED $id $receivedMs');
        data.value = (id, messages);
        var readyFrames = 0;
        var frames = 0;
        double? firstContentFrameMs;
        while (readyFrames < 3) {
          await tester.pump();
          final exception = tester.takeException();
          if (exception != null) throw exception;
          frames++;
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 2)),
          );
          final renderers = find
              .byType(StreamMarkdownRenderer, skipOffstage: false)
              .evaluate();
          final rows = tester.widgetList<ChatMessageExtentBox>(
            find.byType(ChatMessageExtentBox, skipOffstage: false),
          );
          final ready =
              renderers.isNotEmpty &&
              rows.every((row) => !row.extent.hasPendingContent) &&
              renderers.every((element) {
                final widget = element.widget as StreamMarkdownRenderer;
                return widget.state?.streamParsingCompletedSuccessfully == true;
              });
          readyFrames = ready ? readyFrames + 1 : 0;
          if (frames % 1000 == 0) {
            stdout.writeln(
              'CHAT_STRESS_PENDING $id frames=$frames '
              'renderers=${renderers.length} rows=${rows.length} '
              'parses=${parseSamples.length}',
            );
          }
          if (ready) firstContentFrameMs ??= timer.elapsedMicroseconds / 1000;
          if (timer.elapsed > const Duration(seconds: 120)) {
            throw TimeoutException(
              'First content layout: $id pending=${pending.length} renderers=${renderers.length}',
            );
          }
        }
        final totalMs = timer.elapsedMicroseconds / 1000;
        expect(tester.takeException(), isNull);
        expect(messages.last.timestamp, 1700000000000 + count - 1);
        expect(
          find.textContaining(
            'END-$id-${count - 1}',
            findRichText: true,
            skipOffstage: false,
          ),
          findsWidgets,
        );
        final extents = tester
            .widgetList<ChatMessageExtentBox>(
              find.byType(ChatMessageExtentBox, skipOffstage: false),
            )
            .length;
        final sample = <String, Object?>{
          'repeat': repeat,
          'cache': repeat == 0 ? 'first-open' : 'reopen',
          'switchCommandMs': commandMs,
          'messagesReceivedMs': receivedMs,
          'contentLayoutAndPaintMs': totalMs,
          'clientAfterReceiveMs': totalMs - receivedMs,
          'firstContentFrameMs': firstContentFrameMs,
          'windowMessages': messages.length,
          'mountedRows': extents,
          'frames': frames,
          'nativeParseCalls': List.of(parseSamples),
          'engineFrames': [
            for (final frame in frameTimings)
              {
                'buildMs': frame.buildDuration.inMicroseconds / 1000,
                'rasterMs': frame.rasterDuration.inMicroseconds / 1000,
              },
          ],
        };
        samples.add(sample);
        debugPrint('CHAT_STRESS_SAMPLE ${jsonEncode({'case': id, ...sample})}');
        if (serviceUri != null) {
          final profileEnd = developer.Timeline.now;
          await tester.runAsync(() async {
            final http = HttpClient();
            try {
              final base = serviceUri!;
              final uri = base
                  .replace(
                    path:
                        '${base.path.endsWith('/') ? base.path : '${base.path}/'}getCpuSamples',
                  )
                  .replace(
                    queryParameters: {
                      'isolateId': developer.Service.getIsolateId(
                        Isolate.current,
                      )!,
                      'timeOriginMicros': '$profileStart',
                      'timeExtentMicros': '${profileEnd - profileStart}',
                    },
                  );
              final response = await (await http.getUrl(uri)).close();
              final profile = await utf8.decoder.bind(response).join();
              await File(
                '${native.root.parent.path}/cpu-$id-$repeat.json',
              ).writeAsString(profile);
            } finally {
              http.close(force: true);
            }
          });
        }
        await tester.runAsync(() => windowSubscription!.cancel());
      }
      reports.add({...fixture, 'samples': samples});
      final partial = {
        'mode': kProfileMode ? 'profile' : 'debug-headless',
        'runtimeRoot': native.root.path,
        'cases': reports,
      };
      await tester.runAsync(
        () => File('${native.root.parent.path}/latest.json').writeAsString(
          '${const JsonEncoder.withIndent('  ').convert(partial)}\n',
        ),
      );
    }
    return {
      'mode': kProfileMode ? 'profile' : 'debug-headless',
      'runtimeRoot': native.root.path,
      'cases': reports,
    };
  } finally {
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 100)),
    );
    data.dispose();
    scroll.dispose();
    autoScroll.dispose();
    SchedulerBinding.instance.removeTimingsCallback(collectTimings);
    await tester.runAsync(native.close);
  }
}
