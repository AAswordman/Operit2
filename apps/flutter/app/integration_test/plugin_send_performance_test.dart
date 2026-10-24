import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/common/ChatMessageIdentity.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

/// Exercises the production native engine, public API and SQL bridge with fresh
/// storage. Durations are diagnostic; correctness never depends on machine speed.
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'container main realm survives repeated send preparation',
    (tester) async {
      final support = await getApplicationSupportDirectory();
      final parent = await Directory(
        '${support.path}/plugin-send-performance',
      ).create(recursive: true);
      final directory = await parent.createTemp('run-');
      // ignore: avoid_print
      print('PLUGIN_SEND_RUNTIME ${directory.path}');
      await platformCoreProxy.setRuntimeStorageRoots(
        '${directory.path}/runtime',
        '${directory.path}/workspace',
      );
      await ClientLogger.initialize();
      RuntimeHostInteractionSubscriber.install();
      addTearDown(RuntimeHostInteractionSubscriber.uninstall);
      const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
      final manager = clients.application.packageManager();
      const package = 'com.operit.character_cards';
      await manager.getToolPkgPublicApiOwners(
        apiName: 'chat.configuration.resolve',
      );
      final model = await clients.preferencesFunctionalConfigManager
          .getModelBindingForFunction(functionType: FunctionType.chat);
      final tts = await clients.preferences.ttsConfigManager
          .getCurrentTtsConfigId();
      final samples = <Map<String, Object?>>[];
      Future<Map<String, Object?>> resolve(String selection, int sample) async {
        final watch = Stopwatch()..start();
        final result = Map<String, Object?>.from(
          await manager.invokeToolPkgPublicApi(
                packageName: package,
                methodName: 'chat.configuration.resolve',
                payload: {
                  'purpose': 'execution',
                  'chatId': null,
                  'chatExtension': {'version': 1, 'selection': selection},
                  'participantId': null,
                  'promptFunctionType': 'CHAT',
                  'defaultModelBinding': {
                    'providerId': model.providerId,
                    'modelId': model.modelId,
                  },
                  'defaultTtsConfigId': tts,
                },
              )
              as Map,
        );
        samples.add({
          'stage': 'resolve:$selection',
          'sample': sample,
          'ms': watch.elapsedMicroseconds / 1000,
        });
        expect(result['executionData'], isA<Map>());
        return result;
      }

      // The first resolution measures the first use of the character main realm.
      await resolve('model:chat', -1);
      final card = Map<String, Object?>.from(
        await manager.invokeToolPkgPublicApi(
              packageName: package,
              methodName: 'character.create',
              payload: {
                'values': {
                  'name': 'Send performance role',
                  'characterSetting': 'Reply briefly.',
                },
              },
            )
            as Map,
      );
      for (var sample = 0; sample < 6; sample++) {
        await resolve('model:chat', sample);
        await resolve('card:${card['id']}', sample);
      }
      // A local OpenAI protocol fixture measures the complete production send path
      // without credentials, network variance or a live model's generation time.
      final runtimeTraceFile = File(
        '${directory.path}/runtime/logs/operit.log',
      );
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      addTearDown(() => server.close(force: true));
      Stopwatch? sendWatch;
      Completer<double>? requestBoundary;
      int? requestTraceEnd;
      server.listen((request) async {
        final body = await utf8.decoder.bind(request).join();
        final payload = jsonDecode(body) as Map<String, Object?>;
        if (payload['model'] == 'send-probe' &&
            body.contains('isolated-send-probe-') &&
            requestBoundary != null &&
            !requestBoundary.isCompleted) {
          final receivedMs = sendWatch!.elapsedMicroseconds / 1000;
          requestTraceEnd = await runtimeTraceFile.length();
          requestBoundary.complete(receivedMs);
        }
        request.response.headers.contentType = ContentType(
          'text',
          'event-stream',
        );
        request.response.write(
          'data: ${jsonEncode({
            'id': 'fixture',
            'object': 'chat.completion.chunk',
            'created': 1,
            'model': 'send-probe',
            'choices': [
              {
                'index': 0,
                'delta': {'role': 'assistant', 'content': 'fixture reply'},
                'finish_reason': null,
              },
            ],
          })}\n\n',
        );
        request.response.write(
          'data: ${jsonEncode({
            'id': 'fixture',
            'object': 'chat.completion.chunk',
            'created': 1,
            'model': 'send-probe',
            'choices': [
              {'index': 0, 'delta': {}, 'finish_reason': 'stop'},
            ],
          })}\n\ndata: [DONE]\n\n',
        );
        await request.response.close();
      });
      final configs = clients.preferences.modelConfigManager;
      final provider = await configs.createProvider(
        name: 'Isolated send fixture',
        providerTypeId: 'OPENAI',
        endpoint: 'http://127.0.0.1:${server.port}/v1/chat/completions',
      );
      final profile = (await configs.getProviderProfiles()).firstWhere(
        (p) => p.id == provider,
      );
      await configs.updateProviderProfile(
        provider: ProviderProfile.fromJson({
          ...profile.toJson(),
          'apiKey': 'isolated-fixture',
        }),
      );
      await configs.createProviderModel(
        providerId: provider,
        modelId: 'send-probe',
      );
      await configs.createProviderModel(
        providerId: provider,
        modelId: 'background-probe',
      );
      for (final function in [
        FunctionType.chat,
        FunctionType.titleGeneration,
        FunctionType.summary,
        FunctionType.roleResponsePlanner,
      ]) {
        await clients.preferencesFunctionalConfigManager.setModelForFunction(
          functionType: function,
          providerId: provider,
          modelId: function == FunctionType.chat
              ? 'send-probe'
              : 'background-probe',
        );
      }
      for (final worldbookEnabled in [false, true]) {
        if (worldbookEnabled) {
          await manager.importBundledExternalToolPkgContainer(
            containerPackageName: 'com.operit.worldbook',
          );
          expect(
            await manager.getToolPkgContainerRuntime(
              containerPackageName: 'com.operit.worldbook',
            ),
            isNotNull,
          );
          await manager.enableToolPkgContainer(
            containerPackageName: 'com.operit.worldbook',
          );
          expect(
            await manager.getEnabledPackageNames(),
            contains('com.operit.worldbook'),
          );
        }
        for (var sample = 0; sample < 4; sample++) {
          final chatId = await clients.chatRuntimeHolderMain.createNewChat(
            setAsCurrentChat: false,
            sourceChatId: null,
            parentChatId: null,
            input: {
              package: {'version': 1, 'selection': 'card:${card['id']}'},
            },
          );
          final traceOffset = await runtimeTraceFile.length();
          requestTraceEnd = null;
          requestBoundary = Completer<double>();
          sendWatch = Stopwatch()..start();
          await clients.chatRuntimeHolderMain.sendUserMessage(
            promptFunctionType: PromptFunctionType.chat,
            participantId: null,
            chatIdOverride: chatId,
            messageText: 'isolated-send-probe-$sample',
            proxySenderNameOverride: null,
            chatProviderIdOverride: null,
            chatModelIdOverride: null,
            attachments: const [],
            replyToMessage: null,
            turnOptions: const ChatTurnOptions(
              persistTurn: true,
              notifyReply: false,
              hideUserMessage: false,
              disableWarning: true,
              continuation: null,
              chatInputSubmitRequestedHandled: false,
            ),
          );
          final toHttp = await requestBoundary.future.timeout(
            const Duration(seconds: 30),
          );
          final completedMessages = await clients.chatRuntimeHolderMain
              .chatMessagesFlow(chatId: chatId)
              .firstWhere(
                (messages) => messages.any(
                  (m) =>
                      m.sender == 'ai' &&
                      m.parts.any(
                        (part) => part.content.contains('fixture reply'),
                      ) &&
                      m.completedAt > 0,
                ),
              )
              .timeout(const Duration(seconds: 30));
          final persistedMs = sendWatch.elapsedMicroseconds / 1000;
          for (final message in completedMessages.where(
            (m) => m.sender == 'user' || m.sender == 'ai',
          )) {
            expect(message.pluginExtensions[package], {
              'participantId': card['id'],
            });
            expect(message.pluginExtensions['host.chat.identity'], {
              'title': card['name'],
              'avatarUri': card['avatarUri'],
            });
            expect(readChatMessageIdentity(message)?.title, card['name']);
          }
          final completionTrace = await _readTraceRange(
            runtimeTraceFile,
            requestTraceEnd!,
            await runtimeTraceFile.length(),
          );
          final commitStart = completionTrace.indexOf(
            'commit_assistant.start chatId=$chatId',
          );
          expect(commitStart, greaterThanOrEqualTo(0));
          final beforeCommit = completionTrace.substring(0, commitStart);
          expect(beforeCommit, isNot(contains('ToolPkgTiming: kind=hook ')));
          expect(
            beforeCommit,
            isNot(contains('ToolPkgTiming: kind=public_api ')),
          );
          expect(beforeCommit, isNot(contains('tool.execute.request tool=')));
          expect(
            beforeCommit,
            contains(
              'completed request window from prepared state chatId=$chatId ',
            ),
          );
          final histories = await clients.chatRuntimeHolderMain
              .chatHistoriesFlow()
              .firstWhere(
                (items) => items.any(
                  (item) => item.id == chatId && item.currentWindowSize > 0,
                ),
              )
              .timeout(const Duration(seconds: 30));
          final completedWindow = histories
              .firstWhere((item) => item.id == chatId)
              .currentWindowSize;
          final preparationTrace = await _readTraceRange(
            runtimeTraceFile,
            traceOffset,
            requestTraceEnd!,
          );
          expect(
            preparationTrace,
            isNot(contains('chatConfigurationSnapshotStoreApi')),
          );
          expect(
            preparationTrace,
            isNot(contains('chatConfigurationSnapshotReadApi')),
          );
          final policyTiming = _toolPolicyTiming(preparationTrace);
          final toolRequests = <String, int>{};
          for (final match in RegExp(
            r'tool.execute.request tool=(\S+) policy=',
          ).allMatches(preparationTrace)) {
            final name = match.group(1)!;
            toolRequests[name] = (toolRequests[name] ?? 0) + 1;
          }
          if (worldbookEnabled && sample > 0) {
            // Retained owners initialize once; the real system/final hooks must
            // share one variable/history snapshot for this send.
            expect(toolRequests['make_directory'] ?? 0, 0);
            expect(toolRequests['file_exists'] ?? 0, 0);
            expect(toolRequests['get_chat_messages'], 1);
          }
          samples.add({
            'stage': 'full-send-local-fixture',
            'chatId': chatId,
            'sample': sample,
            'worldbookEnabled': worldbookEnabled,
            'toHttpReceivedMs': toHttp,
            'toReplyPersistedMs': persistedMs,
            'toolPromptPolicy': policyTiming,
            'preparationToolRequests': toolRequests,
            'completedContextWindow': completedWindow,
          });
        }
      }
      final trace = await File(
        '${directory.path}/runtime/logs/operit.log',
      ).readAsString();
      // A fast machine must not conceal repeated realm creation. Each of these
      // calls must reuse the realm compiled by the service initialization hook.
      for (final function in ['chatConfigurationResolveApi']) {
        expect(trace, contains('stage=reuse_main_script function=$function '));
        expect(
          trace,
          isNot(contains('stage=compile_main_script function=$function ')),
        );
      }
      expect(
        trace,
        isNot(
          contains('tool.execute.request tool=storage_request policy=Normal'),
        ),
      );
      final report = <String, Object?>{
        'scope':
            'production QJS, CoreLink, tools and SQLite; isolated storage; first-use and retained-realm samples; full sends with/without real worldbook hooks (empty entries) use a local SSE fixture, no live model',
        'hostPlatform': Platform.operatingSystem,
        'runtimeRoot': directory.path,
        'samples': samples,
      };
      binding.reportData = report;
      // Print only timing metadata, never provider credentials or user content.
      // ignore: avoid_print
      print('PLUGIN_SEND_PERFORMANCE ${jsonEncode(report)}');
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );
}

/// Reads the preparation interval ending before the fixture returns its first response.
Future<String> _readTraceRange(File file, int offset, int end) async {
  final reader = await file.open();
  try {
    await reader.setPosition(offset);
    return utf8.decode(await reader.read(end - offset));
  } finally {
    await reader.close();
  }
}

/// Reports all five real policy invocations and rejects changes to dispatch or source reuse.
Map<String, Object?> _toolPolicyTiming(String trace) {
  final hooks = RegExp(
    r'ToolPkgTiming: kind=hook package=com\.operit\.character_cards function=toolPromptPolicy stage=(\S+) prepareMs=(\d+) executeMs=(\d+) totalMs=(\d+) success=(true|false)',
  ).allMatches(trace).toList();
  final entries = RegExp(
    r'JsExecutionTiming: kind=js_start callId=(\S+) function=toolPromptPolicy scriptBytes=(\d+) sourceRegistered=(true|false) queueMs=(\d+) initializeMs=(\d+) invokeMs=(\d+)',
  ).allMatches(trace).toList();
  expect(hooks, hasLength(5));
  expect(entries, hasLength(5));
  expect(
    hooks.map((match) => match.group(1)),
    orderedEquals(const [
      'build_tool_prompt',
      'before_compose_tool_prompt',
      'filter_tool_prompt_items',
      'after_compose_tool_prompt',
      'filter_tool_call_tools',
    ]),
  );
  for (final match in hooks) {
    expect(match.group(5), 'true');
  }
  for (final match in entries) {
    expect(
      match.group(3),
      'false',
      reason: 'Each call must reuse its registered main source',
    );
  }
  return {
    'calls': hooks.length,
    'prepareMs': hooks.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(2)!),
    ),
    'executeMs': hooks.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(3)!),
    ),
    'totalMs': hooks.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(4)!),
    ),
    'queueMs': entries.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(4)!),
    ),
    'initializeMs': entries.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(5)!),
    ),
    'entryMs': entries.fold<int>(
      0,
      (sum, match) => sum + int.parse(match.group(6)!),
    ),
  };
}
