// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../../../common/markdown/StreamMarkdownRenderer.dart';

/// Retains one row's layout distance independently of its mounted render tree.
class ChatMessageExtent extends ChangeNotifier {
  /// Creates the distance record for one stable transcript row identity.
  ChatMessageExtent();

  double? _width;
  double _height = 0;
  bool _hasMeasuredHeight = false;
  double _trailingSpacing = 0;
  final Set<Object> _pendingContent = <Object>{};

  /// Reports whether a descendant is still preparing its first content layout.
  bool get hasPendingContent => _pendingContent.isNotEmpty;

  /// Keeps measured distances independent of estimates for unprepared rows.
  double estimateHeight(double width, double initialEstimate) {
    if (_width != width || !_hasMeasuredHeight) {
      _width = width;
      _height = initialEstimate;
      _hasMeasuredHeight = false;
    }
    return _height;
  }

  /// Returns the distance reserved for the current row constraints.
  double reservedHeight(double width) => _width == width ? _height : 0;

  /// Records real layouts and retains their distance during pending rendering.
  void recordHeight(double width, double height) {
    final retainsMeasurement = _width == width && _hasMeasuredHeight;
    _height = hasPendingContent
        ? math.max(height, reservedHeight(width))
        : height;
    _width = width;
    _hasMeasuredHeight = !hasPendingContent || retainsMeasurement;
  }

  /// Updates row spacing when appending messages changes the trailing row.
  void updateTrailingSpacing(double spacing) {
    if (_width != null) {
      _height += spacing - _trailingSpacing;
    }
    _trailingSpacing = spacing;
  }

  /// Tracks each asynchronous descendant without relying on settle timers.
  void updatePendingContent(Object source, {required bool isPending}) {
    final wasPending = hasPendingContent;
    if (isPending) {
      _pendingContent.add(source);
    } else {
      _pendingContent.remove(source);
    }
    if (wasPending != hasPendingContent) {
      notifyListeners();
    }
  }
}

/// Measures a row and reserves its cached space while content is being rebuilt.
class ChatMessageExtentBox extends SingleChildRenderObjectWidget {
  /// Associates the mounted row with its persistent extent record.
  const ChatMessageExtentBox({
    super.key,
    required this.extent,
    required super.child,
  });

  final ChatMessageExtent extent;

  /// Creates a box that measures the entire row, including its spacing.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderChatMessageExtentBox(extent);

  /// Keeps a retained render box attached to the current row identity.
  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderChatMessageExtentBox renderObject,
  ) {
    renderObject.extent = extent;
  }
}

/// Preserves pending row space without fixing completed content to an old size.
class RenderChatMessageExtentBox extends RenderProxyBox {
  /// Retains the row's independent layout measurement.
  RenderChatMessageExtentBox(this._extent);

  ChatMessageExtent _extent;

  /// Rebinds asynchronous layout notifications when the row identity changes.
  set extent(ChatMessageExtent value) {
    if (identical(value, _extent)) {
      return;
    }
    if (attached) {
      _extent.removeListener(markNeedsLayout);
    }
    _extent = value;
    if (attached) {
      _extent.addListener(markNeedsLayout);
    }
    markNeedsLayout();
  }

  /// Observes pending content while this box participates in layout.
  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _extent.addListener(markNeedsLayout);
  }

  /// Removes the layout listener when a virtualized row leaves the viewport.
  @override
  void detach() {
    _extent.removeListener(markNeedsLayout);
    super.detach();
  }

  /// Uses cached distance only during an explicitly pending content layout.
  @override
  Size computeDryLayout(BoxConstraints constraints) {
    final contentSize = child!.getDryLayout(constraints);
    return _sizeWithReservation(constraints, contentSize);
  }

  /// Measures real content and updates the cached distance in the same layout.
  @override
  void performLayout() {
    child!.layout(constraints, parentUsesSize: true);
    size = _sizeWithReservation(constraints, child!.size);
    _extent.recordHeight(constraints.maxWidth, size.height);
  }

  /// Adds only the space required by the current asynchronous preparation.
  Size _sizeWithReservation(BoxConstraints constraints, Size contentSize) {
    final reservedHeight = _extent.hasPendingContent
        ? _extent.reservedHeight(constraints.maxWidth)
        : 0.0;
    return constraints.constrain(
      Size(contentSize.width, math.max(contentSize.height, reservedHeight)),
    );
  }
}

/// Computes virtualized scroll distance from persistent per-row measurements.
class ChatMessageSliverList extends SliverList {
  /// Shares the same height records across both locator growth directions.
  ChatMessageSliverList({
    super.key,
    required this.extents,
    required IndexedWidgetBuilder itemBuilder,
    required this.preserveViewport,
  }) : super(
         delegate: SliverChildBuilderDelegate(
           itemBuilder,
           childCount: extents.length,
         ),
       );

  final List<ChatMessageExtent> extents;
  final ValueGetter<bool> preserveViewport;

  /// Sums measured tail rows separately from estimates for unprepared content.
  @override
  double estimateMaxScrollOffset(
    SliverConstraints? constraints,
    int firstIndex,
    int lastIndex,
    double leadingScrollOffset,
    double trailingScrollOffset,
  ) {
    final initialEstimate =
        (trailingScrollOffset - leadingScrollOffset) /
        (lastIndex - firstIndex + 1);
    var total = trailingScrollOffset;
    for (var index = lastIndex + 1; index < extents.length; index++) {
      total += extents[index].estimateHeight(
        constraints!.crossAxisExtent,
        initialEstimate,
      );
    }
    return total;
  }

  /// Creates a sliver that preserves the visible row across height changes.
  @override
  RenderSliverList createRenderObject(BuildContext context) =>
      RenderChatMessageSliverList(
        childManager: context as SliverMultiBoxAdaptorElement,
        preserveViewport: preserveViewport,
      );

  /// Updates viewport ownership without replacing the active scroll position.
  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderChatMessageSliverList renderObject,
  ) {
    renderObject.preserveViewport = preserveViewport;
  }
}

/// Applies layout corrections without interrupting the user's drag activity.
class RenderChatMessageSliverList extends RenderSliverList {
  /// Connects normal sliver virtualization to the current ownership policy.
  RenderChatMessageSliverList({
    required super.childManager,
    required this.preserveViewport,
  });

  ValueGetter<bool> preserveViewport;

  /// Holds the visible row's offset when earlier mounted rows change height.
  @override
  void performLayout() {
    RenderBox? anchor;
    double? oldAnchorOffset;
    if (preserveViewport() && constraints.remainingPaintExtent > 0) {
      final viewportStart = constraints.scrollOffset;
      final viewportEnd = viewportStart + constraints.remainingPaintExtent;
      var candidate = firstChild;
      while (candidate != null) {
        final offset = childScrollOffset(candidate);
        if (candidate.hasSize && offset != null) {
          final startsInViewport =
              offset >= viewportStart && offset < viewportEnd;
          final coversViewport =
              offset <= viewportStart &&
              offset + paintExtentOf(candidate) >= viewportEnd;
          if (startsInViewport || coversViewport) {
            anchor = candidate;
            oldAnchorOffset = offset;
            break;
          }
        }
        candidate = childAfter(candidate);
      }
    }
    super.performLayout();
    if (anchor == null ||
        anchor.parent != this ||
        geometry!.scrollOffsetCorrection != null) {
      return;
    }
    final correction = math.max(
      childScrollOffset(anchor)! - oldAnchorOffset!,
      -constraints.scrollOffset,
    );
    if (correction.abs() > precisionErrorTolerance) {
      geometry = SliverGeometry(scrollOffsetCorrection: correction);
    }
  }
}

/// Connects Markdown preparation to its containing row's distance reservation.
Widget observeChatMessageExtent({
  required Key key,
  required ChatMessageExtent extent,
  required Widget child,
}) {
  return NotificationListener<MarkdownLayoutNotification>(
    key: key,
    onNotification: (notification) {
      extent.updatePendingContent(
        notification.source,
        isPending: notification.isPending,
      );
      return true;
    },
    child: ChatMessageExtentBox(extent: extent, child: child),
  );
}
