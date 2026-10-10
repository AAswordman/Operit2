import 'dart:async';
import 'dart:typed_data';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/packages/screens/PackageManagerScreen.dart';
import 'package:operit2/ui/features/packages/dialogs/PackageEnvironmentVariablesDialog.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';

/// Verifies installation notifications update an already mounted package list.
void main() {
  testWidgets('catalog change refreshes list without reopening or rescanning', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsNothing);
    final state = tester.state(find.byType(PackageManagerScreen));
    bridge.installed = true;
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(tester.state(find.byType(PackageManagerScreen)), same(state));
    expect(bridge.scans, 1);
    bridge.installed = false;
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsNothing);
    await tester.pumpWidget(const SizedBox());
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('cached manager stops catalog work and refreshes on activation', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    final active = ValueNotifier<bool>(true);
    addTearDown(active.dispose);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: ValueListenableBuilder<bool>(
          valueListenable: active,
          child: PackageManagerScreen(
            clients: GeneratedCoreProxyClients(bridge),
          ),
          builder: (context, visible, child) =>
              RetainedPage(active: visible, child: child!),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final state = tester.state(find.byType(PackageManagerScreen));
    expect(bridge.watchers, 1);
    active.value = false;
    await tester.pumpAndSettle();
    expect(bridge.watchers, 0);
    final reads = bridge.catalogReads;
    bridge.installed = true;
    for (var i = 0; i < 4; i += 1) {
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await tester.pump(const Duration(milliseconds: 400));
    }
    expect(bridge.catalogReads, reads);
    expect(
      tester.state(find.byType(PackageManagerScreen, skipOffstage: false)),
      same(state),
    );
    active.value = true;
    await tester.pumpAndSettle();
    expect(bridge.watchers, 1);
    expect(bridge.catalogReads, greaterThan(reads));
    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(tester.state(find.byType(PackageManagerScreen)), same(state));
    await tester.pumpWidget(const SizedBox.shrink());
    expect(bridge.watchers, 0);
    expect(tester.takeException(), isNull);
  });

  testWidgets('package manager action opens the runtime environment editor', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
      ),
    );
    await tester.pumpAndSettle();
    final action = find.ancestor(
      of: find.byIcon(Icons.settings_outlined),
      matching: find.byType(FloatingActionButton),
    );
    expect(action, findsOneWidget);
    await tester.tap(action);
    await tester.pumpAndSettle();
    expect(find.byType(PackageEnvironmentVariablesDialog), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(PackageEnvironmentVariablesDialog),
        matching: find.byType(TextField),
      ),
      findsNothing,
    );
    expect(bridge.environmentCatalogReads, 1);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });
}

class _CatalogBridge extends OperitRuntimeBridge {
  /// Rejects push operations outside this catalog fixture.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshot watches outside this catalog fixture.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Exposes the catalog revision stream with observable listener ownership.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'extensionCatalogRevisionFlow') {
      throw StateError('Unexpected catalog watch: ${request.propertyName}');
    }
    return Stream<CoreEvent>.multi((controller) {
      watchers += 1;
      controller.onCancel = () => watchers -= 1;
    });
  }

  bool installed = false;
  int scans = 0;
  int watchers = 0;
  int catalogReads = 0;
  int environmentCatalogReads = 0;

  /// Encodes catalog responses with the native bridge envelope.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[0, await call(request)]);

  /// Returns the catalog state available at the time of each request.
  @override
  Future<Object?> call(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'loadAvailablePackages':
        scans++;
        return null;
      case 'getAvailablePackages':
        environmentCatalogReads++;
        return <String, Object?>{};
      case 'getExtensionScopes':
        return installed
            ? <String, Object?>{'demo_toolpkg': 'device'}
            : <String, Object?>{};
      case 'getExecutableAvailablePackages':
        return <String, Object?>{};
      case 'getToolPkgContainerRuntimes':
        catalogReads += 1;
        return installed ? [_pluginRuntime().toJson()] : [];
      case 'getEnabledPackageNames':
      case 'getToolPkgContainerOrder':
      case 'getBundledExternalPackageCandidates':
      case 'getBundledExternalToolPkgContainerRuntimes':
      case 'getToolPkgLoadIssues':
        return [];
    }
    throw StateError('Unexpected catalog call: ${request.methodName}');
  }
}

/// Creates an installed plugin fixture for catalog refresh assertions.
core_proxy.ToolPkgContainerRuntime _pluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    chatLifecycleHooks: [],
    packageName: 'demo_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Demo ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'DSL test package'},
    ),
    version: '1.0.0',
    apiVersion: '2.0.0',
    publicApi: null,
    publicApis: <core_proxy.ToolPkgRegisteredFunctionHook>[],
    requires: <core_proxy.ToolPkgManifestRequirement>[],
    dependencyIssues: <core_proxy.ToolPkgDependencyIssue>[],
    manifestExtensions: <String, Object?>{},
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'main',
        runtime: 'compose_dsl',
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[
      core_proxy.ToolPkgUiRouteRuntime(
        id: 'main',
        routeId: 'main',
        runtime: 'compose_dsl',
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    chatComposerSlots: <core_proxy.ToolPkgChatComposerSlotRuntime>[],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageMenuItems: <core_proxy.ToolPkgChatMessageMenuItemRuntime>[],
    chatRuntimeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    coreCommands: <core_proxy.ToolPkgCoreCommandRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
    manifestExtensionHandlers:
        <core_proxy.ToolPkgRegisteredManifestExtension>[],
    logoResource: null,
    marketOrigin: null,
  );
}
