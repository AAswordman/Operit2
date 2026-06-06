// ignore_for_file: file_names

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

typedef MainLayoutAttachmentBuilder =
    Widget Function(BuildContext context, Widget child);

class MainLayoutController extends ChangeNotifier {
  MainLayoutAttachmentBuilder? _attachmentBuilder;
  Object? _attachmentOwner;
  bool _notificationScheduled = false;
  bool _disposed = false;

  MainLayoutAttachmentBuilder? get attachmentBuilder => _attachmentBuilder;

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

  void clearAttachment({required Object owner}) {
    if (!identical(owner, _attachmentOwner)) {
      return;
    }
    clear();
  }

  void clear() {
    _attachmentBuilder = null;
    _attachmentOwner = null;
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
