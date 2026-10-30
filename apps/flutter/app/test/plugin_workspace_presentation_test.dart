import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/common/contributions/EmbeddedPluginSurfaceCatalog.dart';
import 'package:operit2/ui/common/contributions/PluginWorkspacePresentation.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceSession.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';

/// Supplies real Link-encoded registration responses with controlled revocation.
class CatalogBridge extends OperitRuntimeBridge {
  String owner = 'owner';
  String surface = 'workspace';
  String routeRuntime = 'compose_dsl';
  bool enabled = true;
  bool duplicate = false;
  int ownersRead = 0;
  Completer<void>? gate;

  /// Models enabled declarations without any plugin business or UI implementation.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'getToolPkgNavigationEntries' && gate != null)
      await gate!.future;
    final Object? value;
    switch (request.methodName) {
      case 'getToolPkgNavigationEntries':
        value = [
          if (enabled)
            core.ToolPkgNavigationEntry(
              containerPackageName: owner,
              toolPkgId: owner,
              entryId: 'page',
              routeId: 'route',
              params: null,
              surface: surface,
              title: 'Page',
              description: '',
              action: null,
              icon: null,
              order: 1,
            ).toJson(),
        ];
      case 'getToolPkgUiRoutes':
        final route = core.ToolPkgUiRoute(
          containerPackageName: owner,
          toolPkgId: owner,
          routeId: 'route',
          uiModuleId: 'page',
          runtime: routeRuntime,
          screen: 'ui/page.js',
          title: 'Page',
          description: '',
          moduleSpec: const {},
          keepAlive: true,
        ).toJson();
        value = [if (enabled) route, if (duplicate) route];
      case 'getToolPkgContainerRuntime':
        ownersRead++;
        value = enabled ? runtime(owner).toJson() : null;
      default:
        throw StateError('Unexpected catalog method: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  /// Rejects unrelated stream setup so catalog tests cannot silently depend on it.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects unrelated snapshot reads.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects unrelated upload streams.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}

/// Provides complete runtime metadata using the same public proxy model as production.
core.ToolPkgContainerRuntime runtime(String owner) =>
    core.ToolPkgContainerRuntime(
      packageName: owner,
      displayName: const core.LocalizedText(values: {'en': 'Owner'}),
      description: const core.LocalizedText(values: {'en': 'Test'}),
      version: '1',
      apiVersion: '2.0.0',
      publicApi: null,
      publicApis: const [],
      requires: const [],
      dependencyIssues: const [],
      manifestExtensions: const {},
      author: const [],
      mainEntry: 'main.js',
      themes: const [],
      license: null,
      sourceType: core.ToolPkgSourceType.externalValue,
      sourcePath: 'test',
      subpackages: const [],
      resources: const [],
      wasmModules: const [],
      workflowTemplates: const [],
      workspaceTemplates: const [],
      uiModules: const [],
      uiRoutes: const [],
      chatComposerSlots: const [],
      navigationEntries: const [],
      desktopWidgets: const [],
      appLifecycleHooks: const [],
      messageProcessingPlugins: const [],
      xmlRenderPlugins: const [],
      inputMenuTogglePlugins: const [],
      chatInputHooks: const [],
      chatViewHooks: const [],
      chatMessageHooks: const [],
      chatLifecycleHooks: const [],
      chatMessageMenuItems: const [],
      chatRuntimeHooks: const [],
      chatResponseHooks: const [],
      hostEventHooks: const [],
      toolLifecycleHooks: const [],
      promptInputHooks: const [],
      promptHistoryHooks: const [],
      promptEstimateHistoryHooks: const [],
      systemPromptComposeHooks: const [],
      toolPromptComposeHooks: const [],
      promptFinalizeHooks: const [],
      promptEstimateFinalizeHooks: const [],
      summaryGenerateHooks: const [],
      coreCommands: const [],
      aiProviders: const [],
      manifestExtensionHandlers: const [],
      logoResource: null,
      marketOrigin: null,
    );

/// Constructs a plugin-local workspace request while keeping its payload opaque.
PluginWorkspaceRequest request([Object? input]) =>
    PluginWorkspaceRequest.fromActionResult({
      'type': 'toolpkg.ui.open',
      'placement': 'workspace',
      'routeId': 'route',
      'instanceKey': 'shared-key',
      'input': input,
    })!;

/// Resolves the same authenticated source path used by message and workspace hosts.
Future<PluginWorkspacePresentation> resolve(
  CatalogBridge bridge, [
  Object? input,
]) => PluginWorkspacePresentation.resolve(
  clients: GeneratedCoreProxyClients(bridge),
  sourceOwnerPackageName: 'owner',
  request: request(input),
  language: 'en',
  hostState: const {'chatId': 'chat-a'},
);

/// Verifies routing boundaries, page reuse and live owner revocation.
void main() {
  test(
    'workspace actions detach opaque JSON and ignore unrelated commands',
    () {
      final input = {
        'nested': <Object?>[1, true, null],
      };
      final parsed = request(input);
      input['nested']!.add('later');
      expect(parsed.input, {
        'nested': [1, true, null],
      });
      expect(
        PluginWorkspaceRequest.fromActionResult({
          'type': 'toolpkg.message.rollback',
        }),
        isNull,
      );
      expect(
        () => PluginWorkspaceRequest.fromActionResult({
          'type': 'toolpkg.ui.open',
          'placement': 'workspace',
        }),
        throwsFormatException,
      );
    },
  );

  test(
    'exact enabled owner receives opaque input separately from host state',
    () async {
      final bridge = CatalogBridge();
      final page = await resolve(bridge, {'any': 'plugin-data'});
      expect(page.hostState, {'chatId': 'chat-a'});
      expect(page.request.input, {'any': 'plugin-data'});
      expect(bridge.ownersRead, 1);
      expect(
        () => page.hostState['chatId'] = 'replacement',
        throwsUnsupportedError,
      );
    },
  );

  test(
    'foreign, disabled, wrong-surface, native and ambiguous routes are rejected',
    () async {
      for (final bridge in [
        CatalogBridge()..owner = 'other',
        CatalogBridge()..enabled = false,
        CatalogBridge()..surface = 'toolbox',
        CatalogBridge()..routeRuntime = 'native',
        CatalogBridge()..duplicate = true,
      ]) {
        await expectLater(resolve(bridge), throwsStateError);
        expect(bridge.ownersRead, 0);
      }
    },
  );

  test(
    'workspace pages reuse the exact namespaced instance in either split pane',
    () async {
      final bridge = CatalogBridge();
      final first = await resolve(bridge, {'revision': 1});
      final next = await resolve(bridge, {'revision': 2});
      final session = WorkspaceSession();
      var opens = 0;
      session.onOpenRequested = () => opens++;
      session.openPluginPage(first);
      expect(session.tabs.length, 2);
      session.openPluginPage(next);
      expect(session.tabs.length, 2);
      expect(session.tabs.last.pluginPresentation, same(next));
      session.secondaryTabs.add(session.tabs.removeLast());
      session.splitAxis = WorkspaceSplitAxis.horizontal;
      session.openPluginPage(first);
      expect(session.tabs.length, 1);
      expect(session.secondaryTabs.single.pluginPresentation, same(first));
      expect(opens, 3);
      expect(session.secondaryTabs.single.kind, WorkspaceTabKind.plugin);
      session.dispose();
    },
  );

  test(
    'shared catalogs remove disabled entries and ignore completions after disposal',
    () async {
      final bridge = CatalogBridge();
      final catalog = EmbeddedPluginSurfaceCatalog(
        clients: GeneratedCoreProxyClients(bridge),
        surfaces: const {'workspace'},
      );
      final loaded = Completer<void>();
      catalog.addListener(() {
        if (!loaded.isCompleted) loaded.complete();
      });
      catalog.setLanguage('en');
      await loaded.future;
      expect(catalog.entries.length, 1);
      bridge.enabled = false;
      final removed = Completer<void>();
      catalog.addListener(() {
        if (!removed.isCompleted) removed.complete();
      });
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
      await removed.future;
      expect(catalog.entries, isEmpty);
      bridge.gate = Completer<void>();
      catalog.setLanguage('zh');
      catalog.dispose();
      bridge.gate!.complete();
      await Future<void>.delayed(Duration.zero);
      expect(catalog.entries, isEmpty);
    },
  );
}
