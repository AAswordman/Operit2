import 'dart:async';

import 'package:flutter/material.dart';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/runtime/PeerListenerSettings.dart';

class TestBridge extends OperitRuntimeBridge {
  Map<String, Object?>? config = {
    'bindAddress': '0.0.0.0:0',
    'transports': ['http', 'webSocket'],
    'discoveryEnabled': true,
    'portMode': 'automatic',
    'updatedAt': 1,
  };
  Map<String, Object?> capabilities = {
    'transports': ['http', 'webSocket', 'tcp', 'bluetooth'],
    'discoveryAdvertisement': true,
  };
  final startedTransports = <List<Object?>>[];
  final calls = <String>[];
  bool failStart = false;
  Completer<void>? startGate;
  Completer<void>? loadGate;
  String? selectedAddress;
  bool failLoad = false;
  bool failCapabilities = false;
  bool failSave = false;
  final writes = <Map<String, Object?>>[];

  /// Handles configuration persistence and explicit listener lifecycle calls.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    switch (request.methodName) {
      case 'listenerCapabilities':
        if (failCapabilities) {
          throw StateError('Cannot read listener capabilities');
        }
        return encodeCoreLink([0, capabilities]);
      case 'localHostConfig':
        await loadGate?.future;
        if (failLoad) throw StateError('Cannot read listener configuration');
        return encodeCoreLink([0, config]);
      case 'saveLocalHostConfig':
        if (failSave) throw StateError('Cannot save listener configuration');
        config = Map<String, Object?>.from(
          (request.args as Map)['config'] as Map,
        );
        writes.add(Map<String, Object?>.from(config!));
        return encodeCoreLink([0, null]);
      case 'startListening':
        startedTransports.add(
          List<Object?>.from((request.args as Map)['transports'] as List),
        );
        await startGate?.future;
        if (failStart) throw StateError('unsupported transport');
        if (config!['portMode'] == 'automatic' && selectedAddress != null) {
          config!['bindAddress'] = selectedAddress;
        }
      case 'stopListening':
        break;
      default:
        throw StateError(request.methodName);
    }
    return encodeCoreLink([0, null]);
  }

  /// Rejects push streams that these settings never open.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots that these settings never request.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects event watches that these settings never create.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}

/// Verifies listener defaults and explicit network binding.
void main() {
  /// Opens the localized connection settings dialog through a real route.
  Future<void> mount(
    WidgetTester tester,
    TestBridge bridge, {
    bool settle = true,
  }) async {
    tester.view.physicalSize = const Size(800, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        locale: const Locale('en'),
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => PeerListenerSettingsDialog(
                  clients: GeneratedCoreProxyClients(bridge),
                ),
              ),
              child: const Text('Open settings'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open settings'));
    if (settle) {
      await tester.pumpAndSettle();
    } else {
      await tester.pump();
    }
  }

  /// Submits the form after dismissing any transient success notification.
  Future<void> save(WidgetTester tester) async {
    ScaffoldMessenger.of(
      tester.element(find.byType(PeerListenerSettings)),
    ).removeCurrentSnackBar();
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Save'));
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
  }

  testWidgets('defaults enable discovery with HTTP and WebSocket', (
    tester,
  ) async {
    final bridge = TestBridge();
    await mount(tester, bridge);
    expect(find.byType(FilterChip), findsNothing);
    expect(bridge.calls, ['localHostConfig', 'listenerCapabilities']);
    expect(bridge.writes, isEmpty);
    expect(tester.widget<Switch>(find.byType(Switch)).value, true);
    expect(find.text('Save'), findsOneWidget);
    expect(find.text('OK'), findsNothing);
    expect(find.text('Connection token'), findsNothing);
    expect(find.byTooltip('Refresh token'), findsNothing);
    expect(find.byTooltip('Copy token'), findsNothing);
    expect(bridge.config!.containsKey('token'), isFalse);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilterChip>(find.widgetWithText(FilterChip, 'HTTP'))
          .selected,
      true,
    );
    expect(
      tester
          .widget<FilterChip>(find.widgetWithText(FilterChip, 'WebSocket'))
          .selected,
      true,
    );
    expect(find.text('Automatic port'), findsOneWidget);
    expect(find.text('Save'), findsOneWidget);
  });
  testWidgets('discovery changes are applied only by the footer save', (
    tester,
  ) async {
    final bridge = TestBridge();
    await mount(tester, bridge);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(tester.widget<Switch>(find.byType(Switch)).value, false);
    expect(bridge.config!['discoveryEnabled'], true);
    expect(bridge.calls, ['localHostConfig', 'listenerCapabilities']);
    expect(bridge.writes, isEmpty);
    await save(tester);
    expect(bridge.config!['discoveryEnabled'], false);
    expect(bridge.calls, [
      'localHostConfig',
      'listenerCapabilities',
      'stopListening',
      'saveLocalHostConfig',
      'startListening',
      'localHostConfig',
    ]);
    expect(find.byType(PeerListenerSettingsDialog), findsNothing);
  });

  testWidgets('dismissing the dialog discards unsaved discovery changes', (
    tester,
  ) async {
    final bridge = TestBridge();
    await mount(tester, bridge);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
    expect(find.byType(PeerListenerSettingsDialog), findsNothing);
    expect(bridge.config!['discoveryEnabled'], true);
    expect(bridge.writes, isEmpty);
  });

  testWidgets('failed save keeps the dialog open and allows retry', (
    tester,
  ) async {
    final bridge = TestBridge()..failSave = true;
    await mount(tester, bridge);
    await save(tester);
    expect(find.byType(PeerListenerSettingsDialog), findsOneWidget);
    expect(
      find.textContaining('Cannot save listener configuration'),
      findsOneWidget,
    );
    expect(bridge.writes, isEmpty);
    bridge.failSave = false;
    await save(tester);
    expect(bridge.writes, hasLength(1));
    expect(find.byType(PeerListenerSettingsDialog), findsNothing);
  });

  testWidgets('save is disabled until settings finish loading', (tester) async {
    final bridge = TestBridge()..loadGate = Completer<void>();
    await mount(tester, bridge, settle: false);
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Save'))
          .onPressed,
      isNull,
    );
    bridge.loadGate!.complete();
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Save'))
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('saving blocks duplicate submissions and premature dismissal', (
    tester,
  ) async {
    final bridge = TestBridge()..startGate = Completer<void>();
    await mount(tester, bridge);
    await tester.tap(find.text('Save'));
    await tester.pump();
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Save'))
          .onPressed,
      isNull,
    );
    await tester.tap(find.text('Save'));
    await tester.tapAt(const Offset(5, 5));
    await tester.pump();
    expect(find.byType(PeerListenerSettingsDialog), findsOneWidget);
    expect(bridge.writes, hasLength(1));
    expect(bridge.startedTransports, hasLength(1));
    bridge.startGate!.complete();
    await tester.pumpAndSettle();
    expect(find.byType(PeerListenerSettingsDialog), findsNothing);
  });

  testWidgets('HTTP and WS multi-select saves both; TCP conflict is explicit', (
    tester,
  ) async {
    final bridge = TestBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    await save(tester);
    expect(bridge.config!['transports'], ['http', 'webSocket']);
    expect(find.byType(PeerListenerSettingsDialog), findsNothing);
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilterChip, 'TCP'));
    await tester.pumpAndSettle();
    final count = bridge.calls.length;
    await save(tester);
    expect(bridge.calls.length, count);
    expect(find.textContaining('TCP cannot currently'), findsOneWidget);
  });
  testWidgets('fixed mode keeps the configured address and port', (
    tester,
  ) async {
    final bridge = TestBridge();
    bridge.config!['portMode'] = 'fixed';
    bridge.config!['bindAddress'] = '0.0.0.0:37195';
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    await save(tester);
    expect(bridge.config!['portMode'], 'fixed');
    expect(bridge.config!['bindAddress'], '0.0.0.0:37195');
  });
  testWidgets('splits IPv6 listen address and keeps fixed port editable', (
    tester,
  ) async {
    final bridge = TestBridge();
    bridge.config!['bindAddress'] = '[::1]:41234';
    bridge.config!['portMode'] = 'fixed';
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('peer-bind-host')))
          .controller
          ?.text,
      '::1',
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('peer-bind-port')))
          .controller
          ?.text,
      '41234',
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('peer-bind-port')))
          .enabled,
      isTrue,
    );
    await save(tester);
    expect(bridge.config!['bindAddress'], '[::1]:41234');
  });
  testWidgets('automatic mode reloads runtime-selected port after save', (
    tester,
  ) async {
    final bridge = TestBridge()..selectedAddress = '0.0.0.0:37196';
    await mount(tester, bridge);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(bridge.config!['portMode'], 'automatic');
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    final portField = tester.widget<TextField>(
      find.byKey(const ValueKey('peer-bind-port')),
    );
    expect(portField.controller?.text, '0');
    expect(portField.enabled, isFalse);
    await save(tester);
    expect(bridge.writes.last['bindAddress'], '0.0.0.0:0');
    expect(bridge.config!['bindAddress'], '0.0.0.0:37196');
    expect(bridge.config!['portMode'], 'automatic');
  });
  testWidgets('saving automatic mode preserves its last assigned port', (
    tester,
  ) async {
    final bridge = TestBridge();
    bridge.config!['bindAddress'] = '0.0.0.0:41234';
    await mount(tester, bridge);
    await save(tester);
    expect(bridge.writes.last['bindAddress'], '0.0.0.0:41234');
    expect(bridge.config!['portMode'], 'automatic');
  });
  testWidgets('switching from fixed to automatic requests system allocation', (
    tester,
  ) async {
    final bridge = TestBridge()..selectedAddress = '0.0.0.0:41234';
    bridge.config!['portMode'] = 'fixed';
    bridge.config!['bindAddress'] = '0.0.0.0:37195';
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Fixed listen address and port'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Automatic port').last);
    await tester.pumpAndSettle();
    await save(tester);
    expect(bridge.writes.last['bindAddress'], '0.0.0.0:0');
    expect(bridge.config!['bindAddress'], '0.0.0.0:41234');
    expect(bridge.config!['portMode'], 'automatic');
  });
  testWidgets('listener failures report the cause without rewriting config', (
    tester,
  ) async {
    final bridge = TestBridge()..failStart = true;
    await mount(tester, bridge);
    final writesBefore = bridge.writes.length;
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    await save(tester);
    expect(bridge.config!['transports'], ['http', 'webSocket']);
    expect(bridge.config!['discoveryEnabled'], true);
    expect(bridge.config!['portMode'], 'automatic');
    expect(bridge.writes.length, writesBefore + 1);
    expect(bridge.calls.where((call) => call == 'startListening').length, 1);
    expect(find.textContaining('unsupported transport'), findsOneWidget);
    expect(find.text('Listener settings applied.'), findsNothing);
  });
  testWidgets('saved fixed configuration is preserved', (tester) async {
    final bridge = TestBridge()
      ..config = {
        'bindAddress': '127.0.0.1:48123',
        'transports': ['http', 'webSocket'],
        'discoveryEnabled': false,
        'portMode': 'fixed',
        'updatedAt': 1,
      };
    await mount(tester, bridge);
    expect(bridge.writes, isEmpty);
    expect(tester.widget<Switch>(find.byType(Switch)).value, false);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('peer-bind-host')))
          .controller
          ?.text,
      '127.0.0.1',
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('peer-bind-port')))
          .controller
          ?.text,
      '48123',
    );
    await save(tester);
    expect(bridge.config!['bindAddress'], '127.0.0.1:48123');
    expect(bridge.config!.containsKey('token'), isFalse);
    expect(bridge.config!['portMode'], 'fixed');
    expect(bridge.config!['discoveryEnabled'], false);
  });
  testWidgets(
    'configuration read errors never initialize replacement defaults',
    (tester) async {
      final bridge = TestBridge()..failLoad = true;
      await mount(tester, bridge);
      expect(bridge.writes, isEmpty);
      expect(
        find.textContaining('Cannot read listener configuration'),
        findsOneWidget,
      );
    },
  );
  testWidgets(
    'missing runtime configuration is an error, not client initialization',
    (tester) async {
      final bridge = TestBridge()..config = null;
      await mount(tester, bridge);
      expect(bridge.calls, ['localHostConfig']);
      expect(bridge.writes, isEmpty);
      expect(
        find.textContaining(
          'Runtime listener preferences were not initialized',
        ),
        findsOneWidget,
      );
    },
  );

  /// Allows TCP independently while disabling every unsupported inbound protocol.
  testWidgets(
    'TCP-only Host activates only TCP and preserves stored preferences',
    (tester) async {
      final bridge = TestBridge()
        ..capabilities = {
          'transports': ['tcp'],
          'discoveryAdvertisement': false,
        };
      bridge.config!['transports'] = ['http', 'webSocket', 'tcp'];
      await mount(tester, bridge);
      expect(bridge.writes, isEmpty);
      final discovery = tester.widget<Switch>(find.byType(Switch));
      expect(discovery.onChanged, isNull);
      expect(discovery.value, false);
      await tester.tap(find.text('Advanced options'));
      await tester.pumpAndSettle();
      for (final protocol in ['HTTP', 'WebSocket', 'Bluetooth', 'Serial']) {
        expect(
          tester
              .widget<FilterChip>(find.widgetWithText(FilterChip, protocol))
              .onSelected,
          isNull,
        );
      }
      expect(
        tester
            .widget<FilterChip>(find.widgetWithText(FilterChip, 'TCP'))
            .onSelected,
        isNotNull,
      );
      await save(tester);
      expect(bridge.startedTransports, [
        ['tcp'],
      ]);
      expect(bridge.config!['transports'], ['http', 'webSocket', 'tcp']);
      expect(bridge.config!['discoveryEnabled'], true);
      expect(find.text('Listener settings applied.'), findsOneWidget);
    },
  );

  /// Does not treat HTTP serving as authorization for WS upgrades or LAN discovery.
  testWidgets('HTTP-only Host disables WS without disabling HTTP', (
    tester,
  ) async {
    final bridge = TestBridge()
      ..capabilities = {
        'transports': ['http'],
        'discoveryAdvertisement': false,
      };
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilterChip>(find.widgetWithText(FilterChip, 'HTTP'))
          .onSelected,
      isNotNull,
    );
    expect(
      tester
          .widget<FilterChip>(find.widgetWithText(FilterChip, 'WebSocket'))
          .onSelected,
      isNull,
    );
    await save(tester);
    expect(bridge.startedTransports, [
      ['http'],
    ]);
    expect(find.text('Listener settings applied.'), findsOneWidget);
  });

  /// Preserves listener preferences when the Host exposes no inbound transports.
  testWidgets('client-only Host preserves settings but opens no listeners', (
    tester,
  ) async {
    final bridge = TestBridge()
      ..capabilities = {'transports': [], 'discoveryAdvertisement': false};
    await mount(tester, bridge);
    await tester.tap(find.text('Advanced options'));
    await tester.pumpAndSettle();
    for (final chip in tester.widgetList<FilterChip>(find.byType(FilterChip))) {
      expect(chip.onSelected, isNull);
    }
    await save(tester);
    expect(bridge.startedTransports, isEmpty);
    expect(bridge.config!['transports'], ['http', 'webSocket']);
  });

  /// Surfaces failed capability reads without inventing transport support.
  testWidgets('capability query failure disables applying listener settings', (
    tester,
  ) async {
    final bridge = TestBridge()..failCapabilities = true;
    await mount(tester, bridge);
    expect(
      find.textContaining('Cannot read listener capabilities'),
      findsOneWidget,
    );
    expect(tester.widget<Switch>(find.byType(Switch)).onChanged, isNull);
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Save'))
          .onPressed,
      isNull,
    );
    expect(bridge.writes, isEmpty);
    expect(bridge.startedTransports, isEmpty);
  });
}
