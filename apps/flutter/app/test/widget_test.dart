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
        final envelope = jsonDecode(call.arguments as String);
        return jsonEncode({'subscriptionId': envelope['subscriptionId']});
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
