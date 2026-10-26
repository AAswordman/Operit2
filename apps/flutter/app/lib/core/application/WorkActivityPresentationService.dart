// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/widgets.dart';
import '../../l10n/generated/app_localizations.dart';
import 'package:operit_keep_alive/operit_keep_alive.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../logging/ClientLogger.dart';
import '../notifications/NotificationActivationService.dart';
import '../proxy/generated/CoreProxyClients.g.dart';

/// Connects process work information to native system surfaces.
class WorkActivityPresentationService with WidgetsBindingObserver {
  /// Creates the main process's work presentation coordinator.
  WorkActivityPresentationService._();

  static final WorkActivityPresentationService instance =
      WorkActivityPresentationService._();
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static WorkPresentationLabels labels = _labelsFor(
    lookupAppLocalizations(
      basicLocaleListResolution(
        WidgetsBinding.instance.platformDispatcher.locales,
        AppLocalizations.supportedLocales,
      ),
    ),
  );
  String? _localeName;

  /// Builds complete display wording from the application's resolved locale.
  static WorkPresentationLabels _labelsFor(AppLocalizations text) =>
      WorkPresentationLabels(
        open: text.backgroundWorkOpen,
        review: text.backgroundWorkReview,
        stop: text.backgroundWorkStop,
        idle: text.backgroundWorkIdle,
        running: text.backgroundWorkRunningCount,
        waiting: text.backgroundWorkWaiting,
        waitingCount: text.backgroundWorkWaitingCount,
        completed: text.backgroundWorkCompleted,
        failed: text.backgroundWorkFailed,
        cancelled: text.backgroundWorkCancelled,
        phases: {
          'processing': text.backgroundWorkProcessing,
          'connecting': text.backgroundWorkConnecting,
          'receiving': text.backgroundWorkReceiving,
          'executingTool': text.backgroundWorkTool,
          'processingToolResult': text.backgroundWorkToolResult,
          'summarizing': text.backgroundWorkSummarizing,
          'executingPlan': text.backgroundWorkPlan,
        },
      );

  /// Updates system wording when Flutter reports a change in the device locales.
  @override
  void didChangeLocales(List<Locale>? locales) {
    _configureLocalizations(
      lookupAppLocalizations(
        basicLocaleListResolution(locales, AppLocalizations.supportedLocales),
      ),
    );
  }

  /// Applies the resolved locale to future and currently displayed system information.
  void _configureLocalizations(AppLocalizations text) {
    if (_localeName == text.localeName) return;
    _localeName = text.localeName;
    labels = _labelsFor(text);
    final current = snapshot.value;
    final host = capabilities.value;
    if (current != null &&
        host != null &&
        host.updatesOwner == WorkUpdatesOwner.client &&
        host.surface != WorkSystemSurface.none) {
      unawaited(
        OperitKeepAlive.instance.publishWork(current, labels).catchError((
          Object cause,
          StackTrace stackTrace,
        ) {
          _reportError(cause, stackTrace);
        }),
      );
    }
  }

  final ValueNotifier<WorkActivitySnapshot?> snapshot = ValueNotifier(null);
  final ValueNotifier<String?> error = ValueNotifier(null);
  final ValueNotifier<WorkPresentationCapabilities?> capabilities =
      ValueNotifier(null);
  Future<void>? _initialization;
  StreamSubscription<void>? _updates;
  StreamSubscription<void>? _actions;
  StreamSubscription<String>? _presentationErrors;

  /// Installs one process subscription and acknowledges native action readiness once.
  Future<void> initialize() => _initialization ??= _initialize();

  /// Connects common Core state to the presentation owner declared by the host.
  Future<void> _initialize() async {
    WidgetsBinding.instance.addObserver(this);
    capabilities.value = await OperitKeepAlive.instance
        .workPresentationCapabilities();
    _presentationErrors = OperitKeepAlive.instance.workErrors.listen((message) {
      _reportError(StateError(message), StackTrace.current);
    });
    _actions = OperitKeepAlive.instance.workActions
        .asyncMap(_handleAction)
        .listen((_) {}, onError: _reportError);
    _updates = _clients.servicesWorkActivityService
        .workUpdatesFlow()
        .asyncMap(_receive)
        .listen((_) {}, onError: _reportError);
    await OperitKeepAlive.instance.enableWorkActions();
  }

  /// Applies complete revisions in order and lets the declared owner render system information.
  Future<void> _receive(String encoded) async {
    final next = WorkActivitySnapshot.fromJson(encoded);
    final previous = snapshot.value;
    if (previous != null && next.revision <= previous.revision) return;
    snapshot.value = next;
    final host = capabilities.value;
    if (host == null) {
      throw StateError(
        'Work presentation capabilities have not been initialized',
      );
    }
    if (host.updatesOwner == WorkUpdatesOwner.client &&
        host.surface != WorkSystemSurface.none) {
      await OperitKeepAlive.instance.publishWork(next, labels);
      error.value = null;
    }
  }

  /// Routes native actions using exact work identity and an explicit conversation destination.
  Future<void> _handleAction(WorkAction action) async {
    switch (action.action) {
      case 'open':
        await NotificationActivationService.instance.receive(action.activation);
      case 'stop':
        await stop(action.workId);
      default:
        throw StateError('Unsupported work action: ${action.action}');
    }
  }

  /// Stops a selected execution through the Core controller that originally started it.
  Future<void> stop(String workId) =>
      _clients.servicesWorkActivityService.stopWork(workId: workId);

  /// Refreshes notification authorization after an actual application foreground transition.
  Future<void> refreshCapabilities() async {
    if (_initialization == null) return;
    await _initialization;
    capabilities.value = await OperitKeepAlive.instance
        .workPresentationCapabilities();
    final current = snapshot.value;
    final host = capabilities.value!;
    if (current != null &&
        host.updatesOwner == WorkUpdatesOwner.client &&
        host.surface != WorkSystemSurface.none) {
      await OperitKeepAlive.instance.publishWork(current, labels);
    }
  }

  /// Records transport, schema, presentation, and system-action errors.
  void _reportError(Object cause, StackTrace stackTrace) {
    error.value = cause.toString();
    ClientLogger.e(
      'Work presentation failed',
      tag: 'WorkActivity',
      error: cause,
      stackTrace: stackTrace,
    );
  }

  /// Releases main-engine subscriptions when the process coordinator is explicitly disposed.
  Future<void> dispose() async {
    WidgetsBinding.instance.removeObserver(this);
    await _updates?.cancel();
    await _actions?.cancel();
    await _presentationErrors?.cancel();
  }
}
