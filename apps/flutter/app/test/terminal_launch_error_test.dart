import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalLaunch.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalSessions.dart';

class _FailingTerminalBridge extends OperitRuntimeBridge {
  final calls = <CoreCallRequest>[];
  final failure = const CoreLinkError(
    code: 'BACKEND',
    message: '/virtual/linux is not mounted',
  );

  /// Records the exact terminal request and returns its backend failure unchanged.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    throw failure;
  }

  /// Rejects bridge methods outside this focused terminal-start test.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

/// Verifies that manual terminal failures retain their own UI and exact host selection.
void main() {
  for (final terminal in ['proot', 'ish']) {
    testWidgets('$terminal startup failure stays in the terminal dialog', (
      tester,
    ) async {
      final bridge = _FailingTerminalBridge();
      final sessions = WorkspaceTerminalSessions(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final terminalType = terminal == 'proot' ? 'bash' : 'shell';
      Future<void>? launch;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () {
                  launch = launchWorkspaceTerminal(
                    context: context,
                    terminal: terminal,
                    terminalType: terminalType,
                    launch: () async {
                      await sessions.startPtySession(
                        sessionName: 'manual-test',
                        terminal: terminal,
                        terminalType: terminalType,
                        workingDirectory: '/virtual/linux/root',
                        rows: 24,
                        columns: 80,
                      );
                    },
                  );
                },
                child: const Text('Open terminal'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Open terminal'));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('终端启动失败'), findsOneWidget);
      expect(
        find.text(
          '$terminal / $terminalType\n\nBACKEND: /virtual/linux is not mounted',
        ),
        findsOneWidget,
      );
      expect(bridge.calls, hasLength(1));
      expect(bridge.calls.single.methodName, 'startTerminalPty');
      expect(bridge.calls.single.args, <String, Object?>{
        'sessionName': 'manual-test',
        'terminal': terminal,
        'terminalType': terminalType,
        'workingDir': '/virtual/linux/root',
        'rows': 24,
        'cols': 80,
      });
      expect(tester.takeException(), isNull);

      await tester.tap(find.text('关闭'));
      await tester.pumpAndSettle();
      await launch;
      expect(find.byType(AlertDialog), findsNothing);
    });
  }

  testWidgets('successful terminal launch has no error surface', (
    tester,
  ) async {
    var starts = 0;
    Future<void>? launch;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () {
              launch = launchWorkspaceTerminal(
                context: context,
                terminal: 'native',
                terminalType: 'bash',
                launch: () async {
                  starts += 1;
                },
              );
            },
            child: const Text('Open terminal'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open terminal'));
    await tester.pumpAndSettle();
    await launch;
    expect(starts, 1);
    expect(find.byType(AlertDialog), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('disposing the entry point does not leak its startup failure', (
    tester,
  ) async {
    final pending = Completer<void>();
    Future<void>? launch;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () {
              launch = launchWorkspaceTerminal(
                context: context,
                terminal: 'proot',
                terminalType: 'bash',
                launch: () => pending.future,
              );
            },
            child: const Text('Open terminal'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open terminal'));
    await tester.pumpWidget(const SizedBox.shrink());
    pending.completeError(StateError('/virtual/linux is not mounted'));
    await tester.pump();
    await launch;
    expect(find.byType(AlertDialog), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
