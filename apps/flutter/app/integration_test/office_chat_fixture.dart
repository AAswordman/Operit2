import 'dart:convert';
import 'dart:io';

import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

const officeOriginal = 'Replace this exact sentence.';
const officeEdited = 'Updated through the native Office tool.';
const officePrompt =
    'In Smoke.docx, replace "$officeOriginal" with '
    '"$officeEdited". Leave the change unsaved.';

/// Deterministic local provider for exercising the production chat/tool loop.
class OfficeChatFixture {
  OfficeChatFixture(this.server, this.packageName);

  final HttpServer server;
  final String packageName;
  final List<String> steps = [];
  Object? failure;
  String? id;

  static Future<OfficeChatFixture> start(
    GeneratedCoreProxyClients clients,
    String packageName,
  ) async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final fixture = OfficeChatFixture(server, packageName);
    try {
      server.listen(fixture._respond);
      final config = clients.preferences.modelConfigManager;
      final provider = await config.createProvider(
        name: 'Isolated Office integration fixture',
        providerTypeId: 'OPENAI',
        endpoint: 'http://127.0.0.1:${server.port}/v1/chat/completions',
      );
      final profile = (await config.getProviderProfiles()).firstWhere(
        (p) => p.id == provider,
      );
      await config.updateProviderProfile(
        provider: ProviderProfile.fromJson({
          ...profile.toJson(),
          'apiKey': 'isolated-fixture',
        }),
      );
      for (final model in ['office-fixture', 'office-background-fixture']) {
        await config.createProviderModel(providerId: provider, modelId: model);
      }
      await config.updateCapabilitiesForModel(
        providerId: provider,
        modelId: 'office-fixture',
        capabilities: const ModelCapabilities(
          directImage: false,
          directAudio: false,
          directVideo: false,
          toolCall: true,
        ),
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
              ? 'office-fixture'
              : 'office-background-fixture',
        );
      }
      return fixture;
    } catch (_) {
      await server.close(force: true);
      rethrow;
    }
  }

  Future<void> _respond(HttpRequest request) async {
    try {
      final payload =
          jsonDecode(await utf8.decoder.bind(request).join()) as Map;
      final model = payload['model'];
      final messages = (payload['messages'] as List).cast<Map>();
      final isEditTurn =
          model == 'office-fixture' &&
          messages.any(
            (message) =>
                message['role'] == 'user' &&
                jsonEncode(
                  message['content'],
                ).contains('Leave the change unsaved.'),
          );
      String? name;
      Map<String, Object?> params = {};
      if (isEditTurn) {
        final result = messages
            .where((message) => message['role'] == 'tool')
            .lastOrNull;
        final resultId = result?['tool_call_id'];
        final call = resultId == null
            ? null
            : messages
                      .expand(
                        (message) =>
                            (message['tool_calls'] as List? ?? []).cast<Map>(),
                      )
                      .lastWhere((call) => call['id'] == resultId)['function']
                  as Map;
        final arguments = call == null
            ? null
            : jsonDecode(call['arguments'] as String) as Map;
        final wire = call?['name'];
        final completed = wire == 'package_proxy' || wire == 'proxy'
            ? (arguments?['tool_name'])
            : wire;
        switch (completed) {
          case null:
            name = 'use_package';
            params = {'package_name': packageName};
          case 'use_package':
            name = 'list';
          case final String target when target == '$packageName:list':
            final data = _findData(
              result!['content'],
              (value) => value['name'] == 'Smoke.docx' && value['id'] is String,
            );
            id = data['id'] as String;
            name = 'read';
            params = {'id': id};
          case final String target when target == '$packageName:read':
            final data = _findData(
              result!['content'],
              (value) =>
                  value['revision'] is int &&
                  jsonEncode(value['data']).contains(officeOriginal),
            );
            name = 'edit';
            params = {
              'id': id,
              'revision': data['revision'],
              'json': jsonEncode({
                'op': 'replace',
                'expected': officeOriginal,
                'text': officeEdited,
              }),
            };
          case final String target when target == '$packageName:edit':
            _findData(
              result!['content'],
              (value) => value['id'] == id && value['dirty'] == true,
            );
          default:
            throw StateError('Unexpected Office tool result: $result');
        }
        steps.add(name ?? 'done');
      }
      final wireName = name == null || name == 'use_package'
          ? name
          : 'package_proxy';
      if (name != null && wireName == 'package_proxy') {
        params = {
          'tool_name': '$packageName:$name',
          'params': jsonEncode(params),
        };
      }
      if (wireName != null &&
          !(payload['tools'] as List? ?? []).cast<Map>().any(
            (tool) => (tool['function'] as Map?)?['name'] == wireName,
          )) {
        throw StateError('Chat did not expose $wireName');
      }
      final delta = <String, Object?>{
        'role': 'assistant',
        if (wireName == null)
          'content': isEditTurn
              ? 'Office fixture edit complete.'
              : 'Office fixture',
        if (wireName != null)
          'tool_calls': [
            {
              'index': 0,
              'id': 'office-$name',
              'type': 'function',
              'function': {'name': wireName, 'arguments': jsonEncode(params)},
            },
          ],
      };
      final response = request.response;
      final finish = wireName == null ? 'stop' : 'tool_calls';
      Map<String, Object?> completion(
        String choice,
        Object? value,
        String? finish,
        bool stream,
      ) => {
        'id': 'office-fixture',
        'object': stream ? 'chat.completion.chunk' : 'chat.completion',
        'created': 1,
        'model': model,
        'choices': [
          {'index': 0, choice: value, 'finish_reason': finish},
        ],
      };
      if (payload['stream'] == true) {
        response.headers.contentType = ContentType('text', 'event-stream');
        response.write(
          'data: ${jsonEncode(completion('delta', delta, null, true))}\n\n',
        );
        response.write(
          'data: ${jsonEncode(completion('delta', {}, finish, true))}\n\ndata: [DONE]\n\n',
        );
      } else {
        response.headers.contentType = ContentType.json;
        response.write(jsonEncode(completion('message', delta, finish, false)));
      }
      await response.close();
    } catch (error) {
      failure ??= error;
      request.response.statusCode = 500;
      await request.response.close();
    }
  }

  static Map _findData(Object? input, bool Function(Map) matches) {
    Map? search(Object? value) {
      if (value is String) {
        try {
          final decoded = jsonDecode(value);
          return decoded == value ? null : search(decoded);
        } on FormatException {
          return null;
        }
      }
      if (value is Map && matches(value)) return value;
      final children = value is Map
          ? value.values
          : value is List
          ? value
          : const [];
      for (final child in children) {
        final result = search(child);
        if (result != null) return result;
      }
      return null;
    }

    return search(input) ??
        (throw StateError('Missing expected Office tool data: $input'));
  }
}
