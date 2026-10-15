import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebViewResourceLoader.dart';

void main() {
  ComposeDslWebViewResourceLoader create({
    String package = 'test.arcade',
    String scheme = 'https',
    ComposeDslWebViewResourceDecisionDispatcher? dispatch,
    Future<List<int>> Function(String)? read,
  }) {
    final loader = ComposeDslWebViewResourceLoader(
      packageName: package,
      scheme: scheme,
      dispatchDecision: dispatch ?? (_) async => {'action': 'block'},
      readFileBytes: read ?? (path) async => utf8.encode(path),
    );
    addTearDown(loader.close);
    return loader;
  }

  Future<Uri> map(ComposeDslWebViewResourceLoader loader, String url) =>
      loader.localUriFor(url, isMainFrame: true);

  test(
    'image file responses sniff magic bytes for imported .image paths',
    () async {
      final formats = {
        'image/png': [137, 80, 78, 71, 13, 10, 26, 10],
        'image/jpeg': [255, 216, 255, 224],
        'image/gif': ascii.encode('GIF89a'),
        'image/webp': [82, 73, 70, 70, 20, 0, 0, 0, 87, 69, 66, 80],
        'image/bmp': [66, 77, 0, 0],
      };
      for (final entry in formats.entries) {
        final loader = create(
          dispatch: (_) async => {
            'action': 'respond',
            'response': {'filePath': '/app/avatar.image'},
          },
          read: (_) async => entry.value,
        );
        final uri = await map(loader, 'https://characters.test/avatars/1');
        final response = await loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri'),
        );
        expect(response.statusCode, 200);
        expect(response.mimeType, entry.key);
        expect(response.body, entry.value);
      }
    },
  );

  test(
    'overlapping image requests share IO; later reads see new bytes and retry errors',
    () async {
      var reads = 0;
      var pending = Completer<List<int>>();
      final loader = create(
        dispatch: (_) async => {
          'action': 'respond',
          'response': {'filePath': '/app/avatar.image'},
        },
        read: (_) {
          reads++;
          return pending.future;
        },
      );
      final uri = await map(loader, 'https://characters.test/avatars/1');
      Future<WebViewLocalResourceResponse> load() =>
          loader.handleRequest(WebViewLocalResourceRequest(url: '$uri'));
      final first = List.generate(20, (_) => load());
      await Future<void>.delayed(Duration.zero);
      expect(reads, 1);
      pending.complete([137, 80, 78, 71, 13, 10, 26, 10]);
      expect(
        (await Future.wait(first)).every((r) => r.mimeType == 'image/png'),
        true,
      );
      pending = Completer<List<int>>();
      final changed = load();
      await Future<void>.delayed(Duration.zero);
      expect(reads, 2);
      pending.complete([255, 216, 255, 224]);
      expect((await changed).mimeType, 'image/jpeg');
      pending = Completer<List<int>>();
      final failed = List.generate(3, (_) => load());
      await Future<void>.delayed(Duration.zero);
      expect(reads, 3);
      pending.completeError(StateError('temporary'));
      expect(
        (await Future.wait(failed)).every((r) => r.statusCode == 500),
        true,
      );
      pending = Completer<List<int>>();
      final retry = load();
      await Future<void>.delayed(Duration.zero);
      pending.complete([66, 77]);
      expect((await retry).mimeType, 'image/bmp');
      expect(reads, 4);
    },
  );

  test('closing the image loader rejects a pending response', () async {
    final pending = Completer<List<int>>();
    final loader = create(
      dispatch: (_) async => {
        'action': 'respond',
        'response': {'filePath': '/app/avatar.image'},
      },
      read: (_) => pending.future,
    );
    final uri = await map(loader, 'https://characters.test/avatars/1');
    final response = loader.handleRequest(
      WebViewLocalResourceRequest(url: '$uri'),
    );
    await Future<void>.delayed(Duration.zero);
    await loader.close();
    pending.complete([66, 77]);
    expect((await response).statusCode, 410);
  });

  for (final scheme in ['https', 'operit-vfs']) {
    group(scheme, () {
      test(
        'origin is stable across instances and file export directories',
        () async {
          final first = create(scheme: scheme);
          final second = create(scheme: scheme);
          final a = await map(first, 'file:///old/export/index.html');
          final b = await map(second, 'file:///new/export/index.html');
          expect(a.scheme, scheme);
          expect(a.host, b.host);
          expect(a.hasPort, false);
          expect(a.host.endsWith('.operit.invalid'), true);
          expect(a.host.split('.').every((label) => label.length <= 63), true);
          await first.close();
          expect(
            (await map(
              create(scheme: scheme),
              'file:///third/index.html',
            )).host,
            a.host,
          );
        },
      );
      test(
        'plugin and original origin isolation; old mappings remain valid',
        () async {
          final loader = create(scheme: scheme);
          final urls = [
            'https://arcade.test/x',
            'http://arcade.test/x',
            'https://arcade.test:8443/x',
            'https://other.test/x',
            'file:///x',
          ];
          final uris = await Future.wait(urls.map((url) => map(loader, url)));
          expect(uris.map((u) => u.host).toSet().length, urls.length);
          for (var i = 0; i < urls.length; i++) {
            expect(loader.originalUrlFor(uris[i].toString()), urls[i]);
            expect(loader.matchesCurrentOrigin(urls[i]), true);
          }
          final other = create(package: 'test.other', scheme: scheme);
          expect((await map(other, urls.first)).host, isNot(uris.first.host));
          expect(other.ownsUrl(uris.first.toString()), false);
        },
      );
      test(
        'preserves encoded paths, raw queries, fragments and relative paths',
        () async {
          final loader = create(scheme: scheme);
          const url =
              'file:///vfs/a%20b/index.html?a=1&a=2&encoded=%2F&__operit_main_frame=7#hello';
          final uri = await map(loader, url);
          expect(loader.originalUrlFor(uri.toString()), url);
          expect(
            loader.originalUrlFor(
              uri.resolve('../image.png?q=%20#x').toString(),
            ),
            'file:///vfs/image.png?q=%20#x',
          );
          expect(await map(loader, uri.toString()), uri);
        },
      );
    });
  }

  test(
    'file responses preserve metadata and native request information',
    () async {
      final paths = <String>[];
      final loader = create(
        dispatch: (request) async {
          expect(request['url'], 'https://arcade.test/arcade/lobby');
          expect(request['method'], 'GET');
          expect(request['headers'], {'Accept': 'text/html'});
          expect(request['isMainFrame'], true);
          return {
            'action': 'respond',
            'response': {
              'filePath': '/vfs/game.html',
              'mimeType': 'text/html',
              'statusCode': 201,
              'reasonPhrase': 'Created',
              'headers': {'Cache-Control': 'no-store'},
            },
          };
        },
        read: (path) async {
          paths.add(path);
          return utf8.encode('<html>小游戏</html>');
        },
      );
      final uri = await map(loader, 'https://arcade.test/arcade/lobby');
      final response = await loader.handleRequest(
        WebViewLocalResourceRequest(
          url: '$uri',
          headers: {'Accept': 'text/html'},
          isMainFrame: true,
        ),
      );
      expect(response.statusCode, 201);
      expect(response.mimeType, 'text/html');
      expect(response.headers['Cache-Control'], 'no-store');
      expect(utf8.decode(response.body), '<html>小游戏</html>');
      expect(paths, ['/vfs/game.html']);
    },
  );

  test(
    'same plugin instances share origin but not resource callbacks',
    () async {
      final loaders = List.generate(
        3,
        (i) => create(
          dispatch: (_) async => {
            'action': 'respond',
            'response': {'text': 'view$i'},
          },
        ),
      );
      final uris = await Future.wait(
        loaders.map((l) => map(l, 'https://page.test/index.html')),
      );
      expect(uris.toSet().length, 1);
      final replies = await Future.wait(
        List.generate(
          3,
          (i) => loaders[i].handleRequest(
            WebViewLocalResourceRequest(url: '${uris[i]}'),
          ),
        ),
      );
      expect(replies.map((r) => utf8.decode(r.body)), [
        'view0',
        'view1',
        'view2',
      ]);
    },
  );

  test('binary, empty base64, HEAD and text encoding', () async {
    var data = <String, Object?>{'base64': 'AP+A'};
    final loader = create(
      dispatch: (_) async => {'action': 'respond', 'response': data},
    );
    final uri = await map(loader, 'file:///index.html');
    Future<WebViewLocalResourceResponse> request([String method = 'GET']) =>
        loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri', method: method),
        );
    expect((await request()).body, [0, 255, 128]);
    expect((await request('HEAD')).body, isEmpty);
    data = {'base64': '', 'text': 'ignored'};
    expect((await request()).body, isEmpty);
    data = {'text': 'é', 'encoding': 'latin1'};
    expect((await request()).body, [233]);
  });

  test('rewrites resolve internally and loops are bounded', () async {
    var loop = false;
    final loader = create(
      dispatch: (r) async {
        if (!loop && (r['url'] as String).endsWith('/target')) {
          return {
            'action': 'respond',
            'response': {'text': 'rewritten'},
          };
        }
        return {'action': 'rewrite', 'url': '/target'};
      },
    );
    final uri = await map(loader, 'https://page.test/start');
    final request = WebViewLocalResourceRequest(url: '$uri');
    expect(
      utf8.decode((await loader.handleRequest(request)).body),
      'rewritten',
    );
    loop = true;
    expect((await loader.handleRequest(request)).statusCode, 508);
  });

  test(
    'unmapped hosts, userinfo, ports and closed loaders are rejected',
    () async {
      final loader = create();
      final uri = await map(loader, 'file:///index.html');
      for (final bad in [
        uri.replace(host: 'other.operit.invalid'),
        uri.replace(userInfo: 'user'),
        uri.replace(port: 8123),
      ]) {
        expect(
          (await loader.handleRequest(
            WebViewLocalResourceRequest(url: '$bad'),
          )).statusCode,
          403,
        );
      }
      for (final bad in [
        'file://remote/a',
        'javascript:alert(1)',
        'https://spoof.operit.invalid/a',
      ]) {
        await expectLater(map(loader, bad), throwsArgumentError);
      }
      await loader.close();
      expect(
        (await loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri'),
        )).statusCode,
        410,
      );
      await expectLater(map(loader, 'file:///a'), throwsStateError);
    },
  );

  test(
    'errors are contained and unsupported body proxying is explicit',
    () async {
      var decision = <String, Object?>{'action': 'allow'};
      final loader = create(dispatch: (_) async => decision);
      final uri = await map(loader, 'https://page.test/index');
      expect(
        (await loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri', method: 'POST'),
        )).statusCode,
        405,
      );
      decision = {
        'action': 'respond',
        'response': {'base64': '!invalid!'},
      };
      expect(
        (await loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri'),
        )).statusCode,
        500,
      );
      decision = {
        'action': 'respond',
        'response': {'statusCode': 302},
      };
      expect(
        (await loader.handleRequest(
          WebViewLocalResourceRequest(url: '$uri'),
        )).statusCode,
        502,
      );
    },
  );
}
