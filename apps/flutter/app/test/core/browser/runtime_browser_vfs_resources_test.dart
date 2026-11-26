import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserVfsResources.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserUrlUtils.dart';
import 'package:webview_all/webview_all.dart';

void main() {
  test(
    'VFS page and relative assets keep their identity across the local origin',
    () async {
      final paths = <String>[];
      final resources = RuntimeBrowserVfsResources((path) async {
        paths.add(path);
        return Uint8List.fromList([1, 2, 3]);
      });
      final publicUrl = normalizeWorkspaceBrowserUrl(
        '/operitapp/workspaces/demo/中文 page.html',
      );
      final local = resources.navigationUri(publicUrl, 'https');
      expect(resources.logicalUrl(local.toString()), publicUrl);
      await resources.handle(
        WebViewLocalResourceRequest(url: local.toString()),
      );
      expect(paths.removeLast(), '/operitapp/workspaces/demo/中文 page.html');
      final asset = local.resolve('assets/style.css');
      final response = await resources.handle(
        WebViewLocalResourceRequest(url: asset.toString()),
      );
      expect(paths, ['/operitapp/workspaces/demo/assets/style.css']);
      expect(response.mimeType, 'text/css');
      expect(response.body, [1, 2, 3]);
    },
  );

  test('external resources and unsupported methods cannot read VFS', () async {
    final resources = RuntimeBrowserVfsResources(
      (_) async => throw StateError('unexpected read'),
    );
    final external = await resources.handle(
      const WebViewLocalResourceRequest(url: 'https://example.com/a'),
    );
    final post = await resources.handle(
      const WebViewLocalResourceRequest(
        url: 'https://browser.operit.invalid/a',
        method: 'POST',
      ),
    );
    expect(external.statusCode, 404);
    expect(post.statusCode, 405);
    expect(
      () => resources.navigationUri('file:///operitapp/test.html', null),
      throwsUnsupportedError,
    );
  });
}
