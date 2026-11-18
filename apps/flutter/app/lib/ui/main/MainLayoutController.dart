// ignore_for_file: file_names

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

typedef MainLayoutAttachmentBuilder =
    Widget Function(BuildContext context, Widget child);

class MainLayoutController extends ChangeNotifier {
  MainLayoutAttachmentBuilder? _attachmentBuilder;
  Object? _attachmentOwner;
  VoidCallback? _backAction;
  Object? _backActionOwner;
  bool _notificationScheduled = false;
  bool _disposed = false;

  MainLayoutAttachmentBuilder? get attachmentBuilder => _attachmentBuilder;

  /// Reports whether the current page has a surface to dismiss before leaving.
  bool get hasBackAction => _backAction != null;

  /// Publishes the current page's dismiss action with explicit ownership.
  void setBackAction(VoidCallback action, {required Object owner}) {
    if (identical(owner, _backActionOwner) && identical(action, _backAction)) {
      return;
    }
    _backAction = action;
    _backActionOwner = owner;
    _notifySafely();
  }

  /// Removes an action only while it still belongs to the requesting surface.
  void clearBackAction({required Object owner}) {
    if (!identical(owner, _backActionOwner)) {
      return;
    }
    _backAction = null;
    _backActionOwner = null;
    _notifySafely();
  }

  /// Consumes back navigation when the current page owns a dismiss action.
  bool handleBack() {
    final action = _backAction;
    if (action == null) {
      return false;
    }
    action();
    return true;
  }

  void setAttachment(
    MainLayoutAttachmentBuilder builder, {
    required Object owner,
  }) {
    if (identical(owner, _attachmentOwner) &&
        identical(builder, _attachmentBuilder)) {
      return;
    }
    _attachmentBuilder = builder;
    _attachmentOwner = owner;
    _notifySafely();
  }

  void refreshAttachment({required Object owner}) {
    if (!identical(owner, _attachmentOwner)) {
      return;
    }
    _notifySafely();
  }

  /// Removes only the layout decoration owned by the requesting page.
  void clearAttachment({required Object owner}) {
    if (!identical(owner, _attachmentOwner)) {
      return;
    }
    _attachmentBuilder = null;
    _attachmentOwner = null;
    _notifySafely();
  }

  /// Releases all page-owned layout state when the current page changes.
  void clear() {
    _attachmentBuilder = null;
    _attachmentOwner = null;
    _backAction = null;
    _backActionOwner = null;
    _notifySafely();
  }

  Widget decorate(BuildContext context, Widget child) {
    final builder = _attachmentBuilder;
    if (builder == null) {
      return child;
    }
    return builder(context, child);
  }

  void _notifySafely() {
    if (_disposed) {
      return;
    }
    if (_notificationScheduled) {
      return;
    }
    _notificationScheduled = true;
    final scheduler = SchedulerBinding.instance;
    if (scheduler.schedulerPhase == SchedulerPhase.idle) {
      scheduler.scheduleFrameCallback(_notifyAfterFrame);
    } else {
      scheduler.addPostFrameCallback(_notifyAfterFrame);
    }
  }

  void _notifyAfterFrame(Duration _) {
    _notificationScheduled = false;
    if (_disposed) {
      return;
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

/// Registers a visible page surface with the main shell's back handling.
class MainLayoutBackAction extends StatefulWidget {
  /// Creates a dismiss action whose lifetime follows its mounted surface.
  const MainLayoutBackAction({
    super.key,
    required this.enabled,
    required this.onBack,
    required this.child,
  });

  final bool enabled;
  final VoidCallback onBack;
  final Widget child;

  /// Creates the owner that registers and releases the surface's action.
  @override
  State<MainLayoutBackAction> createState() => _MainLayoutBackActionState();
}

class _MainLayoutBackActionState extends State<MainLayoutBackAction> {
  late final VoidCallback _backAction = _handleBack;
  MainLayoutController? _controller;
  bool _isCurrentScreen = false;

  /// Tracks both the main layout owner and retained page visibility.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final controller = MainLayoutScope.of(context);
    if (!identical(controller, _controller)) {
      _controller?.clearBackAction(owner: this);
      _controller = controller;
    }
    _isCurrentScreen = MainScreenActivityScope.isCurrentScreenOf(context);
    _syncAction();
  }

  /// Updates the action when the surface opens, closes, or changes its callback.
  @override
  void didUpdateWidget(covariant MainLayoutBackAction oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncAction();
  }

  /// Exposes dismiss actions only for visible surfaces on the current page.
  void _syncAction() {
    if (widget.enabled && _isCurrentScreen) {
      _controller!.setBackAction(_backAction, owner: this);
    } else {
      _controller!.clearBackAction(owner: this);
    }
  }

  /// Consumes the registration before the surface publishes its closed state.
  void _handleBack() {
    _controller!.clearBackAction(owner: this);
    widget.onBack();
  }

  /// Releases the action without clearing a newer page's registration.
  @override
  void dispose() {
    _controller?.clearBackAction(owner: this);
    super.dispose();
  }

  /// Leaves layout and rendering with the surface that owns the action.
  @override
  Widget build(BuildContext context) => widget.child;
}

class MainLayoutScope extends InheritedWidget {
  const MainLayoutScope({
    super.key,
    required this.controller,
    required super.child,
  });

  final MainLayoutController controller;

  static MainLayoutController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<MainLayoutScope>();
    if (scope == null) {
      throw StateError('MainLayoutScope is not installed');
    }
    return scope.controller;
  }

  @override
  bool updateShouldNotify(MainLayoutScope oldWidget) {
    return controller != oldWidget.controller;
  }
}

class MainScreenActivityScope extends InheritedWidget {
  const MainScreenActivityScope({
    super.key,
    required this.isCurrentScreen,
    required super.child,
  });

  final bool isCurrentScreen;

  static bool isCurrentScreenOf(BuildContext context) {
    return context
            .dependOnInheritedWidgetOfExactType<MainScreenActivityScope>()
            ?.isCurrentScreen ??
        true;
  }

  @override
  bool updateShouldNotify(MainScreenActivityScope oldWidget) {
    return isCurrentScreen != oldWidget.isCurrentScreen;
  }
}
