// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';

/// Identifies which surface asked for a transient notice.
enum AppNoticeSource {
  /// A toast published by application code.
  application,

  /// A snack bar redirected into the corner notice stack.
  snackBar,
}

/// One transient notice shown at the top-right of the application window.
class AppNotice {
  /// Creates one sequenced notice.
  const AppNotice({
    required this.id,
    required this.text,
    required this.duration,
    required this.source,
    this.selectable = false,
    this.actionLabel,
    this.onAction,
    this.onDismissed,
  });

  final int id;
  final String text;
  final Duration duration;
  final AppNoticeSource source;
  final bool selectable;
  final String? actionLabel;
  final VoidCallback? onAction;
  final VoidCallback? onDismissed;
}

/// Owns the process-wide stack of corner notices.
class AppToastController {
  /// Creates the process-wide notice controller.
  AppToastController._();

  static final AppToastController instance = AppToastController._();

  static const int _maxVisibleNotices = 4;

  final ValueNotifier<List<AppNotice>> _notices =
      ValueNotifier<List<AppNotice>>(const <AppNotice>[]);
  final Map<int, Timer> _timers = <int, Timer>{};
  final Map<int, DateTime> _deadlines = <int, DateTime>{};
  final Map<int, Duration> _pausedRemaining = <int, Duration>{};
  List<AppNotice> _items = const <AppNotice>[];
  int _nextId = 0;

  /// Exposes the visible notices, newest first.
  ValueListenable<List<AppNotice>> get notices => _notices;

  /// Publishes one notice at the top of the stack.
  int show(
    String text, {
    Duration? duration,
    AppNoticeSource source = AppNoticeSource.application,
    bool selectable = false,
    String? actionLabel,
    VoidCallback? onAction,
    VoidCallback? onDismissed,
  }) {
    final normalized = text.trim();
    if (normalized.isEmpty) {
      throw ArgumentError.value(text, 'text', 'notice text must not be empty');
    }
    if ((actionLabel == null) != (onAction == null)) {
      throw ArgumentError('notice action label and callback must be paired');
    }
    final notice = AppNotice(
      id: ++_nextId,
      text: normalized,
      duration: duration ?? appNoticeReadingDuration(normalized),
      source: source,
      selectable: selectable,
      actionLabel: actionLabel,
      onAction: onAction,
      onDismissed: onDismissed,
    );
    _items = <AppNotice>[notice, ..._items];
    _arm(notice);
    _publish();
    while (_items.length > _maxVisibleNotices) {
      dismiss(_items.last.id);
    }
    return notice.id;
  }

  /// Dismisses one notice when it is still present.
  void dismiss(int id) {
    AppNotice? notice;
    for (final item in _items) {
      if (item.id == id) {
        notice = item;
        break;
      }
    }
    if (notice == null) {
      return;
    }
    _cancelSchedule(id);
    _items = List<AppNotice>.of(_items)..remove(notice);
    _publish();
    notice.onDismissed?.call();
  }

  /// Dismisses the newest snack-bar notice.
  void dismissLatestSnackBar() {
    for (final notice in _items) {
      if (notice.source == AppNoticeSource.snackBar) {
        dismiss(notice.id);
        return;
      }
    }
  }

  /// Dismisses every snack-bar notice and leaves application notices in place.
  void clearSnackBars() {
    final ids = _items
        .where((notice) => notice.source == AppNoticeSource.snackBar)
        .map((notice) => notice.id)
        .toList(growable: false);
    for (final id in ids) {
      dismiss(id);
    }
  }

  /// Freezes the remaining lifetime of one notice while the pointer is over it.
  void pause(int id) {
    final timer = _timers.remove(id);
    final deadline = _deadlines.remove(id);
    if (timer == null || deadline == null) {
      return;
    }
    timer.cancel();
    _pausedRemaining[id] = deadline.difference(DateTime.now());
  }

  /// Continues a paused notice, or dismisses it when its time already elapsed.
  void resume(int id) {
    final remaining = _pausedRemaining.remove(id);
    if (remaining == null || !_contains(id)) {
      return;
    }
    if (remaining <= Duration.zero) {
      dismiss(id);
      return;
    }
    _deadlines[id] = DateTime.now().add(remaining);
    _timers[id] = Timer(remaining, () => dismiss(id));
  }

  /// Drops every notice and timer without running dismissal callbacks.
  @visibleForTesting
  void reset() {
    for (final timer in _timers.values) {
      timer.cancel();
    }
    _timers.clear();
    _deadlines.clear();
    _pausedRemaining.clear();
    _items = const <AppNotice>[];
    _nextId = 0;
    _notices.value = const <AppNotice>[];
  }

  /// Starts the auto-dismiss timer for one newly published notice.
  void _arm(AppNotice notice) {
    _cancelSchedule(notice.id);
    _deadlines[notice.id] = DateTime.now().add(notice.duration);
    _timers[notice.id] = Timer(notice.duration, () => dismiss(notice.id));
  }

  /// Cancels the timer and any paused remainder for one notice.
  void _cancelSchedule(int id) {
    _timers.remove(id)?.cancel();
    _deadlines.remove(id);
    _pausedRemaining.remove(id);
  }

  /// Reports whether the stack still contains one notice id.
  bool _contains(int id) {
    for (final notice in _items) {
      if (notice.id == id) {
        return true;
      }
    }
    return false;
  }

  /// Publishes an immutable snapshot of the current stack.
  void _publish() {
    _notices.value = List<AppNotice>.unmodifiable(_items);
  }
}

/// Computes a readable display duration from the notice length.
Duration appNoticeReadingDuration(String message) {
  final estimatedLines = message
      .split('\n')
      .map((line) => math.max(1, ((line.length + 23) / 24).floor()))
      .fold<int>(0, (sum, lines) => sum + lines);
  final milliseconds = 2500 + estimatedLines * 850;
  final clamped = milliseconds < 3500
      ? 3500
      : (milliseconds > 12000 ? 12000 : milliseconds);
  return Duration(milliseconds: clamped);
}
