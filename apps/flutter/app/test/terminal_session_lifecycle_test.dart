import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspacePtyProcess.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalSessions.dart';

class _TerminalLifecycleBridge extends OperitRuntimeBridge {
  final events = StreamController<CoreEvent>();
  final calls = <String>[];
  var closed = false;
  Uint8List? writtenBytes;

  /// Models a host that rejects a second release of the same PTY.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    switch (request.methodName) {
      case 'writeTerminalPty':
        writtenBytes = (request.args as Map)['data'] as Uint8List;
        return encodeCoreLink([0, writtenBytes!.length]);
      case 'pollTerminalPtyExit':
        if (closed) throw StateError('PTY session does not exist');
        return Uint8List.fromList([0x92, 0, 7]);
      case 'closeTerminalPty':
        if (closed) throw StateError('PTY session does not exist');
        closed = true;
        return Uint8List.fromList([0x92, 0, 0xc0]);
      default:
        throw StateError('Unexpected terminal call: ${request.methodName}');
    }
  }

  /// Emits output completion independently of host session ownership.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) => events.stream;

  /// Rejects unused bridge operations in this session ownership fixture.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

/// Verifies that attached views observe exits without releasing owned sessions.
void main() {
  test(
    'generated terminal stream and input preserve binary payloads',
    () async {
      final bridge = _TerminalLifecycleBridge();
      final process = attachWorkspacePty(
        'session',
        clients: GeneratedCoreProxyClients(bridge),
      );
      final bytes = Uint8List.fromList([0, 255, 195, 169, 27, 91]);
      final received = process.output.first;
      bridge.events.add(
        CoreEvent.raw(
          requestId: null,
          target: 'servicesRuntimeTerminalService',
          propertyName: 'terminalPtyOutput',
          kind: 'Changed',
          valueBytes: encodeCoreLink(bytes),
          decodeValue: (_) =>
              throw StateError('Binary output must use typed decoding'),
        ),
      );
      expect(await received, bytes);
      process.write(bytes);
      await Future<void>.delayed(Duration.zero);
      expect(bridge.writtenBytes, bytes);
      expect(bridge.calls, ['writeTerminalPty']);
      process.kill();
      await bridge.events.close();
    },
  );

  test('closing an exited terminal releases its host session once', () async {
    final bridge = _TerminalLifecycleBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    final process = attachWorkspacePty('session', clients: clients);
    final outputDone = process.output.drain<void>();

    await bridge.events.close();
    expect(await process.exitCode, 7);
    await outputDone;
    expect(bridge.closed, isFalse);
    expect(bridge.calls, ['pollTerminalPtyExit']);

    await WorkspaceTerminalSessions(
      clients: clients,
    ).closePtySession('session');
    expect(bridge.calls, ['pollTerminalPtyExit', 'closeTerminalPty']);
    expect(bridge.closed, isTrue);
  });

  test('detaching a view leaves its host session alive', () async {
    final bridge = _TerminalLifecycleBridge();
    final process = attachWorkspacePty(
      'session',
      clients: GeneratedCoreProxyClients(bridge),
    );
    final outputDone = process.output.drain<void>();

    process.kill();
    await bridge.events.close();
    await outputDone;

    expect(await process.exitCode, -1);
    expect(bridge.calls, isEmpty);
    expect(bridge.closed, isFalse);
  });
}
