import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import '../test/support/chat_switch_stress.dart';

/// Measures native history switching and real rendered frames in a profile application.
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'real Core chat switch and transcript stress',
    (tester) async {
      const path = String.fromEnvironment('OPERIT_CHAT_STRESS_LIBRARY');
      expect(path, isNotEmpty);
      binding.reportData = await runChatSwitchStress(tester, path);
    },
    timeout: const Timeout(Duration(minutes: 30)),
  );
}
