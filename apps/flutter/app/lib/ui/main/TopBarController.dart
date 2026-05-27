// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import 'package:flutter/scheduler.dart';

typedef TopBarActionsBuilder = List<Widget> Function(BuildContext context);
typedef TopBarTitleBuilder = Widget Function(BuildContext context);

class TopBarTitleContent {
  const TopBarTitleContent(this.content);

  final TopBarTitleBuilder content;
}

class TopBarController extends ChangeNotifier {
  TopBarActionsBuilder? _actions;
  TopBarTitleContent? _titleContent;
  bool _notificationScheduled = false;
  bool _disposed = false;

  TopBarActionsBuilder? get actions => _actions;
  TopBarTitleContent? get titleContent => _titleContent;

  void setActions(TopBarActionsBuilder actions) {
    _actions = actions;
    _notifySafely();
  }

  void clearActions() {
    _actions = null;
    _notifySafely();
  }

  void setTitleContent(TopBarTitleContent titleContent) {
    _titleContent = titleContent;
    _notifySafely();
  }

  void clearTitleContent() {
    _titleContent = null;
    _notifySafely();
  }

  void clear() {
    _actions = null;
    _titleContent = null;
    _notifySafely();
  }

  void _notifySafely() {
    if (_disposed) {
      return;
    }
    if (SchedulerBinding.instance.schedulerPhase == SchedulerPhase.idle) {
      notifyListeners();
      return;
    }
    if (_notificationScheduled) {
      return;
    }
    _notificationScheduled = true;
    SchedulerBinding.instance.addPostFrameCallback((_) {
      _notificationScheduled = false;
      if (_disposed) {
        return;
      }
      notifyListeners();
    });
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

class TopBarScope extends InheritedWidget {
  const TopBarScope({
    super.key,
    required this.controller,
    required super.child,
  });

  final TopBarController controller;

  static TopBarController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<TopBarScope>();
    if (scope == null) {
      throw StateError('TopBarScope is not installed');
    }
    return scope.controller;
  }

  @override
  bool updateShouldNotify(TopBarScope oldWidget) {
    return controller != oldWidget.controller;
  }
}
