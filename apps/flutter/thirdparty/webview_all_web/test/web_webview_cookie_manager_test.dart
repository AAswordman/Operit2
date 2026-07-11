import 'package:flutter_test/flutter_test.dart';
import 'package:web/web.dart' as web;
import 'package:webview_all_web/webview_all_web.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  group('WebWebViewCookieManager', () {
    late WebWebViewCookieManager manager;

    setUp(() {
      manager = WebWebViewCookieManager(
        const WebWebViewCookieManagerCreationParams(),
      );
    });

    test(
      'sets and reads visible cookies for the current browser origin',
      () async {
        final String name =
            'webview_all_test_${DateTime.now().microsecondsSinceEpoch}';

        await manager.setCookie(
          WebViewCookie(name: name, value: 'a b=c;d', domain: '', path: '/'),
        );

        final List<WebViewCookie> cookies = await manager.getCookies(
          Uri.parse(web.window.location.href),
        );
        final WebViewCookie cookie = cookies.singleWhere(
          (WebViewCookie cookie) => cookie.name == name,
        );

        expect(cookie.value, 'a b=c;d');

        web.document.cookie =
            '$name=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/';
      },
    );

    test(
      'rejects invalid cookie names before writing to document.cookie',
      () async {
        await expectLater(
          () => manager.setCookie(
            WebViewCookie(
              name: 'bad name',
              value: 'value',
              domain: '',
              path: '/',
            ),
          ),
          throwsA(isA<ArgumentError>()),
        );
      },
    );

    test(
      'rejects invalid cookie paths before writing to document.cookie',
      () async {
        await expectLater(
          () => manager.setCookie(
            WebViewCookie(
              name: 'valid_name',
              value: 'value',
              domain: '',
              path: 'relative',
            ),
          ),
          throwsA(isA<ArgumentError>()),
        );
      },
    );

    test(
      'rejects invalid cookie domains before writing to document.cookie',
      () async {
        await expectLater(
          () => manager.setCookie(
            WebViewCookie(
              name: 'valid_name',
              value: 'value',
              domain: 'example.com;bad',
              path: '/',
            ),
          ),
          throwsA(isA<ArgumentError>()),
        );
      },
    );
  });
}
