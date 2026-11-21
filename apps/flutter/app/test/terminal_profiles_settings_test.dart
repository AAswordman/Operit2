import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/workspace/TerminalProfilesSettings.dart';

/// Emulates Core persistence and installation progress across the actual generated wire clients.
class _TerminalBridge extends OperitRuntimeBridge {
  Map<String, Object?> settings = {
    'revision': 0,
    'defaultProfileId': 'ps5',
    'profiles': [
      {
        'id': 'ps5',
        'name': 'PowerShell 5',
        'terminal': 'native',
        'terminalType': 'powershell',
        'options': <String, String>{},
      },
      {
        'id': 'ps7',
        'name': 'PowerShell 7',
        'terminal': 'native',
        'terminalType': 'powershell',
        'options': {'executable': 'C:/pwsh.exe'},
      },
    ],
  };
  final saves = <Map<String, Object?>>[];
  final installs = <Map<String, Object?>>[];
  bool advertiseDownloads = false;
  bool rejectSave = false;

  /// Answers only the terminal-management protocol exercised by these widget tests.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final args = Map<String, Object?>.from(request.args! as Map);
    final Object? value;
    switch (request.methodName) {
      case 'terminalProfileSettings':
        value = settings;
      case 'terminalProfileSupport':
        value = [
          {
            'terminal': 'native',
            'terminalType': 'powershell',
            'fields': ['executable', 'arguments'],
          },
          {
            'terminal': 'proot',
            'terminalType': 'bash',
            'fields': ['rootfs'],
          },
          {
            'terminal': 'native',
            'terminalType': 'bash',
            'fields': ['executable', 'arguments'],
          },
        ];
      case 'terminalDistributions':
        value = advertiseDownloads
            ? [
                {
                  'id': 'ubuntu-24_04',
                  'name': 'Ubuntu 24.04 LTS',
                  'architecture': 'aarch64',
                  'url': 'https://example.test/rootfs.tar.xz',
                  'sha256': 'a' * 64,
                  'downloadBytes': 1024,
                  'notice': '',
                },
              ]
            : [];
      case 'terminalDistributionInstalls':
        value = installs;
      case 'saveTerminalProfileSettings':
        if (rejectSave) {
          throw const CoreLinkError(
            code: 'CONFLICT',
            message: 'Reload profiles',
          );
        }
        final proposed = Map<String, Object?>.from(args['settings']! as Map);
        expect(proposed['revision'], settings['revision']);
        saves.add(proposed);
        settings = {
          ...proposed,
          'revision': (settings['revision']! as int) + 1,
        };
        value = settings;
      case 'startTerminalDistributionInstall':
        installs.add({
          'installId': 'rootfs_a',
          'distributionId': args['distributionId'],
          'name': args['name'],
          'state': 'queued',
          'downloadedBytes': 0,
          'totalBytes': 1024,
          'profileId': null,
          'error': null,
        });
        value = installs.last;
      default:
        throw UnsupportedError(
          'Unexpected terminal request: ${request.methodName}',
        );
    }
    return encodeCoreLink(<Object?>[0, value]);
  }

  /// Rejects unrelated runtime calls in this focused management fixture.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

/// Renders the real settings panel with generated clients and English localization.
Future<void> _render(
  WidgetTester tester,
  _TerminalBridge bridge, {
  Locale locale = const Locale('en'),
}) async {
  tester.view.physicalSize = const Size(900, 1600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    MaterialApp(
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: SingleChildScrollView(
          child: TerminalProfilesSettings(
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Exercises default selection, multiple instances, validation, and asynchronous installation.
void main() {
  testWidgets('localized built-in names remain host-owned until renamed', (
    tester,
  ) async {
    const fallback = 'A changed Core description';
    final bridge = _TerminalBridge();
    bridge.settings = {
      'schemaVersion': 1,
      'revision': 0,
      'defaultProfileId': 'builtin_0',
      'profiles': [
        {
          'id': 'builtin_0',
          'name': fallback,
          'nameKey': 'terminalBuiltinMacNative',
          'terminal': 'native',
          'terminalType': 'bash',
          'options': <String, String>{},
        },
      ],
    };
    await _render(tester, bridge, locale: const Locale('zh'));
    const display = 'macOS 本机 Bash 终端';
    expect(find.text(display), findsOneWidget);
    expect(find.text('本机终端 / Bash'), findsOneWidget);
    expect(find.text(fallback), findsNothing);
    await tester.tap(find.byTooltip('编辑'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextFormField>(find.byType(TextFormField).first)
          .controller!
          .text,
      display,
    );
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    final saved = (bridge.settings['profiles']! as List).single as Map;
    expect(saved['name'], fallback);
    expect(saved['nameKey'], 'terminalBuiltinMacNative');
    expect(bridge.settings['schemaVersion'], 1);
    await tester.tap(find.byTooltip('编辑'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField).first, '我的开发终端');
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(find.text('我的开发终端'), findsOneWidget);
    final renamed = (bridge.settings['profiles']! as List).single as Map;
    expect(renamed['name'], '我的开发终端');
    expect(renamed['nameKey'], isNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'same-interpreter profiles have an explicit default and protected deletion',
    (tester) async {
      final bridge = _TerminalBridge();
      await _render(tester, bridge);
      await tester.tap(find.text('PowerShell 7'));
      await tester.pumpAndSettle();
      expect(bridge.settings['defaultProfileId'], 'ps7');
      expect(bridge.settings['revision'], 1);
      final deletion = tester.widget<IconButton>(
        find
            .byWidgetPredicate(
              (widget) => widget is IconButton && widget.tooltip == 'Delete',
            )
            .at(1),
      );
      expect(deletion.onPressed, isNull);
      expect((bridge.settings['profiles']! as List), hasLength(2));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'adding another version validates structured argv and preserves the default',
    (tester) async {
      final bridge = _TerminalBridge();
      await _render(tester, bridge);
      await tester.tap(find.text('Add terminal profile'));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byType(TextFormField).at(0),
        'PowerShell preview',
      );
      await tester.enterText(
        find.byType(TextFormField).at(1),
        'C:/preview/pwsh.exe',
      );
      await tester.enterText(find.byType(TextFormField).at(2), '--login');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(bridge.saves, isEmpty);
      expect(find.textContaining('Enter a JSON array'), findsOneWidget);
      await tester.enterText(find.byType(TextFormField).at(2), '["-Login"]');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(bridge.settings['defaultProfileId'], 'ps5');
      final profiles = bridge.settings['profiles']! as List;
      expect(profiles, hasLength(3));
      expect((profiles.last as Map)['name'], 'PowerShell preview');
      expect((profiles.last as Map)['options'], {
        'executable': 'C:/preview/pwsh.exe',
        'arguments': '["-Login"]',
      });
    },
  );

  testWidgets(
    'completed downloads add independent profiles without selecting them implicitly',
    (tester) async {
      final bridge = _TerminalBridge()..advertiseDownloads = true;
      await _render(tester, bridge);
      await tester.tap(find.byTooltip('Download and install'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField), 'Ubuntu work');
      await tester.tap(
        find.widgetWithText(FilledButton, 'Download and install'),
      );
      await tester.pump();
      expect(bridge.installs.single['name'], 'Ubuntu work');
      bridge.installs.single['state'] = 'completed';
      bridge.installs.single['profileId'] = 'rootfs_a';
      bridge.settings = {
        ...bridge.settings,
        'revision': 1,
        'profiles': [
          ...bridge.settings['profiles']! as List,
          {
            'id': 'rootfs_a',
            'name': 'Ubuntu work',
            'terminal': 'proot',
            'terminalType': 'bash',
            'options': {'rootfs': '/private/rootfs_a'},
          },
        ],
      };
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.text('Ubuntu work'), findsNWidgets(2));
      expect(
        find.text('Installed · available in terminal profiles'),
        findsOneWidget,
      );
      expect(bridge.settings['defaultProfileId'], 'ps5');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('failed saves retain the durable default and expose reload', (
    tester,
  ) async {
    final bridge = _TerminalBridge()..rejectSave = true;
    await _render(tester, bridge);
    await tester.tap(find.text('PowerShell 7'));
    await tester.pumpAndSettle();
    expect(bridge.settings['defaultProfileId'], 'ps5');
    expect(find.textContaining('CONFLICT'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });
}
