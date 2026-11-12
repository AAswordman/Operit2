import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies synchronized ordinary appearance through generated clients.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(ClientLogger.initialize);

  OperitThemeController controller(_SharedThemeBridge bridge) {
    final clients = GeneratedCoreProxyClients(bridge);
    return OperitThemeController(
      onChanged: () {},
      preferencesManager: UserPreferencesManager(clients: clients),
      saveStartupThemeMode: (_) async {},
    );
  }

  test(
    'watch payloads update appearance without reads or unrelated rebuilds',
    () async {
      final bridge = _SharedThemeBridge();
      var changes = 0;
      final startupModes = <ThemeMode>[];
      final theme = OperitThemeController(
        onChanged: () => changes++,
        preferencesManager: UserPreferencesManager(
          clients: GeneratedCoreProxyClients(bridge),
        ),
        saveStartupThemeMode: (mode) async => startupModes.add(mode),
      );
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      final watch = bridge._watchRequests['preferencesFlow']!;
      bridge.calls.clear();
      changes = 0;

      bridge._events.add(bridge._event(watch, bridge.preferences));
      await Future<void>.delayed(Duration.zero);
      bridge.preferences['chat_sidebar_tab'] = 'history';
      bridge._events.add(bridge._event(watch, Map.of(bridge.preferences)));
      await Future<void>.delayed(Duration.zero);
      expect(changes, 0);
      expect(bridge.calls, isEmpty);

      bridge.preferences['bubble_show_avatar'] = false;
      bridge._events.add(bridge._event(watch, Map.of(bridge.preferences)));
      await Future<void>.delayed(Duration.zero);
      expect(theme.themePreferenceSnapshot.bubbleShowAvatar, isFalse);
      expect(changes, 1);
      expect(bridge.calls, isEmpty);
      expect(startupModes, [ThemeMode.system]);

      await theme.saveThemeSettings(bubbleShowAvatar: true);
      await Future<void>.delayed(Duration.zero);
      expect(changes, 2);
      expect(startupModes, [ThemeMode.system]);

      bridge.preferences['font_scale'] = 'invalid';
      bridge._events.add(bridge._event(watch, Map.of(bridge.preferences)));
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeLoadError, isNotNull);
      bridge.preferences.remove('font_scale');
      bridge._events.add(bridge._event(watch, Map.of(bridge.preferences)));
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeLoadError, isNull);
      expect(theme.themePreferenceSnapshot.fontScale, 1.0);
    },
  );

  test(
    'selecting a global font releases previous message font overrides atomically',
    () async {
      final bridge = _SharedThemeBridge();
      bridge.preferences.addAll(<String, Object?>{
        'bubble_user_use_custom_font': true,
        'bubble_user_system_font_name': 'serif',
        'bubble_ai_use_custom_font': true,
        'bubble_ai_system_font_name': 'serif',
      });
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      expect(theme.themePreferenceSnapshot.bubbleUserUseCustomFont, isTrue);
      expect(theme.themePreferenceSnapshot.bubbleAiUseCustomFont, isTrue);
      await theme.saveThemeSettings(fontScale: 1.2);
      expect(theme.themePreferenceSnapshot.bubbleUserUseCustomFont, isTrue);
      expect(theme.themePreferenceSnapshot.bubbleAiUseCustomFont, isTrue);
      bridge.updates.clear();
      await theme.saveThemeSettings(
        fontType: UserPreferencesManager.FONT_TYPE_SYSTEM,
        systemFontName: UserPreferencesManager.SYSTEM_FONT_SANS_SERIF,
      );
      expect(bridge.updates.single, <String, Object?>{
        'font_type': 'system',
        'system_font_name': 'sans-serif',
        'bubble_user_use_custom_font': false,
        'bubble_ai_use_custom_font': false,
      });
      final snapshot = theme.themePreferenceSnapshot;
      expect(snapshot.bubbleUserUseCustomFont, isFalse);
      expect(snapshot.bubbleAiUseCustomFont, isFalse);
      expect(
        operitMessageFontFamily(snapshot, isUser: true),
        operitMessageFontFamily(snapshot, isUser: false),
      );
    },
  );

  test(
    'startup reads released preferences and watches shared appearance',
    () async {
      final bridge = _SharedThemeBridge();
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      expect(theme.themeLoadError, isNull);
      expect(bridge.calls, <String>['getPreferences']);
      expect(bridge.watches, <String>['preferencesFlow']);
      expect(
        theme.themePreferenceSnapshot.toJson(),
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      );
    },
  );

  test(
    'mode and scaling edits use the original atomic shared appearance owner',
    () async {
      final bridge = _SharedThemeBridge();
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      await theme.setThemeMode(ThemeMode.dark);
      await theme.saveThemeSettings(fontScale: 1.2);
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeMode, ThemeMode.dark);
      expect(theme.themePreferenceSnapshot.fontScale, 1.2);
      expect(bridge.updates, <Map<String, Object?>>[
        <String, Object?>{'theme_mode': 'dark'},
        <String, Object?>{'font_scale': 1.2},
      ]);
      expect(
        bridge.calls.every(
          (name) =>
              <String>{'getPreferences', 'patchAppearance'}.contains(name),
        ),
        isTrue,
      );
    },
  );

  test(
    'legacy template selections cannot block independent appearance',
    () async {
      final bridge = _SharedThemeBridge();
      bridge.preferences['active_toolpkg_theme'] = <String, Object?>{
        'themeId': 'dev.removed.plugin:default',
        'archiveSha256': 'missing',
      };
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      expect(theme.themeLoadError, isNull);
      expect(theme.themeMode, ThemeMode.system);
      expect(bridge.calls, <String>['getPreferences']);
      expect(bridge.watches, <String>['preferencesFlow']);
    },
  );
  test(
    'editing copied colors clears the complete palette through the ordinary owner',
    () async {
      final bridge = _SharedThemeBridge();
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      final visual =
          jsonDecode(
                await rootBundle.loadString(
                  'packages/operit_ui/assets/fixtures/theme-v1.json',
                ),
              )
              as Map<String, dynamic>;
      bridge.preferences['theme_palette_json'] = jsonEncode(visual['palette']);
      await theme.start();
      expect(theme.themePreferenceSnapshot.themePaletteJson, isNotNull);
      await theme.saveThemeSettings(
        useCustomColors: true,
        customPrimaryColor: 0xff123456,
      );
      expect(bridge.preferences.containsKey('theme_palette_json'), isFalse);
      expect(theme.themePreferenceSnapshot.themePaletteJson, isNull);
      expect(theme.themePreferenceSnapshot.customPrimaryColor, 0xff123456);
      expect(theme.themeLoadError, isNull);
    },
  );
  test(
    'invalid appearance remains an error until explicitly retried or reset',
    () async {
      final bridge = _SharedThemeBridge();
      bridge.preferences['theme_palette_json'] = 'not json';
      bridge.preferences['custom_user_avatar_uri'] =
          'runtime/data/user_assets/theme/avatar.png';
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeLoadError, isNotNull);
      expect(theme.hasLoadedTheme, isFalse);
      final log = await ClientLogger.readText();
      expect(log, contains('Appearance load failed stage=validate_palette'));
      expect(log, contains('FormatException'));
      expect(log, contains('stackTrace='));
      expect(bridge.preferences['theme_palette_json'], 'not json');
      expect(bridge.calls, isNot(contains('useCustomAppearance')));
      await theme.reloadTheme();
      expect(theme.themeLoadError, isNotNull);
      expect(bridge.preferences['theme_palette_json'], 'not json');
      await theme.restoreDefaultAppearance();
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeLoadError, isNull);
      expect(theme.hasLoadedTheme, isTrue);
      expect(
        theme.themePreferenceSnapshot.customUserAvatarUri,
        'runtime/data/user_assets/theme/avatar.png',
      );
      expect(
        bridge.calls.where((call) => call == 'useCustomAppearance'),
        hasLength(1),
      );
    },
  );

  test(
    'preference stream failures retain their original diagnostic stack',
    () async {
      final bridge = _SharedThemeBridge();
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      final error = StateError('Preference connection disconnected');
      final stack = StackTrace.fromString('preference-disconnect-origin');
      bridge._events.addError(error, stack);
      await Future<void>.delayed(Duration.zero);

      expect(theme.themeLoadError, same(error));
      final log = await ClientLogger.readText();
      expect(log, contains('Appearance preference subscription failed'));
      expect(log, contains('Preference connection disconnected'));
      expect(log, contains('preference-disconnect-origin'));
    },
  );

  test(
    'retry restores appearance and subscribes after disconnection',
    () async {
      final bridge = _SharedThemeBridge();
      final theme = controller(bridge);
      addTearDown(theme.dispose);
      addTearDown(bridge.close);
      await theme.start();
      final error = StateError('Core FFI connection is closed');
      bridge._events.addError(error);
      await bridge._events.close();
      bridge._events = StreamController<CoreEvent>.broadcast(sync: true);
      expect(theme.themeLoadError, same(error));

      await theme.reloadTheme();
      expect(theme.themeLoadError, isNull);
      expect(theme.hasLoadedTheme, isTrue);
      expect(bridge.watches, ['preferencesFlow', 'preferencesFlow']);
      expect(bridge.updates, isEmpty);

      bridge.preferences['theme_mode'] = 'dark';
      final watch = bridge._watchRequests['preferencesFlow']!;
      bridge._events.add(bridge._event(watch, bridge.preferences));
      await Future<void>.delayed(Duration.zero);
      expect(theme.themeMode, ThemeMode.dark);
    },
  );
}

/// Exercises ordinary appearance calls and watches through the real Link codecs.
class _SharedThemeBridge extends OperitRuntimeBridge {
  final List<String> calls = <String>[];
  final List<String> watches = <String>[];
  final List<Map<String, Object?>> updates = <Map<String, Object?>>[];
  final Map<String, Object?> preferences = <String, Object?>{};
  final Map<String, CoreWatchRequest> _watchRequests =
      <String, CoreWatchRequest>{};
  StreamController<CoreEvent> _events = StreamController<CoreEvent>.broadcast(
    sync: true,
  );

  /// Implements only the ordinary appearance operations under test.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    final args = request.args as Map;
    switch (request.methodName) {
      case 'getPreferences':
        return encodeCoreLink(<Object?>[
          0,
          <String, Object?>{
            for (final key in (args['keys'] as List).cast<String>())
              if (preferences.containsKey(key)) key: preferences[key],
          },
        ]);
      case 'useCustomAppearance':
        final snapshot = Map<String, Object?>.from(args['snapshot'] as Map);
        preferences.clear();
        preferences['custom_user_avatar_uri'] = snapshot['customUserAvatarUri'];
        return encodeCoreLink(<Object?>[0, null]);
      case 'patchAppearance':
        final patch = Map<String, Object?>.from(args['values'] as Map);
        updates.add(patch);
        for (final key in (args['removeKeys'] as List).cast<String>()) {
          preferences.remove(key);
        }
        preferences.addAll(patch);
        final watch = _watchRequests['preferencesFlow'];
        if (watch != null)
          _events.add(_event(watch, Map<String, Object?>.of(preferences)));
        return encodeCoreLink(<Object?>[0, null]);
      default:
        throw UnsupportedError(
          'Unexpected theme call: ${request.target}/${request.methodName}',
        );
    }
  }

  /// Encodes an event for exactly the generated watch that owns its payload.
  CoreEvent _event(CoreWatchRequest request, Object value) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: 'Snapshot',
    valueBytes: encodeCoreLink(value),
    decodeValue: decodeCoreLink<Object?>,
  );

  /// Registers the ordinary preference stream.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    watches.add(request.propertyName);
    if (request.propertyName != 'preferencesFlow') {
      throw UnsupportedError(
        'Unexpected watch: ${request.target}/${request.propertyName}',
      );
    }
    _watchRequests[request.propertyName] = request;
    return _events.stream.where(
      (event) => event.requestId == request.requestId,
    );
  }

  /// Provides a deterministic watch snapshot for generated clients.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async =>
      _event(request, preferences);

  /// Rejects push operations outside this appearance contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Closes the event source when the test completes.
  Future<void> close() => _events.close();
}
