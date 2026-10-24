import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/chat/screens/AIChatScreen.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Keeps native selection admission pending so binding can be inspected independently of transcript data.
class _ChatBindingBridge extends OperitRuntimeBridge {
  final calls = <CoreCallRequest>[];
  final bindings = <Completer<Uint8List>>[];

  /// Records the exact generated local-selection request without changing a global selection.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    calls.add(request);
    final binding = Completer<Uint8List>();
    bindings.add(binding);
    return binding.future;
  }

  /// Rejects subscriptions before the selected conversation has finished binding.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnsupportedError('binding is pending');

  /// Rejects snapshots before the selected conversation has finished binding.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnsupportedError('binding is pending');

  /// Rejects actions because these tests exercise selection and display ownership.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnsupportedError('no actions requested');
}

/// Mounts the production DSL renderer with the same explicit host client as its launcher.
Widget _chat(_ChatBindingBridge bridge, String chatId) => MaterialApp(
  home: Scaffold(
    body: buildComposeDslLayoutForTest(
      node: {
        'type': 'AiChat',
        'props': {'chatId': chatId, 'showInput': false},
      },
      hostContext: ComposeDslWebViewHostContext(
        clients: GeneratedCoreProxyClients(bridge),
        packageName: 'com.operit.subagents',
        routeInstanceId: 'subagents',
        executionContextKey: 'subagents-parent',
        dispatchAction: (id, [payload]) async => null,
        runtimeOptionsProvider: () => {},
      ),
      onTextInput: (id, text) async => null,
      splitMarkdownContent: (_) async => [],
    ),
  ),
);

/// Checks actual renderer forwarding, per-conversation binding, reuse and visible binding failures.
void main() {
  testWidgets(
    'AiChat targets the child locally and preserves its host client across updates',
    (tester) async {
      final bridge = _ChatBindingBridge();
      await tester.pumpWidget(_chat(bridge, 'child-one'));
      final chat = tester.widget<AIChatEmbed>(find.byType(AIChatEmbed));
      expect(chat.chatId, 'child-one');
      expect(chat.showInput, false);
      expect(chat.clients.bridge, same(bridge));
      expect(bridge.calls.single.methodName, 'switchChatLocal');
      expect(bridge.calls.single.args, {
        '__core_instance_id': 'embedded-chat:child-one',
        'chatId': 'child-one',
      });
      await tester.pumpWidget(_chat(bridge, 'child-one'));
      expect(bridge.calls.length, 1);
      await tester.pumpWidget(_chat(bridge, 'child-two'));
      expect(bridge.calls.length, 2);
      expect(bridge.calls.last.methodName, 'switchChatLocal');
      expect(bridge.calls.last.args, {
        '__core_instance_id': 'embedded-chat:child-two',
        'chatId': 'child-two',
      });
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'AiChat exposes a child selection error without opening the main chat',
    (tester) async {
      final bridge = _ChatBindingBridge();
      await tester.pumpWidget(_chat(bridge, 'child-one'));
      bridge.bindings.single.completeError(
        StateError('child selection failed'),
      );
      await tester.pump();
      expect(find.text('Bad state: child selection failed'), findsOneWidget);
      expect(bridge.calls.length, 1);
      expect(bridge.calls.single.methodName, 'switchChatLocal');
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}
