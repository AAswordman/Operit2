import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatComposerSlotHost.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'embedded_plugin_surface_lifecycle_test.dart' as surface;
import 'plugin_workspace_presentation_test.dart' as catalog;

/// Uses real Compose sessions behind a registered composer contribution.
class _ComposerBridge extends surface.SurfaceBridge {
  int slotQueries = 0;
  bool failSlotQueries = false;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'createSlotDefinitions') {
      slotQueries++;
      if (failSlotQueries) {
        throw StateError('slot query transport failure');
      }
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

/// Wraps the slot host with the same themed shell used by the chat surface.
Widget _slotHost(ChatViewModel viewModel, {String chatId = 'chat-a'}) =>
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: ChatComposerSlotHost(
          viewModel: viewModel,
          chatId: chatId,
          isProcessing: false,
          pendingQueueCount: 0,
        ),
      ),
    );

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

  testWidgets(
    'catalog changes reload composer slots without a chat identity change',
    (tester) async {
      final bridge = _ComposerBridge();
      final viewModel = ChatViewModel(bridge: bridge);
      await tester.pumpWidget(_slotHost(viewModel));
      await surface.flush(tester);
      expect(bridge.slotQueries, 1);
      // Release builds never fire PluginHotReload; the catalog bus must refresh
      // the open chat surface for installs, enables, and disables alike.
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
      await surface.flush(tester);
      expect(bridge.slotQueries, 2);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await surface.flush(tester);
      expect(bridge.slotQueries, 3);
      await tester.pumpWidget(const SizedBox.shrink());
      await surface.flush(tester);
      await bridge.fixture.close();
    },
  );

  testWidgets('disposing the slot host unsubscribes from catalog changes', (
    tester,
  ) async {
    final bridge = _ComposerBridge();
    final viewModel = ChatViewModel(bridge: bridge);
    await tester.pumpWidget(_slotHost(viewModel));
    await surface.flush(tester);
    expect(bridge.slotQueries, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    await surface.flush(tester);
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    // surface.flush asserts no pending FlutterError, catching any
    // setState-after-dispose triggered by a leaked bus subscription.
    await surface.flush(tester);
    expect(bridge.slotQueries, 1);
    await bridge.fixture.close();
  });

  testWidgets(
    'slot query failures render a visible error and recover on catalog change',
    (tester) async {
      final bridge = _ComposerBridge()..failSlotQueries = true;
      final viewModel = ChatViewModel(bridge: bridge);
      await tester.pumpWidget(_slotHost(viewModel));
      await surface.flush(tester);
      expect(bridge.slotQueries, 1);
      expect(find.text('Page load failed'), findsOneWidget);
      bridge.failSlotQueries = false;
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await surface.flush(tester);
      expect(bridge.slotQueries, 2);
      expect(find.text('Page load failed'), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
      await surface.flush(tester);
      await bridge.fixture.close();
    },
  );
}
