// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import 'WorkspaceHtmlPreviewServer.dart';

class WorkspaceHtmlPreviewWidget extends StatefulWidget {
  const WorkspaceHtmlPreviewWidget({
    super.key,
    required this.relativePath,
    required this.onReadWorkspaceFileBytes,
  });

  final String relativePath;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;

  @override
  State<WorkspaceHtmlPreviewWidget> createState() =>
      _WorkspaceHtmlPreviewWidgetState();
}

class _WorkspaceHtmlPreviewWidgetState
    extends State<WorkspaceHtmlPreviewWidget> {
  late final WebViewController _controller;
  late final WorkspaceHtmlPreviewServer _server;
  Future<void>? _loadFuture;
  bool _canGoBack = false;
  bool _canGoForward = false;
  bool _isPageLoading = false;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent)
      ..setNavigationDelegate(
        NavigationDelegate(
          onPageStarted: (_) {
            if (!mounted) {
              return;
            }
            setState(() {
              _isPageLoading = true;
            });
          },
          onPageFinished: (_) {
            _updateNavigationState(isPageLoading: false);
          },
        ),
      );
    _server = WorkspaceHtmlPreviewServer(
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
    );
    _loadFuture = _load();
  }

  @override
  void didUpdateWidget(covariant WorkspaceHtmlPreviewWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.relativePath != widget.relativePath) {
      setState(() {
        _loadFuture = _reload();
      });
    }
  }

  @override
  void dispose() {
    _server.stop();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<void>(
      future: _loadFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snapshot.hasError) {
          return Center(child: Text(snapshot.error.toString()));
        }
        return Column(
          children: <Widget>[
            _WorkspaceHtmlPreviewToolbar(
              canGoBack: _canGoBack,
              canGoForward: _canGoForward,
              isLoading: _isPageLoading,
              onBack: _goBack,
              onForward: _goForward,
              onRefresh: _refresh,
            ),
            Expanded(child: WebViewWidget(controller: _controller)),
          ],
        );
      },
    );
  }

  Future<void> _reload() async {
    await _server.stop();
    await _load();
  }

  Future<void> _load() async {
    final uri = await _server.start(widget.relativePath);
    await _controller.loadRequest(uri);
    await _updateNavigationState(isPageLoading: false);
  }

  Future<void> _updateNavigationState({required bool isPageLoading}) async {
    final canGoBack = await _controller.canGoBack();
    final canGoForward = await _controller.canGoForward();
    if (!mounted) {
      return;
    }
    setState(() {
      _canGoBack = canGoBack;
      _canGoForward = canGoForward;
      _isPageLoading = isPageLoading;
    });
  }

  Future<void> _goBack() async {
    await _controller.goBack();
    await _updateNavigationState(isPageLoading: false);
  }

  Future<void> _goForward() async {
    await _controller.goForward();
    await _updateNavigationState(isPageLoading: false);
  }

  Future<void> _refresh() async {
    await _controller.reload();
    await _updateNavigationState(isPageLoading: true);
  }
}

class _WorkspaceHtmlPreviewToolbar extends StatelessWidget {
  const _WorkspaceHtmlPreviewToolbar({
    required this.canGoBack,
    required this.canGoForward,
    required this.isLoading,
    required this.onBack,
    required this.onForward,
    required this.onRefresh,
  });

  final bool canGoBack;
  final bool canGoForward;
  final bool isLoading;
  final VoidCallback onBack;
  final VoidCallback onForward;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return OperitGlassSurface(
      color: theme.colorScheme.surfaceContainerLow.withValues(alpha: 0.5),
      layer: OperitGlassSurfaceLayer.control,
      border: Border(
        bottom: BorderSide(
          color: theme.colorScheme.outlineVariant.withValues(alpha: 0.36),
        ),
      ),
      child: SizedBox(
        height: 38,
        child: Row(
          children: <Widget>[
            const SizedBox(width: 6),
            _WorkspaceHtmlPreviewToolbarButton(
              tooltip: l10n.back,
              icon: Icons.arrow_back,
              onPressed: canGoBack ? onBack : null,
            ),
            _WorkspaceHtmlPreviewToolbarButton(
              tooltip: l10n.forward,
              icon: Icons.arrow_forward,
              onPressed: canGoForward ? onForward : null,
            ),
            _WorkspaceHtmlPreviewToolbarButton(
              tooltip: l10n.refresh,
              icon: Icons.refresh,
              onPressed: onRefresh,
            ),
            const Spacer(),
            if (isLoading)
              SizedBox(
                width: 18,
                height: 18,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: theme.colorScheme.primary,
                ),
              ),
            const SizedBox(width: 10),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceHtmlPreviewToolbarButton extends StatelessWidget {
  const _WorkspaceHtmlPreviewToolbarButton({
    required this.tooltip,
    required this.icon,
    required this.onPressed,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      icon: Icon(icon, size: 18),
      visualDensity: VisualDensity.compact,
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints.tightFor(width: 34, height: 34),
    );
  }
}
