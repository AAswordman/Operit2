// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/host/AppToastController.dart';

/// Routes every snack bar into the top-right notice stack.
class MacNoticeScaffoldMessenger extends ScaffoldMessenger {
  /// Creates a messenger that presents snack bars as corner notices.
  const MacNoticeScaffoldMessenger({super.key, required super.child});

  /// Creates the state that owns the redirected snack-bar calls.
  @override
  MacNoticeScaffoldMessengerState createState() =>
      MacNoticeScaffoldMessengerState();
}

/// Presents snack bars through [AppToastController] and clears the bottom slot.
class MacNoticeScaffoldMessengerState extends ScaffoldMessengerState {
  /// Publishes the snack bar text as a corner notice and removes the bottom bar.
  @override
  ScaffoldFeatureController<SnackBar, SnackBarClosedReason> showSnackBar(
    SnackBar snackBar, {
    AnimationStyle? snackBarAnimationStyle,
  }) {
    final text = readSnackBarNoticeText(snackBar);
    final selectable = snackBar.content is SelectableText;
    final action = snackBar.action;
    final controller = super.showSnackBar(
      const SnackBar(
        content: SizedBox.shrink(),
        duration: Duration.zero,
        padding: EdgeInsets.zero,
        behavior: SnackBarBehavior.floating,
      ),
      snackBarAnimationStyle: AnimationStyle.noAnimation,
    );
    super.removeCurrentSnackBar();
    AppToastController.instance.show(
      text,
      duration: noticeDurationForSnackBar(text, snackBar.duration),
      source: AppNoticeSource.snackBar,
      selectable: selectable,
      actionLabel: action?.label,
      onAction: action?.onPressed,
    );
    return controller;
  }

  /// Dismisses the newest snack-bar notice.
  @override
  void hideCurrentSnackBar({
    SnackBarClosedReason reason = SnackBarClosedReason.hide,
  }) {
    AppToastController.instance.dismissLatestSnackBar();
  }

  /// Dismisses the newest snack-bar notice without a bottom-bar animation.
  @override
  void removeCurrentSnackBar({
    SnackBarClosedReason reason = SnackBarClosedReason.remove,
  }) {
    AppToastController.instance.dismissLatestSnackBar();
  }

  /// Dismisses every snack-bar notice.
  @override
  void clearSnackBars() {
    AppToastController.instance.clearSnackBars();
  }
}

/// Reads the notice text from a snack bar whose content is a text widget.
String readSnackBarNoticeText(SnackBar snackBar) {
  final text = _readNoticeText(snackBar.content).trim();
  if (text.isEmpty) {
    throw ArgumentError.value(
      snackBar.content.runtimeType,
      'content',
      'SnackBar content has no notice text',
    );
  }
  return text;
}

/// Uses a caller duration when it is not Flutter's four-second snack bar time.
Duration noticeDurationForSnackBar(String text, Duration snackBarDuration) {
  if (snackBarDuration == const Duration(seconds: 4)) {
    return appNoticeReadingDuration(text);
  }
  return snackBarDuration;
}

/// Reads plain text from the snack bar content widget.
String _readNoticeText(Widget widget) {
  return switch (widget) {
    Text(:final data, :final textSpan) => data ?? textSpan!.toPlainText(),
    SelectableText(:final data, :final textSpan) =>
      data ?? textSpan!.toPlainText(),
    RichText(:final text) => text.toPlainText(),
    _ => throw ArgumentError.value(
      widget.runtimeType,
      'content',
      'SnackBar content must be a text widget',
    ),
  };
}
