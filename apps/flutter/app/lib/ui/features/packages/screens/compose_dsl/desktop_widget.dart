// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Owns the single loading and content-building contract for every widget presentation.
class ToolPkgDesktopWidgetFrame {
  /// Keeps the resolved registration together with its completed DSL result.
  const ToolPkgDesktopWidgetFrame._(this.definition, this._render);
  final core_proxy.ToolPkgDesktopWidget definition;
  final _ComposeDslRenderResult _render;

  /// Executes the registered route and onLoad once through the shared Core service.
  static Future<ToolPkgDesktopWidgetFrame> load({
    required GeneratedCoreProxyClients clients,
    required core_proxy.ToolPkgDesktopWidget definition,
    required String instanceId,
    required bool useEnglish,
  }) async {
    final snapshot = await clients.application
        .packageManager()
        .renderToolPkgDesktopWidget(
          containerPackageName: definition.containerPackageName,
          widgetId: definition.widgetId,
          instanceId: instanceId,
          useEnglish: useEnglish,
        );
    return ToolPkgDesktopWidgetFrame._(
      snapshot.widget,
      _ComposeDslRenderResult(
        tree: _ComposeDslNode.fromSnapshot(snapshot.renderResult.tree),
        state: snapshot.renderResult.state,
        memo: snapshot.renderResult.memo,
        actionResult: null,
      ),
    );
  }

  /// Builds the same Flutter subtree for previews, windows, and host-owned frame capture.
  Widget buildContent({
    required GeneratedCoreProxyClients clients,
    required String instanceId,
    required Future<void> Function(String package, String route) onOpenRoute,
  }) {
    /// Routes widget interactions to the current registration's application destination.
    Future<Object?> openRoute(String actionId, [Object? payload]) async {
      await onOpenRoute(definition.containerPackageName, definition.routeId);
      return null;
    }

    return Material(
      type: MaterialType.transparency,
      child: Semantics(
        label: definition.title,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => openRoute('open'),
          child: _ComposeDslRenderer(
            node: _render.tree,
            nodePath: 'desktop-widget:$instanceId',
            onAction: openRoute,
            onTextInput: openRoute,
            webViewHostContext: ComposeDslWebViewHostContext(
              packageName: definition.containerPackageName,
              routeInstanceId: instanceId,
              executionContextKey: 'desktop-widget:$instanceId',
              dispatchAction: openRoute,
              runtimeOptionsProvider: () => throw UnsupportedError(
                'Desktop widgets do not own interactive WebView sessions',
              ),
            ),
            splitMarkdownContent: (content) => clients.chatRuntimeHolderMain
                .splitMarkdownContent(content: content),
          ),
        ),
      ),
    );
  }
}

/// Renders a registered desktop widget using the same DSL renderer on every Flutter host.
class ToolPkgDesktopWidgetView extends StatefulWidget {
  /// Creates one independently refreshable widget instance with its registered route action.
  const ToolPkgDesktopWidgetView({
    super.key,
    required this.clients,
    required this.definition,
    required this.instanceId,
    required this.onOpenRoute,
    this.refreshInterval = const Duration(minutes: 30),
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgDesktopWidget definition;
  final String instanceId;
  final Future<void> Function(String packageName, String routeId) onOpenRoute;
  final Duration refreshInterval;

  /// Creates an owner for rendering and refresh scheduling.
  @override
  State<ToolPkgDesktopWidgetView> createState() =>
      _ToolPkgDesktopWidgetViewState();
}

class _ToolPkgDesktopWidgetViewState extends State<ToolPkgDesktopWidgetView>
    with PageActivityMixin<ToolPkgDesktopWidgetView> {
  Timer? _timer;
  ToolPkgDesktopWidgetFrame? _render;
  String? _error;
  bool _loading = false;
  int _generation = 0;
  String? _language;

  /// Loads localized widget content when the host locale becomes available.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final language = Localizations.localeOf(context).languageCode;
    if (_language != language) {
      _language = language;
      _restart();
    }
  }

  /// Replaces refresh ownership when the host changes the selected registration.
  @override
  void didUpdateWidget(covariant ToolPkgDesktopWidgetView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.definition.containerPackageName !=
            widget.definition.containerPackageName ||
        oldWidget.definition.widgetId != widget.definition.widgetId ||
        oldWidget.instanceId != widget.instanceId ||
        oldWidget.refreshInterval != widget.refreshInterval) {
      _restart();
    }
  }

  /// Suspends plugin render polling while the cached presentation is hidden.
  @override
  void onPageActivityChanged(bool active) {
    if (active && _language != null) {
      _restart();
    } else if (!active) {
      _timer?.cancel();
      _timer = null;
      _generation += 1;
      _loading = false;
    }
  }

  /// Starts a new render generation and a non-overlapping periodic refresh schedule.
  void _restart() {
    if (widget.refreshInterval <= Duration.zero) {
      throw ArgumentError.value(widget.refreshInterval, 'refreshInterval');
    }
    _timer?.cancel();
    _generation++;
    _loading = false;
    if (!isPageActive) return;
    unawaited(_refresh());
    _timer = Timer.periodic(widget.refreshInterval, (_) {
      if (!_loading) unawaited(_refresh());
    });
  }

  /// Executes the registered render route and its onLoad before exposing the snapshot.
  Future<void> _refresh() async {
    if (_loading || !isPageActive) return;
    final generation = _generation;
    setState(() {
      _loading = true;
      _error = null;
      _render = null;
    });
    try {
      final render = await ToolPkgDesktopWidgetFrame.load(
        clients: widget.clients,
        definition: widget.definition,
        instanceId: widget.instanceId,
        useEnglish: _language == 'en',
      );
      if (!mounted || generation != _generation) return;
      setState(() {
        _render = render;
        _loading = false;
      });
    } catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _error = error.toString();
        _loading = false;
      });
    }
  }

  /// Cancels refresh scheduling and invalidates pending render results.
  @override
  void dispose() {
    _generation++;
    _timer?.cancel();
    super.dispose();
  }

  /// Draws the shared DSL snapshot with explicit loading and error states.
  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(_error!),
          IconButton(onPressed: _refresh, icon: const Icon(Icons.refresh)),
        ],
      );
    }
    final render = _render;
    if (render == null) return const SizedBox.shrink();
    return render.buildContent(
      clients: widget.clients,
      instanceId: widget.instanceId,
      onOpenRoute: widget.onOpenRoute,
    );
  }
}
