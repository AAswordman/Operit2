import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  OperitThemeController controller(_NodeThemeBridge bridge) {
    final clients = GeneratedCoreProxyClients(bridge);
    return OperitThemeController(
      onChanged: () {}, clients: clients,
      preferencesManager: UserPreferencesManager(clients: clients),
      saveStartupThemeMode: (_) async {},
    );
  }

  test('startup observes node-global state without character or group calls', () async {
    final bridge = _NodeThemeBridge();
    final theme = controller(bridge);
    addTearDown(theme.dispose);
    addTearDown(bridge.close);
    await theme.start();
    expect(theme.selection, isNull);
    expect(theme.themeLoadError, isNull);
    expect(bridge.calls, <String>['initializeNodeThemeAppearance', 'nodeThemeState']);
    expect(bridge.watches, <String>['nodeThemeStateFlow']);
    expect(theme.themePreferenceSnapshot.toJson(), UserPreferencesManager.defaultThemePreferenceSnapshot.toJson());
  });

  test('mode and scaling edits use only atomic CoreNode appearance updates', () async {
    final bridge = _NodeThemeBridge();
    final theme = controller(bridge);
    addTearDown(theme.dispose);
    addTearDown(bridge.close);
    await theme.start();
    await theme.setThemeMode(ThemeMode.dark);
    await theme.saveThemeSettings(fontScale: 1.2);
    await Future<void>.delayed(Duration.zero);
    expect(theme.themeMode, ThemeMode.dark);
    expect(theme.themePreferenceSnapshot.fontScale, 1.2);
    expect(bridge.updates, <Map<String, String>>[
      <String, String>{'themeMode': '"dark"'}, <String, String>{'fontScale': '1.2'},
    ]);
    expect(bridge.calls.every((name) => <String>{
      'initializeNodeThemeAppearance', 'nodeThemeState', 'updateNodeThemeAppearance',
    }.contains(name)), isTrue);
  });

  test('an unavailable selected package reports an error and retains selection', () async {
    final bridge = _NodeThemeBridge()..selection = <String, Object?>{
      'schemaVersion': 1, 'themeId': 'com.example.missing', 'version': '1.0.0',
      'archiveSha256': List<String>.filled(64, 'a').join(),
    };
    final theme = controller(bridge);
    addTearDown(theme.dispose);
    addTearDown(bridge.close);
    await theme.start();
    expect(theme.themeLoadError.toString(), contains('等待 Space 同步'));
    expect(theme.selection?.themeId, 'com.example.missing');
    expect(theme.packageVisualsLocked, isTrue);
    expect(bridge.calls, isNot(contains('applyTheme')));
    expect(bridge.calls, isNot(contains('selectPersonalThemeAppearance')));
  });

  test('deleteLibraryTheme unselects active package when requested and calls deleteTheme', () async {
    final digest = List<String>.filled(64, 'b').join();
    final bridge = _NodeThemeBridge()..selection = <String, Object?>{
      'schemaVersion': 1, 'themeId': 'com.example.active', 'version': '1.0.0',
      'archiveSha256': digest,
    };
    final theme = controller(bridge);
    addTearDown(theme.dispose);
    addTearDown(bridge.close);
    await theme.start();
    await expectLater(
      () => theme.deleteLibraryTheme(
        themeId: 'com.example.active',
        archiveSha256: digest,
      ),
      throwsA(isA<StateError>()),
    );
    await theme.deleteLibraryTheme(
      themeId: 'com.example.active',
      archiveSha256: digest,
      unselectIfActive: true,
    );
    expect(bridge.calls, containsAllInOrder(<String>[
      'selectPersonalThemeAppearance',
      'deleteTheme',
    ]));
    expect(theme.selection, isNull);
  });
}

/// This fake exercises generated typed call/watch codecs. It does not validate
/// archives, emulate Space sync, register public fonts or prove rendered parity.
class _NodeThemeBridge extends OperitRuntimeBridge {
  final List<String> calls = <String>[];
  final List<String> watches = <String>[];
  final List<Map<String, String>> updates = <Map<String, String>>[];
  final StreamController<CoreEvent> _events = StreamController<CoreEvent>.broadcast(sync: true);
  Map<String, Object?> appearance = UserPreferencesManager.defaultThemePreferenceSnapshot.toJson();
  Map<String, Object?>? selection;
  CoreWatchRequest? _watch;

  Map<String, Object?> get state => <String, Object?>{
    'schemaVersion': 1, 'selection': selection,
    'privateAppearanceJson': jsonEncode(appearance),
  };

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    final args = request.args as Map;
    switch (request.methodName) {
      case 'initializeNodeThemeAppearance':
      case 'nodeThemeState':
        return encodeCoreLink(<Object?>[0, state]);
      case 'updateNodeThemeAppearance':
        final patch = Map<String, String>.from(args['updates'] as Map);
        updates.add(patch);
        for (final field in patch.entries) { appearance[field.key] = jsonDecode(field.value); }
        final watch = _watch;
        if (watch != null) _events.add(_event(watch));
        return encodeCoreLink(<Object?>[0, state]);
      case 'selectPersonalThemeAppearance':
        selection = null;
        return encodeCoreLink(<Object?>[0, state]);
      case 'deleteTheme':
        return encodeCoreLink(<Object?>[0, null]);
      case 'readThemePackage':
        throw StateError('等待 Space 同步主题包');
      default:
        throw UnsupportedError('Unexpected theme call: ${request.target}/${request.methodName}');
    }
  }

  CoreEvent _event(CoreWatchRequest request) => CoreEvent.raw(
    requestId: request.requestId, target: request.target, propertyName: request.propertyName,
    kind: 'Snapshot', valueBytes: encodeCoreLink(state), decodeValue: decodeCoreLink<Object?>,
  );

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    watches.add(request.propertyName);
    if (request.target != 'core/application' || request.propertyName != 'nodeThemeStateFlow') {
      throw UnsupportedError('Unexpected watch: ${request.target}/${request.propertyName}');
    }
    _watch = request;
    return _events.stream;
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async => _event(request);
  @override
  Future<CorePushSink> push(CorePushRequest request) => throw UnimplementedError();

  Future<void> close() => _events.close();
}
