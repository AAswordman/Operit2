// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../market/ArtifactMarketSupport.dart';
import 'GitHubOAuthLoginCallback.dart';

/// Selects whether GitHub authentication is rendered in Operit or externally.
enum GitHubOAuthLoginMode { embedded, external }

/// Lets every Flutter login entry point use the same browser selection flow.
Future<void> showGitHubOAuthLoginDialog({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required Future<void> Function() onLoginCompleted,
}) async {
  final mode = await showDialog<GitHubOAuthLoginMode>(
    context: context,
    builder: (_) => const _GitHubOAuthLoginMethodDialog(),
  );
  if (mode == null || !context.mounted) {
    return;
  }
  await showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (_) => GitHubOAuthLoginDialog(
      clients: clients,
      mode: mode,
      onLoginCompleted: onLoginCompleted,
    ),
  );
}

/// Runs one selected GitHub OAuth browser flow.
class GitHubOAuthLoginDialog extends StatefulWidget {
  const GitHubOAuthLoginDialog({
    super.key,
    required this.clients,
    required this.onLoginCompleted,
    this.mode = GitHubOAuthLoginMode.embedded,
  });

  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onLoginCompleted;
  final GitHubOAuthLoginMode mode;

  @override
  State<GitHubOAuthLoginDialog> createState() => _GitHubOAuthLoginDialogState();
}

class _GitHubOAuthLoginDialogState extends State<GitHubOAuthLoginDialog> {
  static const double _dialogWidth = 880;
  static const double _dialogHeight = 720;

  WebViewController? _browserController;
  GitHubOAuthLoginCallback? _externalCallback;
  GitHubOAuthBrokerLoginStart? _loginStart;
  Uri? _completionRedirectUri;
  Timer? _expiryTimer;
  bool _isPageLoading = true;
  bool _isCompleting = false;
  bool _isExternalBrowserOpened = false;
  bool _cancelled = false;
  String? _browserError;

  @override
  /// Initializes the selected browser flow before requesting an authorization URL.
  void initState() {
    super.initState();
    if (widget.mode == GitHubOAuthLoginMode.embedded) {
      _completionRedirectUri = coreMarketAuthCompletionRedirectUri;
      _browserController = _createEmbeddedBrowserController();
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      unawaited(_startLogin());
    });
  }

  @override
  void dispose() {
    _cancelled = true;
    _expiryTimer?.cancel();
    final callback = _externalCallback;
    if (callback != null) {
      unawaited(callback.close());
    }
    super.dispose();
  }

  /// Creates the embedded WebView with the same completion interception on every platform.
  WebViewController _createEmbeddedBrowserController() {
    return WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onNavigationRequest: _handleNavigationRequest,
          onPageStarted: (_) {
            if (!mounted) {
              return;
            }
            setState(() {
              _isPageLoading = true;
              _browserError = null;
            });
          },
          onPageFinished: _handlePageFinished,
          onUrlChange: _handleUrlChange,
          onWebResourceError: _handleWebResourceError,
        ),
      );
  }

  /// Starts Core's broker transaction and presents the selected browser surface.
  Future<void> _startLogin() async {
    GitHubOAuthLoginCallback? externalCallback;
    try {
      if (widget.mode == GitHubOAuthLoginMode.external) {
        externalCallback = await GitHubOAuthLoginCallback.create(
          coreMarketAuthCompletionRedirectUri,
        );
        _externalCallback = externalCallback;
        _completionRedirectUri = externalCallback.redirectUri;
      }

      final start = await startCoreMarketAuthLogin(
        clients: widget.clients,
        completionRedirectUri: _completionRedirectUri,
      );
      if (!mounted || _cancelled) {
        await externalCallback?.close();
        return;
      }
      setState(() {
        _loginStart = start;
        _isPageLoading = true;
      });
      _scheduleExpiry(start.expiresAt);

      if (widget.mode == GitHubOAuthLoginMode.embedded) {
        await _browserController!.loadRequest(
          Uri.parse(start.authorizationUrl),
        );
        return;
      }

      final callback = externalCallback;
      if (callback == null) {
        throw StateError('GitHub external login callback is unavailable');
      }
      final launched = await launchUrl(
        Uri.parse(start.authorizationUrl),
        mode: LaunchMode.externalApplication,
      );
      if (!launched) {
        throw StateError('Unable to open the system browser for GitHub login');
      }
      if (!mounted || _cancelled) {
        return;
      }
      setState(() {
        _isPageLoading = false;
        _isExternalBrowserOpened = true;
      });

      final completionUrl = await callback.completion;
      if (completionUrl == null || !mounted || _cancelled) {
        return;
      }
      await _completeLogin(completionUrl);
    } catch (error, stackTrace) {
      _closeWithError(error, stackTrace);
    }
  }

  /// Claims the one-time broker result after the selected browser reaches its callback.
  Future<void> _completeLogin(Uri completionUrl) async {
    final start = _loginStart;
    final redirectUri = _completionRedirectUri;
    if (start == null || redirectUri == null || _isCompleting || _cancelled) {
      return;
    }
    if (!isMarketAuthCompletionUri(completionUrl, redirectUri: redirectUri)) {
      return;
    }
    _isCompleting = true;
    _expiryTimer?.cancel();
    if (mounted) {
      setState(() {
        _isPageLoading = true;
      });
    }
    try {
      await completeCoreMarketAuthLogin(
        clients: widget.clients,
        start: start,
        completionUrl: completionUrl,
        completionRedirectUri: redirectUri,
      );
      await widget.onLoginCompleted();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error, stackTrace) {
      _closeWithError(error, stackTrace);
    }
  }

  /// Stops embedded-browser navigation when it reaches the registered callback.
  Future<NavigationDecision> _handleNavigationRequest(
    NavigationRequest request,
  ) async {
    final uri = Uri.tryParse(request.url);
    final redirectUri = _completionRedirectUri;
    if (uri != null &&
        redirectUri != null &&
        isMarketAuthCompletionUri(uri, redirectUri: redirectUri)) {
      unawaited(_completeLogin(uri));
      return NavigationDecision.prevent;
    }
    return NavigationDecision.navigate;
  }

  /// Captures completion on engines that report page finishes directly.
  void _handlePageFinished(String url) {
    _handlePossibleCompletionUrl(url);
    if (!mounted || _isCompleting) {
      return;
    }
    setState(() {
      _isPageLoading = false;
    });
  }

  /// Captures the callback as soon as the browser reports a URL change.
  void _handleUrlChange(UrlChange change) {
    final url = change.url;
    if (url == null) {
      return;
    }
    _handlePossibleCompletionUrl(url);
  }

  /// Dispatches a browser callback URL to Core exactly once.
  void _handlePossibleCompletionUrl(String url) {
    final uri = Uri.tryParse(url);
    final redirectUri = _completionRedirectUri;
    if (uri != null &&
        redirectUri != null &&
        isMarketAuthCompletionUri(uri, redirectUri: redirectUri)) {
      unawaited(_completeLogin(uri));
    }
  }

  /// Shows a main-frame WebView failure above the embedded browser.
  void _handleWebResourceError(WebResourceError error) {
    if (error.isForMainFrame == false || !mounted || _isCompleting) {
      return;
    }
    setState(() {
      _isPageLoading = false;
      _browserError = error.description;
    });
  }

  /// Closes this dialog and reports a login failure to the caller.
  void _closeWithError(Object error, StackTrace stackTrace) {
    debugPrint('GitHub OAuth login failed: $error\n$stackTrace');
    _expiryTimer?.cancel();
    final callback = _externalCallback;
    if (callback != null) {
      unawaited(callback.close());
    }
    if (!mounted) {
      return;
    }
    final messenger = ScaffoldMessenger.of(context);
    Navigator.of(context).pop();
    messenger.showSnackBar(
      SnackBar(
        content: Text(error.toString()),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }

  /// Closes the selected browser flow without attempting to claim a result.
  void _cancelLogin() {
    if (_isCompleting) {
      return;
    }
    _cancelled = true;
    _expiryTimer?.cancel();
    final callback = _externalCallback;
    if (callback != null) {
      unawaited(callback.close());
    }
    Navigator.of(context).pop();
  }

  /// Converts the broker expiration timestamp into a local login deadline.
  void _scheduleExpiry(int expiresAt) {
    _expiryTimer?.cancel();
    final remaining = Duration(
      milliseconds: expiresAt - DateTime.now().millisecondsSinceEpoch,
    );
    if (remaining <= Duration.zero) {
      _closeWithError(
        StateError('GitHub login authorization has expired'),
        StackTrace.current,
      );
      return;
    }
    _expiryTimer = Timer(remaining, () {
      _closeWithError(
        StateError('GitHub login authorization has expired'),
        StackTrace.current,
      );
    });
  }

  @override
  /// Renders the selected GitHub authorization surface.
  Widget build(BuildContext context) {
    final loginStart = _loginStart;
    return Dialog(
      constraints: const BoxConstraints(maxWidth: _dialogWidth),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: _dialogWidth,
        height: _dialogHeight,
        child: Column(
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 18, 16, 14),
              child: Row(
                children: <Widget>[
                  const Icon(Icons.account_circle_outlined),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      widget.mode == GitHubOAuthLoginMode.external
                          ? 'GitHub 登录（系统浏览器）'
                          : 'GitHub 登录（内置浏览器）',
                      style: const TextStyle(
                        fontSize: 20,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  if (_isCompleting)
                    const SizedBox.square(
                      dimension: 20,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  else
                    IconButton(
                      tooltip: '取消登录',
                      onPressed: _cancelLogin,
                      icon: const Icon(Icons.close),
                    ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: widget.mode == GitHubOAuthLoginMode.external
                  ? _buildExternalBody()
                  : _buildEmbeddedBody(context, loginStart),
            ),
          ],
        ),
      ),
    );
  }

  /// Explains the external browser hand-off while the loopback callback waits.
  Widget _buildExternalBody() {
    final waitingForBrowser = _loginStart == null || !_isExternalBrowserOpened;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (waitingForBrowser || _isCompleting)
              const CircularProgressIndicator()
            else
              const Icon(Icons.open_in_browser, size: 64),
            const SizedBox(height: 24),
            Text(
              waitingForBrowser
                  ? '正在打开系统默认浏览器…'
                  : _isCompleting
                  ? '正在完成 GitHub 登录…'
                  : '请在系统默认浏览器中完成 GitHub 登录，然后返回此窗口。',
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 16),
            ),
            if (_isExternalBrowserOpened && !_isCompleting) ...<Widget>[
              const SizedBox(height: 12),
              const Text('浏览器完成回调后，登录状态会自动同步。', textAlign: TextAlign.center),
            ],
          ],
        ),
      ),
    );
  }

  /// Renders the existing embedded WebView and its navigation state.
  Widget _buildEmbeddedBody(
    BuildContext context,
    GitHubOAuthBrokerLoginStart? loginStart,
  ) {
    final controller = _browserController;
    return Stack(
      children: <Widget>[
        if (loginStart != null && controller != null)
          Positioned.fill(child: WebViewWidget(controller: controller))
        else
          const Center(child: CircularProgressIndicator()),
        if (_isPageLoading && loginStart != null)
          const Align(
            alignment: Alignment.topCenter,
            child: LinearProgressIndicator(),
          ),
        if (_browserError != null)
          Positioned(
            top: 12,
            left: 12,
            right: 12,
            child: Material(
              color: Theme.of(context).colorScheme.errorContainer,
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Text(
                  _browserError!,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.onErrorContainer,
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

/// Presents the shared browser-mode choice used by settings and marketplace login.
class _GitHubOAuthLoginMethodDialog extends StatelessWidget {
  const _GitHubOAuthLoginMethodDialog();

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('选择 GitHub 登录方式'),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _GitHubOAuthLoginMethodTile(
              icon: Icons.open_in_browser,
              title: '系统默认浏览器',
              subtitle: '使用浏览器现有的 GitHub 登录状态',
              onTap: () =>
                  Navigator.of(context).pop(GitHubOAuthLoginMode.external),
            ),
            _GitHubOAuthLoginMethodTile(
              icon: Icons.web,
              title: '内置浏览器',
              subtitle: '在 Operit 内完成 GitHub 登录',
              onTap: () =>
                  Navigator.of(context).pop(GitHubOAuthLoginMode.embedded),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
      ],
    );
  }
}

class _GitHubOAuthLoginMethodTile extends StatelessWidget {
  const _GitHubOAuthLoginMethodTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle),
      onTap: onTap,
    );
  }
}
