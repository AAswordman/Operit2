import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  test('WebViewCookieManager forwards platform cookie APIs', () async {
    final fakeCookieManager = _FakePlatformWebViewCookieManager();
    WebViewPlatform.instance = _FakeWebViewPlatform(fakeCookieManager);

    final manager = WebViewCookieManager();
    final cookie = WebViewCookie(
      name: 'session',
      value: 'abc',
      domain: 'example.com',
    );

    expect(await manager.clearCookies(), isTrue);
    await manager.setCookie(cookie);
    final cookies = await manager.getCookies(
      domain: Uri.parse('https://example.com'),
    );

    expect(fakeCookieManager.cleared, isTrue);
    expect(fakeCookieManager.setCookieValue, cookie);
    expect(fakeCookieManager.getCookiesUrl, Uri.parse('https://example.com'));
    expect(cookies, <WebViewCookie>[cookie]);
  });
}

class _FakeWebViewPlatform extends WebViewPlatform {
  _FakeWebViewPlatform(this.cookieManager);

  final _FakePlatformWebViewCookieManager cookieManager;

  @override
  PlatformWebViewCookieManager createPlatformCookieManager(
    PlatformWebViewCookieManagerCreationParams params,
  ) {
    return cookieManager;
  }
}

class _FakePlatformWebViewCookieManager extends PlatformWebViewCookieManager {
  _FakePlatformWebViewCookieManager()
    : super.implementation(const PlatformWebViewCookieManagerCreationParams());

  bool cleared = false;
  WebViewCookie? setCookieValue;
  Uri? getCookiesUrl;

  @override
  Future<bool> clearCookies() async {
    cleared = true;
    return true;
  }

  @override
  Future<void> setCookie(WebViewCookie cookie) async {
    setCookieValue = cookie;
  }

  @override
  Future<List<WebViewCookie>> getCookies(Uri url) async {
    getCookiesUrl = url;
    return <WebViewCookie>[setCookieValue!];
  }
}
