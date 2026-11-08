// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

import '../../../main/MainLayoutController.dart';
import '../../../main/TopBarController.dart';

/// Shows a page's title and actions in the shell top bar while it is the
/// current screen.
///
/// The shell clears the bar whenever navigation brings another page on screen,
/// so the page claims it again each time it becomes the current screen and
/// releases it once another screen takes over. A page that is fading out stays
/// active and can still rebuild, so ownership follows the current screen
/// rather than page activity.
class ExtensionTopBar extends StatefulWidget {
  const ExtensionTopBar({
    super.key,
    required this.child,
    this.title,
    this.actions,
  });

  final Widget child;

  /// Replaces the route title; it is built in the shell's context.
  final WidgetBuilder? title;
  final List<Widget> Function(BuildContext context)? actions;

  @override
  State<ExtensionTopBar> createState() => _ExtensionTopBarState();
}

class _ExtensionTopBarState extends State<ExtensionTopBar> {
  TopBarController? _controller;

  /// Synchronizes ownership after inherited controller or page changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _syncOwnership();
  }

  /// Reads current page identity before publishing updated title or actions.
  @override
  void didUpdateWidget(covariant ExtensionTopBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncOwnership();
  }

  /// Releases only the title and actions still owned by this page.
  @override
  void dispose() {
    _release();
    super.dispose();
  }

  /// Resolves inherited state even when widget updates precede dependency updates.
  void _syncOwnership() {
    final controller = TopBarScope.of(context);
    if (!identical(controller, _controller)) {
      _release();
      _controller = controller;
    }
    if (MainScreenActivityScope.isCurrentScreenOf(context)) {
      _claim();
    } else {
      _release();
    }
  }

  /// Publishes the current page's title and actions into the shared shell.
  void _claim() {
    final controller = _controller!;
    final title = widget.title;
    if (title == null) {
      controller.clearTitleContent(owner: this);
    } else {
      controller.setTitleContent(TopBarTitleContent(title), owner: this);
    }
    controller.setActions(
      widget.actions ?? (context) => const <Widget>[],
      owner: this,
    );
  }

  /// Removes this page's chrome without clearing another page's contribution.
  void _release() {
    _controller?.clearActions(owner: this);
    _controller?.clearTitleContent(owner: this);
  }

  /// Leaves page layout with the caller while the shell hosts its chrome.
  @override
  Widget build(BuildContext context) => widget.child;
}
