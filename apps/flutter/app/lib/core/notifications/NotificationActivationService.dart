// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../bridge/MethodChannelCoreProxy.dart';
import '../logging/ClientLogger.dart';
import '../runtime/RuntimeChannelInboundGateway.dart';

/// Delivers notification activation requests after the main application UI is ready.
class NotificationActivationService {
  NotificationActivationService._();

  /// Returns the process-wide notification activation service.
  static final NotificationActivationService instance =
      NotificationActivationService._();

  static const String _logTag = 'NotificationActivation';
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  final List<String> _queuedChatIds = <String>[];
  Future<void> Function(String chatId)? _chatHandler;
  Future<void> _deliveryTail = Future<void>.value();
  bool _initialized = false;

  /// Installs the native activation receiver and consumes startup activation data.
  void initialize(List<String> arguments) {
    if (_initialized) {
      return;
    }
    _initialized = true;
    installRuntimeChannelInboundDispatch();
    RuntimeChannelInboundGateway.installNotificationActivationHandler(receive);
    _receiveStartupArguments(arguments);
    if (kIsWeb) {
      _receiveWebStartupActivation();
      return;
    }
    if (_hasNativeActivationChannel) {
      unawaited(_receiveNativeStartupActivation());
    }
  }

  /// Sets the main-screen callback that opens the chat selected by an activation.
  void installChatHandler(Future<void> Function(String chatId) handler) {
    _chatHandler = handler;
    _scheduleDelivery();
  }

  /// Removes the main-screen callback before its widget tree is disposed.
  void clearChatHandler() {
    _chatHandler = null;
  }

  /// Receives one activation emitted by a native notification click.
  Future<void> receive(Object? payload) async {
    final activation = _activationMap(payload);
    switch (activation['type']) {
      case 'open_application':
        return;
      case 'open_chat':
        final chatId = activation['chatId'];
        if (chatId is! String || chatId.isEmpty) {
          throw StateError('open_chat activation requires a non-empty chatId');
        }
        _queuedChatIds.add(chatId);
        _scheduleDelivery();
        return;
      default:
        throw StateError(
          'unsupported notification activation type: ${activation['type']}',
        );
    }
  }

  /// Returns whether this platform exposes native notification activation state.
  bool get _hasNativeActivationChannel {
    final platform = defaultTargetPlatform;
    return platform == TargetPlatform.android ||
        platform == TargetPlatform.iOS ||
        platform == TargetPlatform.windows ||
        platform.name == 'ohos';
  }

  /// Queues an activation encoded in the process startup command line.
  void _receiveStartupArguments(List<String> arguments) {
    for (final argument in arguments) {
      final uri = Uri.tryParse(argument);
      if (uri == null || uri.scheme != 'operit2') {
        continue;
      }
      final activation = switch (uri.host) {
        'notification' when uri.path == '/open-app' => const <String, Object?>{
          'type': 'open_application',
        },
        'notification' when uri.path == '/open-chat' => <String, Object?>{
          'type': 'open_chat',
          'chatId': uri.queryParameters['chatId'],
        },
        _ => throw StateError('unsupported notification activation URI: $uri'),
      };
      unawaited(receive(activation));
    }
  }

  /// Queues a Web activation encoded in the current browser URL.
  void _receiveWebStartupActivation() {
    final encodedActivation =
        Uri.base.queryParameters['operitNotificationActivation'];
    if (encodedActivation == null) {
      return;
    }
    unawaited(receive(encodedActivation));
  }

  /// Reads the queued native activation supplied during application startup.
  Future<void> _receiveNativeStartupActivation() async {
    final activation = await _runtimeChannel.invokeMethod<Object?>(
      'notificationActivationInitial',
    );
    if (activation == null) {
      await _runtimeChannel.invokeMethod<void>('notificationActivationReady');
      return;
    }
    await receive(activation);
    await _runtimeChannel.invokeMethod<void>('notificationActivationReady');
  }

  /// Converts the transport payload into the notification activation schema.
  Map<String, Object?> _activationMap(Object? payload) {
    final Object? decoded = switch (payload) {
      String value => jsonDecode(value),
      _ => payload,
    };
    if (decoded is! Map<Object?, Object?>) {
      throw StateError('notification activation must be a JSON object');
    }
    return <String, Object?>{
      for (final entry in decoded.entries)
        if (entry.key is String) entry.key as String: entry.value,
    };
  }

  /// Serializes queued chat activations through the current main-screen callback.
  void _scheduleDelivery() {
    _deliveryTail = _deliveryTail.then((_) async {
      final handler = _chatHandler;
      if (handler == null) {
        return;
      }
      while (_queuedChatIds.isNotEmpty && identical(_chatHandler, handler)) {
        final chatId = _queuedChatIds.removeAt(0);
        try {
          await handler(chatId);
        } catch (error, stackTrace) {
          ClientLogger.e(
            'notification chat activation failed chatId=$chatId',
            tag: _logTag,
            error: error,
            stackTrace: stackTrace,
          );
        }
      }
    });
  }
}
