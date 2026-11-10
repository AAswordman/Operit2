import 'dart:convert';
import 'dart:typed_data';
import 'package:file_selector/file_selector.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit_system_fonts/operit_system_fonts.dart';
import 'package:operit2/ui/theme/DeviceFontService.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/theme/OperitThemeAssets.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'selected device face saves and reloads without a local file path',
    () async {
      final bridge = _FontStorageBridge();
      final registry = ThemeFontRegistry(registration: (_, _) async {});
      final selected = await DeviceFontService(
        platform: _DeviceFonts(),
        registry: registry,
      ).prepare('Al Tarikh');
      final store = ThemeAssetStore(
        runtimeStorage: GeneratedCoreProxyClients(
          bridge,
        ).repositoryRuntimeStorageRepository,
        fontRegistry: registry,
      );
      final imported = await store.importFontBytes(
        bytes: selected.bytes,
        fileName: 'device-font.ttf',
      );
      expect(imported.fileName, 'device-font.ttf');
      expect(imported.storagePath, endsWith('.ttf'));
      expect(bridge.files[imported.storagePath], selected.bytes);
      final restored = await registry.registerBytes(
        reference: imported.storagePath,
        bytes: await store.readFontBytes(imported.storagePath),
      );
      expect(restored.family, selected.family);
    },
  );

  test('empty device font names fail before registration or storage', () async {
    final bridge = _FontStorageBridge();
    var registered = false;
    final store = ThemeAssetStore(
      runtimeStorage: GeneratedCoreProxyClients(
        bridge,
      ).repositoryRuntimeStorageRepository,
      fontRegistry: ThemeFontRegistry(
        registration: (_, _) async {
          registered = true;
        },
      ),
    );
    await expectLater(
      store.importFontBytes(bytes: Uint8List.fromList([1]), fileName: ''),
      throwsStateError,
    );
    expect(registered, isFalse);
    expect(bridge.calls, isEmpty);
  });

  test('oversized fonts are rejected before persistence', () async {
    final bridge = _FontStorageBridge();
    final store = ThemeAssetStore(
      runtimeStorage: GeneratedCoreProxyClients(
        bridge,
      ).repositoryRuntimeStorageRepository,
    );
    await expectLater(store.importFontFile(_OversizedFont()), throwsStateError);
    expect(bridge.files, isEmpty);
    expect(bridge.calls, isEmpty);
  });
  test('registration must succeed before any font is persisted', () async {
    final bridge = _FontStorageBridge();
    final store = ThemeAssetStore(
      runtimeStorage: GeneratedCoreProxyClients(
        bridge,
      ).repositoryRuntimeStorageRepository,
      fontRegistry: ThemeFontRegistry(
        registration: (_, _) async => throw StateError('Invalid font'),
      ),
    );
    await expectLater(
      store.importFontFile(
        XFile.fromData(
          Uint8List.fromList([1, 2, 3]),
          name: 'broken.ttf',
          path: '/broken.ttf',
        ),
      ),
      throwsStateError,
    );
    expect(bridge.files, isEmpty);
    expect(bridge.calls, isEmpty);
  });
  test(
    'large font crosses the 64 MiB frame boundary in bounded chunks',
    () async {
      final bridge = _FontStorageBridge();
      var registered = false;
      final bytes = Uint8List(65 * 1024 * 1024)..setAll(0, [0, 1, 0, 0]);
      final store = ThemeAssetStore(
        runtimeStorage: GeneratedCoreProxyClients(
          bridge,
        ).repositoryRuntimeStorageRepository,
        fontRegistry: ThemeFontRegistry(
          registration: (_, actual) async {
            expect(actual, bytes);
            expect(bridge.files, isEmpty);
            registered = true;
          },
        ),
      );
      final imported = await store.importFontFile(
        XFile.fromData(bytes, name: 'font.ttf', path: '/font.ttf'),
      );
      expect(registered, isTrue);
      expect(bridge.files[imported.storagePath], bytes);
      expect(bridge.maxChunkLength, 1024 * 1024);
      expect(await store.readFontBytes(imported.storagePath), bytes);
      expect(
        bridge.calls.where((name) => name == 'readBase64Chunk').length,
        66,
      );
    },
  );
}

class _FontStorageBridge extends OperitRuntimeBridge {
  final files = <String, Uint8List>{};
  final calls = <String>[];
  int maxChunkLength = 0;
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    final args = request.args as Map;
    final Object? result;
    switch (request.methodName) {
      case 'themeAssetsDirPath':
        result = 'runtime/data/user_assets/theme';
      case 'readBase64Chunk':
        final bytes = files[args['path'] as String];
        if (bytes == null) {
          result = null;
        } else {
          final offset = args['offset'] as int;
          final end = offset + (args['length'] as int);
          result = base64Encode(
            Uint8List.sublistView(
              bytes,
              offset,
              end < bytes.length ? end : bytes.length,
            ),
          );
        }
      default:
        throw StateError('Unexpected call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    expect(request.methodName, 'writeBytesStream');
    calls.add(request.methodName);
    return _FontSink(this, request.args as Map);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}

class _OversizedFont extends XFile {
  _OversizedFont() : super('/oversized.ttf');

  @override
  Future<int> length() async => themePrivateFontMaxBytes + 1;

  @override
  Future<Uint8List> readAsBytes() async =>
      throw StateError('Oversized font must not be read');
}

class _FontSink extends CorePushSink {
  _FontSink(this.bridge, this.args);
  final _FontStorageBridge bridge;
  final Map args;
  final BytesBuilder content = BytesBuilder(copy: false);

  @override
  Future<void> add(Object? args) async {
    final bytes = args as Uint8List;
    expect(bytes.length, lessThanOrEqualTo(1024 * 1024));
    if (bytes.length > bridge.maxChunkLength) {
      bridge.maxChunkLength = bytes.length;
    }
    content.add(bytes);
  }

  @override
  Future<void> close() async {
    expect(content.length, args['expectedByteLength']);
    bridge.files[args['path'] as String] = content.takeBytes();
  }
}

/// Represents bytes already read from a native catalog, with no selected file path.
class _DeviceFonts implements SystemFontsPlatform {
  @override
  Future<List<String>> listFamilies() async => ['Al Tarikh'];

  @override
  Future<Uint8List> readFamily(String family) async =>
      Uint8List.fromList([0, 1, 0, 0, 7, 8, 9]);
}
