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
  bool _isCurrentScreen = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _controller = TopBarScope.of(context);
    final isCurrentScreen = MainScreenActivityScope.isCurrentScreenOf(context);
    if (isCurrentScreen == _isCurrentScreen) {
      return;
    }
    _isCurrentScreen = isCurrentScreen;
    if (isCurrentScreen) {
      _claim();
    } else {
      _release();
    }
  }

  @override
  void didUpdateWidget(covariant ExtensionTopBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_isCurrentScreen) {
      _claim();
    }
  }

  @override
  void dispose() {
    _release();
    super.dispose();
  }

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

  void _release() {
    _controller?.clearActions(owner: this);
    _controller?.clearTitleContent(owner: this);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
