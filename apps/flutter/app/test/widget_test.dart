import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/main/OperitApp.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Operit main shell smoke test', (tester) async {
    await ClientLogger.initialize();
    const channel = MethodChannel('operit/runtime');
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
      call,
    ) async {
      if (call.method == 'call') {
        return encodeCoreLink(<Object?>[0, null]);
      }
      if (call.method == 'watchStream') {
        final envelope = decodeCoreLink<List<Object?>>(
          call.arguments as Uint8List,
        );
        return encodeCoreLink(<Object?>[0, envelope.first]);
      }
      if (call.method == 'closeWatchStream') {
        return encodeCoreLink(<Object?>[0, null]);
      }
      return null;
    });

    await tester.pumpWidget(const OperitApp());
    await tester.pump();

    expect(find.byType(OperitApp), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
