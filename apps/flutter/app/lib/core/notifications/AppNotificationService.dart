// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';

/// Delivers application events through the active system-notification host.
class AppNotificationService with WidgetsBindingObserver {
  AppNotificationService._();

  /// Returns the process-wide application notification service.
  static final AppNotificationService instance = AppNotificationService._();

  static const String _logTag = 'AppNotification';
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  StreamSubscription<RuntimeHostInteractionRequest>? _subscription;
  bool _isForeground = true;

  /// Starts lifecycle tracking and application-notification event delivery.
  void initialize() {
    if (_subscription != null) {
      return;
    }
    _isForeground =
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
    WidgetsBinding.instance.addObserver(this);
    _subscription = _clients.servicesRuntimeHostInteractionService
        .ownerHostInteractionEvents(
          kinds: <RuntimeHostInteractionKind>[
            RuntimeHostInteractionKind.appNotification,
          ],
        )
        .listen(
          (event) => unawaited(_handleRequest(event)),
          onError: (Object error, StackTrace stackTrace) {
            ClientLogger.e(
              'application notification event stream failed',
              tag: _logTag,
              error: error,
              stackTrace: stackTrace,
            );
          },
        );
    ClientLogger.i('application notification service started', tag: _logTag);
  }

  /// Updates foreground state used to decide whether system notification is needed.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _isForeground = state == AppLifecycleState.resumed;
  }

  /// Routes one notification-related owner-host request.
  Future<void> _handleRequest(RuntimeHostInteractionRequest request) async {
    switch (request.kind) {
      case RuntimeHostInteractionKind.appNotification:
        await _handleAppNotification(request);
      case RuntimeHostInteractionKind.browserAutomation:
      case RuntimeHostInteractionKind.browserSession:
      case RuntimeHostInteractionKind.webVisit:
      case RuntimeHostInteractionKind.composeWebViewController:
      case RuntimeHostInteractionKind.composeFilePicker:
      case RuntimeHostInteractionKind.composeFileSave:
      case RuntimeHostInteractionKind.systemCaptureScreenshot:
      case RuntimeHostInteractionKind.systemLanguageCode:
      case RuntimeHostInteractionKind.systemRecognizeText:
      case RuntimeHostInteractionKind.systemOperation:
      case RuntimeHostInteractionKind.fileOpen:
      case RuntimeHostInteractionKind.fileShare:
      case RuntimeHostInteractionKind.audioPlay:
      case RuntimeHostInteractionKind.musicPlayback:
      case RuntimeHostInteractionKind.bluetooth:
      case RuntimeHostInteractionKind.ttsSynthesis:
      case RuntimeHostInteractionKind.ttsPlayback:
      case RuntimeHostInteractionKind.localInference:
        throw StateError(
          'unexpected application notification kind: ${request.kind.name}',
        );
    }
  }

  /// Delivers one non-blocking application notification and acknowledges its event.
  Future<void> _handleAppNotification(
    RuntimeHostInteractionRequest request,
  ) async {
    try {
      final payload = request.appNotification;
      if (payload == null) {
        throw StateError('application notification payload is missing');
      }
      await _sendWhenBackground(
        payload.title,
        payload.message,
        chatId: payload.chatId,
        messageTimestamp: payload.messageTimestamp,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'application notification delivery failed requestId=${request.requestId}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    } finally {
      await _acknowledge(request.requestId);
    }
  }

  /// Sends one system notification only while this application is not foregrounded.
  Future<void> _sendWhenBackground(
    String title,
    String message, {
    required String? chatId,
    int? messageTimestamp,
  }) async {
    final nowMillis = DateTime.now().millisecondsSinceEpoch;
    final appForeground = await _isApplicationForeground();
    final deliver = shouldDeliverAppNotification(
      appForeground: appForeground,
      messageTimestamp: messageTimestamp,
      nowMillis: nowMillis,
    );
    if (!deliver) {
      ClientLogger.v(
        'notification suppressed applicationForeground=true',
        tag: _logTag,
        verboseLevel: VerboseLevel.level5,
      );
      return;
    }
    final ageMillis =
        messageTimestamp == null ? -1 : nowMillis - messageTimestamp;
    ClientLogger.i(
      'notification delivered foreground=$appForeground ageMs=$ageMillis '
      'chatId=${chatId ?? '-'}',
      tag: _logTag,
    );
    await _clients.servicesRuntimeHostInteractionService.sendSystemNotification(
      title: title,
      message: message,
      chatId: chatId,
    );
  }

  /// Returns whether the application currently owns the user's foreground window.
  Future<bool> _isApplicationForeground() async {
    if (!kIsWeb && defaultTargetPlatform == TargetPlatform.windows) {
      final isForeground = await _runtimeChannel.invokeMethod<bool>(
        'applicationIsForeground',
      );
      if (isForeground == null) {
        throw StateError('application foreground response is missing');
      }
      return isForeground;
    }
    return _isForeground;
  }

  /// Removes one consumed non-blocking application notification event.
  Future<void> _acknowledge(String requestId) async {
    try {
      await _clients.servicesRuntimeHostInteractionService
          .acknowledgeOwnerHostInteraction(requestId: requestId);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'application notification acknowledgement failed requestId=$requestId',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    }
  }
}

/// Replayed notifications older than this are treated as produced while away.
const Duration appNotificationAwayReplayThreshold = Duration(seconds: 5);

/// Decides whether one runtime application notification should reach the tray.
///
/// Runtime notifications are published into the host-interaction broker and are
/// only delivered to Dart once a subscriber exists again. A reply that finished
/// while the application was backgrounded therefore arrives after the user has
/// already returned, so a pure foreground check would silently drop it. A
/// notification that is old enough to predate the current foreground session is
/// delivered regardless of the current visibility.
bool shouldDeliverAppNotification({
  required bool appForeground,
  required int? messageTimestamp,
  required int nowMillis,
  Duration awayReplayThreshold = appNotificationAwayReplayThreshold,
}) {
  if (!appForeground) {
    return true;
  }
  if (messageTimestamp == null) {
    return false;
  }
  final ageMillis = nowMillis - messageTimestamp;
  return ageMillis > awayReplayThreshold.inMilliseconds;
}
