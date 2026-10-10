// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

class _ComposeHost extends StatefulWidget {
  /// Creates the compose host instance.
  const _ComposeHost({
    super.key,
    required this.loading,
    required this.error,
    required this.renderResult,
    required this.showLoadingIndicator,
    required this.onAction,
    required this.onTextInput,
    required this.webViewHostContext,
    required this.splitMarkdownContent,
    this.dialogTitle,
  });

  final bool loading;
  final String? error;
  final _ComposeDslRenderResult? renderResult;
  final bool showLoadingIndicator;
  final String? dialogTitle;

  /// Resolves function for the Compose DSL renderer.
  final Future<Object?> Function(String actionId, [Object? payload]) onAction;

  /// Dispatches text edits through the page-wide ordered text queue.
  final Future<Object?> Function(String actionId, String text) onTextInput;
  final ComposeDslWebViewHostContext webViewHostContext;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Creates persistent state for this DSL widget.
  @override
  State<_ComposeHost> createState() => _ComposeHostState();
}

class _ComposeHostState extends State<_ComposeHost> {
  bool _hasDispatchedInitialOnLoad = false;

  /// Synchronizes widget state with the latest DSL node.
  @override
  void didUpdateWidget(covariant _ComposeHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.renderResult?.tree != widget.renderResult?.tree) {
      _dispatchRootOnLoad();
    }
  }

  /// Builds the widget for the current DSL state.
  @override
  Widget build(BuildContext context) {
    final content = _buildContent(context);
    final title = widget.dialogTitle;
    final tree = widget.renderResult?.tree;
    final rootIsDialog = tree?.type == 'Dialog' || tree?.type == 'AlertDialog';
    if (title == null || (!widget.loading && rootIsDialog)) {
      return content;
    }
    return AlertDialog(
      title: Text(title),
      content: SizedBox(width: 620, height: 420, child: content),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(MaterialLocalizations.of(context).closeButtonLabel),
        ),
      ],
    );
  }

  /// Builds loading, error, and rendered content within the selected surface.
  Widget _buildContent(BuildContext context) {
    final tree = widget.renderResult?.tree;
    if (!widget.loading && widget.error == null && tree != null) {
      _dispatchRootOnLoad();
    }
    if (widget.loading && widget.showLoadingIndicator) {
      return const M3LoadingPane();
    }
    if (widget.loading) {
      return const SizedBox.shrink();
    }
    final error = widget.error;
    final errorView = error == null
        ? null
        : Material(
            color: Theme.of(context).colorScheme.errorContainer,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Text(
                error,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onErrorContainer,
                ),
              ),
            ),
          );
    if (tree == null) {
      return errorView == null ? const _NoUiView() : Center(child: errorView);
    }
    // Keep the mounted renderer in a stable slot while reporting action failures.
    // Removing it would fire disposal callbacks, whose commits can mount it again.
    return Stack(
      children: [
        _ComposeDslRenderer(
          node: tree,
          onAction: widget.onAction,
          onTextInput: widget.onTextInput,
          webViewHostContext: widget.webViewHostContext,
          splitMarkdownContent: widget.splitMarkdownContent,
          embedDialog: widget.dialogTitle != null,
        ),
        if (errorView != null)
          Positioned(top: 0, left: 0, right: 0, child: errorView),
      ],
    );
  }

  /// Resolves dispatch root on load for the Compose DSL renderer.
  void _dispatchRootOnLoad() {
    if (_hasDispatchedInitialOnLoad || widget.loading || widget.error != null) {
      return;
    }
    final rootNode = widget.renderResult?.tree;
    if (rootNode == null) {
      return;
    }
    final rootOnLoadActionId = _actionId(rootNode.props['onLoad']);
    if (rootOnLoadActionId == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (!mounted || _hasDispatchedInitialOnLoad) {
        return;
      }
      final currentRootNode = widget.renderResult?.tree;
      if (currentRootNode == null) {
        return;
      }
      final currentRootOnLoadActionId = _actionId(
        currentRootNode.props['onLoad'],
      );
      if (currentRootOnLoadActionId != rootOnLoadActionId) {
        return;
      }
      _hasDispatchedInitialOnLoad = true;
      await widget.onAction(rootOnLoadActionId, null);
    });
  }
}

/// Exposes error presentation with the production host lifecycle for widget regression tests.
@visibleForTesting
Widget buildComposeDslHostForTest({
  required Map<String, Object?> node,
  required ComposeDslWebViewHostContext hostContext,
  String? error,
}) {
  final tree = _ComposeDslNode.parse(node);
  if (tree == null)
    throw const FormatException('A Compose host requires a valid node');
  return _ComposeHost(
    loading: false,
    error: error,
    renderResult: _ComposeDslRenderResult(tree: tree, actionResult: null),
    showLoadingIndicator: false,
    onAction: hostContext.dispatchAction,
    onTextInput: (id, text) => hostContext.dispatchAction(id, text),
    webViewHostContext: hostContext,
    splitMarkdownContent: (_) async => [],
  );
}
