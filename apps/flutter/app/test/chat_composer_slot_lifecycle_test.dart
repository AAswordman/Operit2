import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatComposerSlotHost.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'embedded_plugin_surface_lifecycle_test.dart' as surface;
import 'plugin_workspace_presentation_test.dart' as catalog;

/// Uses real Compose sessions behind a registered composer contribution.
class _ComposerBridge extends surface.SurfaceBridge {
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'createSlotDefinitions') {
      return encodeCoreLink([
        0,
        [
          core.ToolPkgChatComposerSlot(
            containerRuntime: catalog.runtime('owner'),
            containerPackageName: 'owner',
            contributionId: 'goal',
            slot: 'above_input',
            screen: 'ui/goal.js',
            screenExport: null,
            order: 0,
            keepAlive: true,
          ).toJson(),
        ],
      ]);
    }
    return super.callBytes(request);
  }
}

void main() {
  testWidgets(
    'processing transitions update inputs without reopening composer streams',
    (tester) async {
      final bridge = _ComposerBridge();
      final viewModel = ChatViewModel(bridge: bridge);
      Widget host(String chatId, bool processing, int queue) => OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: ChatComposerSlotHost(
            viewModel: viewModel,
            chatId: chatId,
            isProcessing: processing,
            pendingQueueCount: queue,
          ),
        ),
      );

      await tester.pumpWidget(host('chat-a', false, 0));
      await surface.flush(tester);
      expect(bridge.opened, 1);
      for (var turn = 0; turn < 4; turn++) {
        await tester.pumpWidget(host('chat-a', turn.isEven, turn));
        await surface.flush(tester);
        expect(bridge.opened, 1);
        expect(bridge.released, 0);
        final render = bridge.fixture.commands.lastWhere(
          (command) => command.operation == 'render',
        );
        expect(
          render.runtimeOptions['__operit_input_state'],
          containsPair('isProcessing', turn.isEven),
        );
        expect(
          render.runtimeOptions['__operit_input_state'],
          containsPair('pendingQueueCount', turn),
        );
      }
      await tester.pumpWidget(host('chat-b', false, 0));
      await surface.flush(tester);
      expect(bridge.opened, 2);
      expect(bridge.released, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      await surface.flush(tester);
      expect(bridge.released, 2);
      await bridge.fixture.close();
    },
  );
}
