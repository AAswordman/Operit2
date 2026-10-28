// ignore_for_file: file_names

import 'dart:async';
import 'PluginHotReload.dart';
import 'WorkActivityPresentationService.dart';

import 'package:flutter/foundation.dart';
import 'package:operit_keep_alive/operit_keep_alive.dart';
import 'package:flutter/widgets.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../host/RuntimeHostInteractionSubscriber.dart';
import '../logging/ClientLogger.dart';
import '../notifications/AppNotificationService.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../runtime/RuntimeBootstrapManager.dart';
import '../runtime/RuntimeDeviceInfoProvider.dart';

class CoreApplicationService with WidgetsBindingObserver {
  CoreApplicationService._();

  static final CoreApplicationService instance = CoreApplicationService._();
  static const String _logTag = 'CoreApplication';
  static const GeneratedCoreProxyClients _coreClients =
      GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final RuntimeBootstrapManager _runtimeManager =
      RuntimeBootstrapManager.instance;
  final StreamController<Object> _startupErrors =
      StreamController<Object>.broadcast();

  bool _initialized = false;
  AppLifecycleState? _lastEmittedLifecycleState;
  KeepAliveLease? _coreResidency;
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
      ClientLogger.v(
        'initialize skipped alreadyInitialized=true',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      return;
    }
    final stopwatch = Stopwatch()..start();
    ClientLogger.v(
      'initialize start',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    _initialized = true;
    OperitKeepAlive.instance.report.addListener(_handleResidencyChanged);
    PluginHotReload.register(_coreClients);
    WidgetsBinding.instance.addObserver(this);
    _runtimeManager.addListener(_handleRuntimeBootstrapChanged);
    unawaited(_startRuntimeServices());
    ClientLogger.v(
      'initialize done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
  }

  /// Forwards Flutter lifecycle changes through the normalized runtime event ingress.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed &&
        _runtimeManager.runtimeConfigured) {
      unawaited(_syncHostSubscriber());
      unawaited(WorkActivityPresentationService.instance.refreshCapabilities());
    }
    unawaited(_emitLifecycleEvent(state));
  }

  /// Emits one lifecycle event after the selected runtime becomes callable.
  Future<void> _emitLifecycleEvent(AppLifecycleState state) async {
    if (!_runtimeManager.runtimeConfigured ||
        _lastEmittedLifecycleState == state) {
      return;
    }
    _lastEmittedLifecycleState = state;
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
  void _handleRuntimeBootstrapChanged() {
    ClientLogger.i(
      'runtime bootstrap changed configured=${_runtimeManager.runtimeConfigured}',
      tag: _logTag,
    );
    unawaited(_startRuntimeServices());
  }

  /// Serializes process-level Core service startup.
  Future<void> _startRuntimeServices() {
    final activeStart = _runtimeServicesStart;
    if (activeStart != null) {
      ClientLogger.v(
        'runtime services start already active',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      return activeStart;
    }
    ClientLogger.v(
      'runtime services start scheduled',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    final start = _startRuntimeServicesOnce();
    _runtimeServicesStart = start;
    return start.whenComplete(() {
      if (identical(_runtimeServicesStart, start)) {
        _runtimeServicesStart = null;
      }
    });
  }

  /// Starts host subscriptions and configured node listeners outside the widget lifecycle.
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
      ClientLogger.attachPersistentStorage();
      ClientLogger.v(
        'runtime services start localConfirmed=${_runtimeManager.config.confirmed}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      await _retainCoreRuntime();
      await _syncHostSubscriber();
      AppNotificationService.instance.initialize();
      await WorkActivityPresentationService.instance.initialize();
      if (!_runtimeManager.config.confirmed) {
        ClientLogger.i(
          'runtime services start done localStorageConfirmed=false elapsedMs=${stopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
        return;
      }
      final deviceInfo = await RuntimeDeviceInfoProvider.current();
      Future<void> spaceStep(String operation, Future<void> Function() run) async {
        try {
          await run();
        } catch (error, stackTrace) {
          ClientLogger.e(
            operation,
            tag: _logTag,
            error: error,
            stackTrace: stackTrace,
          );
        }
      }
      await spaceStep('Initialize Space device presentation', () async {
        await _coreClients.server.runtimeRemoteLinkService.initializeDeviceInfo(
          supplied: deviceInfo,
        );
      });
      await spaceStep('Update Space device user name', () async {
        await _coreClients.server.runtimeRemoteLinkService
            .updateCurrentDeviceUserName(
              userName: _runtimeManager.activeIdentity.name,
            );
      });
      // 配置和监听生命周期归 runtime；Dart 只调用生成的类型化 Proxy。
      final peerService = _coreClients.server.runtimeRemoteLinkService;
      final config = await peerService.localHostConfig();
      if (config == null) {
        throw StateError('Runtime listener preferences were not initialized');
      }
      // Activate only configured transports whose inbound Host operation exists.
      final capabilities = await peerService.listenerCapabilities();
      final transports = config.transports
          .where(capabilities.transports.contains)
          .toList();
      if (transports.isNotEmpty) {
        await peerService.startListening(transports: transports);
      }
      await spaceStep('Start Space synchronization', () async {
        await _coreClients.server.runtimeRemoteLinkService.startSpaceSync();
      });
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

  /// Retains the process-owned Core independently of pages and window visibility.
  Future<void> _retainCoreRuntime() async {
    if (_coreResidency != null) return;
    final lease = await OperitKeepAlive.instance.acquire(
      reason: 'Core runtime',
    );
    _coreResidency = lease;
    ClientLogger.i(
      'Core residency protection=${lease.report.protection.name} '
      'closeToTray=${lease.report.closeToTray} detail=${lease.report.detail}',
      tag: _logTag,
    );
  }

  /// Records actual host protection changes, including expiration and native resource loss.
  void _handleResidencyChanged() {
    final report = OperitKeepAlive.instance.report.value;
    if (report == null) return;
    ClientLogger.i(
      'Core residency changed protection=${report.protection.name} '
      'leases=${report.activeLeases} detail=${report.detail}',
      tag: _logTag,
    );
  }

  /// Synchronizes owner-host event handling with the selected runtime role.
  Future<void> _syncHostSubscriber() async {
    final shouldInstall = _ownsHostInteractions;
    final installed = RuntimeHostInteractionSubscriber.isInstalled;
    if (shouldInstall && !installed) {
      final subscriberStopwatch = Stopwatch()..start();
      ClientLogger.v(
        'host subscriber install start',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      RuntimeHostInteractionSubscriber.install();
      ClientLogger.v(
        'host subscriber install done elapsedMs=${subscriberStopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      return;
    }
    if (!shouldInstall && installed) {
      final subscriberStopwatch = Stopwatch()..start();
      ClientLogger.v(
        'host subscriber uninstall start',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      await RuntimeHostInteractionSubscriber.uninstall();
      ClientLogger.v(
        'host subscriber uninstall done elapsedMs=${subscriberStopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
    }
  }

  /// Returns whether this process owns native host interaction handling.
  bool get _ownsHostInteractions {
    return !kIsWeb;
  }
}
