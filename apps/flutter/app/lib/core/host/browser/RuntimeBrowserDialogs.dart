// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

/// Coordinates page dialogs and their manual or automated response per session.
class RuntimeBrowserDialogs {
  _PendingDialog? _pending;
  final Set<Completer<String>> _observers = <Completer<String>>{};

  /// Presents one dialog until either the user or automation resolves it.
  Future<Object?> open({
    required String type,
    required String message,
    required String url,
    String? defaultText,
    required Future<Object?> Function(Future<void> dismissed) present,
  }) async {
    if (_pending != null) {
      throw StateError('A browser dialog is already pending');
    }
    final dialog = _PendingDialog(type, message, url, defaultText);
    _pending = dialog;
    for (final observer in _observers.toList()) {
      if (!observer.isCompleted) observer.complete(dialog.description);
    }
    unawaited(
      Future<Object?>.sync(() => present(dialog.dismissed.future)).then(
        (value) {
          if (!dialog.result.isCompleted) dialog.result.complete(value);
        },
        onError: (Object error, StackTrace stack) {
          if (!dialog.result.isCompleted) {
            dialog.result.completeError(error, stack);
          }
        },
      ),
    );
    try {
      return await dialog.result.future;
    } finally {
      if (identical(_pending, dialog)) _pending = null;
      if (!dialog.dismissed.isCompleted) dialog.dismissed.complete();
    }
  }

  /// Inspects or resolves the current dialog using the public action contract.
  String dialog({required String action, String? promptText}) {
    if (action != 'get' && action != 'accept' && action != 'dismiss') {
      throw ArgumentError.value(
        action,
        'action',
        'Expected get, accept, or dismiss',
      );
    }
    if (promptText != null && action != 'accept') {
      throw ArgumentError('promptText requires action accept');
    }
    if (action != 'get') {
      final pending = _pending;
      if (pending == null || pending.result.isCompleted) {
        throw StateError('No active browser dialog');
      }
      if (promptText != null && pending.type != 'prompt') {
        throw ArgumentError('promptText requires a prompt dialog');
      }
      _resolve(accept: action == 'accept', promptText: promptText);
    }
    return jsonEncode(<String, Object?>{'dialog': _pending?.metadata});
  }

  /// Responds to the active dialog, preserving rejection and prompt text.
  void _resolve({required bool accept, String? promptText}) {
    final dialog = _pending;
    if (dialog == null || dialog.result.isCompleted) {
      throw StateError('No active browser dialog');
    }
    dialog.result.complete(switch (dialog.type) {
      'alert' => null,
      'confirm' => accept,
      'prompt' => accept ? promptText ?? dialog.defaultText ?? '' : null,
      _ => throw StateError('Unknown browser dialog type: ${dialog.type}'),
    });
    _pending = null;
    if (!dialog.dismissed.isCompleted) dialog.dismissed.complete();
  }

  /// Returns a pending operation early when it opens a page dialog.
  Future<String> observe(Future<String> Function() action) async {
    final existing = _pending;
    if (existing != null) return existing.description;
    final observer = Completer<String>();
    _observers.add(observer);
    try {
      return await Future.any(<Future<String>>[action(), observer.future]);
    } finally {
      _observers.remove(observer);
    }
  }

  /// Releases a pending dialog when its browser session closes.
  void close() {
    final dialog = _pending;
    if (dialog != null && !dialog.result.isCompleted) {
      dialog.result.completeError(StateError('Browser session closed'));
    }
  }
}

class _PendingDialog {
  /// Creates the pending response and its owning page metadata.
  _PendingDialog(this.type, this.message, this.url, this.defaultText);
  final String type;
  final String message;
  final String url;
  final String? defaultText;
  final Completer<Object?> result = Completer<Object?>();
  final Completer<void> dismissed = Completer<void>();

  /// Describes the dialog to the automation caller without resolving it.
  String get description => jsonEncode(<String, Object?>{'dialog': metadata});

  /// Exposes the same dialog shape for inspection and interrupted actions.
  Map<String, Object?> get metadata => <String, Object?>{
    'type': type,
    'message': message,
    'url': url,
    'defaultValue': defaultText ?? '',
  };
}
