// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../host/RuntimeHostInteractionSubscriber.dart';
import '../link_host/LinkHostServer.dart';
import '../logging/ClientLogger.dart';
import '../runtime/RuntimeConnectionManager.dart';

class CoreApplicationService {
  CoreApplicationService._();

  static final CoreApplicationService instance = CoreApplicationService._();
  static const String _logTag = 'CoreApplication';
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  final RuntimeConnectionManager _runtimeManager =
      RuntimeConnectionManager.instance;
  final StreamController<Object> _startupErrors =
      StreamController<Object>.broadcast();

  bool _initialized = false;
  bool _hostSubscriberInstalled = false;
  bool _localBackgroundServiceStartAttempted = false;
  bool _linkHostStartAttempted = false;
  Future<void>? _runtimeServicesStart;
  Object? _pendingStartupError;

  Stream<Object> get startupErrors => _startupErrors.stream;

  /// Returns and clears the latest unpresented Core startup error.
  Object? consumeStartupError() {
    final error = _pendingStartupError;
    _pendingStartupError = null;
    return error;
  }

  /// Installs process-level Core lifecycle listeners.
  void initialize() {
    if (_initialized) {
      ClientLogger.d(
        'initialize skipped alreadyInitialized=true',
        tag: _logTag,
      );
      return;
    }
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('initialize start', tag: _logTag);
    _initialized = true;
    _runtimeManager.addListener(_handleRuntimeConnectionChanged);
    unawaited(_startRuntimeServices());
    ClientLogger.i(
      'initialize done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Starts Core services when runtime configuration becomes usable.
  void _handleRuntimeConnectionChanged() {
    ClientLogger.i(
      'runtime connection changed mode=${_runtimeManager.config.mode.name} configured=${_runtimeManager.runtimeConfigured}',
      tag: _logTag,
    );
    unawaited(_startRuntimeServices());
  }

  /// Serializes process-level Core service startup.
  Future<void> _startRuntimeServices() {
    final activeStart = _runtimeServicesStart;
    if (activeStart != null) {
      ClientLogger.d('runtime services start already active', tag: _logTag);
      return activeStart;
    }
    ClientLogger.i('runtime services start scheduled', tag: _logTag);
    final start = _startRuntimeServicesOnce();
    _runtimeServicesStart = start;
    return start.whenComplete(() {
      if (identical(_runtimeServicesStart, start)) {
        _runtimeServicesStart = null;
      }
    });
  }

  /// Starts host subscriptions and LinkHost outside the widget lifecycle.
  Future<void> _startRuntimeServicesOnce() async {
    final stopwatch = Stopwatch()..start();
    if (!_runtimeManager.runtimeConfigured) {
      ClientLogger.i(
        'runtime services waiting for configuration',
        tag: _logTag,
      );
      return;
    }
    try {
      ClientLogger.i(
        'runtime services start mode=${_runtimeManager.config.mode.name} localConfirmed=${_runtimeManager.config.localStorage.confirmed}',
        tag: _logTag,
      );
      await _startLocalBackgroundService();
      if (!_hostSubscriberInstalled) {
        final subscriberStopwatch = Stopwatch()..start();
        ClientLogger.i('host subscriber install start', tag: _logTag);
        RuntimeHostInteractionSubscriber.install();
        _hostSubscriberInstalled = true;
        ClientLogger.i(
          'host subscriber install done elapsedMs=${subscriberStopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
      }
      if (_linkHostStartAttempted ||
          !_runtimeManager.config.localStorage.confirmed) {
        ClientLogger.i(
          'runtime services start done linkHostAttempted=$_linkHostStartAttempted elapsedMs=${stopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
        return;
      }
      _linkHostStartAttempted = true;
      final linkHostStopwatch = Stopwatch()..start();
      ClientLogger.i('link host initialize start', tag: _logTag);
      await LinkHostServer.instance.initializeFromConfig();
      ClientLogger.i(
        'link host initialize done elapsedMs=${linkHostStopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      ClientLogger.i(
        'runtime services start done elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Core application service failed during startup',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      if (_startupErrors.hasListener) {
        _startupErrors.add(error);
      } else {
        _pendingStartupError = error;
      }
    }
  }

  /// Starts the Android foreground Core service for a local runtime.
  Future<void> _startLocalBackgroundService() async {
    if (_localBackgroundServiceStartAttempted ||
        kIsWeb ||
        defaultTargetPlatform != TargetPlatform.android ||
        _runtimeManager.config.mode != RuntimeConnectionMode.local) {
      ClientLogger.d(
        'local background service not started attempted=$_localBackgroundServiceStartAttempted isWeb=$kIsWeb platform=$defaultTargetPlatform mode=${_runtimeManager.config.mode.name}',
        tag: _logTag,
      );
      return;
    }
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('local background service start', tag: _logTag);
    _localBackgroundServiceStartAttempted = true;
    await _runtimeChannel.invokeMethod<void>('startLocalCoreService');
    ClientLogger.i(
      'local background service done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }
}
