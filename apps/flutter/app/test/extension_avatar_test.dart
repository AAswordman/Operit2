import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/components/ExtensionAvatar.dart';

void main() {
  testWidgets('a remounted cached logo renders in its first frame', (
    tester,
  ) async {
    final bridge = _LogoBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    await tester.pumpWidget(_app(clients));
    await tester.pumpAndSettle();
    expect(find.byType(ExtensionLogoImage), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(_app(GeneratedCoreProxyClients(bridge)));
    // Do not settle: a blank first frame is the regression under test.
    expect(find.byType(ExtensionLogoImage), findsOneWidget);
    expect(bridge.reads, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets('toggling enabled preserves the loaded logo element', (
    tester,
  ) async {
    final clients = GeneratedCoreProxyClients(_LogoBridge());
    await tester.pumpWidget(_app(clients));
    await tester.pumpAndSettle();
    final logoElement = tester.element(find.byType(ExtensionLogoImage));

    await tester.pumpWidget(_app(clients, enabled: false));
    await tester.pump(const Duration(milliseconds: 110));
    expect(tester.element(find.byType(ExtensionLogoImage)), same(logoElement));
    await tester.pumpAndSettle();
    await tester.pumpWidget(_app(clients));
    await tester.pumpAndSettle();
    expect(tester.element(find.byType(ExtensionLogoImage)), same(logoElement));
    expect(tester.takeException(), isNull);
  });

  test(
    'reads are shared and isolated by version, resource and bridge',
    () async {
      final bridge = _LogoBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      final first = _load(clients);
      expect(_load(clients), same(first));
      final logo = await first;
      expect(await _load(clients), same(logo));
      expect(bridge.reads, 1);
      await _load(clients, version: '2');
      await _load(clients, resourceKey: 'other.svg');
      expect(bridge.reads, 3);
      final other = _LogoBridge();
      await _load(GeneratedCoreProxyClients(other));
      expect(other.reads, 1);
    },
  );

  test('failed logo reads can be retried', () async {
    final bridge = _LogoBridge()..fail = true;
    final clients = GeneratedCoreProxyClients(bridge);
    await expectLater(_load(clients), throwsStateError);
    bridge.fail = false;
    expect(await _load(clients), isNotNull);
    expect(bridge.reads, 2);
  });
}

Future<ToolPkgLogoBytes?> _load(
  GeneratedCoreProxyClients clients, {
  String version = '1',
  String resourceKey = 'logo.svg',
}) => ExtensionLogoCache.load(
  clients,
  packageName: 'sample',
  resourceKey: resourceKey,
  version: version,
);

Widget _app(GeneratedCoreProxyClients clients, {bool enabled = true}) =>
    MaterialApp(
      home: Scaffold(
        body: Center(
          child: CatalogEntryAvatar(
            clients: clients,
            entry: _entry,
            enabled: enabled,
          ),
        ),
      ),
    );

const _entry = ExtensionCatalogEntry(
  id: 'sample',
  kind: 'toolpkg',
  source: 'manual',
  displayName: 'Sample',
  description: '',
  version: '1',
  authors: [],
  category: '',
  scope: 'device',
  activationState: 'enabled',
  runtimeState: 'running',
  updateState: 'none',
  isCustomized: false,
  marketEntryId: null,
  installedMarketVersionId: null,
  logoResourceKey: 'logo.svg',
  logoMimeType: 'image/svg+xml',
  toolCount: 0,
  subpackageCount: 0,
  uiModuleCount: 0,
  hasUi: false,
  chatSidebarEntryIds: [],
  dependencyIssueCount: 0,
);

class _LogoBridge extends OperitRuntimeBridge {
  int reads = 0;
  bool fail = false;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'readToolPkgLogoBytes');
    reads += 1;
    if (fail) throw StateError('Read failed');
    return encodeCoreLink(<Object?>[
      0,
      {
        'resourceKey': 'logo.svg',
        'mimeType': 'image/svg+xml',
        'fileName': 'logo.svg',
        'bytes': Uint8List.fromList(
          '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40">'
                  '<rect width="40" height="40" fill="red"/></svg>'
              .codeUnits,
        ),
      },
    ]);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnimplementedError();
}
