import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/contributions/EmbeddedPluginSurfaceCatalog.dart';
import 'package:operit2/ui/common/contributions/EmbeddedPluginSurfaceHost.dart';
import 'package:operit2/ui/common/contributions/PluginWorkspacePageHost.dart';
import 'package:operit2/ui/common/contributions/PluginWorkspacePresentation.dart';
import 'package:operit2/ui/features/chat/components/WorkspaceShell.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';

import 'plugin_workspace_presentation_test.dart' as catalog_test;
import 'support/compose_session_fixture.dart';

/// Exercises real launcher sessions while leaving rendering inside the plugin boundary.
class SurfaceBridge extends catalog_test.CatalogBridge {
  int opened = 0;
  int released = 0;
  late final ComposeSessionFixture fixture = ComposeSessionFixture(
    render: (state) => {
      'type': 'Button',
      'props': {
        'onClick': {'__actionId': 'open'},
      },
      'children': [
        {
          'type': 'Text',
          'props': {'text': 'revision-${state['revision']}'},
          'children': <Object?>[],
          'slots': <String, Object?>{},
        },
      ],
      'slots': <String, Object?>{},
    },
    action: (state, action) async {
      if (surface != 'workspace') return {'revision': state['revision']};
      fixture.publishNavigation(fixture.commands.last.requestId, 'route', {
        'revision': state['revision'],
      });
      return null;
    },
  );

  /// Uses the generated transport for both catalog and page-session methods.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object? result;
    switch (request.methodName) {
      case 'openComposeDslSession':
        opened++;
        result = fixture.open();
      case 'acquireToolPkgExecutionEngine':
        result = null;
      case 'releaseToolPkgExecutionEngine':
        released++;
        result = null;
      case 'getToolPkgComposeDslScript':
      case 'readToolPkgTextResource':
        result = 'export default function render() {}';
      case 'close':
        result = null;
      default:
        return super.callBytes(request);
    }
    return encodeCoreLink([0, result]);
  }

  /// Supplies the actual retained Compose watch protocol.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      fixture.updates(request);

  /// Supplies the actual retained Compose command protocol.
  @override
  Future<CorePushSink> push(CorePushRequest request) => fixture.submit(request);
}

/// Flushes asynchronous stream cancellation as well as scheduled widget frames.
Future<void> flush(WidgetTester tester) async {
  for (var i = 0; i < 3; i++) {
    await tester.pump();
    await tester.runAsync(() => Future<void>.delayed(Duration.zero));
  }
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
}

/// Covers lifecycle behavior through mounted production hosts, not catalog-only mocks.
void main() {
  testWidgets(
    'embedded input updates preserve sessions; reload and disable revoke them',
    (tester) async {
      final bridge = SurfaceBridge()..surface = 'chat_message_after';
      final clients = GeneratedCoreProxyClients(bridge);
      final catalog = EmbeddedPluginSurfaceCatalog(
        clients: clients,
        surfaces: const {'chat_message_after'},
      );
      Object? result;

      /// Rebuilds the same mounted entry with fresh receiving host inputs.
      Widget host(int revision) => MaterialApp(
        home: Scaffold(
          body: EmbeddedPluginSurfaceHost(
            catalog: catalog,
            surface: 'chat_message_after',
            instanceIdentity: 'message-a',
            hostState: {'revision': revision},
            onActionResult: (_, value) => result = value,
          ),
        ),
      );

      catalog.setLanguage('en');
      await tester.pumpWidget(host(1));
      await flush(tester);
      expect(find.text('revision-1'), findsOneWidget);
      expect(bridge.opened, 1);
      await tester.pumpWidget(host(2));
      await flush(tester);
      expect(find.text('revision-2'), findsOneWidget);
      expect(bridge.opened, 1);
      await tester.tap(find.text('revision-2'));
      await flush(tester);
      expect(result, {'revision': 2});
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
      await flush(tester);
      expect(bridge.opened, 1);
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: true);
      await flush(tester);
      expect(bridge.opened, 2);
      expect(bridge.released, 1);
      bridge.enabled = false;
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
      await flush(tester);
      expect(find.text('revision-2'), findsNothing);
      expect(bridge.released, 2);
      await tester.pumpWidget(const SizedBox.shrink());
      await flush(tester);
      catalog.dispose();
      await bridge.fixture.close();
    },
  );

  testWidgets(
    'workspace follows live host context and routes ordinary navigation without an action result',
    (tester) async {
      final bridge = SurfaceBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      final presentation = (await PluginWorkspacePresentation.fromNavigation(
        clients: clients,
        sourceOwnerPackageName: 'owner',
        routeId: 'route',
        args: const {},
        language: 'en',
        hostState: const {'revision': 0},
      ))!;
      final session = WorkspaceShell.session;
      final previousOpen = session.onOpenRequested;
      var openRequests = 0;
      session.onOpenRequested = () => openRequests++;
      addTearDown(() {
        session.onOpenRequested = previousOpen;
        session.tabs.removeWhere((tab) => tab.pluginPresentation != null);
        session.selectedIndex = 0;
      });

      /// Retains the page identity while supplying the current workspace session state.
      Widget host(int revision) => MaterialApp(
        home: Scaffold(
          body: PluginWorkspacePageHost(
            presentation: presentation,
            workspacePath: '/workspace',
            hostState: {'revision': revision},
          ),
        ),
      );

      await tester.pumpWidget(host(1));
      await flush(tester);
      expect(find.text('revision-1'), findsOneWidget);
      await tester.pumpWidget(host(3));
      await flush(tester);
      expect(find.text('revision-3'), findsOneWidget);
      expect(bridge.opened, 1);
      await tester.tap(find.text('revision-3'));
      await flush(tester);
      final opened = session.tabs.last.pluginPresentation!;
      expect(opened.input, {'revision': 3});
      expect(opened.hostState['revision'], 3);
      expect(openRequests, 1);
      // The same route updates the existing tab and also works after the UI command ends.
      final lastCommand = bridge.fixture.commands.last;
      final tabCount = session.tabs.length;
      bridge.fixture.publishNavigation(lastCommand.requestId, 'route', {
        'revision': 4,
      });
      await flush(tester);
      expect(session.tabs.length, tabCount);
      expect(session.tabs.last.pluginPresentation!.input, {'revision': 4});
      expect(openRequests, 2);
      bridge.enabled = false;
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
      await flush(tester);
      expect(find.text('revision-3'), findsNothing);
      expect(bridge.released, 1);
      bridge.fixture.publishNavigation(lastCommand.requestId, 'route', {
        'revision': 5,
      });
      await flush(tester);
      expect(openRequests, 2);
      expect(session.tabs.last.pluginPresentation!.input, {'revision': 4});
      await tester.pumpWidget(const SizedBox.shrink());
      await flush(tester);
      await bridge.fixture.close();
    },
  );
}
