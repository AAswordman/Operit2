import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/chat/components/MessageContextMenu.dart';

/// Verifies every selected reply can be deleted when another revision exists.
void main() {
  test(
    'copy preserves plugin-owned markup without interpreting its business',
    () {
      const text = '<memory>Plugin-owned payload</memory>\nVisible message';
      expect(cleanMessageContent(text), text);
    },
  );

  for (final sender in ['user', 'ai']) {
    testWidgets('native $sender menu has no hardcoded plugin business', (
      tester,
    ) async {
      await _openMenu(
        tester,
        sender: sender,
        selectedVariantIndex: 0,
        variantCount: 1,
      );
      expect(find.text('加入记忆队列'), findsNothing);
      expect(find.text('修改记忆'), findsNothing);
    });
  }

  testWidgets(
    'forwards arbitrary registered action and opaque message namespace',
    (tester) async {
      final bridge = _MessageMenuBridge(withPluginAction: true);
      await _openMenu(
        tester,
        bridge: bridge,
        selectedVariantIndex: 2,
        variantCount: 3,
      );
      await tester.tap(find.text('Opaque plugin action'));
      await tester.pumpAndSettle();
      expect(bridge.invocations, hasLength(1));
      final args = bridge.invocations.single;
      expect(args['containerPackageName'], 'arbitrary.plugin');
      expect(args['itemId'], 'opaque_action');
      expect(args['chatId'], 'chat');
      final message = args['message'] as Map;
      expect(message['timestamp'], 123);
      expect(message['selectedVariantIndex'], 2);
      expect(message['pluginExtensions'], {
        'arbitrary.plugin': {'opaque': 'unchanged'},
      });
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'does not offer variant deletion for a reply without alternates',
    (tester) async {
      await _openMenu(tester, selectedVariantIndex: 0, variantCount: 1);

      expect(find.text('删除当前变体'), findsNothing);
      expect(find.text('删除'), findsOneWidget);
    },
  );

  for (final selectedVariantIndex in <int>[0, 1, 2]) {
    testWidgets(
      'deletes selected revision $selectedVariantIndex before refreshing',
      (tester) async {
        final actions = <String>[];
        await _openMenu(
          tester,
          selectedVariantIndex: selectedVariantIndex,
          variantCount: 3,
          onDeleteMessageVariant: (timestamp, variantIndex) async {
            actions.add('delete:$timestamp:$variantIndex');
          },
          onRefresh: () async {
            actions.add('refresh');
          },
        );

        expect(find.text('删除当前变体'), findsOneWidget);
        await tester.tap(find.text('删除当前变体'));
        await tester.pumpAndSettle();

        expect(actions, <String>[
          'delete:123:$selectedVariantIndex',
          'refresh',
        ]);
        expect(tester.takeException(), isNull);
      },
    );
  }
}

/// Opens the real long-press menu for a message with explicit variant metadata.
Future<void> _openMenu(
  WidgetTester tester, {
  required int selectedVariantIndex,
  required int variantCount,
  String sender = 'ai',
  _MessageMenuBridge? bridge,
  MessageVariantAction? onDeleteMessageVariant,
  Future<void> Function()? onRefresh,
}) async {
  await tester.binding.setSurfaceSize(const Size(800, 1200));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  final clients = GeneratedCoreProxyClients(bridge ?? _MessageMenuBridge());
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Center(
          child: MessageContextMenu(
            message: ChatMessage(
              pluginExtensions: const {
                'arbitrary.plugin': {'opaque': 'unchanged'},
              },
              sender: sender,
              parts: const <MessagePart>[],
              timestamp: 123,
              roleName: 'assistant',
              selectedVariantIndex: selectedVariantIndex,
              variantCount: variantCount,
              provider: 'test',
              modelName: 'test',
              inputTokens: 0,
              outputTokens: 0,
              cachedInputTokens: 0,
              sentAt: 0,
              outputDurationMs: 0,
              waitDurationMs: 0,
              completedAt: 1,
              displayMode: ChatMessageDisplayMode.normal,
              isFavorite: false,
              contentStream: null,
            ),
            chatId: 'chat',
            messageIndex: 0,
            clients: clients,
            packageManager: clients.application.packageManager(),
            onToggleFavoriteMessage: (timestamp, isFavorite) async {},
            onDeleteMessageVariant: onDeleteMessageVariant,
            onRefresh: onRefresh,
            child: const SizedBox(
              width: 200,
              height: 60,
              child: Text('Assistant reply'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.longPress(find.text('Assistant reply'));
  await tester.pumpAndSettle();
  expect(find.text(sender == 'ai' ? '重新生成' : '编辑并重发'), findsOneWidget);
}

/// Supplies an empty extension menu through the actual Core bridge codec.
class _MessageMenuBridge extends OperitRuntimeBridge {
  _MessageMenuBridge({this.withPluginAction = false});
  final bool withPluginAction;
  final invocations = <Map<String, Object?>>[];

  /// Returns only the extension-menu response expected by this fixture.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.target, 'core/application.packageManager');
    switch (request.methodName) {
      case 'getToolPkgChatMessageMenuItems':
        return encodeCoreLink(<Object?>[
          0,
          [
            if (withPluginAction)
              const ToolPkgChatMessageMenuItem(
                containerPackageName: 'arbitrary.plugin',
                itemId: 'opaque_action',
                title: 'Opaque plugin action',
                icon: null,
                order: 1,
                dialog: null,
              ).toJson(),
          ],
        ]);
      case 'invokeToolPkgChatMessageMenuItem':
        invocations.add(Map<String, Object?>.from(request.args as Map));
        return encodeCoreLink(<Object?>[0, null]);
      default:
        throw StateError(
          'Unexpected host menu operation: ${request.methodName}',
        );
    }
  }

  /// Rejects push operations outside this menu fixture.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshot operations outside this menu fixture.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects stream operations outside this menu fixture.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
