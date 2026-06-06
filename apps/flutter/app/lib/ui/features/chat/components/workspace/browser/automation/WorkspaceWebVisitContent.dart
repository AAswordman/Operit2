// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../theme/OperitGlassSurface.dart';

class WorkspaceWebVisitContent extends StatefulWidget {
  const WorkspaceWebVisitContent({
    super.key,
    required this.request,
    required this.onFinished,
  });

  final WebVisitRequest request;
  final ValueChanged<WebVisitResponse> onFinished;

  @override
  State<WorkspaceWebVisitContent> createState() =>
      _WorkspaceWebVisitContentState();
}

class _WorkspaceWebVisitContentState extends State<WorkspaceWebVisitContent> {
  static const int _visitWebWaitSeconds = 0;
  static const int _captchaWaitSeconds = 60;
  static const Duration _loadTimeout = Duration(seconds: 10);
  static const Duration _extractDelay = Duration(milliseconds: 800);
  static const Duration _scriptTimeout = Duration(seconds: 10);

  late final WebViewController _controller;
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
  bool _hasSslError = false;
  final ValueNotifier<int> _autoCountdownSeconds = ValueNotifier<int>(0);
  String _currentUrl = '';
  String _pageTitle = '';
  WebVisitResult? _pageResult;

  @override
  void initState() {
    super.initState();
    _currentUrl = _normalizeUrl(widget.request.url);
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent)
      ..setNavigationDelegate(
        NavigationDelegate(
          onPageStarted: _handlePageStarted,
          onPageFinished: _handlePageFinished,
          onNavigationRequest: _handleNavigationRequest,
          onWebResourceError: _handleWebResourceError,
          onHttpError: _handleHttpError,
          onSslAuthError: (request) {
            if (!_completed && mounted) {
              setState(() {
                _hasSslError = true;
              });
            }
            request.proceed();
          },
        ),
      );
    unawaited(_start());
  }

  @override
  void dispose() {
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    _autoCountdownSeconds.dispose();
    super.dispose();
  }

  Future<void> _start() async {
    try {
      _armLoadTimeout();
      final userAgent = widget.request.userAgent.trim();
      if (userAgent.isNotEmpty) {
        await _controller.setUserAgent(userAgent);
      }
      await _controller.loadRequest(Uri.parse(_currentUrl), headers: _headers);
    } catch (error) {
      _finishError(error.toString());
    }
  }

  Map<String, String> get _headers {
    return <String, String>{
      for (final header in widget.request.headers) header.name: header.value,
    };
  }

  void _handlePageStarted(String url) {
    if (_completed || !mounted) {
      return;
    }
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    setState(() {
      _currentUrl = url;
      _hasSslError = false;
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

  Future<void> _handlePageFinished(String url) async {
    if (_completed || !mounted) {
      return;
    }
    _loadTimeoutTimer?.cancel();
    final title = await _controller.getTitle();
    if (_completed || !mounted) {
      return;
    }
    setState(() {
      _currentUrl = url;
      _pageTitle = title ?? '';
      _pageLoaded = true;
    });

    if (url.contains('google.com/sorry/index')) {
      _finishError('Google CAPTCHA detected. Please try again later.');
      return;
    }

    final captcha = await _detectCaptcha();
    if (_completed || !mounted) {
      return;
    }
    if (captcha) {
      setState(() {
        _isCaptchaVerification = true;
        _autoModeEnabled = false;
      });
      _startCountdown(_captchaWaitSeconds);
      return;
    }
    if (_autoModeEnabled) {
      _scheduleExtraction();
    }
  }

  NavigationDecision _handleNavigationRequest(NavigationRequest request) {
    final uri = Uri.tryParse(request.url);
    final scheme = uri?.scheme.toLowerCase();
    if (scheme != 'http' && scheme != 'https') {
      return NavigationDecision.prevent;
    }
    if (!_completed && mounted) {
      setState(() {
        _currentUrl = request.url;
        _isLoading = true;
        _pageLoaded = false;
      });
    }
    return NavigationDecision.navigate;
  }

  void _handleWebResourceError(WebResourceError error) {
    if (_completed || !mounted) {
      return;
    }
    setState(() {
      _isLoading = false;
    });
  }

  void _handleHttpError(HttpResponseError error) {
    if (_completed || !mounted) {
      return;
    }
    setState(() {
      _isLoading = false;
    });
  }

  Future<bool> _detectCaptcha() async {
    final raw = await _controller
        .runJavaScriptReturningResult(r'''
(function() {
  const body = document.body;
  if (!body) return false;
  const text = String(body.innerText || "");
  const html = String(body.innerHTML || "");
  return text.includes("人机验证") || /captcha/i.test(html);
})()
''')
        .timeout(_scriptTimeout);
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
    if (!mounted) {
      return;
    }
    setState(() {
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
    if (mounted) {
      setState(() {
        _extractionRequested = true;
        _isLoading = true;
      });
    }
    try {
      await _controller
          .runJavaScript(r'''
window.scrollTo(0, document.body ? document.body.scrollHeight : document.documentElement.scrollHeight);
''')
          .timeout(_scriptTimeout);
      final raw = await _controller
          .runJavaScriptReturningResult(
            _webVisitExtractionScript(widget.request.includeImageLinks),
          )
          .timeout(_scriptTimeout);
      final result = _decodeVisitResult(raw);
      if (_completed || !mounted) {
        return;
      }
      setState(() {
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
      _finishError(error.toString());
    }
  }

  WebVisitResult _decodeVisitResult(Object raw) {
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
    if (_completed || !mounted) {
      return;
    }
    setState(() {
      _autoCountdownActive = true;
      _autoCountdownSeconds.value = seconds;
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
      if (_autoCountdownSeconds.value <= 1) {
        timer.cancel();
        if (_isCaptchaVerification) {
          if (mounted) {
            setState(() {
              _autoCountdownActive = false;
            });
          }
          return;
        }
        _continueWithExtractedContent();
        return;
      }
      _autoCountdownSeconds.value -= 1;
    });
  }

  void _cancelCountdown() {
    _countdownTimer?.cancel();
    if (!mounted) {
      return;
    }
    setState(() {
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
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    widget.onFinished(
      WebVisitResponse(
        requestId: widget.request.requestId,
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
    _loadTimeoutTimer?.cancel();
    _extractDelayTimer?.cancel();
    _countdownTimer?.cancel();
    widget.onFinished(
      WebVisitResponse(
        requestId: widget.request.requestId,
        success: false,
        error: message,
      ),
    );
  }

  void _handleLeftAction() {
    if (_autoCountdownActive) {
      _cancelCountdown();
      return;
    }
    if (!_hasExtractedContent && _autoModeEnabled) {
      setState(() {
        _autoModeEnabled = false;
      });
      return;
    }
    _finishError('visit_web cancelled');
  }

  void _handleRightAction() {
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

  String get _statusText {
    if (!_pageLoaded) {
      return 'Loading page';
    }
    if (_isLoading) {
      return 'Reading page';
    }
    if (_isCaptchaVerification) {
      return 'Captcha ${_autoCountdownSeconds.value}s';
    }
    if (_autoCountdownActive) {
      return 'Returning in ${_autoCountdownSeconds.value}s';
    }
    if (_hasExtractedContent) {
      return 'Content ready';
    }
    return 'Waiting';
  }

  String get _leftButtonText {
    if (_autoCountdownActive) {
      return 'Cancel countdown';
    }
    if (!_hasExtractedContent && _autoModeEnabled) {
      return 'Manual mode';
    }
    return 'Cancel';
  }

  String get _rightButtonText {
    if (_autoCountdownActive) {
      return 'Continue now ${_autoCountdownSeconds.value}s';
    }
    if (_hasExtractedContent) {
      return 'Continue';
    }
    if (!_autoModeEnabled) {
      return 'Extract now';
    }
    return 'Extract';
  }

  String get _helperText {
    if (!_pageLoaded) {
      return 'visit_web is loading the page in a temporary workspace tab.';
    }
    if (_autoCountdownActive) {
      return 'You can continue immediately or stop the automatic countdown.';
    }
    if (!_hasExtractedContent && !_autoModeEnabled) {
      return 'Automatic extraction is paused. Continue when the page is ready.';
    }
    return '';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
      child: Column(
        children: <Widget>[
          Row(
            children: <Widget>[
              const Expanded(
                child: Text(
                  'Visit Web',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                ),
              ),
              DecoratedBox(
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  child: ValueListenableBuilder<int>(
                    valueListenable: _autoCountdownSeconds,
                    builder: (context, _, _) {
                      return Text(
                        _statusText,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 11,
                          color: theme.colorScheme.onPrimaryContainer,
                        ),
                      );
                    },
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          OperitGlassSurface(
            color: theme.colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.34,
            ),
            layer: OperitGlassSurfaceLayer.card,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: theme.colorScheme.outlineVariant.withValues(alpha: 0.2),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Column(
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      if (_hasSslError) ...<Widget>[
                        DecoratedBox(
                          decoration: BoxDecoration(
                            color: const Color(0xFFFFE6CC),
                            borderRadius: BorderRadius.circular(999),
                          ),
                          child: const Padding(
                            padding: EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 2,
                            ),
                            child: Text(
                              'SSL',
                              style: TextStyle(
                                fontSize: 11,
                                color: Color(0xFF7A3E00),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                      ],
                      Expanded(
                        child: Text(
                          _currentUrl,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 13,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ],
                  ),
                  if (_pageTitle.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 4),
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        _pageTitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 12,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),
          Expanded(
            child: ClipRRect(
              borderRadius: BorderRadius.circular(12),
              child: ColoredBox(
                color: Colors.white,
                child: WebViewWidget(controller: _controller),
              ),
            ),
          ),
          const SizedBox(height: 10),
          Row(
            children: <Widget>[
              Expanded(
                child: FilledButton.tonal(
                  onPressed: _handleLeftAction,
                  child: Text(
                    _leftButtonText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: FilledButton(
                  onPressed: _pageLoaded ? _handleRightAction : null,
                  child: ValueListenableBuilder<int>(
                    valueListenable: _autoCountdownSeconds,
                    builder: (context, _, _) {
                      return Text(
                        _rightButtonText,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      );
                    },
                  ),
                ),
              ),
            ],
          ),
          if (_helperText.isNotEmpty) ...<Widget>[
            const SizedBox(height: 4),
            Text(
              _helperText,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 11,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
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
