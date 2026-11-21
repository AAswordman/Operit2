import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all_ohos/src/ohos_webview_controller.dart';
import 'package:webview_all_ohos/src/ohos_webview_native.dart' as native;
import 'package:webview_platform_interface/webview_platform_interface.dart';

/// Verifies application handoffs and subframe boundaries at the ArkWeb Dart adapter.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const host = MethodChannel('com.abandoft.webview_all_ohos/host_api');
  const external = MethodChannel('operit/webview_external_navigation');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  setUp(() {
    messenger.setMockMethodCallHandler(host, (_) async => null);
  });
  tearDown(() {
    messenger.setMockMethodCallHandler(host, null);
    messenger.setMockMethodCallHandler(external, null);
  });

  test(
    'application launches wait for the native result and preserve refusal',
    () async {
      final pending = Completer<bool>();
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(external, (call) {
        calls.add(call);
        return pending.future;
      });
      final controller = OhosWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      var completed = false;
      final opening = controller
          .openExternalApplication('weixin://chat?id=1')
          .then((value) {
            completed = true;
            return value;
          });
      await Future<void>.delayed(Duration.zero);
      expect(completed, isFalse);
      expect(calls.single.method, 'open');
      expect(calls.single.arguments, {'url': 'weixin://chat?id=1'});
      pending.complete(false);
      expect(await opening, isFalse);

      messenger.setMockMethodCallHandler(external, (_) async {
        throw PlatformException(
          code: 'external_navigation_failed',
          message: 'System refused',
        );
      });
      await expectLater(
        controller.openExternalApplication('missing://app'),
        throwsA(isA<PlatformException>()),
      );
    },
  );

  test(
    'application subframes reach the owner without replacing the main page',
    () async {
      final delegate = OhosNavigationDelegate(
        const PlatformNavigationDelegateCreationParams(),
      );
      final requests = <NavigationRequest>[];
      await delegate.setOnLoadRequest(
        (_) async => fail('A subframe must not replace the main page'),
      );
      await delegate.setOnNavigationRequest((request) {
        requests.add(request);
        return NavigationDecision.prevent;
      });
      final view = native.WebView();
      for (final url in ['https://example.test/frame', 'weixin://chat']) {
        delegate.ohosWebViewClient.requestLoading!(
          view,
          native.WebResourceRequest(
            url: url,
            isForMainFrame: false,
            isRedirect: false,
            hasGesture: true,
            method: 'GET',
            requestHeaders: {},
          ),
        );
      }
      expect(requests.single.url, 'weixin://chat');
      expect(requests.single.isMainFrame, isFalse);
    },
  );
}
