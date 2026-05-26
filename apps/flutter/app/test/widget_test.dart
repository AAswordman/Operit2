import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/main/OperitApp.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Operit main shell smoke test', (tester) async {
    const channel = MethodChannel('operit/runtime');
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
      call,
    ) async {
      if (call.method == 'hostDescriptor') {
        return jsonEncode({
          'id': 'test',
          'displayName': 'Test Host',
          'pathStyleDescriptionEn': '',
          'pathStyleDescriptionCn': '',
          'examplePaths': const <String>[],
          'usesEnvironmentParameter': false,
          'environmentParameterDescriptionEn': '',
          'environmentParameterDescriptionCn': '',
          'capabilities': const <String>[],
          'fileSystemHost': true,
          'webVisitHost': true,
          'systemOperationHost': true,
          'managedRuntimeHost': true,
          'runtimeStorageHost': true,
          'runtimeSqliteHost': true,
        });
      }
      if (call.method == 'call') {
        final request = jsonDecode(call.arguments as String);
        return jsonEncode({
          'requestId': request['requestId'],
          'result': {'Ok': '0.1.0'},
        });
      }
      if (call.method == 'watchSnapshot') {
        final request = jsonDecode(call.arguments as String);
        final propertyName = request['propertyName'] as String;
        return jsonEncode({
          'requestId': request['requestId'],
          'targetPath': request['targetPath'],
          'propertyName': propertyName,
          'kind': 'Snapshot',
          'value': propertyName == 'chatHistoryFlow' ? const [] : null,
        });
      }
      if (call.method == 'watchStream') {
        return jsonEncode({'subscriptionId': 'test-subscription'});
      }
      if (call.method == 'pollWatchStream') {
        return jsonEncode([
          {
            'requestId': null,
            'targetPath': {
              'segments': ['chatRuntimeHolder', 'main'],
            },
            'propertyName': 'getResponseStream',
            'kind': 'Completed',
            'value': {
              'chatId': 'test-chat',
              'type': 'completed',
              'value': null,
            },
          },
        ]);
      }
      if (call.method == 'closeWatchStream') {
        return jsonEncode({'ok': true});
      }
      return null;
    });

    await tester.pumpWidget(const OperitApp());
    await tester.pump();

    expect(find.text('AI Chat'), findsWidgets);
    expect(find.text('Message Operit'), findsOneWidget);
  });
}
