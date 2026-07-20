// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../host/RuntimeHostInteractionSubscriber.dart';
import '../link_access/LinkAccessHost.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../runtime/RuntimeAutoSyncManager.dart';
import '../runtime/RuntimeConnectionManager.dart';

class CoreApplicationService with WidgetsBindingObserver {
  CoreApplicationService._();

  static final CoreApplicationService instance = CoreApplicationService._();
  static const String _logTag = 'CoreApplication';
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');
  static const GeneratedCoreProxyClients _coreClients =
      GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final RuntimeConnectionManager _runtimeManager =
      RuntimeConnectionManager.instance;
  final RuntimeAutoSyncManager _autoSyncManager =
      RuntimeAutoSyncManager.instance;
  final StreamController<Object> _startupErrors =
      StreamController<Object>.broadcast();

  bool _initialized = false;
  bool _hostSubscriberInstalled = false;
  bool _localBackgroundServiceStartAttempted = false;
  bool _linkHostStartAttempted = false;
  Future<void>? _runtimeServicesStart;
  Future<void>? _debugRuntimeRebuild;
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
    WidgetsBinding.instance.addObserver(this);
    _runtimeManager.addListener(_handleRuntimeConnectionChanged);
    unawaited(_startRuntimeServices());
    ClientLogger.i(
      'initialize done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Forwards Flutter lifecycle changes through the normalized runtime event ingress.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    unawaited(_emitLifecycleEvent(state));
  }

  /// Emits one lifecycle event after the selected runtime becomes callable.
  Future<void> _emitLifecycleEvent(AppLifecycleState state) async {
    if (!_runtimeManager.runtimeConfigured) {
      return;
    }
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    final topic = switch (state) {
      AppLifecycleState.resumed => RuntimeEventTopic.appLifecycleResumed,
      AppLifecycleState.inactive => RuntimeEventTopic.appLifecycleInactive,
      AppLifecycleState.paused => RuntimeEventTopic.appLifecyclePaused,
      AppLifecycleState.detached => RuntimeEventTopic.appLifecycleDetached,
      AppLifecycleState.hidden => RuntimeEventTopic.appLifecycleHidden,
    };
    try {
      await _coreClients.application.ingestRuntimeEvent(
        event: RuntimeEvent(
          domain: RuntimeEventDomain.host,
          source: RuntimeEventSource.flutterLifecycle,
          topic: topic,
          platform: _runtimeEventPlatform,
          payload: <String, Object?>{'state': state.name},
          occurredAtMillis: timestamp,
        ),
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'lifecycle event delivery failed state=${state.name}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  /// Returns the stable runtime platform identifier for this Flutter process.
  RuntimeEventPlatform get _runtimeEventPlatform {
    if (kIsWeb) {
      return RuntimeEventPlatform.web;
    }
    final platform = defaultTargetPlatform;
    if (platform.name == 'ohos') {
      return RuntimeEventPlatform.ohos;
    }
    if (platform == TargetPlatform.android) {
      return RuntimeEventPlatform.android;
    }
    if (platform == TargetPlatform.iOS) {
      return RuntimeEventPlatform.ios;
    }
    if (platform == TargetPlatform.linux) {
      return RuntimeEventPlatform.linux;
    }
    if (platform == TargetPlatform.macOS) {
      return RuntimeEventPlatform.macos;
    }
    if (platform == TargetPlatform.windows) {
      return RuntimeEventPlatform.windows;
    }
    throw UnsupportedError(
      'Runtime events are not supported on ${platform.name}',
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
      await _syncHostSubscriber();
      if (!_runtimeManager.config.localStorage.confirmed) {
        ClientLogger.i(
          'runtime services start done localStorageConfirmed=false elapsedMs=${stopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
        return;
      }
      await _ensureLinkHostStarted();
      await _autoSyncManager.initialize();
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

  /// Starts LinkHost once the local runtime storage is confirmed.
  Future<void> _ensureLinkHostStarted() async {
    if (_linkHostStartAttempted) {
      ClientLogger.d(
        'link host initialize skipped attempted=true',
        tag: _logTag,
      );
      return;
    }
    _linkHostStartAttempted = true;
    final linkHostStopwatch = Stopwatch()..start();
    ClientLogger.i('link host initialize start', tag: _logTag);
    await LinkAccessHost.instance.initializeFromConfig();
    ClientLogger.i(
      'link host initialize done elapsedMs=${linkHostStopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
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

  /// Rebuilds the Windows Debug Rust runtime and restores local host services.
  Future<void> rebuildAndRestartLocalRuntimeForDebug() {
    final activeRebuild = _debugRuntimeRebuild;
    if (activeRebuild != null) {
      return activeRebuild;
    }
    final rebuild = _rebuildAndRestartLocalRuntimeForDebugOnce();
    _debugRuntimeRebuild = rebuild;
    return rebuild.whenComplete(() {
      if (identical(_debugRuntimeRebuild, rebuild)) {
        _debugRuntimeRebuild = null;
      }
    });
  }

  /// Performs one Windows Debug Rust runtime rebuild and host service restore.
  Future<void> _rebuildAndRestartLocalRuntimeForDebugOnce() async {
    if (!kDebugMode ||
        kIsWeb ||
        defaultTargetPlatform != TargetPlatform.windows) {
      throw UnsupportedError(
        'Debug Rust runtime rebuild is only available on Windows Debug builds',
      );
    }
    if (_runtimeManager.config.mode != RuntimeConnectionMode.local) {
      throw StateError('Debug Rust runtime rebuild requires the local runtime');
    }
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('Debug Rust runtime rebuild start', tag: _logTag);
    await RuntimeHostInteractionSubscriber.uninstall();
    _hostSubscriberInstalled = false;
    await LocalRuntimeStorageBridge.rebuildAndRestartForDebug();
    if (_runtimeManager.config.localStorage.confirmed) {
      await LinkAccessHost.instance.initializeFromConfig();
    }
    await _syncHostSubscriber();
    ClientLogger.i(
      'Debug Rust runtime rebuild done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Synchronizes owner-host event handling with the selected runtime role.
  Future<void> _syncHostSubscriber() async {
    final shouldInstall = _ownsHostInteractions;
    if (shouldInstall && !_hostSubscriberInstalled) {
      final subscriberStopwatch = Stopwatch()..start();
      ClientLogger.i('host subscriber install start', tag: _logTag);
      RuntimeHostInteractionSubscriber.install();
      _hostSubscriberInstalled = true;
      ClientLogger.i(
        'host subscriber install done elapsedMs=${subscriberStopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      return;
    }
    if (!shouldInstall && _hostSubscriberInstalled) {
      final subscriberStopwatch = Stopwatch()..start();
      ClientLogger.i('host subscriber uninstall start', tag: _logTag);
      await RuntimeHostInteractionSubscriber.uninstall();
      _hostSubscriberInstalled = false;
      ClientLogger.i(
        'host subscriber uninstall done elapsedMs=${subscriberStopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
    }
  }

  /// Returns whether this process owns native host interaction handling.
  bool get _ownsHostInteractions {
    return !kIsWeb &&
        _runtimeManager.config.mode == RuntimeConnectionMode.local;
  }
}
