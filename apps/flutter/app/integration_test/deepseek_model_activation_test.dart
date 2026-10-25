import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/components/SettingsControlStyles.dart';
import 'package:operit2/ui/features/settings/model/ModelSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Native regression for the Oct 25 03:56 feedback. Only the HTTP peer is a
/// fixture: activation uses production UI/preferences, and sends use the
/// production character runtime, hooks, DeepSeek provider and storage.
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'DeepSeek UI activation persists and selected models can send',
    (tester) async {
      final support = await getApplicationSupportDirectory();
      final parent = await Directory(
        '${support.path}/deepseek-activation-integration',
      ).create(recursive: true);
      final directory = await parent.createTemp('run-');
      // ignore: avoid_print
      print('DEEPSEEK_RUNTIME ${directory.path}');
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
      final card = Map<String, Object?>.from(
        await manager.invokeToolPkgPublicApi(
              packageName: package,
              methodName: 'character.create',
              payload: {
                'values': {
                  'name': 'DeepSeek activation regression',
                  'characterSetting': 'Reply briefly.',
                },
              },
            )
            as Map,
      );
      await manager.invokeToolPkgPublicApi(
        packageName: package,
        methodName: 'memory.create',
        payload: {
          'ownerKey': 'character:${card['id']}',
          'values': {
            'title': 'activation regression fixture',
            'content':
                'activation regression fixture known-durable-activation-fact',
          },
        },
      );
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      addTearDown(() => server.close(force: true));
      final received = <Map<String, Object?>>[];
      var rejectRequest = false;
      server.listen((request) async {
        final payload =
            jsonDecode(await utf8.decoder.bind(request).join())
                as Map<String, Object?>;
        received.add(payload);
        if (rejectRequest && payload['model'] != 'background-probe') {
          request.response.statusCode = HttpStatus.unauthorized;
          request.response.headers.contentType = ContentType.json;
          request.response.write(
            jsonEncode({
              'error': {'message': 'activation fixture rejected request'},
            }),
          );
          await request.response.close();
          return;
        }
        if (payload['stream'] != true) {
          request.response.headers.contentType = ContentType.json;
          request.response.write(
            jsonEncode({
              'id': 'activation-background-fixture',
              'object': 'chat.completion',
              'model': payload['model'],
              'choices': [
                {
                  'index': 0,
                  'message': {
                    'role': 'assistant',
                    'content': 'Activation regression',
                  },
                  'finish_reason': 'stop',
                },
              ],
            }),
          );
          await request.response.close();
          return;
        }
        request.response.headers.set('Content-Type', 'text/event-stream');
        for (final delta in [
          {'role': 'assistant', 'content': 'activation fixture reply'},
          <String, Object?>{},
        ]) {
          request.response.write(
            'data: ${jsonEncode({
              'id': 'activation-fixture',
              'object': 'chat.completion.chunk',
              'created': 1,
              'model': payload['model'],
              'choices': [
                {'index': 0, 'delta': delta, 'finish_reason': delta.isEmpty ? 'stop' : null},
              ],
            })}\n\n',
          );
        }
        request.response.write('data: [DONE]\n\n');
        await request.response.close();
      });
      final configs = clients.preferencesModelConfigManager;
      final providerId = await configs.createProvider(
        name: 'DeepSeek activation fixture',
        providerTypeId: 'DEEPSEEK',
        endpoint: 'http://127.0.0.1:${server.port}/v1/chat/completions',
      );
      final provider = (await configs.getProviderProfiles()).firstWhere(
        (value) => value.id == providerId,
      );
      await configs.updateProviderProfile(
        provider: ProviderProfile.fromJson({
          ...provider.toJson(),
          'apiKey': 'isolated-fixture',
        }),
      );
      const models = ['deepseek-v4-flash', 'deepseek-v4-pro'];
      for (final model in models) {
        await configs.createProviderModel(
          providerId: providerId,
          modelId: model,
        );
      }
      await configs.createProviderModel(
        providerId: providerId,
        modelId: 'background-probe',
      );
      for (final function in [
        FunctionType.titleGeneration,
        FunctionType.summary,
        FunctionType.roleResponsePlanner,
      ]) {
        await clients.preferencesFunctionalConfigManager.setModelForFunction(
          functionType: function,
          providerId: providerId,
          modelId: 'background-probe',
        );
      }
      final panelKey = GlobalKey<ModelSettingsPanelState>();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: ModelSettingsPanel(key: panelKey, clients: clients),
          ),
        ),
      );
      await _pumpUntil(
        tester,
        () => find.text(provider.name).evaluate().isNotEmpty,
      );
      await tester.ensureVisible(find.text(provider.name));
      await tester.tap(find.text(provider.name));
      await _pumpUntil(
        tester,
        () => find.text(models.first).evaluate().isNotEmpty,
      );
      final l10n = AppLocalizations.of(panelKey.currentContext!)!;
      final results = <Map<String, Object?>>[];
      String? failedChatId;
      for (var sample = 0; sample < 6; sample++) {
        final model = models[sample % models.length];
        // ignore: avoid_print
        print('DEEPSEEK_CASE_START $sample');
        final watch = Stopwatch()..start();
        final row = find
            .ancestor(of: find.text(model).last, matching: find.byType(Row))
            .first;
        final button = find.descendant(
          of: row,
          matching: find.byType(SettingsSetActiveButton),
        );
        expect(button, findsOneWidget);
        await tester.ensureVisible(button);
        await tester.tap(button);
        await _pumpUntil(
          tester,
          () => find
              .descendant(
                of: row,
                matching: find.text(l10n.settingsModelCurrentActive),
              )
              .evaluate()
              .isNotEmpty,
        );
        expect(tester.takeException(), isNull);
        final selected = await clients.preferencesFunctionalConfigManager
            .getModelBindingForFunction(functionType: FunctionType.chat);
        expect(selected.providerId, providerId);
        expect(selected.modelId, model);
        final activationMs = watch.elapsedMilliseconds;
        final chatId = sample == 5
            ? failedChatId!
            : await clients.chatRuntimeHolderMain.createNewChat(
                setAsCurrentChat: false,
                sourceChatId: null,
                parentChatId: null,
                input: {
                  package: {
                    'version': 1,
                    'selection': sample < 2
                        ? 'model:chat'
                        : 'card:${card['id']}',
                  },
                },
              );
        final message = 'activation regression fixture sample $sample';
        rejectRequest = sample == 4;
        final states = <String>[];
        final stateSubscription = clients.chatRuntimeHolderMain
            .inputProcessingStateByChatIdFlow()
            .listen((snapshot) {
              final state = snapshot[chatId];
              if (state != null &&
                  (states.isEmpty || states.last != state.tag)) {
                states.add(state.tag);
              }
            });
        final previousReplies =
            (await clients.chatRuntimeHolderMain
                    .chatMessagesFlow(chatId: chatId)
                    .first)
                .where(
                  (value) =>
                      value.sender == 'ai' &&
                      value.completedAt > 0 &&
                      value.parts.any(
                        (part) =>
                            part.content.contains('activation fixture reply'),
                      ),
                )
                .length;
        await clients.chatRuntimeHolderMain.sendUserMessage(
          promptFunctionType: PromptFunctionType.chat,
          participantId: null,
          chatIdOverride: chatId,
          messageText: message,
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
        if (rejectRequest) {
          failedChatId = chatId;
          final failedState = await clients.chatRuntimeHolderMain
              .inputProcessingStateByChatIdFlow()
              .firstWhere((snapshot) => snapshot[chatId]?.tag == 'Error')
              .timeout(const Duration(seconds: 60));
          expect(
            failedState[chatId]!.message,
            contains('activation fixture rejected request'),
          );
        } else {
          await clients.chatRuntimeHolderMain
              .chatMessagesFlow(chatId: chatId)
              .firstWhere(
                (messages) =>
                    messages
                        .where(
                          (value) =>
                              value.sender == 'ai' &&
                              value.completedAt > 0 &&
                              value.parts.any(
                                (part) => part.content.contains(
                                  'activation fixture reply',
                                ),
                              ),
                        )
                        .length >
                    previousReplies,
              )
              .timeout(const Duration(seconds: 30));
        }
        await clients.chatRuntimeHolderMain
            .activeStreamingChatIdsFlow()
            .firstWhere((ids) => !ids.contains(chatId))
            .timeout(const Duration(seconds: 30));
        await stateSubscription.cancel();
        final request = received.firstWhere(
          (payload) =>
              payload['stream'] == true &&
              payload['model'] != 'background-probe' &&
              jsonEncode(payload['messages']).contains(message),
        );
        expect(request['model'], model);
        if (sample < 5) {
          expect(
            jsonEncode(
              request['messages'],
            ).contains('known-durable-activation-fact'),
            sample >= 2,
          );
        }
        expect(states, contains('Connecting'));
        if (!rejectRequest) {
          expect(
            states.skipWhile((tag) => tag != 'Connecting'),
            isNot(contains('Error')),
          );
        }
        results.add({
          'model': model,
          'selection': sample < 2 ? 'model' : 'character',
          'activationMs': activationMs,
          'completedMs': watch.elapsedMilliseconds,
          'expectedFailure': rejectRequest,
          'recovery': sample == 5,
          'states': states,
        });
        expect(tester.takeException(), isNull);
        // ignore: avoid_print
        print('DEEPSEEK_CASE_DONE ${jsonEncode(results.last)}');
      }
      // ignore: avoid_print
      print(
        'DEEPSEEK_ACTIVATION ${jsonEncode({'runtimeRoot': directory.path, 'results': results})}',
      );
      await tester.pumpWidget(const SizedBox.shrink());
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );
}

Future<void> _pumpUntil(
  WidgetTester tester,
  bool Function() ready, {
  Duration timeout = const Duration(seconds: 30),
}) async {
  final deadline = DateTime.now().add(timeout);
  while (!ready()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TimeoutException(
        'Native activation/send condition did not settle',
        timeout,
      );
    }
    await tester.pump(const Duration(milliseconds: 100));
  }
  await tester.pump(const Duration(milliseconds: 300));
}
