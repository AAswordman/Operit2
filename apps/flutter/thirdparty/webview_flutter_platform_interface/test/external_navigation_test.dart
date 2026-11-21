import 'package:flutter_test/flutter_test.dart';
import 'package:url_launcher_platform_interface/link.dart';
import 'package:url_launcher_platform_interface/url_launcher_platform_interface.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies that shared adapters preserve the exact URL and external launch mode.
void main() {
  late UrlLauncherPlatform original;
  late _Launcher launcher;
  setUp(() {
    original = UrlLauncherPlatform.instance;
    UrlLauncherPlatform.instance = launcher = _Launcher();
  });
  tearDown(() {
    UrlLauncherPlatform.instance = original;
  });

  test(
    'custom schemes and HTTPS use the platform application resolver',
    () async {
      for (final url in [
        'mailto:a@example.test?subject=Hello%20world',
        'weixin://chat?id=1',
        'https://example.test/app',
      ]) {
        expect(await _Controller().openExternalApplication(url), isTrue);
        expect(launcher.url, url);
        expect(launcher.options!.mode, PreferredLaunchMode.externalApplication);
      }
    },
  );
  test('failed launches are not advertised as successful', () async {
    launcher.result = false;
    expect(
      await _Controller().openExternalApplication('missing://app'),
      isFalse,
    );
  });
  test('browser-private resources never reach the application resolver', () {
    for (final url in [
      'file:///private',
      'content://private/file',
      'about:blank',
      'data:text/plain,test',
      'blob:https://example.test/id',
      'javascript:void(0)',
    ]) {
      expect(
        () => _Controller().openExternalApplication(url),
        throwsArgumentError,
      );
    }
    expect(launcher.url, isNull);
  });
  test(
    'Android Intent syntax is rejected by other platform adapters',
    () async {
      expect(
        () => _Controller().openExternalApplication(
          'intent://scan/#Intent;scheme=zxing;end',
        ),
        throwsUnsupportedError,
      );
      expect(launcher.url, isNull);
    },
  );
}

class _Controller extends PlatformWebViewController {
  /// Uses the shared implementation without creating a native browser view.
  _Controller()
    : super.implementation(const PlatformWebViewControllerCreationParams());
}

class _Launcher extends UrlLauncherPlatform {
  String? url;
  LaunchOptions? options;
  bool result = true;

  /// Leaves link widgets unused while testing imperative application launches.
  @override
  LinkDelegate? get linkDelegate => null;

  /// Records the request delivered to the OS-specific launcher implementation.
  @override
  Future<bool> launchUrl(String url, LaunchOptions options) async {
    this.url = url;
    this.options = options;
    return result;
  }
}
