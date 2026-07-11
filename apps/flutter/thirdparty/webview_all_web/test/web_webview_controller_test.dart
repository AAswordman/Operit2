// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:convert';
import 'dart:js_interop';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mockito/annotations.dart';
import 'package:mockito/mockito.dart';
import 'package:web/web.dart' as web;
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:webview_all_web/webview_all_web.dart';

import 'web_webview_controller_test.mocks.dart';

@GenerateMocks(
  <Type>[],
  customMocks: <MockSpec<Object>>[
    MockSpec<HttpRequestFactory>(onMissingStub: OnMissingStub.returnDefault),
  ],
)
void main() {
  WidgetsFlutterBinding.ensureInitialized();

  group('WebWebViewController', () {
    group('WebWebViewControllerCreationParams', () {
      test('sets iFrame fields', () {
        final params = WebWebViewControllerCreationParams();

        expect(params.iFrame.id, contains('webView'));
        expect(params.iFrame.style.width, '100%');
        expect(params.iFrame.style.height, '100%');
        expect(params.iFrame.style.borderStyle, 'none');
        expect(params.iFrame.style.borderWidth, '0px');
      });

      test('applies custom iFrame attributes', () {
        final params = WebWebViewControllerCreationParams(
          iFrameAllow: 'camera',
          iFrameSandbox: 'allow-scripts',
          iFrameReferrerPolicy: 'no-referrer',
          iFrameAttributes: const <String, String?>{
            'allow': 'fullscreen',
            'title': 'Embedded preview',
          },
        );

        expect(params.iFrame.getAttribute('allow'), 'fullscreen');
        expect(params.iFrame.getAttribute('sandbox'), 'allow-scripts');
        expect(params.iFrame.getAttribute('referrerpolicy'), 'no-referrer');
        expect(params.iFrame.getAttribute('title'), 'Embedded preview');
      });

      test('rejects empty custom iFrame attribute names', () {
        expect(
          () => WebWebViewControllerCreationParams(
            iFrameAttributes: const <String, String?>{'': 'value'},
          ),
          throwsArgumentError,
        );
      });
    });

    group('loadHtmlString', () {
      test('loadHtmlString loads html into iframe srcdoc', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await controller.loadHtmlString('test html');
        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame
              .getAttribute('srcdoc'),
          'test html',
        );
      });

      test('loadHtmlString keeps logical current URL', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await controller.loadHtmlString(
          '<p>content</p>',
          baseUrl: 'https://example.com/base/',
        );

        expect(await controller.currentUrl(), 'https://example.com/base/');
      });

      test('loadHtmlString preserves raw HTML in srcdoc', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await controller.loadHtmlString('#');
        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame
              .getAttribute('srcdoc'),
          '#',
        );
      });
    });

    group('loadRequest', () {
      test('throws ArgumentError on missing scheme', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await expectLater(
          () async => controller.loadRequest(
            LoadRequestParams(uri: Uri.parse('flutter.dev')),
          ),
          throwsA(const TypeMatcher<ArgumentError>()),
        );
      });

      test('skips XHR for simple GETs (no headers, no data)', () async {
        final mockHttpRequestFactory = MockHttpRequestFactory();
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(
            httpRequestFactory: mockHttpRequestFactory,
          ),
        );

        when(
          mockHttpRequestFactory.request(
            any,
            method: anyNamed('method'),
            requestHeaders: anyNamed('requestHeaders'),
            sendData: anyNamed('sendData'),
          ),
        ).thenThrow(
          StateError('The `request` method should not have been called.'),
        );

        await controller.loadRequest(
          LoadRequestParams(uri: Uri.parse('https://flutter.dev')),
        );

        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame.src,
          'https://flutter.dev/',
        );
        expect(await controller.currentUrl(), 'https://flutter.dev');
      });

      test('makes request and loads response into iframe', () async {
        final mockHttpRequestFactory = MockHttpRequestFactory();
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(
            httpRequestFactory: mockHttpRequestFactory,
          ),
        );

        final fakeResponse = web.Response(
          'test data'.toJS,
          <String, Object>{
                'headers': <String, Object>{'content-type': 'text/plain'},
              }.jsify()!
              as web.ResponseInit,
        );

        when(
          mockHttpRequestFactory.request(
            any,
            method: anyNamed('method'),
            requestHeaders: anyNamed('requestHeaders'),
            sendData: anyNamed('sendData'),
          ),
        ).thenAnswer((_) => Future<web.Response>.value(fakeResponse));

        await controller.loadRequest(
          LoadRequestParams(
            uri: Uri.parse('https://flutter.dev'),
            method: LoadRequestMethod.post,
            body: Uint8List.fromList('test body'.codeUnits),
            headers: const <String, String>{'Foo': 'Bar'},
          ),
        );

        verify(
          mockHttpRequestFactory.request(
            'https://flutter.dev',
            method: 'post',
            requestHeaders: <String, String>{'Foo': 'Bar'},
            sendData: Uint8List.fromList('test body'.codeUnits),
          ),
        );

        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame.src,
          'data:;charset=utf-8,${Uri.encodeFull('test data')}',
        );
        expect(await controller.currentUrl(), 'https://flutter.dev');
      });

      test('reports HTTP status errors with request and response', () async {
        final mockHttpRequestFactory = MockHttpRequestFactory();
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(
            httpRequestFactory: mockHttpRequestFactory,
          ),
        );
        final delegate = WebNavigationDelegate(
          const WebNavigationDelegateCreationParams(),
        );
        final List<HttpResponseError> errors = <HttpResponseError>[];
        final fakeResponse = web.Response(
          'not found'.toJS,
          <String, Object>{
                'status': 404,
                'statusText': 'Not Found',
                'headers': <String, Object>{
                  'content-type': 'text/plain',
                  'x-test': 'yes',
                },
              }.jsify()!
              as web.ResponseInit,
        );

        await delegate.setOnHttpError(errors.add);
        await controller.setPlatformNavigationDelegate(delegate);
        when(
          mockHttpRequestFactory.request(
            any,
            method: anyNamed('method'),
            requestHeaders: anyNamed('requestHeaders'),
            sendData: anyNamed('sendData'),
          ),
        ).thenAnswer((_) => Future<web.Response>.value(fakeResponse));

        await controller.loadRequest(
          LoadRequestParams(
            uri: Uri.parse('https://flutter.dev/missing'),
            method: LoadRequestMethod.post,
            headers: const <String, String>{'Accept': 'text/plain'},
          ),
        );

        expect(errors, hasLength(1));
        expect(
          errors.single.request?.uri,
          Uri.parse('https://flutter.dev/missing'),
        );
        expect(errors.single.request, isA<WebWebResourceRequest>());
        final WebWebResourceRequest request =
            errors.single.request! as WebWebResourceRequest;
        expect(request.method, 'POST');
        expect(request.headers, const <String, String>{'Accept': 'text/plain'});
        expect(request.isForMainFrame, isTrue);
        expect(
          errors.single.response?.uri,
          Uri.parse('https://flutter.dev/missing'),
        );
        expect(errors.single.response, isA<WebWebResourceResponse>());
        final WebWebResourceResponse response =
            errors.single.response! as WebWebResourceResponse;
        expect(errors.single.response?.statusCode, 404);
        expect(errors.single.response?.headers, const <String, String>{
          'content-type': 'text/plain',
          'x-test': 'yes',
        });
        expect(response.mimeType, 'text/plain');
        expect(response.reasonPhrase, 'Not Found');
      });

      test('parses content-type response header correctly', () async {
        final mockHttpRequestFactory = MockHttpRequestFactory();
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(
            httpRequestFactory: mockHttpRequestFactory,
          ),
        );

        final Encoding iso = Encoding.getByName('latin1')!;

        final fakeResponse = web.Response(
          String.fromCharCodes(iso.encode('España')).toJS,
          <String, Object>{
                'headers': <String, Object>{
                  'content-type': 'Text/HTmL; charset=latin1',
                },
              }.jsify()!
              as web.ResponseInit,
        );

        when(
          mockHttpRequestFactory.request(
            any,
            method: anyNamed('method'),
            requestHeaders: anyNamed('requestHeaders'),
            sendData: anyNamed('sendData'),
          ),
        ).thenAnswer((_) => Future<web.Response>.value(fakeResponse));

        await controller.loadRequest(
          LoadRequestParams(
            uri: Uri.parse('https://flutter.dev'),
            method: LoadRequestMethod.post,
          ),
        );

        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame.src,
          'data:text/html;charset=iso-8859-1,Espa%F1a',
        );
      });

      test('escapes "#" correctly', () async {
        final mockHttpRequestFactory = MockHttpRequestFactory();
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(
            httpRequestFactory: mockHttpRequestFactory,
          ),
        );

        final fakeResponse = web.Response(
          '#'.toJS,
          <String, Object>{
                'headers': <String, Object>{'content-type': 'text/html'},
              }.jsify()!
              as web.ResponseInit,
        );

        when(
          mockHttpRequestFactory.request(
            any,
            method: anyNamed('method'),
            requestHeaders: anyNamed('requestHeaders'),
            sendData: anyNamed('sendData'),
          ),
        ).thenAnswer((_) => Future<web.Response>.value(fakeResponse));

        await controller.loadRequest(
          LoadRequestParams(
            uri: Uri.parse('https://flutter.dev'),
            method: LoadRequestMethod.post,
            body: Uint8List.fromList('test body'.codeUnits),
            headers: const <String, String>{'Foo': 'Bar'},
          ),
        );

        expect(
          (controller.params as WebWebViewControllerCreationParams).iFrame.src,
          contains('%23'),
        );
      });

      test(
        'reports XHR request failures as main-frame resource errors',
        () async {
          final mockHttpRequestFactory = MockHttpRequestFactory();
          final controller = WebWebViewController(
            WebWebViewControllerCreationParams(
              httpRequestFactory: mockHttpRequestFactory,
            ),
          );
          final delegate = WebNavigationDelegate(
            const WebNavigationDelegateCreationParams(),
          );
          WebResourceError? resourceError;

          await delegate.setOnWebResourceError((WebResourceError error) {
            resourceError = error;
          });
          await controller.setPlatformNavigationDelegate(delegate);

          when(
            mockHttpRequestFactory.request(
              any,
              method: anyNamed('method'),
              requestHeaders: anyNamed('requestHeaders'),
              sendData: anyNamed('sendData'),
            ),
          ).thenThrow(StateError('network down'));

          await expectLater(
            () => controller.loadRequest(
              LoadRequestParams(
                uri: Uri.parse('https://flutter.dev'),
                method: LoadRequestMethod.post,
              ),
            ),
            throwsStateError,
          );

          expect(resourceError, isNotNull);
          expect(resourceError!.isForMainFrame, isTrue);
          expect(resourceError!.errorType, WebResourceErrorType.connect);
          expect(resourceError!.url, 'https://flutter.dev');
        },
      );

      test(
        'prevents delegated simple GET navigation before state changes',
        () async {
          final mockHttpRequestFactory = MockHttpRequestFactory();
          final controller = WebWebViewController(
            WebWebViewControllerCreationParams(
              httpRequestFactory: mockHttpRequestFactory,
            ),
          );
          final delegate = WebNavigationDelegate(
            const WebNavigationDelegateCreationParams(),
          );
          final List<NavigationRequest> navigationRequests =
              <NavigationRequest>[];
          final List<String> pageStarts = <String>[];
          final List<String?> urlChanges = <String?>[];
          final List<int> progressValues = <int>[];

          await delegate.setOnNavigationRequest((NavigationRequest request) {
            navigationRequests.add(request);
            return NavigationDecision.prevent;
          });
          await delegate.setOnPageStarted(pageStarts.add);
          await delegate.setOnUrlChange((UrlChange change) {
            urlChanges.add(change.url);
          });
          await delegate.setOnProgress(progressValues.add);
          await controller.setPlatformNavigationDelegate(delegate);

          await controller.loadRequest(
            LoadRequestParams(uri: Uri.parse('https://flutter.dev/prevented')),
          );

          expect(navigationRequests, hasLength(1));
          expect(
            navigationRequests.single.url,
            'https://flutter.dev/prevented',
          );
          expect(navigationRequests.single.isMainFrame, isTrue);
          expect(await controller.currentUrl(), isNull);
          expect(
            (controller.params as WebWebViewControllerCreationParams)
                .iFrame
                .src,
            isEmpty,
          );
          expect(pageStarts, isEmpty);
          expect(urlChanges, isEmpty);
          expect(progressValues, isEmpty);
          verifyNever(
            mockHttpRequestFactory.request(
              any,
              method: anyNamed('method'),
              requestHeaders: anyNamed('requestHeaders'),
              sendData: anyNamed('sendData'),
            ),
          );
        },
      );
    });

    group('loadFlutterAsset', () {
      test('keeps loadFileWithParams unsupported on web', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await expectLater(
          () => controller.loadFileWithParams(
            const LoadFileParams(absoluteFilePath: '/tmp/index.html'),
          ),
          throwsUnsupportedError,
        );
      });

      test(
        'resolves Flutter web asset URLs under the assets directory',
        () async {
          final controller = WebWebViewController(
            WebWebViewControllerCreationParams(),
          );

          await controller.loadFlutterAsset('docs/My File.html');

          expect(
            (controller.params as WebWebViewControllerCreationParams)
                .iFrame
                .src,
            Uri.base.resolve('assets/docs/My%20File.html').toString(),
          );
        },
      );

      test('rejects empty asset keys', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await expectLater(
          () => controller.loadFlutterAsset(''),
          throwsA(isA<ArgumentError>()),
        );
      });

      test('rejects root-only asset keys', () async {
        final controller = WebWebViewController(
          WebWebViewControllerCreationParams(),
        );

        await expectLater(
          () => controller.loadFlutterAsset('/'),
          throwsA(isA<ArgumentError>()),
        );
      });
    });

    group('userAgent', () {
      test(
        'rejects unsupported overrides and allows reset to default',
        () async {
          final controller = WebWebViewController(
            WebWebViewControllerCreationParams(),
          );
          final String? originalUserAgent = await controller.getUserAgent();

          await expectLater(
            () => controller.setUserAgent('custom-agent'),
            throwsUnsupportedError,
          );

          expect(await controller.getUserAgent(), originalUserAgent);

          await controller.setUserAgent(null);
          expect(await controller.getUserAgent(), originalUserAgent);
        },
      );
    });

    group('javascript', () {
      test('runs JavaScript in accessible iframe content', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);

        await controller.runJavaScript('window.webViewAllValue = 37');
        expect(
          await controller.runJavaScriptReturningResult(
            'window.webViewAllValue + 5',
          ),
          42,
        );
        expect(
          await controller.runJavaScriptReturningResult(
            '({message: "ok", count: 2, values: [1, 2]})',
          ),
          <String, Object>{
            'message': 'ok',
            'count': 2,
            'values': <Object>[1, 2],
          },
        );
      });

      test('rejects null or undefined JavaScript results', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);

        await expectLater(
          () => controller.runJavaScriptReturningResult('undefined'),
          throwsA(isA<ArgumentError>()),
        );
        await expectLater(
          () => controller.runJavaScriptReturningResult('null'),
          throwsA(isA<ArgumentError>()),
        );
      });

      test('can disable JavaScript for iframe loads', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        addTearDown(() {
          params.iFrame.remove();
        });
        web.document.body!.append(params.iFrame);

        await controller.setJavaScriptMode(JavaScriptMode.disabled);
        final Future<web.Event> disabledLoad = params.iFrame.onLoad.first;
        await controller.loadHtmlString('''
<!doctype html>
<html>
  <body>
    <script>
      document.body.setAttribute('data-script-ran', 'yes');
    </script>
  </body>
</html>
''');
        await disabledLoad;

        expect(params.iFrame.getAttribute('sandbox'), isNotNull);
        expect(
          params.iFrame.contentDocument?.body?.getAttribute('data-script-ran'),
          isNull,
        );
        await expectLater(
          () => controller.runJavaScript('window.webViewAllValue = 1'),
          throwsStateError,
        );

        await controller.setJavaScriptMode(JavaScriptMode.unrestricted);
        final Future<web.Event> unrestrictedLoad = params.iFrame.onLoad.first;
        await controller.loadHtmlString('''
<!doctype html>
<html>
  <body>
    <script>
      document.body.setAttribute('data-script-ran', 'yes');
    </script>
  </body>
</html>
''');
        await unrestrictedLoad;

        expect(params.iFrame.getAttribute('sandbox'), isNull);
        expect(
          params.iFrame.contentDocument?.body?.getAttribute('data-script-ran'),
          'yes',
        );
      });

      test(
        'preserves custom iframe sandbox around JavaScript mode changes',
        () async {
          final params = WebWebViewControllerCreationParams(
            iFrameSandbox: 'allow-scripts allow-forms',
          );
          final controller = WebWebViewController(params);

          expect(
            params.iFrame.getAttribute('sandbox'),
            'allow-scripts allow-forms',
          );

          await controller.setJavaScriptMode(JavaScriptMode.disabled);

          expect(params.iFrame.getAttribute('sandbox'), isNotNull);
          expect(
            params.iFrame.getAttribute('sandbox'),
            isNot(contains('allow-scripts')),
          );

          await controller.setIFrameSandbox('allow-popups');

          expect(
            params.iFrame.getAttribute('sandbox'),
            isNot(contains('allow-scripts')),
          );

          await controller.setJavaScriptMode(JavaScriptMode.unrestricted);

          expect(params.iFrame.getAttribute('sandbox'), 'allow-popups');

          await controller.setIFrameSandbox(null);

          expect(params.iFrame.getAttribute('sandbox'), isNull);
        },
      );

      test('sets and removes custom iframe attributes at runtime', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);

        await controller.setIFrameAllow('camera; microphone');
        await controller.setIFrameReferrerPolicy('origin');
        await controller.setIFrameAttribute('title', 'Runtime preview');

        expect(params.iFrame.getAttribute('allow'), 'camera; microphone');
        expect(params.iFrame.getAttribute('referrerpolicy'), 'origin');
        expect(params.iFrame.getAttribute('title'), 'Runtime preview');

        await controller.setIFrameAllow(null);
        await controller.setIFrameReferrerPolicy(null);
        await controller.setIFrameAttribute('title', null);

        expect(params.iFrame.getAttribute('allow'), isNull);
        expect(params.iFrame.getAttribute('referrerpolicy'), isNull);
        expect(params.iFrame.getAttribute('title'), isNull);
        await expectLater(
          controller.setIFrameAttribute('', 'value'),
          throwsArgumentError,
        );
      });

      test('delivers JavaScript channel messages', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final messages = <String>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);
        await controller.addJavaScriptChannel(
          JavaScriptChannelParams(
            name: 'TestChannel',
            onMessageReceived: (JavaScriptMessage message) {
              messages.add(message.message);
            },
          ),
        );

        await controller.runJavaScript('TestChannel.postMessage("hello")');
        for (int i = 0; i < 20 && messages.isEmpty; i += 1) {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        }

        expect(messages, <String>['hello']);

        await controller.removeJavaScriptChannel('TestChannel');
        expect(
          await controller.runJavaScriptReturningResult('typeof TestChannel'),
          'undefined',
        );
      });

      test('installs JavaScript channels added before content loads', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final messages = <String>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await controller.addJavaScriptChannel(
          JavaScriptChannelParams(
            name: 'EarlyChannel',
            onMessageReceived: (JavaScriptMessage message) {
              messages.add(message.message);
            },
          ),
        );
        await _attachAndLoadScrollableHtml(controller, params);

        await controller.runJavaScript('EarlyChannel.postMessage(123)');
        for (int i = 0; i < 20 && messages.isEmpty; i += 1) {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        }

        expect(messages, <String>['123']);
      });
    });

    group('console', () {
      test(
        'forwards console messages from accessible iframe content',
        () async {
          final params = WebWebViewControllerCreationParams();
          final controller = WebWebViewController(params);
          final messages = <JavaScriptConsoleMessage>[];
          addTearDown(() {
            params.iFrame.remove();
          });

          await _attachAndLoadScrollableHtml(controller, params);
          await controller.setOnConsoleMessage(messages.add);
          await controller.runJavaScript('''
          console.log('plain', 7);
          console.warn({kind: 'warning'});
          const circular = {};
          circular.self = circular;
          console.log(undefined, circular);
          console.error('bad');
          console.debug('details');
          console.info('note');
        ''');

          for (int i = 0; i < 20 && messages.length < 6; i += 1) {
            await Future<void>.delayed(const Duration(milliseconds: 25));
          }

          expect(
            messages.map((JavaScriptConsoleMessage message) => message.level),
            <JavaScriptLogLevel>[
              JavaScriptLogLevel.log,
              JavaScriptLogLevel.warning,
              JavaScriptLogLevel.log,
              JavaScriptLogLevel.error,
              JavaScriptLogLevel.debug,
              JavaScriptLogLevel.info,
            ],
          );
          expect(messages[0].message, 'plain 7');
          expect(messages[1].message, '{"kind":"warning"}');
          expect(messages[2].message, 'undefined [object Object]');
          expect(messages[3].message, 'bad');
          expect(messages[4].message, 'details');
          expect(messages[5].message, 'note');
        },
      );

      test('installs console forwarding after content loads', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final messages = <JavaScriptConsoleMessage>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await controller.setOnConsoleMessage(messages.add);
        await _attachAndLoadScrollableHtml(controller, params);
        await controller.runJavaScript("console.log('early')");

        for (int i = 0; i < 20 && messages.isEmpty; i += 1) {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        }

        expect(messages, hasLength(1));
        expect(messages.single.level, JavaScriptLogLevel.log);
        expect(messages.single.message, 'early');
      });
    });

    group('javascript dialogs', () {
      test(
        'forwards alert dialog requests from accessible iframe content',
        () async {
          final params = WebWebViewControllerCreationParams();
          final controller = WebWebViewController(params);
          final requests = <JavaScriptAlertDialogRequest>[];
          addTearDown(() {
            params.iFrame.remove();
          });

          await _attachAndLoadScrollableHtml(controller, params);
          await controller.setOnJavaScriptAlertDialog((
            JavaScriptAlertDialogRequest request,
          ) async {
            requests.add(request);
          });
          await controller.runJavaScript("alert('hello alert')");

          for (int i = 0; i < 20 && requests.isEmpty; i += 1) {
            await Future<void>.delayed(const Duration(milliseconds: 25));
          }

          expect(requests, hasLength(1));
          expect(requests.single.message, 'hello alert');
          expect(requests.single.url, isNotEmpty);
        },
      );

      test('installs alert forwarding after content loads', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final requests = <JavaScriptAlertDialogRequest>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await controller.setOnJavaScriptAlertDialog((
          JavaScriptAlertDialogRequest request,
        ) async {
          requests.add(request);
        });
        await _attachAndLoadScrollableHtml(controller, params);
        await controller.runJavaScript('alert(123)');

        for (int i = 0; i < 20 && requests.isEmpty; i += 1) {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        }

        expect(requests, hasLength(1));
        expect(requests.single.message, '123');
      });

      test(
        'forwards confirm dialog requests and returns the decision',
        () async {
          final params = WebWebViewControllerCreationParams();
          final controller = WebWebViewController(params);
          final requests = <JavaScriptConfirmDialogRequest>[];
          addTearDown(() {
            params.iFrame.remove();
          });

          await _attachAndLoadScrollableHtml(controller, params);
          await controller.setOnJavaScriptConfirmDialog((
            JavaScriptConfirmDialogRequest request,
          ) {
            requests.add(request);
            return SynchronousFuture<bool>(request.message == 'continue?');
          });

          final Object accepted = await controller.runJavaScriptReturningResult(
            "confirm('continue?')",
          );
          final Object rejected = await controller.runJavaScriptReturningResult(
            "confirm('stop?')",
          );

          expect(accepted, isTrue);
          expect(rejected, isFalse);
          expect(requests, hasLength(2));
          expect(requests.first.message, 'continue?');
          expect(requests.first.url, isNotEmpty);
          expect(requests.last.message, 'stop?');
        },
      );

      test('keeps dialog bridge entries for multiple controllers', () async {
        final paramsA = WebWebViewControllerCreationParams();
        final paramsB = WebWebViewControllerCreationParams();
        final controllerA = WebWebViewController(paramsA);
        final controllerB = WebWebViewController(paramsB);
        addTearDown(() {
          paramsA.iFrame.remove();
          paramsB.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controllerA, paramsA);
        await _attachAndLoadScrollableHtml(controllerB, paramsB);
        await controllerA.setOnJavaScriptConfirmDialog((
          JavaScriptConfirmDialogRequest request,
        ) {
          return SynchronousFuture<bool>(request.message == 'controller-a');
        });
        await controllerB.setOnJavaScriptConfirmDialog((
          JavaScriptConfirmDialogRequest request,
        ) {
          return SynchronousFuture<bool>(request.message == 'controller-b');
        });

        final Object resultA = await controllerA.runJavaScriptReturningResult(
          "confirm('controller-a')",
        );
        final Object resultB = await controllerB.runJavaScriptReturningResult(
          "confirm('controller-b')",
        );
        final Object rejectedA = await controllerA.runJavaScriptReturningResult(
          "confirm('controller-b')",
        );

        expect(resultA, isTrue);
        expect(resultB, isTrue);
        expect(rejectedA, isFalse);
      });

      test('installs prompt forwarding after content loads', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final requests = <JavaScriptTextInputDialogRequest>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await controller.setOnJavaScriptTextInputDialog((
          JavaScriptTextInputDialogRequest request,
        ) {
          requests.add(request);
          return SynchronousFuture<String>('hello ${request.defaultText}');
        });
        await _attachAndLoadScrollableHtml(controller, params);

        final Object result = await controller.runJavaScriptReturningResult(
          "prompt('name', 'world')",
        );

        expect(result, 'hello world');
        expect(requests, hasLength(1));
        expect(requests.single.message, 'name');
        expect(requests.single.url, isNotEmpty);
        expect(requests.single.defaultText, 'world');
      });
    });

    group('permissions', () {
      test('denies same-origin getUserMedia requests', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final requests = <PlatformWebViewPermissionRequest>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);
        await _installFakeGetUserMedia(controller);
        await controller.setOnPlatformPermissionRequest((
          PlatformWebViewPermissionRequest request,
        ) {
          requests.add(request);
          request.deny();
        });

        await controller.runJavaScript('''
          window.permissionResult = 'pending';
          navigator.mediaDevices.getUserMedia({ audio: true, video: true })
            .then(function() {
              window.permissionResult = 'granted';
            })
            .catch(function(error) {
              window.permissionResult = error.name;
            });
        ''');

        final Object result = await _waitForJavaScriptValue(
          controller,
          'window.permissionResult',
          isNot('pending'),
        );

        expect(result, 'NotAllowedError');
        expect(requests, hasLength(1));
        expect(
          requests.single.types,
          containsAll(<WebViewPermissionResourceType>{
            WebViewPermissionResourceType.camera,
            WebViewPermissionResourceType.microphone,
          }),
        );
        expect(
          await controller.runJavaScriptReturningResult(
            'document.body.getAttribute("data-original-get-user-media-called")',
          ),
          'no',
        );
      });

      test('grants same-origin getUserMedia requests', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final requests = <PlatformWebViewPermissionRequest>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);
        await _installFakeGetUserMedia(controller);
        await controller.setOnPlatformPermissionRequest((
          PlatformWebViewPermissionRequest request,
        ) {
          requests.add(request);
          request.grant();
        });

        await controller.runJavaScript('''
          window.permissionResult = 'pending';
          navigator.mediaDevices.getUserMedia({ audio: true })
            .then(function(stream) {
              window.permissionResult = stream;
            })
            .catch(function(error) {
              window.permissionResult = error.name;
            });
        ''');

        final Object result = await _waitForJavaScriptValue(
          controller,
          'window.permissionResult',
          isNot('pending'),
        );

        expect(result, 'fake-stream');
        expect(requests, hasLength(1));
        expect(requests.single.types, <WebViewPermissionResourceType>{
          WebViewPermissionResourceType.microphone,
        });
        expect(
          await controller.runJavaScriptReturningResult(
            'document.body.getAttribute("data-original-get-user-media-called")',
          ),
          'yes',
        );
      });
    });

    group('enableZoom', () {
      test('toggles iframe touch-action zoom suppression', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);

        await controller.enableZoom(false);
        expect(
          params.iFrame.style.getPropertyValue('touch-action'),
          'pan-x pan-y',
        );

        await controller.enableZoom(true);
        expect(params.iFrame.style.getPropertyValue('touch-action'), '');
      });
    });

    group('scrolling', () {
      test('scrolls accessible iframe content', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);

        await controller.scrollTo(20, 40);
        expect(await controller.getScrollPosition(), const Offset(20, 40));

        await controller.scrollBy(5, 6);
        expect(await controller.getScrollPosition(), const Offset(25, 46));
      });

      test('reports scroll position changes for accessible content', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);
        final changes = <ScrollPositionChange>[];
        addTearDown(() {
          params.iFrame.remove();
        });

        await _attachAndLoadScrollableHtml(controller, params);
        await controller.setOnScrollPositionChange(changes.add);
        await controller.scrollTo(0, 80);

        for (int i = 0; i < 20 && changes.isEmpty; i += 1) {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        }

        expect(changes, isNotEmpty);
        expect(changes.last.y, 80);
      });
    });

    group('scrollbars', () {
      test(
        'applies scrollbar stylesheet to accessible iframe content',
        () async {
          final params = WebWebViewControllerCreationParams();
          final controller = WebWebViewController(params);
          addTearDown(() {
            params.iFrame.remove();
          });

          await _attachAndLoadScrollableHtml(controller, params);

          expect(controller.supportsSetScrollBarsEnabled(), isTrue);

          await controller.setVerticalScrollBarEnabled(false);
          expect(
            _scrollbarStyle(params)?.textContent,
            contains('::-webkit-scrollbar:vertical'),
          );
          expect(
            _scrollbarStyle(params)?.textContent,
            isNot(contains('::-webkit-scrollbar:horizontal')),
          );

          await controller.setHorizontalScrollBarEnabled(false);
          expect(
            _scrollbarStyle(params)?.textContent,
            contains('::-webkit-scrollbar:vertical'),
          );
          expect(
            _scrollbarStyle(params)?.textContent,
            contains('::-webkit-scrollbar:horizontal'),
          );

          await controller.setVerticalScrollBarEnabled(true);
          expect(
            _scrollbarStyle(params)?.textContent,
            isNot(contains('::-webkit-scrollbar:vertical')),
          );
          expect(
            _scrollbarStyle(params)?.textContent,
            contains('::-webkit-scrollbar:horizontal'),
          );

          await controller.setHorizontalScrollBarEnabled(true);
          expect(_scrollbarStyle(params), isNull);
        },
      );
    });

    group('setOverScrollMode', () {
      test('sets iframe overscroll behavior', () async {
        final params = WebWebViewControllerCreationParams();
        final controller = WebWebViewController(params);

        await controller.setOverScrollMode(WebViewOverScrollMode.never);
        expect(
          params.iFrame.style.getPropertyValue('overscroll-behavior'),
          'none',
        );

        await controller.setOverScrollMode(
          WebViewOverScrollMode.ifContentScrolls,
        );
        expect(
          params.iFrame.style.getPropertyValue('overscroll-behavior'),
          'contain',
        );

        await controller.setOverScrollMode(WebViewOverScrollMode.always);
        expect(
          params.iFrame.style.getPropertyValue('overscroll-behavior'),
          'auto',
        );
      });
    });
  });
}

Future<void> _installFakeGetUserMedia(WebWebViewController controller) {
  return controller.runJavaScript('''
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: {
        getUserMedia: function() {
          document.body.setAttribute(
            'data-original-get-user-media-called',
            'yes'
          );
          return Promise.resolve('fake-stream');
        }
      }
    });
    document.body.setAttribute('data-original-get-user-media-called', 'no');
  ''');
}

Future<Object> _waitForJavaScriptValue(
  WebWebViewController controller,
  String expression,
  Matcher matcher,
) async {
  Object? result;
  for (int i = 0; i < 40; i += 1) {
    result = await controller.runJavaScriptReturningResult(expression);
    if (matcher.matches(result, <Object, Object>{})) {
      return result;
    }
    await Future<void>.delayed(const Duration(milliseconds: 25));
  }
  return result!;
}

Future<void> _attachAndLoadScrollableHtml(
  WebWebViewController controller,
  WebWebViewControllerCreationParams params,
) async {
  params.iFrame.style.width = '200px';
  params.iFrame.style.height = '200px';
  web.document.body!.append(params.iFrame);

  final Future<web.Event> loadFuture = params.iFrame.onLoad.first;
  await controller.loadHtmlString('''
<!doctype html>
<html>
  <head><title>Scrollable</title></head>
  <body style="margin:0;width:2000px;height:2000px;">
    <div style="width:2000px;height:2000px;"></div>
  </body>
</html>
''');
  await loadFuture.timeout(const Duration(seconds: 5));
  await Future<void>.delayed(Duration.zero);
}

web.Element? _scrollbarStyle(WebWebViewControllerCreationParams params) {
  return params.iFrame.contentDocument?.getElementById(
    '__webview_all_scrollbars',
  );
}
