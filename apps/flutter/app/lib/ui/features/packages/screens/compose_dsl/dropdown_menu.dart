// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Identifies the v1 popup contract, which uses the plugin's own trigger.
bool _isControlledDropdown(String type, Map<String, Object?> props) =>
    type == 'DropdownMenu' && props['expanded'] is bool;

/// Displays a controlled popup over the layout that owns its trigger.
class _ComposeControlledDropdown extends StatefulWidget {
  /// Creates an open popup whose lifetime is controlled by the Compose node.
  const _ComposeControlledDropdown({
    required this.properties,
    required this.verticalOffset,
    required this.onDismiss,
    required this.menuBuilder,
  });

  final Map<String, Object?> properties;
  final double verticalOffset;
  final VoidCallback onDismiss;
  final Widget Function(BuildContext, double) menuBuilder;

  /// Owns the overlay while the menu is expanded.
  @override
  State<_ComposeControlledDropdown> createState() =>
      _ComposeControlledDropdownState();
}

class _ComposeControlledDropdownState
    extends State<_ComposeControlledDropdown> {
  final _overlay = OverlayPortalController()..show();

  /// Keeps all menu content out of the trigger's layout and hit testing.
  @override
  Widget build(BuildContext context) {
    final dismissOnBack = widget.properties['dismissOnBackPress'] != false;
    final dismissOnTap = widget.properties['dismissOnClickOutside'] != false;
    final focusable = widget.properties['focusable'] != false;
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop && dismissOnBack) widget.onDismiss();
      },
      child: OverlayPortal.overlayChildLayoutBuilder(
        controller: _overlay,
        overlayChildBuilder: (context, info) {
          final anchor = MatrixUtils.transformRect(
            info.childPaintTransform,
            Offset.zero & info.childSize,
          );
          return Stack(
            fit: StackFit.expand,
            children: [
              ModalBarrier(
                dismissible: dismissOnTap,
                onDismiss: widget.onDismiss,
                semanticsLabel: MaterialLocalizations.of(
                  context,
                ).modalBarrierDismissLabel,
              ),
              CustomSingleChildLayout(
                delegate: _ComposeDropdownPosition(
                  anchor: anchor,
                  insets:
                      MediaQuery.paddingOf(context) +
                      MediaQuery.viewInsetsOf(context) +
                      const EdgeInsets.all(8),
                  verticalOffset: widget.verticalOffset,
                  textDirection: Directionality.of(context),
                ),
                child: CallbackShortcuts(
                  bindings: {
                    if (dismissOnBack)
                      const SingleActivator(LogicalKeyboardKey.escape):
                          widget.onDismiss,
                  },
                  child: FocusScope(
                    autofocus: focusable,
                    canRequestFocus: focusable,
                    descendantsAreFocusable: focusable,
                    child: widget.menuBuilder(context, anchor.width),
                  ),
                ),
              ),
            ],
          );
        },
        child: const SizedBox.shrink(),
      ),
    );
  }
}

/// Fits the popup to the viewport and aligns it with the trigger's start edge.
class _ComposeDropdownPosition extends SingleChildLayoutDelegate {
  /// Captures the trigger bounds and usable overlay insets for this layout.
  const _ComposeDropdownPosition({
    required this.anchor,
    required this.insets,
    required this.verticalOffset,
    required this.textDirection,
  });

  final Rect anchor;
  final EdgeInsets insets;
  final double verticalOffset;
  final TextDirection textDirection;

  /// Limits menu height to the larger space above or below the trigger.
  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    final viewport = insets.deflateRect(Offset.zero & constraints.biggest);
    final above = (anchor.top - verticalOffset - viewport.top).clamp(
      0.0,
      viewport.height,
    );
    final below = (viewport.bottom - anchor.bottom - verticalOffset).clamp(
      0.0,
      viewport.height,
    );
    return BoxConstraints(
      maxWidth: viewport.width,
      maxHeight: math.max(above, below),
    );
  }

  /// Opens below when the menu fits, otherwise opens above the trigger.
  @override
  Offset getPositionForChild(Size size, Size childSize) {
    final viewport = insets.deflateRect(Offset.zero & size);
    final below = anchor.bottom + verticalOffset;
    final left = textDirection == TextDirection.ltr
        ? anchor.left
        : anchor.right - childSize.width;
    final top = below + childSize.height <= viewport.bottom
        ? below
        : anchor.top - verticalOffset - childSize.height;
    return Offset(
      left.clamp(viewport.left, viewport.right - childSize.width),
      top.clamp(viewport.top, viewport.bottom - childSize.height),
    );
  }

  /// Repositions when its anchor, insets, or layout direction change.
  @override
  bool shouldRelayout(_ComposeDropdownPosition oldDelegate) =>
      anchor != oldDelegate.anchor ||
      insets != oldDelegate.insets ||
      verticalOffset != oldDelegate.verticalOffset ||
      textDirection != oldDelegate.textDirection;
}
