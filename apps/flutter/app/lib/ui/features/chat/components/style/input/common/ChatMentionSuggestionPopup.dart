// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

/// Keeps mention suggestions in the same overlay lifecycle as their composer.
class ChatMentionSuggestionPopup extends StatefulWidget {
  /// Anchors optional suggestions to the input surface without replacing it.
  const ChatMentionSuggestionPopup({
    super.key,
    required this.child,
    required this.panel,
    this.dismissBehavior = HitTestBehavior.opaque,
  });

  final Widget child;
  final Widget? panel;
  final HitTestBehavior dismissBehavior;

  /// Retains dismissal state while the current suggestions remain unchanged.
  @override
  State<ChatMentionSuggestionPopup> createState() =>
      _ChatMentionSuggestionPopupState();
}

class _ChatMentionSuggestionPopupState
    extends State<ChatMentionSuggestionPopup> {
  bool _dismissed = false;

  /// Allows newly supplied suggestions to open after an outside dismissal.
  @override
  void didUpdateWidget(covariant ChatMentionSuggestionPopup oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.panel != widget.panel) {
      _dismissed = false;
    }
  }

  /// Removes the portal while keeping the input draft and focus intact.
  void _dismiss() {
    setState(() => _dismissed = true);
  }

  /// Gives the visible portal exactly the input surface's layout bounds.
  @override
  Widget build(BuildContext context) {
    final panel = widget.panel;
    return Stack(
      fit: StackFit.passthrough,
      children: [
        widget.child,
        if (panel != null && !_dismissed)
          Positioned.fill(
            child: _VisibleMentionPopup(
              panel: panel,
              dismissBehavior: widget.dismissBehavior,
              onDismiss: _dismiss,
            ),
          ),
      ],
    );
  }
}

/// Owns the portal only while the current suggestions are visible.
class _VisibleMentionPopup extends StatefulWidget {
  /// Captures the suggestion content and the input style's dismissal behavior.
  const _VisibleMentionPopup({
    required this.panel,
    required this.dismissBehavior,
    required this.onDismiss,
  });

  final Widget panel;
  final HitTestBehavior dismissBehavior;
  final VoidCallback onDismiss;

  /// Creates a controller whose overlay detaches with the owning portal.
  @override
  State<_VisibleMentionPopup> createState() => _VisibleMentionPopupState();
}

class _VisibleMentionPopupState extends State<_VisibleMentionPopup> {
  final _controller = OverlayPortalController()..show();
  final _targetKey = GlobalKey();
  int? _frameCallbackId;

  /// Refreshes geometry on existing frames after portal ownership is resolved.
  void _updatePosition(Duration timestamp) {
    _frameCallbackId = null;
    setState(() {});
  }

  /// Stops tracking while the input is inactive or moving between overlays.
  @override
  void deactivate() {
    final callbackId = _frameCallbackId;
    if (callbackId != null) {
      SchedulerBinding.instance.cancelFrameCallbackWithId(callbackId);
      _frameCallbackId = null;
    }
    super.deactivate();
  }

  /// Places suggestions after both the input and its actual overlay lay out.
  @override
  Widget build(BuildContext context) {
    _frameCallbackId ??= SchedulerBinding.instance.scheduleFrameCallback(
      _updatePosition,
      scheduleNewFrame: false,
    );
    return OverlayPortal(
      controller: _controller,
      overlayChildBuilder: (context) => Stack(
        children: [
          Positioned.fill(
            child: Listener(
              behavior: widget.dismissBehavior,
              onPointerDown: (_) => widget.onDismiss(),
              child: const SizedBox.expand(),
            ),
          ),
          Positioned.fill(
            child: _MentionPopupLayout(
              targetKey: _targetKey,
              // Keyboard insets can move the input without resizing its overlay.
              padding: MediaQuery.of(context).padding,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: widget.panel,
              ),
            ),
          ),
        ],
      ),
      child: SizedBox.expand(key: _targetKey),
    );
  }
}

/// Positions portal content after its anchor and overlay have finished layout.
class _MentionPopupLayout extends SingleChildRenderObjectWidget {
  /// Captures the anchor and inherited viewport padding for this presentation.
  const _MentionPopupLayout({
    required this.targetKey,
    required this.padding,
    required super.child,
  });

  final GlobalKey targetKey;
  final EdgeInsets padding;

  /// Resolves overlay ownership while the portal context is active.
  @override
  _RenderMentionPopupLayout createRenderObject(BuildContext context) =>
      _RenderMentionPopupLayout(
        targetKey: targetKey,
        overlay: Overlay.of(context),
        padding: padding,
      );

  /// Repositions suggestions when their content, viewport, or owner changes.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderMentionPopupLayout renderObject,
  ) {
    renderObject
      ..targetKey = targetKey
      ..overlay = Overlay.of(context)
      ..padding = padding
      ..markNeedsLayout();
  }
}

/// Reads paint transforms inside a layout callback without rebuilding widgets.
class _RenderMentionPopupLayout extends RenderShiftedBox {
  /// Keeps the exact anchor and overlay selected by the active portal.
  _RenderMentionPopupLayout({
    required this.targetKey,
    required this.overlay,
    required this.padding,
  }) : super(null);

  GlobalKey targetKey;
  OverlayState overlay;
  EdgeInsets padding;
  late Offset _anchor;

  /// Reads ancestor transforms where Flutter permits cross-branch size access.
  void _resolveAnchor(BoxConstraints constraints) {
    final overlayBox = overlay.context.findRenderObject()! as RenderBox;
    final target = targetKey.currentContext!.findRenderObject()! as RenderBox;
    _anchor = target.localToGlobal(Offset.zero, ancestor: overlayBox);
  }

  /// Occupies the overlay while the suggestion panel keeps its own height.
  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.biggest;

  /// Constrains suggestions to the page and leaves eight pixels above the input.
  @override
  void performLayout() {
    size = constraints.biggest;
    invokeLayoutCallback(_resolveAnchor);
    final leftPadding = 12.0 + padding.left;
    final rightPadding = 12.0 + padding.right;
    final availableWidth = size.width - leftPadding - rightPadding;
    final width = availableWidth < 700 ? availableWidth : 700.0;
    final maxHeight = (_anchor.dy - padding.top - 20).clamp(96.0, 420.0);
    final panel = child!;
    panel.layout(
      BoxConstraints(minWidth: width, maxWidth: width, maxHeight: maxHeight),
      parentUsesSize: true,
    );
    final left = _anchor.dx.clamp(
      leftPadding,
      size.width - rightPadding - width,
    );
    final parentData = panel.parentData! as BoxParentData;
    parentData.offset = Offset(left, _anchor.dy - 8 - panel.size.height);
  }
}
