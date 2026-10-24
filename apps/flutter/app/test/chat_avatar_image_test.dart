import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/ChatAvatarImage.dart';
import 'package:operit2/ui/features/packages/screens/ComposeDslImageSource.dart';

final _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
);

/// Verifies chat avatars use the plugin file service and Flutter image decoder.
void main() {
  testWidgets(
    'custom avatar reads its VFS file and survives message rebuilds',
    (tester) async {
      const path = '/app/config/plugins/character_cards/avatars/custom.image';
      const changedPath =
          '/app/config/plugins/character_cards/avatars/new.image';
      final bridge = _AvatarFileBridge(_png);
      final clients = GeneratedCoreProxyClients(bridge);

      await _pumpAvatar(tester, path, clients);
      expect(bridge.paths, [path]);
      expect(tester.takeException(), isNull);
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
      final provider = tester.widget<Image>(find.byType(Image)).image;
      expect(provider, isA<ComposeDslVfsImage>());

      await _pumpAvatar(tester, path, clients);
      expect(bridge.paths, [path]);
      expect(tester.widget<Image>(find.byType(Image)).image, same(provider));

      await _pumpAvatar(tester, changedPath, clients);
      expect(bridge.paths, [path, changedPath]);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'an updated runtime bridge reads the same avatar in its own store',
    (tester) async {
      const path = '/app/config/avatars/bridge.image';
      final first = _AvatarFileBridge(_png);
      final second = _AvatarFileBridge(_png);
      await _pumpAvatar(tester, path, GeneratedCoreProxyClients(first));
      await _pumpAvatar(tester, path, GeneratedCoreProxyClients(second));
      expect(first.paths, [path]);
      expect(second.paths, [path]);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('legacy inline avatars decode without reading a storage file', (
    tester,
  ) async {
    final bridge = _AvatarFileBridge(_png);
    final source = 'data:image/png;base64,${base64Encode(_png)}';
    await _pumpAvatar(tester, source, GeneratedCoreProxyClients(bridge));
    expect(bridge.paths, isEmpty);
    expect(tester.widget<Image>(find.byType(Image)).image, isA<MemoryImage>());
    expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets('animated custom avatars render multiple frames', (tester) async {
    final bytes = base64Decode(
      'R0lGODlhAQABAIAAAAAAAP///yH/C05FVFNDQVBFMi4wAwEAAAAh+QQACgAAACwAAAAAAQABAAACAkQBACH5BAAKAAAALAAAAAABAAEAAAICTAEAOw==',
    );
    final codec = await tester.runAsync(() => ui.instantiateImageCodec(bytes));
    expect(codec!.frameCount, 2);
    codec.dispose();
    final bridge = _AvatarFileBridge(bytes);
    await tester.pumpWidget(
      MaterialApp(
        home: SizedBox(
          width: 40,
          height: 40,
          child: ChatAvatarImage(
            avatarUri: '/app/config/avatars/animated.image',
            fit: BoxFit.cover,
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump();
    final firstFrame = tester.widget<RawImage>(find.byType(RawImage)).image;
    expect(firstFrame, isNotNull);
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump(const Duration(milliseconds: 120));
    await tester.pump();
    expect(
      tester.widget<RawImage>(find.byType(RawImage)).image,
      isNot(same(firstFrame)),
    );
    expect(bridge.paths, ['/app/config/avatars/animated.image']);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

/// Waits for actual image decoding independently of the test's fake clock.
Future<void> _pumpAvatar(
  WidgetTester tester,
  String source,
  GeneratedCoreProxyClients clients,
) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Center(
        child: SizedBox(
          width: 40,
          height: 40,
          child: ChatAvatarImage(
            avatarUri: source,
            fit: BoxFit.cover,
            clients: clients,
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  await tester.runAsync(
    () => precacheImage(
      tester.widget<Image>(find.byType(Image)).image,
      tester.element(find.byType(ChatAvatarImage)),
    ),
  );
  await tester.pump();
}

/// Serves avatar bytes through the generated binary file tool contract.
class _AvatarFileBridge extends OperitRuntimeBridge {
  /// Creates a file service fixture containing one image payload.
  _AvatarFileBridge(this.bytes);

  final Uint8List bytes;
  final paths = <String>[];

  /// Validates that avatars read files through Core instead of theme storage.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'executeTool');
    final tool = (request.args as Map)['tool'] as Map;
    expect(tool['name'], 'read_file_binary');
    final parameters = tool['parameters'] as List;
    expect(parameters, hasLength(1));
    expect((parameters.single as Map)['name'], 'path');
    final path = (parameters.single as Map)['value'] as String;
    paths.add(path);
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': true,
        'result': {
          '__type': 'BinaryFileContentData',
          'path': path,
          'contentBase64': base64Encode(bytes),
          'size': bytes.length,
        },
        'error': null,
      },
    ]);
  }

  /// Rejects stream writes because image reads use ordinary Core calls.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots because image reads do not watch runtime state.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects streams because image reads do not watch runtime state.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
