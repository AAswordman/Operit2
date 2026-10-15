// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Describes the fixed-extent window delivered by the page's lazyItems helper.
class _ComposeLazyWindow {
  /// Retains validated viewport metadata and its existing action identity.
  const _ComposeLazyWindow(
    this.firstIndex,
    this.itemCount,
    this.itemExtent,
    this.actionId,
  );

  final int firstIndex;
  final int itemCount;
  final double itemExtent;
  final String actionId;

  /// Rejects inconsistent window metadata before constructing a Flutter sliver.
  factory _ComposeLazyWindow.parse(Object? raw, int childCount) {
    if (raw is! Map) {
      throw const FormatException('Compose viewport must be an object');
    }
    final first = raw['firstIndex'];
    final count = raw['itemCount'];
    final extent = raw['itemExtent'];
    if (first is! num ||
        !first.isFinite ||
        first != first.toInt() ||
        first < 0 ||
        count is! num ||
        !count.isFinite ||
        count != count.toInt() ||
        count < first + childCount ||
        extent is! num ||
        !extent.isFinite ||
        extent <= 0) {
      throw const FormatException(
        'Invalid Compose viewport range or item extent',
      );
    }
    return _ComposeLazyWindow(
      first.toInt(),
      count.toInt(),
      extent.toDouble(),
      _requiredActionId(raw['onRangeChanged'], 'viewport.onRangeChanged'),
    );
  }
}

/// Hosts static or windowed lazy content through the same Flutter sliver and action channel.
class _ComposeLazyList extends StatefulWidget {
  /// Creates a list using serialized children and their deferred widget builder.
  const _ComposeLazyList({
    required this.axis,
    required this.reverse,
    required this.autoScrollToEnd,
    required this.spacing,
    required this.alignment,
    required this.nodes,
    required this.window,
    required this.onAction,
    required this.itemBuilder,
  });

  final Axis axis;
  final bool reverse;
  final bool autoScrollToEnd;
  final double spacing;
  final Alignment alignment;
  final List<_ComposeDslNode> nodes;
  final _ComposeLazyWindow? window;
  final Future<Object?> Function(String, [Object?]) onAction;
  final IndexedWidgetBuilder itemBuilder;

  /// Creates the scroll and viewport request owner.
  @override
  State<_ComposeLazyList> createState() => _ComposeLazyListState();
}

class _ComposeLazyListState extends State<_ComposeLazyList> {
  final ScrollController _controller = ScrollController();
  bool _layoutScheduled = false;
  bool _rangeInFlight = false;
  ({int firstIndex, int lastIndex})? _lastRequested;

  /// Observes scrolling with one listener owned by this mounted list.
  @override
  void initState() {
    super.initState();
    _controller.addListener(_scheduleLayout);
  }

  /// Resets request identity when a different catalog or action replaces the list.
  @override
  void didUpdateWidget(_ComposeLazyList oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.window?.itemCount != widget.window?.itemCount ||
        oldWidget.window?.actionId != widget.window?.actionId ||
        oldWidget.window?.itemExtent != widget.window?.itemExtent) {
      _lastRequested = null;
    }
  }

  /// Coalesces scroll and layout notifications before reading final viewport dimensions.
  void _scheduleLayout() {
    if (_layoutScheduled) return;
    _layoutScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _layoutScheduled = false;
      if (!mounted || !_controller.hasClients) return;
      final position = _controller.position;
      if (!position.hasContentDimensions) return;
      if (widget.autoScrollToEnd) {
        final target = widget.reverse
            ? position.minScrollExtent
            : position.maxScrollExtent;
        if ((target - position.pixels).abs() > 0.5) _controller.jumpTo(target);
      }
      unawaited(_requestRange());
    });
  }

  /// Applies the current text scale to the list's unscaled item extent.
  double _itemExtent(_ComposeLazyWindow window) =>
      window.itemExtent * MediaQuery.textScalerOf(context).scale(14) / 14;

  /// Serializes range actions and preloads two items on either side of the visible interval.
  Future<void> _requestRange() async {
    final window = widget.window;
    if (window == null || _rangeInFlight || window.itemCount == 0) return;
    final position = _controller.position;
    final extent = _itemExtent(window);
    final offset = position.pixels.clamp(0.0, position.maxScrollExtent);
    final first = math.min(
      window.itemCount,
      math.max(0, (offset / extent).floor() - 2),
    );
    final last = math.min(
      window.itemCount,
      ((offset + position.viewportDimension) / extent).ceil() + 2,
    );
    if (first >= window.firstIndex &&
        last <= window.firstIndex + widget.nodes.length) {
      return;
    }
    final range = (firstIndex: first, lastIndex: last);
    if (range == _lastRequested) return;
    _lastRequested = range;
    _rangeInFlight = true;
    try {
      await widget.onAction(window.actionId, {
        'firstIndex': first,
        'lastIndex': last,
      });
    } finally {
      _rangeInFlight = false;
      if (mounted) _scheduleLayout();
    }
  }

  /// Releases the scroll listener and controller with the owning list.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Builds a real item or reserves its fixed extent while its range action is pending.
  Widget _item(BuildContext context, int index, int itemCount) {
    final window = widget.window;
    final local = window == null ? index : index - window.firstIndex;
    if (local < 0 || local >= widget.nodes.length) {
      return SizedBox(key: ValueKey(('compose-lazy-extent', index)));
    }
    final child = widget.itemBuilder(context, local);
    return ComposeDslLazyListItem(
      key: child.key,
      axis: widget.axis,
      alignment: widget.alignment,
      child: Padding(
        padding: widget.axis == Axis.vertical
            ? EdgeInsets.only(
                bottom: index + 1 < itemCount ? widget.spacing : 0,
              )
            : EdgeInsets.only(
                right: index + 1 < itemCount ? widget.spacing : 0,
              ),
        child: child,
      ),
    );
  }

  /// Builds static and generated content with one keyed Flutter delegate.
  @override
  Widget build(BuildContext context) {
    _scheduleLayout();
    final window = widget.window;
    final itemCount = window == null ? widget.nodes.length : window.itemCount;
    final firstIndex = window == null ? 0 : window.firstIndex;
    final indices = <Object, int>{};
    for (var index = 0; index < widget.nodes.length; index++) {
      final key = widget.nodes[index].props['key'];
      if (key != null) {
        if (indices.containsKey(key)) {
          throw StateError('Duplicate Compose list key: $key');
        }
        indices[key] = firstIndex + index;
      }
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final bounded = widget.axis == Axis.vertical
            ? constraints.hasBoundedHeight
            : constraints.hasBoundedWidth;
        if (window != null && !bounded) {
          throw StateError(
            'Generated Compose lazy lists require bounded main-axis constraints',
          );
        }
        return NotificationListener<ScrollMetricsNotification>(
          onNotification: (_) {
            _scheduleLayout();
            return false;
          },
          child: ComposeDslLazyListView(
            controller: _controller,
            scrollDirection: widget.axis,
            reverse: widget.reverse,
            shrinkWrap: !bounded,
            itemExtent: window == null ? null : _itemExtent(window),
            cacheExtent: window == null ? null : 0,
            childrenDelegate: SliverChildBuilderDelegate(
              (context, index) => _item(context, index, itemCount),
              childCount: itemCount,
              findChildIndexCallback: (key) =>
                  key is ValueKey ? indices[key.value] : null,
            ),
          ),
        );
      },
    );
  }
}
