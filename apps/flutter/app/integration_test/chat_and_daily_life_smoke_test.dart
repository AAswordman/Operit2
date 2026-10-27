import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';
import 'package:path_provider/path_provider.dart';

import 'support/macos_process_metrics.dart';

/// Explicit opt-in: uses the active identity and sends one real model request.
/// flutter drive --target=integration_test/chat_and_daily_life_smoke_test.dart
///   --driver=test_driver/live_smoke_driver.dart -d macos
///   --profile --dart-define=OPERIT_LIVE_SMOKE=true
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'active identity sends a message and executes daily life tools',
    (tester) async {
      expect(Platform.isMacOS, isTrue);
      await ClientLogger.initialize();
      await RuntimeBootstrapManager.instance.initialize();
      RuntimeHostInteractionSubscriber.install();
      addTearDown(RuntimeHostInteractionSubscriber.uninstall);
      await tester.pumpWidget(
        const MaterialApp(home: Scaffold(body: Text('聊天与日常生活工具实测'))),
      );
      // This probe exercises native services with a static page. Live test
      // bindings otherwise request a frame after every vsync, inflating CPU.
      binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.benchmark;
      const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
      final manager = clients.application.packageManager();
      final support = await getApplicationSupportDirectory();
      final output = File(
        '${support.path}/performance-smoke/chat-tools-${DateTime.now().millisecondsSinceEpoch}.json',
      );
      await output.parent.create(recursive: true);
      final report = <String, Object?>{
        'pid': pid,
        'mode': 'profile-native-services-static-test-page',
        'framePolicy': 'benchmark',
        'toolCallsPerStage': 5,
        'stages': <Object?>[],
      };
      final metrics = MacOSProcessMetrics();
      addTearDown(metrics.close);
      Future<void> save() => output.writeAsString(
        const JsonEncoder.withIndent('  ').convert(report),
      );
      Future<T> measure<T>(String name, Future<T> Function() action) async {
        final before = metrics.read();
        final watch = Stopwatch()..start();
        try {
          return await action();
        } finally {
          final after = metrics.read();
          watch.stop();
          final seconds = watch.elapsedMicroseconds / 1e6;
          final stage = <String, Object?>{
            'name': name,
            'elapsedMs': watch.elapsedMicroseconds / 1000,
            'cpuSeconds': after.cpuSeconds - before.cpuSeconds,
            'cpuPercentOneCore':
                (after.cpuSeconds - before.cpuSeconds) / seconds * 100,
            'interruptWakeupsPerSecond':
                (after.wakeups - before.wakeups) / seconds,
            'footprintMiB': after.footprint / (1024 * 1024),
          };
          (report['stages']! as List<Object?>).add(stage);
          await save();
          // ignore: avoid_print
          print('LIVE_SMOKE_STAGE ${jsonEncode(stage)}');
        }
      }

      // ignore: avoid_print
      print('LIVE_SMOKE_ARTIFACT ${output.path}');
      try {
        await manager.getToolPkgPublicApiOwners(
          apiName: 'chat.configuration.resolve',
        );
        report['dailyLifeEnabled'] = (await manager.getEnabledPackageNames())
            .contains('daily_life');
        final model = await clients.preferencesFunctionalConfigManager
            .getModelBindingForFunction(functionType: FunctionType.chat);
        report['model'] = {
          'providerId': model.providerId,
          'modelId': model.modelId,
        };
        await save();
        await measure(
          'idleBefore',
          () => Future<void>.delayed(const Duration(seconds: 10)),
        );
        for (final name in [
          'daily_life:get_current_date',
          'daily_life:device_status',
        ]) {
          final results = <Object?>[];
          await measure(name, () async {
            for (var i = 0; i < 5; i++) {
              final result = await clients.application
                  .aiToolHandler()
                  .executeTool(
                    tool: CoreOperitToolsToolExecutionManagerAiTool(
                      name: name,
                      parameters: const [],
                    ),
                  )
                  .timeout(const Duration(seconds: 30));
              results.add(result.toJson());
              expect(result.success, isTrue, reason: '$name: ${result.error}');
            }
          });
          report[name] = results;
          await save();
        }
        final chatId = await clients.chatRuntimeHolderMain.createNewChat(
          setAsCurrentChat: true,
          sourceChatId: null,
          parentChatId: null,
          input: {
            'com.operit.character_cards': {
              'version': 1,
              'selection': 'model:chat',
            },
          },
        );
        report['chatId'] = chatId;
        await save();
        final messages = await measure('sendMessage', () async {
          await clients.chatRuntimeHolderMain.sendUserMessage(
            promptFunctionType: PromptFunctionType.chat,
            participantId: null,
            chatIdOverride: chatId,
            messageText: '这是一条性能回归测试消息。请只回复：收到，消息发送正常。',
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
          return clients.chatRuntimeHolderMain
              .chatMessagesFlow(chatId: chatId)
              .firstWhere(
                (messages) =>
                    messages.any((m) => m.sender == 'ai' && m.completedAt > 0),
              )
              .timeout(const Duration(seconds: 120));
        });
        report['messages'] = messages
            .map(
              (m) => {
                'sender': m.sender,
                'completedAt': m.completedAt,
                'text': m.parts.map((p) => p.content).join(),
                'model': m.modelName,
                'inputTokens': m.inputTokens,
                'outputTokens': m.outputTokens,
                'waitDurationMs': m.waitDurationMs,
                'outputDurationMs': m.outputDurationMs,
              },
            )
            .toList();
        expect(messages.where((m) => m.sender == 'user'), hasLength(1));
        final reply = messages
            .where((m) => m.sender == 'ai')
            .map((m) => m.parts.map((p) => p.content).join())
            .join();
        expect(reply, contains('消息发送正常'));
        await measure(
          'idleAfter',
          () => Future<void>.delayed(const Duration(seconds: 10)),
        );
        report['success'] = true;
      } catch (error) {
        report['success'] = false;
        report['error'] = error.toString();
        rethrow;
      } finally {
        await save();
        // ignore: avoid_print
        print('LIVE_SMOKE_RESULT ${jsonEncode(report)}');
      }
    },
    skip: !const bool.fromEnvironment('OPERIT_LIVE_SMOKE'),
    timeout: const Timeout(Duration(minutes: 5)),
  );
}
