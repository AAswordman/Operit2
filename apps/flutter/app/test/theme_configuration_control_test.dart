import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/appearance/ThemeConfigurationControl.dart';
import 'package:operit_ui/operit_ui.dart' show buildOperitThemeData;

/// Verifies the real named-theme selector and generated proxy calls without a platform runtime.
void main() {
  testWidgets(
    'creation exposes grouped sources and changes the explicit selection',
    (tester) async {
      final bridge = _ThemeBridge()..packages.add(_templatePackage());
      await _mount(tester, bridge, size: const Size(1000, 800));
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      final dialog = find.byType(Dialog);
      expect(
        find.descendant(
          of: dialog,
          matching: find.byType(DropdownButtonFormField<String>),
        ),
        findsNothing,
      );
      expect(find.text('Default appearance'), findsOneWidget);
      expect(find.text('Current appearance'), findsOneWidget);
      expect(find.text('Inherit a saved configuration'), findsOneWidget);
      expect(find.text('Use a plugin template'), findsOneWidget);
      expect(find.text('Plugin / night'), findsOneWidget);
      final defaults = find.byKey(const ValueKey('theme-source-defaults'));
      final current = find.byKey(const ValueKey('theme-source-current'));
      expect(
        find.descendant(
          of: defaults,
          matching: find.byIcon(Icons.check_circle),
        ),
        findsOneWidget,
      );
      await tester.tap(current);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: current, matching: find.byIcon(Icons.check_circle)),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: defaults,
          matching: find.byIcon(Icons.check_circle),
        ),
        findsNothing,
      );
      await tester.enterText(find.byType(TextFormField), 'Current copy');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.last.name, 'Current copy');
      expect(
        bridge.calls.where((call) => call.methodName == 'getPreferences'),
        hasLength(1),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'narrow creation keeps actions visible and scrolls long configuration lists',
    (tester) async {
      final bridge = _ThemeBridge();
      bridge.configs.addAll(
        List.generate(
          12,
          (index) => _config(
            'saved-$index',
            'A longer saved appearance configuration $index',
          ),
        ),
      );
      await _mount(tester, bridge, size: const Size(320, 500));
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('Create').hitTestable(), findsOneWidget);
      expect(find.text('Cancel').hitTestable(), findsOneWidget);
      final last = find.byKey(const ValueKey('theme-source-config:saved-11'));
      await tester.scrollUntilVisible(
        last,
        200,
        scrollable: find.descendant(
          of: find.byKey(const ValueKey('theme-creation-scroll')),
          matching: find.byType(Scrollable),
        ).first,
      );
      await tester.tap(last);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: last, matching: find.byIcon(Icons.check_circle)),
        findsOneWidget,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.where((call) => call.methodName == 'create'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'saved theme selection follows acknowledged catalog state and shows apply failures',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      bridge.failure = StateError('Theme owner rejected selection');
      await tester.tap(find.text('Night').last);
      await tester.pumpAndSettle();
      expect(
        find.textContaining('Theme owner rejected selection'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<DropdownButton<String>>(find.byType(DropdownButton<String>))
            .value,
        'day',
      );
      bridge.failure = null;
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Night').last);
      await tester.pumpAndSettle();
      expect(bridge.active, 'night');
      expect(
        tester
            .widget<DropdownButton<String>>(find.byType(DropdownButton<String>))
            .value,
        'night',
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'apply').last.args,
        {'id': 'night'},
      );
    },
  );

  testWidgets(
    'creating the first theme uses the declared initial appearance and selects its actual ID',
    (tester) async {
      final bridge = _ThemeBridge()
        ..configs.clear()
        ..active = null;
      await _mount(tester, bridge);
      expect(
        tester
            .widget<DropdownButton<String>>(find.byType(DropdownButton<String>))
            .value,
        '',
      );
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      expect(find.text('New theme configuration'), findsOneWidget);
      await tester.enterText(find.byType(TextFormField), 'My theme');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      final create = bridge.calls.singleWhere(
        (call) => call.methodName == 'create',
      );
      final args = create.args as Map;
      expect(args['name'], 'My theme');
      expect(
        args['snapshot'],
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      );
      expect(bridge.active, 'created');
      expect(find.text('My theme'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'new themes start from defaults and preserve the existing named appearance',
    (tester) async {
      final bridge = _ThemeBridge();
      final previous = bridge.configs.first;
      final edited = Map<String, Object?>.from(previous.snapshot)
        ..['themeMode'] = 'dark';
      bridge.configs[0] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: previous.id,
        name: previous.name,
        snapshot: edited,
        createdAt: previous.createdAt,
        updatedAt: 2,
      );
      await _mount(tester, bridge);
      await tester.tap(find.byTooltip('New theme configuration'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Fresh theme');
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.first.snapshot, edited);
      expect(
        bridge.configs.last.snapshot,
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      );
      expect(bridge.active, 'created');
      expect(
        bridge.calls.where((call) => call.methodName == 'getPreferences'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'inheriting a saved configuration copies its appearance without editing the source',
    (tester) async {
      final bridge = _ThemeBridge();
      final previous = bridge.configs.first;
      final edited = Map<String, Object?>.from(previous.snapshot)
        ..['fontScale'] = 1.4;
      bridge.configs[0] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
        id: previous.id,
        name: previous.name,
        snapshot: edited,
        createdAt: previous.createdAt,
        updatedAt: 2,
      );
      await _mount(tester, bridge);
      await tester.tap(find.byKey(const ValueKey('theme-config-create')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Inherited');
      final source = find.byKey(const ValueKey('theme-source-config:day'));
      await tester.ensureVisible(source);
      await tester.tap(source);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Create'));
      await tester.pumpAndSettle();
      expect(bridge.configs.first.snapshot, edited);
      expect(bridge.configs.last.snapshot, edited);
      expect(bridge.active, 'created');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'renaming keeps the stored snapshot and deletion lists only inactive configurations',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      final original = Map<String, Object?>.from(bridge.configs.first.snapshot);
      await tester.tap(find.byKey(const ValueKey('theme-config-rename')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Renamed day');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.singleWhere((call) => call.methodName == 'rename').args,
        {'id': 'day', 'name': 'Renamed day'},
      );
      expect(bridge.configs.first.snapshot, original);
      await tester.tap(find.byKey(const ValueKey('theme-config-delete')));
      await tester.pumpAndSettle();
      expect(find.text('Night'), findsOneWidget);
      await tester.tap(find.text('Night'));
      await tester.pumpAndSettle();
      expect(bridge.configs.map((config) => config.id), ['day']);
      expect(bridge.active, 'day');
    },
  );

  testWidgets(
    'custom appearance explicitly detaches a named theme without resetting ordinary settings',
    (tester) async {
      final bridge = _ThemeBridge();
      await _mount(tester, bridge);
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Unbound appearance').last);
      await tester.pumpAndSettle();
      expect(bridge.active, isNull);
      final call = bridge.calls.singleWhere(
        (call) => call.methodName == 'useCustomAppearance',
      );
      expect(
        (call.args as Map)['snapshot'],
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'delete'),
        isEmpty,
      );
    },
  );
}

/// Mounts a compact actual selector with the formal localization and generated proxy dependencies.
Future<void> _mount(
  WidgetTester tester,
  _ThemeBridge bridge, {
  Size size = const Size(320, 700),
  Locale locale = const Locale('en'),
}) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      locale: locale,
      theme: buildOperitThemeData(
        ColorScheme.fromSeed(seedColor: const Color(0xFFBBDEFB)),
        UserPreferencesManager.defaultThemePreferenceSnapshot,
        fontFamily: 'Aptos',
        fontFamilyFallback: const <String>[],
      ),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: ThemeConfigurationControl(
          clients: GeneratedCoreProxyClients(bridge),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
}

/// Controls only the external theme-owner boundary while the actual widget and codecs execute.
class _ThemeBridge extends OperitRuntimeBridge {
  /// Declares the two genuine configuration records supplied to the selector test.
  _ThemeBridge() : configs = [_config('day', 'Day'), _config('night', 'Night')];

  final List<core.CoreDataPreferencesThemeConfigManagerThemeConfig> configs;
  final List<Map<String, Object?>> packages = [];
  final List<CoreCallRequest> calls = [];
  final Map<CoreWatchRequest, StreamController<CoreEvent>> watches = {};
  String? active = 'day';
  Object? failure;

  /// Encodes only supported owner operations and publishes committed state after acknowledgements.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map;
    Object? result;
    switch (request.methodName) {
      case 'list':
        result = configs.map((config) => config.toJson()).toList();
      case 'getToolPkgContainerRuntimes':
        result = packages;
      case 'getPreferences':
        result = <String, String>{};
      case 'apply':
        if (failure case final Object error) throw error;
        active = args['id'] as String;
        result = configs.singleWhere((config) => config.id == active).toJson();
        _publish();
      case 'create':
        final config = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: 'created',
          name: args['name'] as String,
          snapshot: Map<String, Object?>.from(args['snapshot'] as Map),
          createdAt: 1,
          updatedAt: 1,
        );
        configs.add(config);
        result = config.toJson();
        _publish();
      case 'get':
        result = configs
            .singleWhere((config) => config.id == args['id'])
            .toJson();
      case 'rename':
        final index = configs.indexWhere((config) => config.id == args['id']);
        final old = configs[index];
        configs[index] = core.CoreDataPreferencesThemeConfigManagerThemeConfig(
          id: old.id,
          name: args['name'] as String,
          snapshot: old.snapshot,
          createdAt: old.createdAt,
          updatedAt: 2,
        );
        result = configs[index].toJson();
        _publish();
      case 'delete':
        if (args['id'] == active) {
          throw StateError('Cannot delete active configuration');
        }
        configs.removeWhere((config) => config.id == args['id']);
        result = null;
        _publish();
      case 'useCustomAppearance':
        active = null;
        result = null;
        _publish();
      default:
        throw StateError(
          'Unexpected theme owner operation: ${request.methodName}',
        );
    }
    return encodeCoreLink([0, result]);
  }

  /// Publishes the real declared owner input over the generated typed watch codec.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'watch') {
      throw StateError('Unexpected theme watch');
    }
    late StreamController<CoreEvent> controller;
    controller = StreamController<CoreEvent>(
      onListen: () => controller.add(_event(request)),
      onCancel: () => watches.remove(request),
    );
    watches[request] = controller;
    return controller.stream;
  }

  /// Emits one committed catalog update to every actual widget subscription.
  void _publish() {
    for (final entry in watches.entries) {
      entry.value.add(_event(entry.key));
    }
  }

  /// Encodes a complete typed catalog event without a stringified payload.
  CoreEvent _event(CoreWatchRequest request) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: 'Changed',
    decodeValue: decodeCoreLink<Object?>,
    valueBytes: encodeCoreLink(
      core.ThemeConfigState(
        configs: configs,
        activeThemeConfigId: active,
      ).toJson(),
    ),
  );

  /// Rejects unrelated push requests instead of simulating another host capability.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected theme push');

  /// Rejects snapshot requests because this selector observes the actual stream contract.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected theme snapshot');
}

/// Builds one complete explicit named-theme record for the external catalog fixture.
core.CoreDataPreferencesThemeConfigManagerThemeConfig _config(
  String id,
  String name,
) => core.CoreDataPreferencesThemeConfigManagerThemeConfig(
  id: id,
  name: name,
  snapshot: UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
  createdAt: 1,
  updatedAt: 1,
);

/// Supplies an ordinary plugin registry entry with a visible template declaration.
Map<String, Object?> _templatePackage() => <String, Object?>{
  'packageName': 'dev.example.plugin',
  'displayName': <String, Object?>{
    'values': <String, String>{'en': 'Plugin'},
  },
  'description': <String, Object?>{
    'values': <String, String>{'en': 'Example'},
  },
  'logoResource': null,
  'marketOrigin': null,
  'version': '1.0',
  'apiVersion': '1',
  'mainEntry': '',
  'sourceType': 'EXTERNAL',
  'sourcePath': 'host:///plugin.toolpkg',
  'themes': <Object?>[
    <String, Object?>{
      'id': 'night',
      'path': 'theme.json',
      'preview': 'preview.png',
      'files': <Object?>[],
    },
  ],
  for (final key in <String>[
    'requires',
    'author',
    'subpackages',
    'resources',
    'wasmModules',
    'workflowTemplates',
    'workspaceTemplates',
    'uiModules',
    'uiRoutes',
    'chatComposerSlots',
    'navigationEntries',
    'desktopWidgets',
    'appLifecycleHooks',
    'messageProcessingPlugins',
    'xmlRenderPlugins',
    'inputMenuTogglePlugins',
    'chatInputHooks',
    'chatViewHooks',
    'chatMessageHooks',
    'chatLifecycleHooks',
    'chatMessageMenuItems',
    'chatRuntimeHooks',
    'hostEventHooks',
    'toolLifecycleHooks',
    'promptInputHooks',
    'promptHistoryHooks',
    'promptEstimateHistoryHooks',
    'systemPromptComposeHooks',
    'toolPromptComposeHooks',
    'promptFinalizeHooks',
    'promptEstimateFinalizeHooks',
    'summaryGenerateHooks',
    'coreCommands',
    'aiProviders',
    'manifestExtensionHandlers',
  ])
    key: <Object?>[],
};
