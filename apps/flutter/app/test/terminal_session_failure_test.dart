import 'dart:async';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspacePtyProcess.dart';

class _ExitedBridge extends OperitRuntimeBridge {
  final events = StreamController<CoreEvent>();
  final calls = <String>[];
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    throw StateError('iSH terminal session is not running');
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) => events.stream;
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  for (final resize in [true, false]) {
    test(
      '${resize ? "resize" : "write"} failure stays local to terminal',
      () async {
        final bridge = _ExitedBridge();
        final process = attachWorkspacePty(
          'exited',
          clients: GeneratedCoreProxyClients(bridge),
        );
        final errors = <Object>[];
        process.output.listen(
          (_) {},
          onError: (Object error) => errors.add(error),
        );
        final exit = process.exitCode;
        if (resize) {
          process.resize(24, 42);
          await Future<void>.delayed(const Duration(milliseconds: 100));
        } else {
          process.write(Uint8List.fromList([13]));
          await Future<void>.delayed(Duration.zero);
        }
        await Future<void>.delayed(Duration.zero);
        expect(errors, hasLength(1));
        expect(await exit.timeout(const Duration(seconds: 2)), -1);
        final calls = bridge.calls.length;
        process.resize(30, 60);
        process.write(Uint8List.fromList([13]));
        await Future<void>.delayed(const Duration(milliseconds: 100));
        expect(bridge.calls.length, calls);
        unawaited(bridge.events.close());
      },
    );
  }
}
