import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/markdown/MarkdownImageRenderer.dart';
import 'package:operit2/ui/common/markdown/MarkdownRemoteImage.dart';
import 'package:operit2/ui/features/packages/screens/ComposeDslImageSource.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

final png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
);

/// Verifies shared VFS image resolution and the production Markdown rendering path.
void main() {
  testWidgets(
    'Markdown shared images read Core VFS and reuse the decoded image',
    (tester) async {
      const path =
          '/app/data/plugin_data/space/com.operit.image_display/chart.png';
      final bridge = _VfsFileBridge({
        '__type': 'BinaryFileContentData',
        'path': path,
        'contentBase64': base64Encode(png),
        'size': png.length,
      });
      final clients = GeneratedCoreProxyClients(bridge);
      final source = extractMarkdownImageUrl('![Chart]($path)');

      /// Renders the same Markdown image through one stable Core connection.
      Future<void> render() async {
        await tester.pumpWidget(
          MaterialApp(
            home: SizedBox(
              width: 200,
              height: 140,
              child: MarkdownRemoteImage(
                url: source,
                alt: 'Chart',
                clients: clients,
              ),
            ),
          ),
        );
        await tester.pump();
        await tester.runAsync(
          () => precacheImage(
            tester.widget<Image>(find.byType(Image)).image,
            tester.element(find.byType(MarkdownRemoteImage)),
          ),
        );
        await tester.pump();
      }

      await render();
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
      expect(bridge.calls.single.methodName, 'executeTool');
      expect((bridge.calls.single.args as Map)['tool'], {
        'name': 'read_file_binary',
        'parameters': [
          {'name': 'path', 'value': path},
        ],
      });
      await render();
      expect(bridge.calls, hasLength(1));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Markdown VFS failures display an error after one file request', (
    tester,
  ) async {
    final bridge = _VfsFileBridge(
      {'__type': 'StringResultData', 'value': ''},
      success: false,
      error: 'VFS access denied',
    );
    await tester.pumpWidget(
      MaterialApp(
        home: MarkdownRemoteImage(
          url:
              '/app/data/plugin_data/space/com.operit.image_display/missing.png',
          clients: GeneratedCoreProxyClients(bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    expect(bridge.calls.single.methodName, 'executeTool');
    expect(tester.takeException(), isNull);
  });

  test(
    'VFS uses the generated Core binary file service without rewriting the path',
    () async {
      const path = '/app/data/cache/toolpkg_resource_exports/operit-avatar.png';
      final bridge = _VfsFileBridge({
        '__type': 'BinaryFileContentData',
        'path': path,
        'contentBase64': base64Encode(png),
        'size': png.length,
      });
      final bytes = await readComposeDslVfsBytes(
        GeneratedCoreProxyClients(bridge),
        path,
      );
      expect(bytes, orderedEquals(png));
      final request = bridge.calls.single;
      expect(request.methodName, 'executeTool');
      expect((request.args as Map)['tool'], {
        'name': 'read_file_binary',
        'parameters': [
          {'name': 'path', 'value': path},
        ],
      });
    },
  );
  test(
    'Core VFS rejection preserves the original error and rejects nonbinary results',
    () async {
      final result = {'__type': 'StringResultData', 'value': 'not binary'};
      final rejected = _VfsFileBridge(
        result,
        success: false,
        error: 'VFS access denied',
      );
      await expectLater(
        readComposeDslVfsBytes(
          GeneratedCoreProxyClients(rejected),
          '/app/denied.png',
        ),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            contains('VFS access denied'),
          ),
        ),
      );
      final wrongType = _VfsFileBridge(result);
      await expectLater(
        readComposeDslVfsBytes(
          GeneratedCoreProxyClients(wrongType),
          '/app/wrong.png',
        ),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            contains('binary file content'),
          ),
        ),
      );
    },
  );
  test(
    'known virtual roots and explicit virtual URIs resolve before OS files',
    () async {
      final cases = <String, String>{
        '/app/data/cache/toolpkg_resource_exports/operit-avatar.png':
            '/app/data/cache/toolpkg_resource_exports/operit-avatar.png',
        '/mnt/macos/Users/test/avatar.png': '/mnt/macos/Users/test/avatar.png',
        'file:///app/data/avatar.png': '/app/data/avatar.png',
        'file:///app/data/avatar%20one.png': '/app/data/avatar one.png',
        'file://localhost/app/data/avatar.png': '/app/data/avatar.png',
        'vfs:///app/data/avatar.png': '/app/data/avatar.png',
        'operit-vfs://app/data/avatar.png': '/app/data/avatar.png',
        'runtime/data/user_assets/avatar.png':
            'runtime/data/user_assets/avatar.png',
      };
      for (final entry in cases.entries) {
        final reads = <String>[];
        final provider = await resolveComposeDslImage(
          entry.key,
          readVfsBytes: (path) async {
            reads.add(path);
            return png;
          },
        );
        expect(
          reads,
          isEmpty,
          reason: 'Reads begin only when an image is displayed',
        );
        expect(provider, isA<ComposeDslVfsImage>());
        expect((provider as ComposeDslVfsImage).path, entry.value);
        expect(await provider.readBytes(provider.path), same(png));
        expect(reads, [entry.value]);
      }
    },
  );
  test('ordinary data network native file sources do not call VFS', () async {
    Future<Uint8List> noVfs(String path) async =>
        throw StateError('Unexpected VFS: $path');
    expect(
      await resolveComposeDslImage(
        'data:image/png;base64,${base64Encode(png)}',
        readVfsBytes: noVfs,
      ),
      isA<MemoryImage>(),
    );
    expect(
      await resolveComposeDslImage(
        'https://example.test/avatar.png',
        readVfsBytes: noVfs,
      ),
      isA<NetworkImage>(),
    );
    expect(
      await resolveComposeDslImage(
        '/Users/test/avatar.png',
        readVfsBytes: noVfs,
      ),
      isA<FileImage>(),
    );
    expect(
      await resolveComposeDslImage(
        'file:///Users/test/avatar.png',
        readVfsBytes: noVfs,
      ),
      isA<FileImage>(),
    );
  });
  test(
    'identical data avatars share bytes and Flutter image-cache identity',
    () async {
      final source = 'data:image/png;base64,${base64Encode(png)}';
      final providers = await Future.wait(
        List.generate(
          20,
          (_) => resolveComposeDslImage(
            source,
            readVfsBytes: (_) async => throw StateError('Unexpected VFS'),
          ),
        ),
      );
      expect(providers.toSet(), hasLength(1));
      final provider = providers.first as MemoryImage;
      expect(provider.bytes, orderedEquals(png));
      for (final item in providers) {
        expect(await item.obtainKey(ImageConfiguration.empty), same(provider));
      }
    },
  );
  test(
    'data-avatar cache is LRU bounded and oversized entries are not retained',
    () {
      final first = 'data:image/png;base64,${base64Encode(png)}';
      final second = 'data:image/png;base64,${base64Encode([...png, 1])}';
      final third = 'data:image/png;base64,${base64Encode([...png, 2])}';
      final cache = ComposeDslDataImageCache(maximumEntries: 2);
      final a = cache.resolve(first), b = cache.resolve(second);
      expect(cache.resolve(first), same(a)); // Make A recent, so C evicts B.
      cache.resolve(third);
      expect(cache.resolve(first), same(a));
      expect(cache.resolve(second), isNot(same(b)));
      final tiny = ComposeDslDataImageCache(maximumBytes: 1);
      expect(tiny.resolve(first), isNot(same(tiny.resolve(first))));
      final budget = first.length * 2 + png.length;
      final one = ComposeDslDataImageCache(maximumBytes: budget);
      final retained = one.resolve(first);
      one.resolve(second); // Larger than budget, so does not evict retained A.
      expect(one.resolve(first), same(retained));
    },
  );
  testWidgets(
    'actual DSL VFS image survives repaint without another read or image provider',
    (tester) async {
      final reads = <String>[];
      final host = ComposeDslWebViewHostContext(
        packageName: 'test.images',
        routeInstanceId: 'route',
        executionContextKey: 'session',
        dispatchAction: (_, [payload]) async => null,
        runtimeOptionsProvider: () => {},
        readFileBytes: (path) async {
          reads.add(path);
          return png;
        },
      );
      Widget render(String path) => MaterialApp(
        home: buildComposeDslLayoutForTest(
          node: {
            'type': 'Image',
            'props': {'key': 'avatar', 'path': path, 'width': 44, 'height': 44},
            'children': [],
            'slots': {},
          },
          hostContext: host,
          splitMarkdownContent: (_) async => [],
          onTextInput: (_, text) async => null,
        ),
      );
      await tester.pumpWidget(render('/app/data/cache/avatar.png'));
      await tester.pumpAndSettle();
      expect(reads, ['/app/data/cache/avatar.png']);
      expect(tester.takeException(), isNull);
      final first = tester.widget<Image>(find.byType(Image)).image;
      expect(first, isA<ComposeDslVfsImage>());
      await tester.pumpWidget(render('/app/data/cache/avatar.png'));
      await tester.pumpAndSettle();
      expect(reads, hasLength(1));
      expect(tester.widget<Image>(find.byType(Image)).image, same(first));
      await tester.pumpWidget(render('file:///mnt/macos/avatar.png'));
      await tester.pumpAndSettle();
      expect(reads, ['/app/data/cache/avatar.png', '/mnt/macos/avatar.png']);
      expect(
        tester.widget<Image>(find.byType(Image)).image,
        isNot(same(first)),
      );
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets('same-path images refresh when the owning session changes', (
    tester,
  ) async {
    final reads = <String>[];
    Widget render(String session) => MaterialApp(
      home: buildComposeDslLayoutForTest(
        node: {
          'type': 'Image',
          'props': {'path': '/app/avatar.image'},
          'children': [],
          'slots': {},
        },
        hostContext: ComposeDslWebViewHostContext(
          packageName: 'test.images',
          routeInstanceId: 'route',
          executionContextKey: session,
          dispatchAction: (_, [payload]) async => null,
          runtimeOptionsProvider: () => {},
          readFileBytes: (_) async {
            reads.add(session);
            return png;
          },
        ),
        splitMarkdownContent: (_) async => [],
        onTextInput: (_, text) async => null,
      ),
    );
    await tester.pumpWidget(render('first'));
    await tester.pumpAndSettle();
    final first = tester.widget<Image>(find.byType(Image)).image;
    await tester.pumpWidget(render('second'));
    await tester.pumpAndSettle();
    expect(reads, ['first', 'second']);
    expect(
      tester.widget<Image>(find.byType(Image)).image,
      isNot(equals(first)),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'twenty VFS images share one in-flight read and retry failed reads',
    (tester) async {
      var reads = 0;
      var fail = true;
      final pending = Completer<Uint8List>();
      Future<Uint8List> read(String path) {
        reads++;
        if (fail) return Future.error(StateError('VFS denied'));
        return pending.future;
      }

      const path = '/app/data/shared-avatar.png';
      final providers = await Future.wait(
        List.generate(
          20,
          (_) => resolveComposeDslImage(
            path,
            readVfsBytes: read,
            vfsOwner: 'shared-test',
          ),
        ),
      );
      expect(providers.toSet(), hasLength(1));
      Widget images() => MaterialApp(
        home: Row(
          children: providers
              .map(
                (provider) => Image(
                  image: provider,
                  width: 1,
                  height: 1,
                  errorBuilder: (_, error, stack) => const SizedBox(),
                ),
              )
              .toList(),
        ),
      );
      await tester.pumpWidget(images());
      await tester.pumpAndSettle();
      expect(reads, 1);
      await tester.pumpWidget(const SizedBox());
      fail = false;
      await tester.pumpWidget(images());
      await tester.pump();
      expect(reads, 2);
      pending.complete(png);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final otherOwner = await resolveComposeDslImage(
        path,
        readVfsBytes: read,
        vfsOwner: 'other-session',
      );
      expect(otherOwner, isNot(equals(providers.first)));
      expect(await providers.first.evict(), isTrue);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(images());
      await tester.pumpAndSettle();
      expect(
        reads,
        3,
        reason: 'An explicitly evicted path must read fresh content',
      );
    },
  );

  testWidgets(
    'failed native VFS image displays an error affordance without framework file exception',
    (tester) async {
      final host = ComposeDslWebViewHostContext(
        packageName: 'test.images',
        routeInstanceId: 'route',
        executionContextKey: 'session',
        dispatchAction: (_, [payload]) async => null,
        runtimeOptionsProvider: () => {},
        readFileBytes: (_) async => throw StateError('VFS denied'),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: buildComposeDslLayoutForTest(
            node: {
              'type': 'Image',
              'props': {'path': '/app/data/missing.png'},
              'children': [],
              'slots': {},
            },
            hostContext: host,
            splitMarkdownContent: (_) async => [],
            onTextInput: (_, text) async => null,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}

/// Uses the actual binary bridge ABI and generated client decoder.
class _VfsFileBridge extends OperitRuntimeBridge {
  _VfsFileBridge(this.result, {this.success = true, this.error});
  final Map<String, Object?> result;
  final bool success;
  final String? error;
  final calls = <CoreCallRequest>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': success,
        'result': result,
        'error': error,
      },
    ]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
