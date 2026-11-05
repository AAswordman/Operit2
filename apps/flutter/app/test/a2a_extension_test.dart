import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/packages/dialogs/A2AImportDialog.dart';
import 'package:operit2/ui/features/packages/dialogs/A2ATaskDialog.dart';

void main() {
  testWidgets(
    'A2A discovery saves the connection through Core package management',
    (tester) async {
      final bridge = _A2ABridge();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: A2AImportDialog(clients: GeneratedCoreProxyClients(bridge)),
          ),
        ),
      );
      await tester.enterText(find.byType(TextField).at(0), 'research');
      await tester.enterText(
        find.byType(TextField).at(1),
        'https://agent.example',
      );
      await tester.ensureVisible(find.text('读取 Agent Card'));
      await tester.tap(find.text('读取 Agent Card'));
      await tester.pumpAndSettle();
      expect(find.text('Research Agent'), findsOneWidget);
      await tester.tap(find.text('保存'));
      await tester.pumpAndSettle();
      final config = bridge.manifest?['a2aAgents']['research'];
      expect(config['url'], 'https://agent.example');
      expect(config['card']['name'], 'Research Agent');
      expect(config['disabled'], false);
    },
  );
  testWidgets('editing an A2A connection preserves its disabled state', (
    tester,
  ) async {
    final bridge = _A2ABridge();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: A2AImportDialog(
            clients: GeneratedCoreProxyClients(bridge),
            id: 'research',
            configJson: jsonEncode({
              'url': 'https://agent.example',
              'headers': <String, String>{},
              'disabled': true,
            }),
          ),
        ),
      ),
    );
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.manifest?['a2aAgents']['research']['disabled'], true);
  });
  testWidgets('A2A follow-up retains the remote task and context', (
    tester,
  ) async {
    final bridge = _A2ABridge();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: A2ATaskDialog(
            clients: GeneratedCoreProxyClients(bridge),
            id: 'research',
          ),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField).first, 'Research this');
    await tester.tap(find.text('发送'));
    await tester.pumpAndSettle();
    expect(
      tester.widget<TextField>(find.byType(TextField).at(1)).controller!.text,
      'task-1',
    );
    expect(
      tester.widget<TextField>(find.byType(TextField).at(2)).controller!.text,
      'context-1',
    );
    await tester.enterText(find.byType(TextField).first, 'Here is more detail');
    await tester.tap(find.text('发送'));
    await tester.pumpAndSettle();
    expect(bridge.invocations.last['task_id'], 'task-1');
    expect(bridge.invocations.last['context_id'], 'context-1');
    expect(bridge.invocations.last['message'], 'Here is more detail');
    expect(tester.takeException(), isNull);
  });
}

class _A2ABridge extends OperitRuntimeBridge {
  Map<String, dynamic>? manifest;
  final invocations = <Map<String, dynamic>>[];
  @override
  Future<Object?> call(CoreCallRequest request) async {
    final args = request.args as Map;
    switch (request.methodName) {
      case 'discoverA2AAgent':
        final config =
            jsonDecode(args['configJson'] as String) as Map<String, dynamic>;
        config['card'] = {
          'name': 'Research Agent',
          'description': 'Research documents',
          'skills': <dynamic>[],
        };
        return jsonEncode(config);
      case 'importA2AConfig':
        manifest =
            jsonDecode(args['configJson'] as String) as Map<String, dynamic>;
        return ['research'];
      case 'invokeA2A':
        invocations.add(
          jsonDecode(args['argumentsJson'] as String) as Map<String, dynamic>,
        );
        return jsonEncode({
          'task': {
            'id': 'task-1',
            'contextId': 'context-1',
            'status': {'state': 'TASK_STATE_INPUT_REQUIRED'},
          },
        });
      default:
        throw StateError('Unexpected call: ${request.methodName}');
    }
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[0, await call(request)]);
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
