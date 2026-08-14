import 'dart:typed_data';

import 'package:flutter/foundation.dart'
    show
        DebugPrintCallback,
        TargetPlatform,
        debugDefaultTargetPlatformOverride,
        debugPrint;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  test('WebViewController forwards public API calls to the platform', () async {
    final _FakePlatformWebViewController platform =
        _FakePlatformWebViewController();
    final WebViewController controller = WebViewController.fromPlatform(
      platform,
    );
    final _FakePlatformNavigationDelegate navigationDelegate =
        _FakePlatformNavigationDelegate();
    final Uint8List body = Uint8List.fromList(<int>[1, 2, 3]);

    await controller.loadFile('/tmp/index.html');
    await controller.loadFlutterAsset('assets/index.html');
    await controller.loadHtmlString(
      '<html></html>',
      baseUrl: 'https://example.test/base/',
    );
    await controller.loadRequest(
      Uri.parse('https://example.test/form'),
      method: LoadRequestMethod.post,
      headers: const <String, String>{'X-Test': 'yes'},
      body: body,
    );
    await controller.currentUrl();
    await controller.canGoBack();
    await controller.canGoForward();
    await controller.goBack();
    await controller.goForward();
    await controller.reload();
    await controller.setNavigationDelegate(
      NavigationDelegate.fromPlatform(navigationDelegate),
    );
    await controller.clearCache();
    await controller.clearLocalStorage();
    await controller.runJavaScript('window.value = 1');
    final Object result = await controller.runJavaScriptReturningResult(
      'window.value',
    );
    await controller.addJavaScriptChannel(
      'TestChannel',
      onMessageReceived: (JavaScriptMessage message) {
        platform.receivedJavaScriptMessage = message.message;
      },
    );
    platform.javaScriptChannelParams!.onMessageReceived(
      const JavaScriptMessage(message: 'hello'),
    );
    await controller.removeJavaScriptChannel('TestChannel');
    await controller.getTitle();
    await controller.scrollTo(10, 20);
    await controller.scrollBy(3, 4);
    await controller.getScrollPosition();
    await controller.enableZoom(false);
    await controller.setBackgroundColor(const Color(0xFF112233));
    await controller.setJavaScriptMode(JavaScriptMode.disabled);
    await controller.setUserAgent('test-agent');
    await controller.setOnConsoleMessage((JavaScriptConsoleMessage message) {
      platform.receivedConsoleMessage = message.message;
    });
    platform.onConsoleMessage!(
      const JavaScriptConsoleMessage(
        level: JavaScriptLogLevel.warning,
        message: 'warn',
      ),
    );
    await controller.setOnJavaScriptAlertDialog((
      JavaScriptAlertDialogRequest request,
    ) async {
      platform.receivedAlertMessage = request.message;
    });
    await platform.onJavaScriptAlertDialog!(
      const JavaScriptAlertDialogRequest(
        message: 'alert',
        url: 'https://example.test/',
      ),
    );
    await controller.setOnJavaScriptConfirmDialog((
      JavaScriptConfirmDialogRequest request,
    ) async {
      platform.receivedConfirmMessage = request.message;
      return false;
    });
    final bool confirmed = await platform.onJavaScriptConfirmDialog!(
      const JavaScriptConfirmDialogRequest(
        message: 'confirm',
        url: 'https://example.test/',
      ),
    );
    await controller.setOnJavaScriptTextInputDialog((
      JavaScriptTextInputDialogRequest request,
    ) async {
      platform.receivedPromptMessage = request.message;
      return 'typed';
    });
    final String promptText = await platform.onJavaScriptTextInputDialog!(
      const JavaScriptTextInputDialogRequest(
        message: 'prompt',
        url: 'https://example.test/',
        defaultText: 'default',
      ),
    );
    await controller.getUserAgent();
    await controller.setOnScrollPositionChange((ScrollPositionChange change) {
      platform.receivedScrollChange = change;
    });
    platform.onScrollPositionChange!(const ScrollPositionChange(7, 8));
    await controller.setVerticalScrollBarEnabled(false);
    await controller.setHorizontalScrollBarEnabled(false);
    final bool supportsScrollbars = await controller
        .supportsSetScrollBarsEnabled();
    await controller.setOverScrollMode(WebViewOverScrollMode.never);

    expect(platform.filePath, '/tmp/index.html');
    expect(platform.assetKey, 'assets/index.html');
    expect(platform.html, '<html></html>');
    expect(platform.baseUrl, 'https://example.test/base/');
    expect(
      platform.loadRequestParams!.uri,
      Uri.parse('https://example.test/form'),
    );
    expect(platform.loadRequestParams!.method, LoadRequestMethod.post);
    expect(platform.loadRequestParams!.headers, <String, String>{
      'X-Test': 'yes',
    });
    expect(platform.loadRequestParams!.body, body);
    expect(platform.navigationDelegate, navigationDelegate);
    expect(result, 42);
    expect(platform.javaScript, 'window.value = 1');
    expect(platform.javaScriptReturningResult, 'window.value');
    expect(platform.javaScriptChannelParams!.name, 'TestChannel');
    expect(platform.receivedJavaScriptMessage, 'hello');
    expect(platform.removedJavaScriptChannel, 'TestChannel');
    expect(platform.scrollToPosition, const Offset(10, 20));
    expect(platform.scrollByOffset, const Offset(3, 4));
    expect(platform.zoomEnabled, isFalse);
    expect(platform.backgroundColor, const Color(0xFF112233));
    expect(platform.javaScriptMode, JavaScriptMode.disabled);
    expect(platform.userAgent, 'test-agent');
    expect(platform.receivedConsoleMessage, 'warn');
    expect(platform.receivedAlertMessage, 'alert');
    expect(platform.receivedConfirmMessage, 'confirm');
    expect(confirmed, isFalse);
    expect(platform.receivedPromptMessage, 'prompt');
    expect(promptText, 'typed');
    expect(platform.receivedScrollChange?.x, 7);
    expect(platform.receivedScrollChange?.y, 8);
    expect(platform.verticalScrollBarEnabled, isFalse);
    expect(platform.horizontalScrollBarEnabled, isFalse);
    expect(supportsScrollbars, isTrue);
    expect(platform.overScrollMode, WebViewOverScrollMode.never);
  });

  test('WebViewController wraps platform permission requests', () async {
    final _FakePlatformWebViewController platform =
        _FakePlatformWebViewController();
    final List<WebViewPermissionRequest> requests =
        <WebViewPermissionRequest>[];
    WebViewController.fromPlatform(platform, onPermissionRequest: requests.add);
    final _FakePlatformWebViewPermissionRequest platformRequest =
        _FakePlatformWebViewPermissionRequest(
          types: const <WebViewPermissionResourceType>{
            WebViewPermissionResourceType.camera,
            WebViewPermissionResourceType.microphone,
          },
        );

    platform.onPermissionRequest!(platformRequest);
    await requests.single.grant();
    await requests.single.deny();

    expect(requests, hasLength(1));
    expect(requests.single.types, platformRequest.types);
    expect(requests.single.platform, platformRequest);
    expect(platformRequest.granted, isTrue);
    expect(platformRequest.denied, isTrue);
  });

  test('WebViewController ignores macOS background colors', () async {
    final _FakePlatformWebViewController platform =
        _FakePlatformWebViewController();
    final WebViewController controller = WebViewController.fromPlatform(
      platform,
    );
    final DebugPrintCallback previousDebugPrint = debugPrint;
    final List<String?> logs = <String?>[];
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    debugPrint = (String? message, {int? wrapWidth}) {
      logs.add(message);
    };
    addTearDown(() {
      debugDefaultTargetPlatformOverride = null;
      debugPrint = previousDebugPrint;
    });

    await controller.setBackgroundColor(const Color(0x80000000));
    await controller.setBackgroundColor(const Color(0xff112233));

    expect(platform.backgroundColor, isNull);
    expect(logs, hasLength(2));
    expect(logs.first, contains('setBackgroundColor is not supported'));
  });

  test('NavigationDelegate registers and wraps platform callbacks', () async {
    final _FakePlatformNavigationDelegate platform =
        _FakePlatformNavigationDelegate();
    final List<String> pageEvents = <String>[];
    final List<int> progressEvents = <int>[];
    final List<WebResourceError> resourceErrors = <WebResourceError>[];
    final List<UrlChange> urlChanges = <UrlChange>[];
    final List<HttpAuthRequest> authRequests = <HttpAuthRequest>[];
    final List<HttpResponseError> httpErrors = <HttpResponseError>[];
    final List<SslAuthError> sslErrors = <SslAuthError>[];

    final NavigationDelegate delegate = NavigationDelegate.fromPlatform(
      platform,
      onNavigationRequest: (NavigationRequest request) {
        pageEvents.add('request:${request.url}');
        return NavigationDecision.prevent;
      },
      onPageStarted: (String url) => pageEvents.add('started:$url'),
      onPageFinished: (String url) => pageEvents.add('finished:$url'),
      onProgress: progressEvents.add,
      onWebResourceError: resourceErrors.add,
      onUrlChange: urlChanges.add,
      onHttpAuthRequest: authRequests.add,
      onHttpError: httpErrors.add,
      onSslAuthError: sslErrors.add,
    );

    final NavigationDecision decision = await platform.onNavigationRequest!(
      const NavigationRequest(url: 'https://example.test/', isMainFrame: true),
    );
    platform.onPageStarted!('https://example.test/start');
    platform.onPageFinished!('https://example.test/finish');
    platform.onProgress!(75);
    platform.onWebResourceError!(
      const WebResourceError(errorCode: 1, description: 'error'),
    );
    platform.onUrlChange!(const UrlChange(url: 'https://example.test/change'));
    platform.onHttpAuthRequest!(
      HttpAuthRequest(
        host: 'example.test',
        realm: 'restricted',
        onProceed: (_) {},
        onCancel: () {},
      ),
    );
    platform.onHttpError!(
      HttpResponseError(
        request: WebResourceRequest(uri: Uri.parse('https://example.test/')),
        response: WebResourceResponse(
          uri: Uri.parse('https://example.test/'),
          statusCode: 500,
        ),
      ),
    );
    final _FakePlatformSslAuthError platformSslError =
        _FakePlatformSslAuthError();
    platform.onSslAuthError!(platformSslError);
    await sslErrors.single.proceed();
    await sslErrors.single.cancel();

    expect(delegate.platform, platform);
    expect(decision, NavigationDecision.prevent);
    expect(pageEvents, <String>[
      'request:https://example.test/',
      'started:https://example.test/start',
      'finished:https://example.test/finish',
    ]);
    expect(progressEvents, <int>[75]);
    expect(resourceErrors.single.description, 'error');
    expect(urlChanges.single.url, 'https://example.test/change');
    expect(authRequests.single.host, 'example.test');
    expect(httpErrors.single.response?.statusCode, 500);
    expect(sslErrors.single.platform, platformSslError);
    expect(platformSslError.proceeded, isTrue);
    expect(platformSslError.cancelled, isTrue);
  });

  testWidgets('WebViewWidget delegates build to the platform widget', (
    WidgetTester tester,
  ) async {
    final _FakePlatformWebViewController controller =
        _FakePlatformWebViewController();
    final _FakePlatformWebViewWidget platform = _FakePlatformWebViewWidget(
      PlatformWebViewWidgetCreationParams(
        controller: controller,
        layoutDirection: TextDirection.rtl,
      ),
    );
    final WebViewWidget widget = WebViewWidget.fromPlatform(platform: platform);

    expect(widget.layoutDirection, TextDirection.rtl);
    await tester.pumpWidget(
      Directionality(textDirection: TextDirection.ltr, child: widget),
    );

    expect(find.text('fake-webview'), findsOneWidget);
    expect(platform.buildCount, 1);
  });
}

class _FakePlatformWebViewController extends PlatformWebViewController {
  _FakePlatformWebViewController()
    : super.implementation(const PlatformWebViewControllerCreationParams());

  String? filePath;
  String? assetKey;
  String? html;
  String? baseUrl;
  LoadRequestParams? loadRequestParams;
  PlatformNavigationDelegate? navigationDelegate;
  String? javaScript;
  String? javaScriptReturningResult;
  JavaScriptChannelParams? javaScriptChannelParams;
  String? receivedJavaScriptMessage;
  String? removedJavaScriptChannel;
  Offset? scrollToPosition;
  Offset? scrollByOffset;
  bool? zoomEnabled;
  Color? backgroundColor;
  JavaScriptMode? javaScriptMode;
  String? userAgent;
  void Function(PlatformWebViewPermissionRequest request)? onPermissionRequest;
  void Function(JavaScriptConsoleMessage consoleMessage)? onConsoleMessage;
  String? receivedConsoleMessage;
  Future<void> Function(JavaScriptAlertDialogRequest request)?
  onJavaScriptAlertDialog;
  String? receivedAlertMessage;
  Future<bool> Function(JavaScriptConfirmDialogRequest request)?
  onJavaScriptConfirmDialog;
  String? receivedConfirmMessage;
  Future<String> Function(JavaScriptTextInputDialogRequest request)?
  onJavaScriptTextInputDialog;
  String? receivedPromptMessage;
  void Function(ScrollPositionChange scrollPositionChange)?
  onScrollPositionChange;
  ScrollPositionChange? receivedScrollChange;
  bool? verticalScrollBarEnabled;
  bool? horizontalScrollBarEnabled;
  WebViewOverScrollMode? overScrollMode;

  @override
  Future<void> loadFile(String absoluteFilePath) async {
    filePath = absoluteFilePath;
  }

  @override
  Future<void> loadFlutterAsset(String key) async {
    assetKey = key;
  }

  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) async {
    this.html = html;
    this.baseUrl = baseUrl;
  }

  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    loadRequestParams = params;
  }

  @override
  Future<String?> currentUrl() async => 'https://example.test/current';

  @override
  Future<bool> canGoBack() async => true;

  @override
  Future<bool> canGoForward() async => false;

  @override
  Future<void> goBack() async {}

  @override
  Future<void> goForward() async {}

  @override
  Future<void> reload() async {}

  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {
    navigationDelegate = handler;
  }

  @override
  Future<void> clearCache() async {}

  @override
  Future<void> clearLocalStorage() async {}

  @override
  Future<void> runJavaScript(String javaScript) async {
    this.javaScript = javaScript;
  }

  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    javaScriptReturningResult = javaScript;
    return 42;
  }

  @override
  Future<void> addJavaScriptChannel(
    JavaScriptChannelParams javaScriptChannelParams,
  ) async {
    this.javaScriptChannelParams = javaScriptChannelParams;
  }

  @override
  Future<void> removeJavaScriptChannel(String javaScriptChannelName) async {
    removedJavaScriptChannel = javaScriptChannelName;
  }

  @override
  Future<String?> getTitle() async => 'Example';

  @override
  Future<void> scrollTo(int x, int y) async {
    scrollToPosition = Offset(x.toDouble(), y.toDouble());
  }

  @override
  Future<void> scrollBy(int x, int y) async {
    scrollByOffset = Offset(x.toDouble(), y.toDouble());
  }

  @override
  Future<Offset> getScrollPosition() async => const Offset(5, 6);

  @override
  Future<void> enableZoom(bool enabled) async {
    zoomEnabled = enabled;
  }

  @override
  Future<void> setBackgroundColor(Color color) async {
    backgroundColor = color;
  }

  @override
  Future<void> setJavaScriptMode(JavaScriptMode javaScriptMode) async {
    this.javaScriptMode = javaScriptMode;
  }

  @override
  Future<void> setUserAgent(String? userAgent) async {
    this.userAgent = userAgent;
  }

  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest request) onPermissionRequest,
  ) async {
    this.onPermissionRequest = onPermissionRequest;
  }

  @override
  Future<String?> getUserAgent() async => userAgent;

  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage consoleMessage) onConsoleMessage,
  ) async {
    this.onConsoleMessage = onConsoleMessage;
  }

  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest request)
    onJavaScriptAlertDialog,
  ) async {
    this.onJavaScriptAlertDialog = onJavaScriptAlertDialog;
  }

  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest request)
    onJavaScriptConfirmDialog,
  ) async {
    this.onJavaScriptConfirmDialog = onJavaScriptConfirmDialog;
  }

  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest request)
    onJavaScriptTextInputDialog,
  ) async {
    this.onJavaScriptTextInputDialog = onJavaScriptTextInputDialog;
  }

  @override
  Future<void> setOnScrollPositionChange(
    void Function(ScrollPositionChange scrollPositionChange)?
    onScrollPositionChange,
  ) async {
    this.onScrollPositionChange = onScrollPositionChange;
  }

  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) async {
    verticalScrollBarEnabled = enabled;
  }

  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) async {
    horizontalScrollBarEnabled = enabled;
  }

  @override
  bool supportsSetScrollBarsEnabled() => true;

  @override
  Future<void> setOverScrollMode(WebViewOverScrollMode mode) async {
    overScrollMode = mode;
  }
}

class _FakePlatformNavigationDelegate extends PlatformNavigationDelegate {
  _FakePlatformNavigationDelegate()
    : super.implementation(const PlatformNavigationDelegateCreationParams());

  NavigationRequestCallback? onNavigationRequest;
  PageEventCallback? onPageStarted;
  PageEventCallback? onPageFinished;
  ProgressCallback? onProgress;
  WebResourceErrorCallback? onWebResourceError;
  UrlChangeCallback? onUrlChange;
  HttpAuthRequestCallback? onHttpAuthRequest;
  HttpResponseErrorCallback? onHttpError;
  SslAuthErrorCallback? onSslAuthError;

  @override
  Future<void> setOnNavigationRequest(
    NavigationRequestCallback onNavigationRequest,
  ) async {
    this.onNavigationRequest = onNavigationRequest;
  }

  @override
  Future<void> setOnPageStarted(PageEventCallback onPageStarted) async {
    this.onPageStarted = onPageStarted;
  }

  @override
  Future<void> setOnPageFinished(PageEventCallback onPageFinished) async {
    this.onPageFinished = onPageFinished;
  }

  @override
  Future<void> setOnProgress(ProgressCallback onProgress) async {
    this.onProgress = onProgress;
  }

  @override
  Future<void> setOnWebResourceError(
    WebResourceErrorCallback onWebResourceError,
  ) async {
    this.onWebResourceError = onWebResourceError;
  }

  @override
  Future<void> setOnUrlChange(UrlChangeCallback onUrlChange) async {
    this.onUrlChange = onUrlChange;
  }

  @override
  Future<void> setOnHttpAuthRequest(
    HttpAuthRequestCallback onHttpAuthRequest,
  ) async {
    this.onHttpAuthRequest = onHttpAuthRequest;
  }

  @override
  Future<void> setOnHttpError(HttpResponseErrorCallback onHttpError) async {
    this.onHttpError = onHttpError;
  }

  @override
  Future<void> setOnSSlAuthError(SslAuthErrorCallback onSslAuthError) async {
    this.onSslAuthError = onSslAuthError;
  }
}

// ignore: must_be_immutable
class _FakePlatformWebViewPermissionRequest
    extends PlatformWebViewPermissionRequest {
  _FakePlatformWebViewPermissionRequest({required super.types});

  bool granted = false;
  bool denied = false;

  @override
  Future<void> grant() async {
    granted = true;
  }

  @override
  Future<void> deny() async {
    denied = true;
  }
}

class _FakePlatformSslAuthError extends PlatformSslAuthError {
  _FakePlatformSslAuthError()
    : super(certificate: null, description: 'ssl error');

  bool proceeded = false;
  bool cancelled = false;

  @override
  Future<void> proceed() async {
    proceeded = true;
  }

  @override
  Future<void> cancel() async {
    cancelled = true;
  }
}

class _FakePlatformWebViewWidget extends PlatformWebViewWidget {
  _FakePlatformWebViewWidget(super.params) : super.implementation();

  int buildCount = 0;

  @override
  Widget build(BuildContext context) {
    buildCount += 1;
    return const Text('fake-webview');
  }
}
