// ignore_for_file: file_names

import 'dart:async';

typedef RuntimeChannelNotificationActivationHandler =
    Future<void> Function(Object? payload);

/// Dispatches native Runtime-channel events that are independent of Core watches.
class RuntimeChannelInboundGateway {
  RuntimeChannelInboundGateway._();

  static RuntimeChannelNotificationActivationHandler?
  _notificationActivationHandler;

  /// Installs the process-level receiver for native notification activation events.
  static void installNotificationActivationHandler(
    RuntimeChannelNotificationActivationHandler handler,
  ) {
    _notificationActivationHandler = handler;
  }

  /// Dispatches one native notification activation to its registered receiver.
  static Future<void> dispatchNotificationActivation(Object? payload) async {
    final handler = _notificationActivationHandler;
    if (handler == null) {
      throw StateError('notification activation handler is not installed');
    }
    await handler(payload);
  }
}
