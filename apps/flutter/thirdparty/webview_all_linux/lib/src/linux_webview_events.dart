part of 'linux_webview_controller.dart';

extension LinuxWebViewControllerEventHandling on LinuxWebViewController {
  void _handleEvent(dynamic event) {
    if (event is! Map) {
      return;
    }

    final String? type = event['type'] as String?;
    switch (type) {
      case 'download':
        final id = event['id'] as String;
        final state = WebViewDownloadState.values.byName(
          event['state'] as String,
        );
        final total = (event['totalBytes'] as num).toInt();
        reportDownloadEvent(
          WebViewDownloadEvent(
            id: id,
            url: event['url'] as String,
            fileName: event['fileName'] as String,
            state: state,
            receivedBytes: (event['receivedBytes'] as num).toInt(),
            totalBytes: total > 0 ? total : null,
            error: event['error'] as String?,
            cancel: () => _invoke<void>('cancelDownload', {'id': id}),
            readBytes: state == WebViewDownloadState.completed
                ? () async {
                    try {
                      return await File(event['path'] as String).readAsBytes();
                    } finally {
                      await _invoke<void>('releaseDownload', {'id': id});
                    }
                  }
                : null,
          ),
        );
        if (state == WebViewDownloadState.failed ||
            state == WebViewDownloadState.cancelled) {
          unawaited(_invoke<void>('releaseDownload', {'id': id}));
        }
        break;
      case 'urlChanged':
        final String url = '${event['url'] ?? ''}';
        _currentUrl = url;
        _navigationDelegate?.handleUrlChange(url);
        break;
      case 'pageStarted':
        _cancelPendingAsyncJavaScriptInvocations(
          const JavaScriptExecutionException(
            name: 'NavigationError',
            message: 'The page navigated before JavaScript completed.',
          ),
        );
        final String url = '${event['url'] ?? ''}';
        _currentUrl = url;
        _navigationDelegate?.handlePageStarted(url);
        break;
      case 'pageFinished':
        final String url = '${event['url'] ?? ''}';
        _currentUrl = url;
        _navigationDelegate?.handlePageFinished(url);
        break;
      case 'progress':
        _navigationDelegate?.handleProgress(
          (event['progress'] as num?)?.round() ?? 0,
        );
        break;
      case 'historyChanged':
        _canGoBack = event['canGoBack'] == true;
        _canGoForward = event['canGoForward'] == true;
        break;
      case 'titleChanged':
        _title = event['title'] as String?;
        break;
      case 'webResourceError':
        _navigationDelegate?.handleWebResourceError(
          LinuxWebResourceError.fromMap(event),
        );
        break;
      case 'httpError':
        if (_navigationDelegate != null) {
          final Uri? uri = Uri.tryParse('${event['url'] ?? ''}');
          final Map<String, String> requestHeaders = _stringMapFromEvent(
            event['requestHeaders'],
          );
          final Map<String, String> responseHeaders = _stringMapFromEvent(
            event['headers'],
          );
          _navigationDelegate!.handleHttpError(
            HttpResponseError(
              request: uri == null
                  ? null
                  : LinuxWebResourceRequest(
                      uri: uri,
                      method: event['method'] as String?,
                      headers: requestHeaders,
                      isForMainFrame: event['isForMainFrame'] as bool? ?? true,
                    ),
              response: LinuxWebResourceResponse(
                uri: uri,
                statusCode: (event['statusCode'] as num?)?.toInt() ?? 0,
                headers: responseHeaders,
                mimeType: event['mimeType'] as String?,
              ),
            ),
          );
        }
        break;
      case 'javaScriptChannelMessage':
        final String name = '${event['channelName'] ?? ''}';
        final JavaScriptChannelParams? params = _javaScriptChannels[name];
        if (params != null) {
          params.onMessageReceived(
            JavaScriptMessage(message: '${event['message'] ?? ''}'),
          );
        }
        break;
      case 'asyncJavaScriptResult':
        _handleAsyncJavaScriptMessage('${event['message'] ?? ''}');
        break;
      case 'consoleMessage':
        final callback = _onConsoleMessage;
        if (callback != null) {
          callback(
            JavaScriptConsoleMessage(
              level: _parseJavaScriptLogLevel(event['level'] as String?),
              message: '${event['message'] ?? ''}',
            ),
          );
        }
        break;
      case 'scrollPositionChange':
        final callback = _onScrollPositionChange;
        if (callback != null) {
          callback(
            ScrollPositionChange(
              (event['x'] as num?)?.toDouble() ?? 0,
              (event['y'] as num?)?.toDouble() ?? 0,
            ),
          );
        }
        break;
      case 'navigationRequest':
        _dispatchEventHandler(
          'navigation request',
          () => _handleNavigationRequestEvent(event),
        );
        break;
      case 'httpAuthRequest':
        _dispatchEventHandler(
          'HTTP authentication request',
          () => _handleHttpAuthRequestEvent(event),
        );
        break;
      case 'sslAuthError':
        _dispatchEventHandler(
          'TLS authentication request',
          () => _handleSslAuthErrorEvent(event),
        );
        break;
      case 'permissionRequest':
        _dispatchEventHandler(
          'permission request',
          () => _handlePermissionRequestEvent(event),
        );
        break;
      case 'javaScriptDialog':
        _dispatchEventHandler(
          'JavaScript dialog',
          () => _handleJavaScriptDialogEvent(event),
        );
        break;
    }
  }

  Future<void> _handleNavigationRequestEvent(
    Map<dynamic, dynamic> event,
  ) async {
    final int requestId = (event['requestId'] as num?)?.toInt() ?? -1;
    bool allow = true;
    if (_navigationDelegate?.hasNavigationRequestHandler ?? false) {
      try {
        final NavigationDecision? decision = await _navigationDelegate!
            .decideNavigation(
              NavigationRequest(
                url: '${event['url'] ?? ''}',
                isMainFrame: event['isMainFrame'] != false,
              ),
            );
        allow = decision == NavigationDecision.navigate;
      } catch (error, stackTrace) {
        allow = false;
        _reportEventError('application navigation callback', error, stackTrace);
      }
    }

    await _invoke<void>('completeNavigationRequest', <String, Object?>{
      'requestId': requestId,
      'allow': allow,
    });
  }

  Future<void> _handleHttpAuthRequestEvent(Map<dynamic, dynamic> event) async {
    final int requestId = (event['requestId'] as num?)?.toInt() ?? -1;
    if (!(_navigationDelegate?.hasHttpAuthRequestHandler ?? false)) {
      await _invoke<void>('completeHttpAuthRequest', <String, Object?>{
        'requestId': requestId,
        'action': 'cancel',
      });
      return;
    }

    var completed = false;
    Future<void> complete(Map<String, Object?> arguments) {
      if (completed) {
        return Future<void>.value();
      }
      completed = true;
      return _invoke<void>('completeHttpAuthRequest', arguments);
    }

    try {
      _navigationDelegate!.handleHttpAuthRequest(
        HttpAuthRequest(
          host: '${event['host'] ?? ''}',
          realm: event['realm'] as String?,
          onProceed: (WebViewCredential credential) {
            _dispatchEventHandler(
              'HTTP authentication completion',
              () => complete(<String, Object?>{
                'requestId': requestId,
                'action': 'proceed',
                'user': credential.user,
                'password': credential.password,
              }),
            );
          },
          onCancel: () {
            _dispatchEventHandler(
              'HTTP authentication completion',
              () => complete(<String, Object?>{
                'requestId': requestId,
                'action': 'cancel',
              }),
            );
          },
        ),
      );
    } catch (error, stackTrace) {
      _reportEventError(
        'application HTTP authentication callback',
        error,
        stackTrace,
      );
      await complete(<String, Object?>{
        'requestId': requestId,
        'action': 'cancel',
      });
    }
  }

  Future<void> _handleSslAuthErrorEvent(Map<dynamic, dynamic> event) async {
    final int requestId = (event['requestId'] as num?)?.toInt() ?? -1;
    if (!(_navigationDelegate?.hasSslAuthErrorHandler ?? false)) {
      await _invoke<void>('completeSslAuthError', <String, Object?>{
        'requestId': requestId,
        'proceed': false,
      });
      return;
    }

    final LinuxPlatformSslAuthError request = LinuxPlatformSslAuthError(
      description: '${event['description'] ?? 'TLS certificate error'}',
      onProceed: () {
        return _invoke<void>('completeSslAuthError', <String, Object?>{
          'requestId': requestId,
          'proceed': true,
        });
      },
      onCancel: () {
        return _invoke<void>('completeSslAuthError', <String, Object?>{
          'requestId': requestId,
          'proceed': false,
        });
      },
    );
    try {
      _navigationDelegate!.handleSslAuthError(request);
    } catch (error, stackTrace) {
      _reportEventError(
        'application TLS authentication callback',
        error,
        stackTrace,
      );
      await request.cancel();
    }
  }

  Future<void> _handlePermissionRequestEvent(
    Map<dynamic, dynamic> event,
  ) async {
    final int requestId = (event['requestId'] as num?)?.toInt() ?? -1;
    final callback = _onPermissionRequest;
    if (callback == null) {
      await _invoke<void>('completePermissionRequest', <String, Object?>{
        'requestId': requestId,
        'grant': false,
      });
      return;
    }

    final List<Object?> rawTypes =
        (event['types'] as List<Object?>?) ?? const <Object?>[];
    final Set<WebViewPermissionResourceType> types = rawTypes
        .map((Object? type) => _parsePermissionType(type as String?))
        .whereType<WebViewPermissionResourceType>()
        .toSet();
    if (types.isEmpty) {
      await _invoke<void>('completePermissionRequest', <String, Object?>{
        'requestId': requestId,
        'grant': false,
      });
      return;
    }

    final LinuxPlatformWebViewPermissionRequest request =
        LinuxPlatformWebViewPermissionRequest(
          types: types,
          onGrant: () {
            return _invoke<void>('completePermissionRequest', <String, Object?>{
              'requestId': requestId,
              'grant': true,
            });
          },
          onDeny: () {
            return _invoke<void>('completePermissionRequest', <String, Object?>{
              'requestId': requestId,
              'grant': false,
            });
          },
        );
    try {
      callback(request);
    } catch (error, stackTrace) {
      _reportEventError('application permission callback', error, stackTrace);
      await request.deny();
    }
  }

  Future<void> _handleJavaScriptDialogEvent(Map<dynamic, dynamic> event) async {
    final int requestId = (event['requestId'] as num?)?.toInt() ?? -1;
    final String dialogType = '${event['dialogType'] ?? ''}';
    final String message = '${event['message'] ?? ''}';
    final String url = '${event['url'] ?? ''}';
    try {
      switch (dialogType) {
        case 'alert':
          final callback = _onJavaScriptAlertDialog;
          if (callback == null) {
            await _completeJavaScriptDialog(requestId, action: 'confirm');
            return;
          }
          await callback(
            JavaScriptAlertDialogRequest(message: message, url: url),
          );
          await _completeJavaScriptDialog(requestId, action: 'confirm');
          return;
        case 'confirm':
        case 'beforeUnloadConfirm':
          final callback = _onJavaScriptConfirmDialog;
          if (callback == null) {
            await _completeJavaScriptDialog(requestId, action: 'confirm');
            return;
          }
          final bool confirmed = await callback(
            JavaScriptConfirmDialogRequest(message: message, url: url),
          );
          await _completeJavaScriptDialog(
            requestId,
            action: confirmed ? 'confirm' : 'cancel',
          );
          return;
        case 'prompt':
          final callback = _onJavaScriptTextInputDialog;
          if (callback == null) {
            await _completeJavaScriptDialog(
              requestId,
              action: 'confirm',
              text: (event['defaultText'] as String?) ?? '',
            );
            return;
          }
          final String text = await callback(
            JavaScriptTextInputDialogRequest(
              message: message,
              url: url,
              defaultText: event['defaultText'] as String?,
            ),
          );
          await _completeJavaScriptDialog(
            requestId,
            action: 'confirm',
            text: text,
          );
          return;
      }
    } catch (error, stackTrace) {
      _reportEventError(
        'application JavaScript dialog callback',
        error,
        stackTrace,
      );
      await _completeJavaScriptDialog(requestId, action: 'cancel');
      return;
    }

    await _completeJavaScriptDialog(requestId, action: 'cancel');
  }

  Future<void> _completeJavaScriptDialog(
    int requestId, {
    required String action,
    String? text,
  }) {
    return _invoke<void>('completeJavaScriptDialog', <String, Object?>{
      'requestId': requestId,
      'action': action,
      'text': text,
    });
  }

  JavaScriptLogLevel _parseJavaScriptLogLevel(String? level) {
    switch (level) {
      case 'debug':
        return JavaScriptLogLevel.debug;
      case 'error':
        return JavaScriptLogLevel.error;
      case 'info':
        return JavaScriptLogLevel.info;
      case 'warning':
        return JavaScriptLogLevel.warning;
      default:
        return JavaScriptLogLevel.log;
    }
  }

  WebViewPermissionResourceType? _parsePermissionType(String? type) {
    switch (type) {
      case 'camera':
        return WebViewPermissionResourceType.camera;
      case 'microphone':
        return WebViewPermissionResourceType.microphone;
    }
    return null;
  }
}

Map<String, String> _stringMapFromEvent(Object? value) {
  final Map<String, String> result = <String, String>{};
  if (value case final Map<dynamic, dynamic> rawHeaders) {
    for (final MapEntry<dynamic, dynamic> header in rawHeaders.entries) {
      result['${header.key}'] = '${header.value}';
    }
  }
  return result;
}
