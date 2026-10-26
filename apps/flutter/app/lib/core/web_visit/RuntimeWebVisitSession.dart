// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import '../browser/BrowserSessions.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import 'WebVisitModels.dart';

/// Executes visit_web through the same owner sessions as browser automation.
/// Widgets observe this task; mounting or disposing them does not drive it.
class RuntimeWebVisitSession extends ChangeNotifier {
  RuntimeWebVisitSession({required this.request, BrowserSessions? sessions})
    : _sessions = sessions ?? BrowserSessions();

  static const int _visitWebWaitSeconds = 0;
  static const int _captchaWaitSeconds = 10;
  static const Duration _loadTimeout = Duration(seconds: 10);
  static const Duration _extractDelay = Duration(milliseconds: 800);
  static const Duration _scriptTimeout = Duration(seconds: 10);

  final WebVisitRequest request;
  final BrowserSessions _sessions;
  final Completer<WebVisitResponse> _response = Completer<WebVisitResponse>();
  StreamSubscription<BrowserSessionEvent>? _events;
  RuntimeBrowserSessionInfo? browserSession;
  bool _disposed = false;
  Timer? _deadlineTimer;

  Future<WebVisitResponse> get response => _response.future;
  bool get _active => !_disposed;

  void _updateState(VoidCallback change) {
    if (_disposed || _completed) return;
    change();
    notifyListeners();
  }

  Timer? _loadTimeoutTimer;
  Timer? _extractDelayTimer;
  Timer? _countdownTimer;
  bool _completed = false;
  bool _isLoading = true;
  bool _pageLoaded = false;
  bool _hasExtractedContent = false;
  bool _extractionRequested = false;
  bool _autoModeEnabled = true;
  bool _autoCountdownActive = false;
  bool _isCaptchaVerification = false;
  int _navigationGeneration = 0;
  int _autoCountdownSeconds = 0;
  String _currentUrl = '';
  String _pageTitle = '';
  WebVisitResult? _pageResult;

  /// Starts immediately; presentation is optional and never gates execution.
  void start() {
    _currentUrl = _normalizeUrl(request.url);
    // The owner broker waits 60 seconds; leave time for bounded cleanup.
    _deadlineTimer = Timer(const Duration(seconds: 45), () {
      _finishError('visit_web timed out');
    });
    unawaited(_start());
  }

  Future<void> disposeSession() async {
    if (_disposed) return;
    if (!_completed) _finishError('visit_web cancelled');
    _disposed = true;
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    _deadlineTimer?.cancel();
    final id = browserSession?.sessionId;
    try {
      // Stream shutdown and session close must not wait on one another. Some
      // hosts only finish cancelling a subscription after closing its session.
      await Future.wait<void>([
        if (_events != null) _events!.cancel(),
        if (id != null) _sessions.close(id),
      ]).timeout(_scriptTimeout);
    } catch (error, stack) {
      ClientLogger.w(
        'Closing visit browser session $id failed',
        tag: 'RuntimeWebVisit',
        error: error,
        stackTrace: stack,
      );
    }
    super.dispose();
  }

  Future<void> _start() async {
    try {
      final session = await _sessions.createSession(
        initialUrl: _currentUrl,
        userAgent: request.userAgent.trim().isEmpty ? null : request.userAgent,
        headers: _headers,
      );
      if (_disposed || _completed) {
        await _sessions.close(session.sessionId);
        return;
      }
      browserSession = session;
      notifyListeners();
      _events = _sessions
          .watchEvents(session.sessionId)
          .listen(
            _handleEvent,
            onError: (Object error) => _finishError(error.toString()),
            onDone: () {
              if (!_completed) {
                _finishError('Browser session event stream closed');
              }
            },
          );
      _armLoadTimeout();
      // Navigation can finish before the event subscription attaches. Probe the
      // current document once so that fast/cached pages need no replay event.
      final raw = await _evaluate(
        'JSON.stringify({readyState: document.readyState, url: location.href})',
      );
      final state = raw is String ? jsonDecode(raw) : raw;
      if (!_completed &&
          !_pageLoaded &&
          state is Map &&
          state['readyState'] == 'complete' &&
          state['url'] is String &&
          (Uri.tryParse(state['url'] as String)?.hasAuthority ?? false)) {
        unawaited(_handlePageFinished(state['url'] as String));
      }
    } catch (error) {
      _finishError(error.toString());
    }
  }

  void _handleEvent(BrowserSessionEvent event) {
    if (_disposed || _completed) return;
    if (event.session case final info?) {
      browserSession = info;
    }
    switch (event.eventType) {
      case 'started':
        _handlePageStarted(event.session?.currentUrl ?? _currentUrl);
      case 'finished':
        unawaited(
          _handlePageFinished(event.session?.currentUrl ?? _currentUrl),
        );
      case 'closed':
        _finishError('Browser session closed');
    }
  }

  Future<Object?> _evaluate(String script) async {
    if (_completed || _disposed) throw StateError('visit_web completed');
    final result = await Future.any<RuntimeBrowserCommandResult>([
      _sessions.evaluate(browserSession!.sessionId, script),
      response.then<RuntimeBrowserCommandResult>(
        (_) => throw StateError('visit_web completed'),
      ),
    ]).timeout(_scriptTimeout);
    if (!result.success) {
      throw StateError(result.error ?? 'Browser evaluation failed');
    }
    return jsonDecode(result.resultJson);
  }

  Map<String, String> get _headers {
    return <String, String>{
      for (final header in request.headers) header.name: header.value,
    };
  }

  /// Resets page state when a new WebView navigation begins.
  void _handlePageStarted(String url) {
    if (_completed || !_active) {
      return;
    }
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    _navigationGeneration += 1;
    _updateState(() {
      _currentUrl = url;
      _isLoading = true;
      _pageLoaded = false;
      _hasExtractedContent = false;
      _extractionRequested = false;
      _autoCountdownActive = false;
      _autoModeEnabled = true;
      _isCaptchaVerification = false;
      _pageResult = null;
    });
    _armLoadTimeout();
  }

  /// Processes the completed page and starts the visit workflow.
  Future<void> _handlePageFinished(String url) async {
    if (_completed || !_active || _pageLoaded) {
      return;
    }
    final generation = _navigationGeneration;
    try {
      _loadTimeoutTimer?.cancel();
      final title = browserSession?.title;
      if (!_isCurrentNavigation(generation)) {
        return;
      }
      _updateState(() {
        _currentUrl = url;
        _pageTitle = title ?? '';
        _pageLoaded = true;
      });

      if (url.contains('google.com/sorry/index')) {
        _finishError('Google CAPTCHA detected. Please try again later.');
        return;
      }

      final captcha = await _detectCaptcha();
      if (!_isCurrentNavigation(generation)) {
        return;
      }
      if (captcha) {
        _updateState(() {
          _isCaptchaVerification = true;
          _autoModeEnabled = false;
        });
        _startCountdown(_captchaWaitSeconds);
        return;
      }
      if (_autoModeEnabled) {
        _scheduleExtraction();
      }
    } catch (error) {
      if (!_isCurrentNavigation(generation)) {
        return;
      }
      _finishError(error.toString());
    }
  }

  /// Checks whether an asynchronous callback still belongs to the active page.
  bool _isCurrentNavigation(int generation) {
    return _active && !_completed && generation == _navigationGeneration;
  }

  Future<bool> _detectCaptcha() async {
    final raw = await _evaluate(r'''
(function() {
  const body = document.body;
  if (!body) return false;
  const text = String(body.innerText || "");
  const html = String(body.innerHTML || "");
  return text.includes("人机验证") || /captcha/i.test(html);
})()
''');
    return raw == true || raw.toString() == 'true';
  }

  void _armLoadTimeout() {
    _loadTimeoutTimer?.cancel();
    _loadTimeoutTimer = Timer(_loadTimeout, _extractAfterLoadTimeout);
  }

  void _extractAfterLoadTimeout() {
    if (_completed ||
        _pageLoaded ||
        _hasExtractedContent ||
        _extractionRequested ||
        !_autoModeEnabled ||
        _isCaptchaVerification) {
      return;
    }
    if (!_active) {
      return;
    }
    _updateState(() {
      _pageLoaded = true;
    });
    unawaited(_extractPageContent());
  }

  void _scheduleExtraction() {
    _extractDelayTimer?.cancel();
    _extractDelayTimer = Timer(_extractDelay, () {
      if (_completed ||
          _hasExtractedContent ||
          _extractionRequested ||
          !_autoModeEnabled) {
        return;
      }
      unawaited(_extractPageContent());
    });
  }

  Future<void> _extractPageContent() async {
    if (_completed || _extractionRequested) {
      return;
    }
    final generation = _navigationGeneration;
    if (_active) {
      _updateState(() {
        _extractionRequested = true;
        _isLoading = true;
      });
    }
    try {
      await _evaluate(r'''
window.scrollTo(0, document.body ? document.body.scrollHeight : document.documentElement.scrollHeight);
''');
      if (!_isCurrentNavigation(generation)) return;
      final raw = await _evaluate(
        _webVisitExtractionScript(request.includeImageLinks),
      );
      final result = _decodeVisitResult(raw);
      if (!_isCurrentNavigation(generation)) return;
      _updateState(() {
        _isLoading = false;
        _hasExtractedContent = true;
        _pageResult = result;
        _currentUrl = result.url;
        _pageTitle = result.title;
      });
      if (_autoModeEnabled) {
        _startCountdown(_visitWebWaitSeconds);
      }
    } catch (error) {
      if (_isCurrentNavigation(generation)) _finishError(error.toString());
    }
  }

  WebVisitResult _decodeVisitResult(Object? raw) {
    if (raw == null) {
      throw StateError('Web page extraction returned no result');
    }
    Object? decoded = raw;
    if (decoded is String) {
      decoded = jsonDecode(decoded);
      if (decoded is String) {
        decoded = jsonDecode(decoded);
      }
    }
    final json = decoded as Map<String, Object?>;
    final metadataJson = json['metadata'] as Map<String, Object?>;
    final linksJson = json['links'] as List<Object?>;
    final imagesJson = json['imageLinks'] as List<Object?>;
    return WebVisitResult(
      url: json['url'] as String,
      title: json['title'] as String,
      content: json['content'] as String,
      metadata: metadataJson.map(
        (key, value) => MapEntry<String, String>(key, value as String),
      ),
      links: linksJson
          .cast<Map<String, Object?>>()
          .map(
            (item) => WebVisitLink(
              url: item['url'] as String,
              text: item['text'] as String,
            ),
          )
          .toList(growable: false),
      imageLinks: imagesJson.cast<String>().toList(growable: false),
    );
  }

  void _startCountdown(int seconds) {
    _countdownTimer?.cancel();
    if (_completed || !_active) {
      return;
    }
    _updateState(() {
      _autoCountdownActive = true;
      _autoCountdownSeconds = seconds;
    });
    if (seconds <= 0) {
      _continueWithExtractedContent();
      return;
    }
    _countdownTimer = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (_completed || !_autoCountdownActive) {
        timer.cancel();
        return;
      }
      if (_autoCountdownSeconds <= 1) {
        timer.cancel();
        if (_isCaptchaVerification) {
          if (_active) {
            _updateState(() {
              _autoCountdownActive = false;
            });
          }
          return;
        }
        _continueWithExtractedContent();
        return;
      }
      _updateState(() => _autoCountdownSeconds -= 1);
    });
  }

  void _cancelCountdown() {
    _countdownTimer?.cancel();
    if (!_active) {
      return;
    }
    _updateState(() {
      _autoCountdownActive = false;
    });
  }

  void _continueWithExtractedContent() {
    final result = _pageResult;
    if (result == null) {
      return;
    }
    _finishSuccess(result);
  }

  void _finishSuccess(WebVisitResult result) {
    if (_completed) {
      return;
    }
    _completed = true;
    _deadlineTimer?.cancel();
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    _response.complete(
      WebVisitResponse(
        requestId: request.requestId,
        success: true,
        result: result,
      ),
    );
  }

  void _finishError(String message) {
    if (_completed) {
      return;
    }
    _completed = true;
    _deadlineTimer?.cancel();
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    _response.complete(
      WebVisitResponse(
        requestId: request.requestId,
        success: false,
        error: message,
      ),
    );
  }

  void handleLeftAction() {
    if (_autoCountdownActive) {
      _cancelCountdown();
      return;
    }
    if (!_hasExtractedContent && _autoModeEnabled) {
      _updateState(() {
        _autoModeEnabled = false;
      });
      return;
    }
    _finishError('visit_web cancelled');
  }

  void handleRightAction() {
    if (_autoCountdownActive) {
      _cancelCountdown();
      _continueWithExtractedContent();
      return;
    }
    if (_hasExtractedContent) {
      _continueWithExtractedContent();
      return;
    }
    unawaited(_extractPageContent());
  }

  String get statusText {
    if (!_pageLoaded) {
      return 'Loading page';
    }
    if (_isLoading) {
      return 'Reading page';
    }
    if (_isCaptchaVerification) {
      return 'Captcha ${_autoCountdownSeconds}s';
    }
    if (_autoCountdownActive) {
      return 'Returning in ${_autoCountdownSeconds}s';
    }
    if (_hasExtractedContent) {
      return 'Content ready';
    }
    return 'Waiting';
  }

  String get leftButtonText {
    if (_autoCountdownActive) {
      return 'Cancel countdown';
    }
    if (!_hasExtractedContent && _autoModeEnabled) {
      return 'Manual mode';
    }
    return 'Cancel';
  }

  String get rightButtonText {
    if (_autoCountdownActive) {
      return 'Continue now ${_autoCountdownSeconds}s';
    }
    if (_hasExtractedContent) {
      return 'Continue';
    }
    if (!_autoModeEnabled) {
      return 'Extract now';
    }
    return 'Extract';
  }

  String get helperText {
    if (!_pageLoaded) {
      return 'visit_web is loading the page in a runtime browser session.';
    }
    if (_autoCountdownActive) {
      return 'You can continue immediately or stop the automatic countdown.';
    }
    if (!_hasExtractedContent && !_autoModeEnabled) {
      return 'Automatic extraction is paused. Continue when the page is ready.';
    }
    return '';
  }

  bool get pageLoaded => _pageLoaded;
  String get currentUrl => _currentUrl;
  String get pageTitle => _pageTitle;
  void cancel() => _finishError('visit_web cancelled');
}

String _normalizeUrl(String rawUrl) {
  final trimmed = rawUrl.trim();
  final uri = Uri.tryParse(trimmed);
  if (uri != null && uri.hasScheme) {
    return trimmed;
  }
  return 'https://$trimmed';
}

String _webVisitExtractionScript(bool includeImageLinks) {
  return '''
JSON.stringify((() => {
  const includeImages = $includeImageLinks;
  const absoluteUrl = (value) => {
    try { return new URL(value, location.href).href; } catch (_) { return ""; }
  };
  const cleanText = (value) => String(value || "")
    .replace(/\\u00a0/g, " ")
    .split(/\\r?\\n/)
    .map(line => line.trim())
    .filter((line, index, arr) => line.length > 0 || (index > 0 && arr[index - 1].trim().length > 0))
    .join("\\n")
    .trim();
  const metadata = {};
  for (const meta of Array.from(document.querySelectorAll("meta"))) {
    const key = meta.getAttribute("name") || meta.getAttribute("property");
    const content = meta.getAttribute("content");
    if (key && content && !metadata[key]) metadata[key] = content;
  }
  const seenLinks = new Set();
  const links = [];
  for (const anchor of Array.from(document.querySelectorAll("a[href]"))) {
    const url = absoluteUrl(anchor.getAttribute("href"));
    const text = cleanText(anchor.innerText || anchor.getAttribute("aria-label") || anchor.getAttribute("title") || url);
    if (!url || !text || seenLinks.has(url + "\\n" + text)) continue;
    seenLinks.add(url + "\\n" + text);
    links.push({ url, text });
  }
  const imageLinks = [];
  if (includeImages) {
    const seenImages = new Set();
    for (const image of Array.from(document.querySelectorAll("img"))) {
      const src = image.currentSrc || image.getAttribute("src") || image.getAttribute("data-src") || "";
      const url = absoluteUrl(src);
      if (!url || url.startsWith("data:") || url.startsWith("blob:") || seenImages.has(url)) continue;
      seenImages.add(url);
      imageLinks.push(url);
    }
  }
  const title = cleanText(document.title || (document.querySelector("h1") && document.querySelector("h1").innerText) || "Web Page");
  const content = cleanText(document.body ? document.body.innerText : document.documentElement.innerText);
  return { url: location.href, title, content, metadata, links, imageLinks };
})())
''';
}
