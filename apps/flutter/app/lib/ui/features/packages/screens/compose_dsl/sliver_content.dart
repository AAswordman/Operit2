// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Projects session-owned nodes into an external viewport without changing box rendering.
class _ComposeDslSliverContent extends StatefulWidget {
  /// Reuses the session's node handles and dispatchers for each visible row.
  const _ComposeDslSliverContent({required this.renderer});

  final _ComposeProgramExecutor renderer;

  /// Creates the observer for the root and its immediate item identities.
  @override
  State<_ComposeDslSliverContent> createState() =>
      _ComposeDslSliverContentState();
}

class _ComposeDslSliverContentState extends State<_ComposeDslSliverContent> {
  final Set<_ComposeProgramInstance> _dependencies = {};
  _ComposeLazyWindow? _window;
  int _windowItemCount = 0;
  ({int firstIndex, int lastIndex})? _pendingRange;
  ({int firstIndex, int lastIndex})? _lastRequestedRange;
  bool _rangeScheduled = false;
  bool _rangeInFlight = false;

  /// Coalesces layout requests, keeping the latest viewport during an action.
  void _scheduleRange() {
    if (_rangeScheduled || _rangeInFlight || _pendingRange == null) return;
    _rangeScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      _rangeScheduled = false;
      if (!mounted) return;
      final window = _window;
      final range = _pendingRange;
      _pendingRange = null;
      if (window == null || range == null || range == _lastRequestedRange) {
        return;
      }
      if (range.firstIndex >= window.firstIndex &&
          range.lastIndex <= window.firstIndex + _windowItemCount) {
        return;
      }
      _lastRequestedRange = range;
      _rangeInFlight = true;
      try {
        await widget.renderer.onAction(window.actionId, {
          'firstIndex': range.firstIndex,
          'lastIndex': range.lastIndex,
        });
      } finally {
        _rangeInFlight = false;
        if (mounted) _scheduleRange();
      }
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  /// Uses this sliver's cache interval, already relative to its static prefix.
  void _requestRange(
    SliverConstraints constraints,
    _ComposeLazyWindow window,
    double extent,
  ) {
    final start = math.max(
      0.0,
      constraints.scrollOffset + constraints.cacheOrigin,
    );
    final end = math.min(
      window.itemCount * extent,
      start + constraints.remainingCacheExtent,
    );
    if (end <= start) {
      _pendingRange = null;
      return;
    }
    _pendingRange = window.rangeToRequest(
      (start / extent).floor(),
      (end / extent).ceil(),
      _windowItemCount,
      rangeIncludesCache: true,
    );
    _scheduleRange();
  }

  /// Reads the live root without reconstructing a snapshot of its descendants.
  _ComposeProgramNode get node => widget.renderer.node;

  /// Reads root parameters directly for the sliver-owned viewport callback.
  Map<String, Object?> get props => node.props;

  /// Schedules layout after an observed root or item identity changes.
  void _invalidate() {
    if (mounted) setState(() {});
  }

  /// Releases observers before replacing the root or unmounting the projection.
  void _clearDependencies() {
    for (final dependency in _dependencies) {
      dependency.removeListener(_invalidate);
    }
    _dependencies.clear();
  }

  /// Releases all retained-node observers.
  @override
  void dispose() {
    _clearDependencies();
    super.dispose();
  }

  /// Observes structural reads while visible rows retain their own subscriptions.
  @override
  Widget build(BuildContext context) {
    _clearDependencies();
    if (node.isDisposed) {
      return const SliverToBoxAdapter(child: SizedBox.shrink());
    }
    final reads = <_ComposeProgramInstance>{};
    try {
      return _ComposeProgramReadScope.track(reads, _sliverContent);
    } finally {
      for (final dependency in reads) {
        if (!dependency.isDisposed && _dependencies.add(dependency)) {
          dependency.addListener(_invalidate);
        }
      }
    }
  }

  /// Projects static prefixes and fixed-extent windows into the owning viewport.
  Widget _sliverContent() {
    if (node.type != 'LazyColumn') {
      return SliverToBoxAdapter(child: widget.renderer);
    }
    if (props['reverseLayout'] == true ||
        props['autoScrollToEnd'] == true) {
      throw StateError(
        'Embedded LazyColumn content cannot reverse or auto-scroll its parent viewport.',
      );
    }
    final items = <_ComposeProgramNode>[];
    final dialogs = <_ComposeProgramNode>[];
    final content = node.slots['content'];
    final children = content != null && content.isNotEmpty
        ? content
        : node.children;
    for (final child in children) {
      if (child.type == 'Dialog' || child.type == 'AlertDialog') {
        dialogs.add(child);
      } else {
        items.add(child);
      }
    }
    final window = props['viewport'] == null
        ? null
        : _ComposeLazyWindow.parse(props['viewport'], items.length);
    if (_window?.firstIndex != window?.firstIndex ||
        _window?.itemCount != window?.itemCount ||
        _window?.itemExtent != window?.itemExtent ||
        _window?.staticItemCount != window?.staticItemCount) {
      _lastRequestedRange = null;
      _pendingRange = null;
    }
    _window = window;
    final staticItemCount = window?.staticItemCount ?? items.length;
    _windowItemCount = items.length - staticItemCount;
    final staticItems = items.take(staticItemCount).toList(growable: false);
    final windowItems = items.skip(staticItemCount).toList(growable: false);
    final staticIndices = <Object, int>{};
    for (var index = 0; index < staticItems.length; index++) {
      final key = staticItems[index].props['key'];
      if (key != null) {
        if (staticIndices.containsKey(key)) {
          throw StateError('Duplicate Compose list key: $key');
        }
        staticIndices[key] = index;
      }
    }
    final windowIndices = <Object, int>{};
    for (var index = 0; index < windowItems.length; index++) {
      final key = windowItems[index].props['key'];
      if (key != null) {
        if (windowIndices.containsKey(key)) {
          throw StateError('Duplicate Compose list key: $key');
        }
        windowIndices[key] = window == null ? index : window.firstIndex + index;
      }
    }
    final spacing = _number(props['spacing']) ?? 0;
    final alignment = Alignment(switch (_normalizeToken(
      _string(props['horizontalAlignment']),
    )) {
      'center' || 'centerhorizontally' => 0,
      'end' || 'right' => 1,
      _ => -1,
    }, -1);
    EdgeInsetsGeometry padding =
        _commonPaddingFromProps(props) ?? EdgeInsets.zero;
    for (final operation in _modifierOps(props['modifier'])) {
      if (_normalizeToken(_string(operation['name'])) == 'padding') {
        padding = padding.add(_edgeInsets(operation['args'] as List<Object?>));
      }
    }
    Widget listItem(
      _ComposeProgramNode child,
      String path, {
      required bool addSpacing,
    }) => KeyedSubtree(
      key: child.props['key'] == null ? null : ValueKey(child.props['key']),
      child: Padding(
        padding: EdgeInsets.only(bottom: addSpacing ? spacing : 0.0),
        child: Align(
          alignment: alignment,
          heightFactor: 1,
          child: _sliverItem(child, path),
        ),
      ),
    );
    final staticSliver = SliverList(
      delegate: SliverChildBuilderDelegate(
        (context, index) => listItem(
          staticItems[index],
          'content/$index',
          addSpacing:
              index + 1 < staticItems.length || (window?.itemCount ?? 0) > 0,
        ),
        childCount: staticItems.length,
        findChildIndexCallback: (key) =>
            key is ValueKey ? staticIndices[key.value] : null,
      ),
    );
    final extent = window == null
        ? 0.0
        : window.itemExtent * MediaQuery.textScalerOf(context).scale(14) / 14;
    final lazySliver = window == null
        ? null
        : SliverLayoutBuilder(
            builder: (context, constraints) {
              _requestRange(constraints, window, extent);
              return SliverFixedExtentList(
                itemExtent: extent,
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    final local = index - window.firstIndex;
                    if (local < 0 || local >= windowItems.length) {
                      return const SizedBox.shrink();
                    }
                    return listItem(
                      windowItems[local],
                      'content/${staticItemCount + index}',
                      addSpacing: index + 1 < window.itemCount,
                    );
                  },
                  childCount: window.itemCount,
                  findChildIndexCallback: (key) =>
                      key is ValueKey ? windowIndices[key.value] : null,
                ),
              );
            },
          );
    return SliverMainAxisGroup(
      slivers: [
        for (var index = 0; index < dialogs.length; index++)
          SliverToBoxAdapter(
            key: dialogs[index].props['key'] == null
                ? null
                : ValueKey((
                    'compose-sliver-dialog',
                    dialogs[index].props['key'],
                  )),
            child: _sliverItem(dialogs[index], 'dialog/$index'),
          ),
        SliverPadding(
          key: const ValueKey('compose-sliver-items'),
          padding: padding,
          sliver: SliverMainAxisGroup(slivers: [staticSliver, ?lazySliver]),
        ),
      ],
    );
  }

  /// Builds one keyed row or persistent modal through the ordinary node renderer.
  Widget _sliverItem(_ComposeProgramNode child, String path) => _ComposeProgramExecutor(
    key: child.props['key'] == null ? null : ValueKey(child.props['key']),
    node: child,
    onAction: widget.renderer.onAction,
    onTextInput: widget.renderer.onTextInput,
    webViewHostContext: widget.renderer.webViewHostContext,
    splitMarkdownContent: widget.renderer.splitMarkdownContent,
    dispatchOnLoad: widget.renderer.dispatchOnLoad,
    nodePath: '${widget.renderer.nodePath}:$path',
  );
}
