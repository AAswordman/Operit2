import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'support/chat_switch_stress.dart';

/// Runs an opt-in benchmark against an explicitly supplied production native library.
void main() {
  final path = Platform.environment['OPERIT_CHAT_STRESS_LIBRARY'];
  testWidgets(
    'real Core chat switch and transcript stress',
    (tester) async {
      await runChatSwitchStress(tester, path!);
    },
    skip: path == null,
    timeout: const Timeout(Duration(minutes: 30)),
  );
}
