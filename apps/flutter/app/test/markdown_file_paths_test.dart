import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/markdown/MarkdownFileScope.dart';
import 'package:operit2/ui/common/markdown/MarkdownImagePreview.dart';
import 'package:operit2/ui/common/markdown/MarkdownRemoteImage.dart';
import 'package:operit2/ui/features/packages/screens/ComposeDslImageSource.dart';

void main() {
  test('historical relative references keep their saved directory', () {
    const old = '/operitapp/outputs/2026-11-04_00-00-00-000';
    expect(
      resolveMarkdownFilePath('images/chart.png', old),
      '$old/images/chart.png',
    );
    expect(
      resolveMarkdownFilePath('../other/report.md', old),
      '/operitapp/outputs/other/report.md',
    );
    expect(resolveMarkdownFilePath('./中文%20图.png', old), '$old/中文 图.png');
    expect(
      resolveMarkdownFilePath('../../../../escape', old),
      '../../../../escape',
    );
  });
  test(
    'absolute, qualified, network and embedded sources retain their meaning',
    () {
      for (final value in [
        '/virtual/linux/tmp/file',
        'core-node:/operitapp/outputs/x/a.png',
        'https://example.com/a.png',
        'data:image/png;base64,AA==',
        '#section',
      ]) {
        expect(
          resolveMarkdownFilePath(value, '/operitapp/workspaces/a'),
          value,
        );
      }
      expect(
        composeDslVfsImagePath('core-node:/operitapp/outputs/x/a.png'),
        'core-node:/operitapp/outputs/x/a.png',
      );
      expect(composeDslVfsImagePath('https://example.com/a.png'), isNull);
    },
  );

  for (final source in ['images/chart.png', 'https://example.com/chart.png']) {
    testWidgets(
      'relative file scope retains its owner and preview for $source',
      (tester) async {
        final bridge = _ScopedImageBridge();
        final clients = GeneratedCoreProxyClients(bridge);
        const base = '/operitapp/outputs/family';
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: MarkdownFileScope(
                basePath: base,
                nodeId: 'origin',
                clients: clients,
                child: MarkdownRemoteImage(url: source),
              ),
            ),
          ),
        );
        await _settleImages(tester);
        final expected = source.startsWith('https:') ? source : '$base/$source';
        expect(bridge.sources, [expected]);
        expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
        await tester.tap(find.byType(Image));
        await _settleImages(tester);
        final scope = MarkdownFileScope.maybeOf(
          tester.element(find.byType(MarkdownImagePreview)),
        )!;
        expect(scope.basePath, base);
        expect(scope.nodeId, 'origin');
        expect(scope.clients, same(clients));
        expect(bridge.sources.every((path) => path == expected), isTrue);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'relative output images use their source only after a missing local copy',
    (tester) async {
      final bridge = _ScopedImageBridge(missingLocal: true);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MarkdownFileScope(
              basePath: '/operitapp/outputs/family',
              nodeId: 'origin',
              clients: GeneratedCoreProxyClients(bridge),
              child: const MarkdownRemoteImage(url: 'images/chart.png'),
            ),
          ),
        ),
      );
      await _settleImages(tester);
      expect(bridge.sources, [
        '/operitapp/outputs/family/images/chart.png',
        'origin:/operitapp/outputs/family/images/chart.png',
      ]);
      expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}

Future<void> _settleImages(WidgetTester tester) async {
  await tester.pumpAndSettle();
  for (final element in find.byType(Image).evaluate()) {
    final image = element.widget as Image;
    await tester.runAsync(() => precacheImage(image.image, element));
  }
  await tester.pumpAndSettle();
}

class _ScopedImageBridge extends OperitRuntimeBridge {
  _ScopedImageBridge({this.missingLocal = false});
  final bool missingLocal;
  final sources = <String>[];
  final bytes = base64Decode(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'executeTool');
    final tool = (request.args as Map)['tool'] as Map;
    expect(tool['name'], 'read_file_binary');
    final path =
        ((tool['parameters'] as List).single as Map)['value'] as String;
    sources.add(path);
    if (missingLocal && path.startsWith('/')) {
      throw StateError('not synchronized');
    }
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': true,
        'error': null,
        'result': {
          '__type': 'BinaryFileContentData',
          'path': path,
          'contentBase64': base64Encode(bytes),
          'size': bytes.length,
        },
      },
    ]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    expect(request.propertyName, 'imageBytes');
    sources.add((request.args as Map)['url'] as String);
    yield CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Changed',
      valueBytes: encodeCoreLink({
        'bytes': bytes,
        'displayUrl': null,
        'error': null,
      }),
      decodeValue: (data) => decodeCoreLink<Object?>(data),
    );
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError('Images must not upload content');

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError('Images use the existing stream contract');
}
