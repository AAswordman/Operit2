/// Opt-in profiling entry point: starts the real app and exercises real services.
/// flutter run -t integration_test/business_cpu_probe.dart -d macos --profile
///   --dart-define=OPERIT_BUSINESS_CPU=true --vm-service-port=8181
library;

import 'dart:convert';
import 'dart:developer' show Timeline;
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/main.dart' as application;
import 'package:path_provider/path_provider.dart';

import 'support/macos_process_metrics.dart';

const _enabled = bool.fromEnvironment('OPERIT_BUSINESS_CPU');
const _toolCalls = int.fromEnvironment(
  'OPERIT_BUSINESS_TOOL_CALLS',
  defaultValue: 200,
);

Future<void> main() async {
  application.main(const []);
  await WidgetsBinding.instance.waitUntilFirstFrameRasterized;
  if (!_enabled) return;
  final metrics = Platform.isMacOS ? MacOSProcessMetrics() : null;
  final stages = <Map<String, Object?>>[];
  final report = <String, Object?>{
    'pid': pid,
    'mode': '${Platform.operatingSystem}-full-application',
    'cpuMeasurement': metrics == null
        ? 'external-process-sampler'
        : 'proc_pid_rusage',
    'toolCallsPerStage': _toolCalls,
    'stages': stages,
  };
  final support = await getApplicationSupportDirectory();
  final file = File(
    '${support.path}/performance-smoke/business-cpu-${DateTime.now().millisecondsSinceEpoch}.json',
  );
  await file.parent.create(recursive: true);
  Future<void> save() =>
      file.writeAsString(const JsonEncoder.withIndent('  ').convert(report));
  Future<T> measure<T>(String name, Future<T> Function() action) async {
    final startWall = DateTime.now().microsecondsSinceEpoch;
    final before = metrics?.read();
    final origin = Timeline.now;
    final watch = Stopwatch()..start();
    try {
      return await action();
    } finally {
      watch.stop();
      final extent = Timeline.now - origin;
      final after = metrics?.read();
      final stage = <String, Object?>{
        'name': name,
        'startWallMicros': startWall,
        'endWallMicros': DateTime.now().microsecondsSinceEpoch,
        'vmTimeOriginMicros': origin,
        'vmTimeExtentMicros': extent,
        'elapsedMs': watch.elapsedMicroseconds / 1000,
        if (before != null && after != null) ...{
          'cpuMs': (after.cpuSeconds - before.cpuSeconds) * 1000,
          'cpuPercentOneCore':
              (after.cpuSeconds - before.cpuSeconds) /
              (watch.elapsedMicroseconds / 1e6) *
              100,
          'footprintMiB': after.footprint / (1024 * 1024),
        },
      };
      stages.add(stage);
      await save();
      // ignore: avoid_print
      print('BUSINESS_CPU_STAGE ${jsonEncode(stage)}');
    }
  }

  // Gives the external native profiler time to attach, without polling the app.
  // ignore: avoid_print
  print(
    'BUSINESS_CPU_READY ${jsonEncode({'pid': pid, 'artifact': file.path})}',
  );
  try {
    await Future<void>.delayed(const Duration(seconds: 10));
    const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
    final manager = clients.application.packageManager();
    await manager.getToolPkgPublicApiOwners(
      apiName: 'chat.configuration.resolve',
    );
    await measure(
      'baseline',
      () => Future<void>.delayed(const Duration(seconds: 2)),
    );
    for (final name in [
      'daily_life:get_current_date',
      'daily_life:device_status',
    ]) {
      Future<void> invoke() async {
        final result = await clients.application
            .aiToolHandler()
            .executeTool(
              tool: CoreOperitToolsToolExecutionManagerAiTool(
                name: name,
                parameters: const [],
              ),
            )
            .timeout(const Duration(seconds: 30));
        if (!result.success) throw StateError('$name: ${result.error}');
      }

      await invoke();
      await measure(name, () async {
        for (var i = 0; i < _toolCalls; i++) {
          await invoke();
        }
      });
      // Allows the host collector to save the completed stage's CPU samples.
      await Future<void>.delayed(const Duration(milliseconds: 300));
    }
    final chatId = await clients.chatRuntimeHolderMain.createNewChat(
      setAsCurrentChat: true,
      sourceChatId: null,
      parentChatId: null,
      input: {
        'com.operit.character_cards': {'version': 1, 'selection': 'model:chat'},
      },
    );
    report['chatId'] = chatId;
    await WidgetsBinding.instance.endOfFrame;
    var chatAreas = 0;
    void visit(Element element) {
      if (element.widget.runtimeType.toString() == 'ChatArea') {
        chatAreas++;
      }
      element.visitChildElements(visit);
    }

    WidgetsBinding.instance.rootElement?.visitChildElements(visit);
    report['mountedChatAreas'] = chatAreas;
    if (chatAreas == 0) {
      throw StateError('The real chat UI must be mounted for this probe');
    }
    var messageUpdates = 0;
    final completed = clients.chatRuntimeHolderMain
        .chatMessagesFlow(chatId: chatId)
        .map((messages) {
          messageUpdates++;
          return messages;
        })
        .firstWhere(
          (messages) =>
              messages.any((m) => m.sender == 'ai' && m.completedAt > 0),
        );
    final messages = await measure('receiveStreamWithRealUi', () async {
      await clients.chatRuntimeHolderMain.sendUserMessage(
        promptFunctionType: PromptFunctionType.chat,
        participantId: null,
        chatIdOverride: chatId,
        messageText:
            '这是一次流式消息性能测试。请用中文写约800字的日常整理建议，使用8个Markdown二级标题，每节一段，包含少量加粗和列表。直接输出正文，不调用工具，最后单独写“流式接收测试完成”。',
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
      return completed.timeout(const Duration(seconds: 120));
    });
    final assistant = messages.where((m) => m.sender == 'ai').last;
    final text = assistant.parts.map((p) => p.content).join();
    report['reply'] = {
      'model': assistant.modelName,
      'characters': text.length,
      'inputTokens': assistant.inputTokens,
      'outputTokens': assistant.outputTokens,
      'waitDurationMs': assistant.waitDurationMs,
      'outputDurationMs': assistant.outputDurationMs,
      'messageSnapshotUpdates': messageUpdates,
      'completionMarkerPresent': text.contains('流式接收测试完成'),
    };
    if (text.length < 300 || !text.contains('流式接收测试完成')) {
      throw StateError('Expected a substantial completed streaming response');
    }
    await measure(
      'uiAfterCompletion',
      () => Future<void>.delayed(const Duration(seconds: 2)),
    );
    report['success'] = true;
  } catch (error, stack) {
    report['success'] = false;
    report['error'] = error.toString();
    // ignore: avoid_print
    print('BUSINESS_CPU_ERROR $error\n$stack');
  } finally {
    await save();
    metrics?.close();
    // ignore: avoid_print
    print('BUSINESS_CPU_RESULT ${jsonEncode(report)}');
  }
}
