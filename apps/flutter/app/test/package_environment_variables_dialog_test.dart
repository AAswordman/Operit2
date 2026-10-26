import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/packages/dialogs/PackageEnvironmentVariablesDialog.dart';

/// Verifies installation ownership throughout the environment editor.
void main() {
  testWidgets('same-name variables remain independent across installations', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('space_plugin', ['TOKEN']),
        _package('device_plugin', ['TOKEN']),
      ],
      enabled: ['space_plugin', 'device_plugin'],
      environments: {
        'space_plugin': {'TOKEN': 'space-token'},
        'device_plugin': {'TOKEN': 'device-token'},
      },
    );
    await _openEditor(tester, bridge);
    expect(_controller(tester, 'space_plugin', 'TOKEN').text, 'space-token');
    expect(_controller(tester, 'device_plugin', 'TOKEN').text, 'device-token');
    expect(
      _controller(tester, 'space_plugin', 'TOKEN'),
      isNot(same(_controller(tester, 'device_plugin', 'TOKEN'))),
    );
    await tester.enterText(
      _field('space_plugin', 'TOKEN'),
      'updated-space-token',
    );
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.environments['space_plugin'], {
      'TOKEN': 'updated-space-token',
    });
    expect(bridge.environments['device_plugin'], {'TOKEN': 'device-token'});
    expect(bridge.writes, 1);
  });

  testWidgets('subpackages share their installation controller and save once', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('plugin', ['SHARED']),
        _package('child', ['SHARED']),
        _package('disabled', ['HIDDEN']),
      ],
      enabled: ['plugin', 'child'],
      owners: {'child': 'plugin'},
      environments: {
        'plugin': {'SHARED': 'shared-value'},
      },
    );
    await _openEditor(tester, bridge);
    expect(find.text('disabled'), findsNothing);
    expect(
      _controller(tester, 'child', 'SHARED'),
      same(_controller(tester, 'plugin', 'SHARED')),
    );
    await tester.enterText(_field('child', 'SHARED'), 'updated-value');
    expect(_controller(tester, 'plugin', 'SHARED').text, 'updated-value');
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.environments['plugin'], {'SHARED': 'updated-value'});
    expect(bridge.writes, 1);
  });

  testWidgets('saves changed keys and preserves unrelated concurrent edits', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('demo', ['TOKEN', 'URL', 'UNCHANGED']),
      ],
      enabled: ['demo'],
      environments: {
        'demo': {
          'TOKEN': 'old-token',
          'UNCHANGED': 'initial',
          'UNRELATED': 'keep',
        },
      },
    );
    await _openEditor(tester, bridge);
    bridge.environments['demo']!['CONCURRENT'] = 'keep-new-value';
    bridge.environments['demo']!['UNCHANGED'] = 'remote-edit';
    await tester.enterText(_field('demo', 'TOKEN'), '   ');
    await tester.enterText(_field('demo', 'URL'), '  custom-value  ');
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.environments['demo'], {
      'UNCHANGED': 'remote-edit',
      'UNRELATED': 'keep',
      'CONCURRENT': 'keep-new-value',
      'URL': '  custom-value  ',
    });
    expect(bridge.writes, 1);
  });

  testWidgets('cancel discards edits without writing configuration', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('demo', ['TOKEN']),
      ],
      enabled: ['demo'],
      environments: {
        'demo': {'TOKEN': 'old-token'},
      },
    );
    await _openEditor(tester, bridge);
    await tester.enterText(_field('demo', 'TOKEN'), 'unsaved-token');
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(bridge.environments['demo'], {'TOKEN': 'old-token'});
    expect(bridge.writes, 0);
  });

  testWidgets('saving unchanged values publishes no configuration writes', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('demo', ['TOKEN']),
      ],
      enabled: ['demo'],
    );
    await _openEditor(tester, bridge);
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.writes, 0);
    expect(find.byType(PackageEnvironmentVariablesDialog), findsNothing);
  });

  testWidgets('shows declaration metadata without applying defaults', (
    tester,
  ) async {
    final package = _package('demo', ['TOKEN']);
    final json = package.toJson();
    json['env'] = [
      core_proxy.EnvVar(
        name: 'TOKEN',
        description: const core_proxy.LocalizedText(
          values: {'zh': 'Token description'},
        ),
        requiredValue: true,
        defaultValue: 'manifest-default',
      ).toJson(),
    ];
    final bridge = _EnvironmentBridge(
      packages: [core_proxy.ToolPackage.fromJson(json)],
      enabled: ['demo'],
    );
    await _openEditor(tester, bridge);
    expect(find.text('必填'), findsOneWidget);
    expect(find.text('默认: manifest-default'), findsOneWidget);
    expect(_controller(tester, 'demo', 'TOKEN').text, '');
  });

  testWidgets('empty declarations do not read environment configuration', (
    tester,
  ) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('disabled', ['HIDDEN']),
        _package('enabled', []),
      ],
      enabled: ['enabled'],
    );
    await _openEditor(tester, bridge);
    expect(find.text('当前已启用的工具包没有声明需要的环境变量。'), findsOneWidget);
    expect(find.byType(TextField), findsNothing);
    expect(bridge.readPackages, isEmpty);
  });

  testWidgets('variable lists scroll on a narrow viewport without overflow', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 480);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final bridge = _EnvironmentBridge(
      packages: [_package('demo', List.generate(12, (index) => 'VAR_$index'))],
      enabled: ['demo'],
    );
    await _openEditor(tester, bridge);
    await tester.ensureVisible(_field('demo', 'VAR_11'));
    await tester.pumpAndSettle();
    await tester.enterText(_field('demo', 'VAR_11'), 'last-value');
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(bridge.environments['demo'], {'VAR_11': 'last-value'});
    expect(tester.takeException(), isNull);
  });

  testWidgets('load errors remain visible and disable saving', (tester) async {
    final bridge = _EnvironmentBridge(loadError: 'environment-load-failed');
    await _openEditor(tester, bridge);
    expect(find.textContaining('environment-load-failed'), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
    expect(bridge.writes, 0);
  });

  testWidgets('save errors retain edits for correction', (tester) async {
    final bridge = _EnvironmentBridge(
      packages: [
        _package('demo', ['TOKEN']),
      ],
      enabled: ['demo'],
      saveError: 'environment-save-failed',
    );
    await _openEditor(tester, bridge);
    await tester.enterText(_field('demo', 'TOKEN'), 'edited-token');
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(find.textContaining('environment-save-failed'), findsOneWidget);
    expect(_controller(tester, 'demo', 'TOKEN').text, 'edited-token');
    expect(bridge.environments['demo'], isEmpty);
    expect(find.byType(PackageEnvironmentVariablesDialog), findsOneWidget);
  });
}

/// Mounts the localized editor and waits for the runtime responses.
Future<void> _openEditor(WidgetTester tester, _EnvironmentBridge bridge) async {
  await tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      locale: const Locale('zh'),
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () => PackageEnvironmentVariablesDialog.show(
              context: context,
              clients: GeneratedCoreProxyClients(bridge),
            ),
            child: const Text('open-editor'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open-editor'));
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
}

/// Locates a variable field in its displayed package section.
Finder _field(String packageName, String name) =>
    find.byKey(ValueKey<String>('package-env:$packageName:$name'));

/// Returns the controller attached to a displayed package variable.
TextEditingController _controller(
  WidgetTester tester,
  String packageName,
  String name,
) => tester.widget<TextField>(_field(packageName, name)).controller!;

/// Creates a manifest with explicit environment declarations.
core_proxy.ToolPackage _package(String name, List<String> keys) =>
    core_proxy.ToolPackage(
      name: name,
      description: const core_proxy.LocalizedText(values: {}),
      displayName: const core_proxy.LocalizedText(values: {}),
      tools: const [],
      states: const [],
      env: [
        for (final key in keys)
          core_proxy.EnvVar(
            name: key,
            description: const core_proxy.LocalizedText(values: {}),
            requiredValue: false,
            defaultValue: null,
          ),
      ],
      isBuiltIn: false,
      enabledByDefault: false,
      category: 'Other',
      author: const [],
    );

class _EnvironmentBridge extends OperitRuntimeBridge {
  /// Creates independent persisted configuration for each explicit installation owner.
  _EnvironmentBridge({
    this.packages = const [],
    this.enabled = const [],
    Map<String, String> owners = const {},
    Map<String, Map<String, String>> environments = const {},
    this.loadError,
    this.saveError,
  }) : owners = {
         for (final package in packages) package.name: package.name,
         ...owners,
       },
       environments = {
         for (final package in packages) package.name: {},
         for (final entry in environments.entries)
           entry.key: Map.of(entry.value),
       };

  final List<core_proxy.ToolPackage> packages;
  final List<String> enabled;
  final Map<String, String> owners;
  final Map<String, Map<String, String>> environments;
  final String? loadError;
  final String? saveError;
  final List<String> readPackages = [];
  int writes = 0;

  /// Encodes runtime responses in the common bridge envelope.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[0, await call(request)]);

  /// Implements the package catalog and installation-owned environment contract.
  @override
  Future<Object?> call(CoreCallRequest request) async {
    switch ((request.target, request.methodName)) {
      case ('core/application.packageManager', 'getAvailablePackages'):
        if (loadError != null) throw StateError(loadError!);
        return {for (final package in packages) package.name: package.toJson()};
      case ('core/application.packageManager', 'getEnabledPackageNames'):
        return enabled;
      case ('core/preferences.envPreferences', 'getPackageEnvironment'):
        final packageName = (request.args as Map)['packageName'] as String;
        readPackages.add(packageName);
        final owner = owners[packageName]!;
        return {
          'ownerId': owner,
          'variables': Map<String, String>.of(environments[owner]!),
        };
      case ('core/preferences.envPreferences', 'editPackageEnvironment'):
        if (saveError != null) throw StateError(saveError!);
        final args = request.args as Map;
        final owner = owners[args['packageName']]!;
        for (final entry in (args['changes'] as Map).entries) {
          if (entry.value == null) {
            environments[owner]!.remove(entry.key);
          } else {
            environments[owner]![entry.key as String] = entry.value as String;
          }
        }
        writes++;
        return null;
    }
    throw StateError(
      'Unexpected environment call: ${request.target}.${request.methodName}',
    );
  }

  /// Rejects push operations outside this environment fixture.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshot watches outside this environment fixture.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects stream watches outside this environment fixture.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
