// ignore_for_file: file_names
import 'dart:async';

import 'package:flutter/foundation.dart';

import '../logging/ClientLogger.dart';
import 'AppUpdateModels.dart';
import 'AppUpdateBasePicker.dart';
import 'CoreAppUpdateBackend.dart';

/// Owns one check/download across expanded and collapsed sidebars and dialogs.
class AppUpdateController extends ChangeNotifier {
  /// Binds Core decisions and the platform-neutral installer picker.
  AppUpdateController({
    required AppUpdateBackend backend,
    Future<String?> Function() pickBase = pickAppUpdateBase,
    Future<void> Function(String) deleteSelection =
        deleteAppUpdateBaseSelection,
  }) : _backend = backend,
       _pickBase = pickBase,
       _deleteSelection = deleteSelection;

  static final instance = AppUpdateController(backend: CoreAppUpdateBackend());
  final AppUpdateBackend _backend;
  final Future<String?> Function() _pickBase;
  final Future<void> Function(String) _deleteSelection;
  AppUpdatePlan? plan;
  AppUpdatePhase phase = AppUpdatePhase.idle;
  AppUpdateRelease? release;
  String? packageReceipt;
  String? error;
  int receivedBytes = 0;
  int? totalBytes;
  AppUpdatePreparationProgress? preparation;
  final Stopwatch _preparationClock = Stopwatch();
  Timer? _preparationTimer;
  bool restartRequired = false;
  String? installMessage;
  final Set<String> _promptedVersions = <String>{};
  bool _disposed = false;
  bool _cancelRequested = false;
  StreamSubscription<AppUpdateNetwork>? _networkSubscription;
  AppUpdateNetwork _network = AppUpdateNetwork.offline;
  bool _background = false;
  String? _backgroundAttempt;

  bool get visible => release != null;

  /// Shares reminder state across sidebar rebuilds for this application session.
  bool get hasUnpromptedUpdate =>
      release != null && !_promptedVersions.contains(release!.version);

  void markUpdatePrompted() {
    final selected = release;
    if (selected != null) _promptedVersions.add(selected.version);
  }

  bool get busy => switch (phase) {
    AppUpdatePhase.planning ||
    AppUpdatePhase.validatingBase ||
    AppUpdatePhase.selectingSource ||
    AppUpdatePhase.reconstructing ||
    AppUpdatePhase.downloading ||
    AppUpdatePhase.preparing ||
    AppUpdatePhase.installing => true,
    _ => false,
  };
  double? get progress => totalBytes != null && totalBytes! > 0
      ? (receivedBytes / totalBytes!).clamp(0.0, 1.0)
      : null;

  /// Shows time spent in local processing, including native compression waits.
  Duration get preparationElapsed => _preparationClock.elapsed;

  /// Keeps visible activity independent of the frequency of native progress callbacks.
  void _startPreparation() {
    if (_disposed) return;
    phase = AppUpdatePhase.reconstructing;
    _preparationClock.start();
    _preparationTimer ??= Timer.periodic(
      const Duration(seconds: 1),
      (_) => _emit(),
    );
  }

  /// Stops repaint ticks whenever local processing finishes or yields to transfer.
  void _stopPreparation() {
    _preparationClock.stop();
    _preparationTimer?.cancel();
    _preparationTimer = null;
  }

  void _emit() {
    if (!_disposed) notifyListeners();
  }

  /// Requests native discovery; the core owns check caching and release selection.
  Future<void> check({bool force = false}) async {
    _networkSubscription ??= _backend.backgroundNetwork.distinct().listen(
      _networkChanged,
      onError: (Object error, StackTrace stack) {
        _networkChanged(AppUpdateNetwork.offline);
        ClientLogger.w(
          'Update network status unavailable',
          tag: 'AppUpdate',
          error: error,
          stackTrace: stack,
        );
      },
    );
    if (!force && (phase == AppUpdatePhase.failed || release != null)) return;
    if (busy || phase == AppUpdatePhase.checking || packageReceipt != null) {
      return;
    }
    phase = AppUpdatePhase.checking;
    _emit();
    try {
      release = await _backend.check();
      error = null;
      phase = release == null ? AppUpdatePhase.idle : AppUpdatePhase.available;
    } catch (exception, stack) {
      // Network failure must not interrupt startup or erase an already known update.
      error = exception.toString();
      phase = release == null ? AppUpdatePhase.idle : AppUpdatePhase.available;
      ClientLogger.w(
        'Application update check failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
    }
    _emit();
    unawaited(_prefetch());
  }

  /// Allows any connection below 30 MB, using the selected route's total transfer size.
  bool get _backgroundAllowed {
    if (_network == AppUpdateNetwork.offline) return false;
    if (_network == AppUpdateNetwork.unmetered) return true;
    final bytes = switch (plan?.mode) {
      AppUpdateMode.incremental => plan!.incrementalBytes,
      AppUpdateMode.full => plan!.fullBytes,
      _ => null,
    };
    return bytes != null && bytes >= 0 && bytes < 30000000;
  }

  /// Cancels only transfers that the new connection cannot carry automatically.
  void _networkChanged(AppUpdateNetwork network) {
    final wasAllowed = _backgroundAllowed;
    _network = network;
    if (!_cancelRequested &&
        (network == AppUpdateNetwork.offline ||
            (!wasAllowed && _backgroundAllowed))) {
      _backgroundAttempt = null;
    }
    if (_background &&
        !_cancelRequested &&
        (network == AppUpdateNetwork.offline ||
            (phase != AppUpdatePhase.planning && !_backgroundAllowed))) {
      _backgroundAttempt = null;
      _cancelRequested = true;
      unawaited(_cancelBackground());
    }
    if (network != AppUpdateNetwork.offline) unawaited(_prefetch());
  }

  /// Keeps an automatic cancellation failure in logs while suppressing installation locally.
  Future<void> _cancelBackground() async {
    try {
      await _backend.cancelDownload();
    } catch (exception, stack) {
      ClientLogger.w(
        'Background update cancellation failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
    }
  }

  /// Prepares one target quietly, without prompting for originals or invoking installation.
  Future<void> _prefetch() async {
    final selected = release;
    if (_disposed ||
        _network == AppUpdateNetwork.offline ||
        selected == null ||
        busy ||
        phase != AppUpdatePhase.available ||
        packageReceipt != null ||
        _backgroundAttempt == selected.version) {
      return;
    }
    _backgroundAttempt = selected.version;
    _background = true;
    _cancelRequested = false;
    plan = null;
    phase = AppUpdatePhase.planning;
    _emit();
    try {
      plan = await _backend.plan(selected);
      if (plan!.mode == AppUpdateMode.needsBase && plan!.fullAvailable) {
        plan = await _backend.chooseFull(selected);
      }
      if (plan!.mode == AppUpdateMode.needsBase ||
          plan!.mode == AppUpdateMode.unavailable) {
        phase = AppUpdatePhase.available;
        return;
      }
      if (!_backgroundAllowed || _cancelRequested) {
        phase = AppUpdatePhase.available;
        return;
      }
      await _download();
    } catch (exception, stack) {
      phase = AppUpdatePhase.available;
      if (exception is! AppUpdateCancelled) {
        ClientLogger.w(
          'Background update preparation failed',
          tag: 'AppUpdate',
          error: exception,
          stackTrace: stack,
        );
      }
    } finally {
      _stopPreparation();
      _background = false;
      _emit();
      if (_network != AppUpdateNetwork.offline && _backgroundAttempt == null) {
        unawaited(_prefetch());
      }
    }
  }

  /// Calculates availability after the update dialog becomes visible.
  Future<void> start() async {
    final selected = release;
    if (selected == null || busy || phase == AppUpdatePhase.checking) return;
    error = null;
    _cancelRequested = false;
    restartRequired = false;
    try {
      if (packageReceipt == null) {
        phase = AppUpdatePhase.planning;
        _emit();
        plan = await _backend.plan(selected);
        if (!_acceptPlan()) return;
      }
      await _download();
    } catch (exception, stack) {
      _fail(exception, stack);
    }
  }

  /// Pauses for an original or acknowledges a plan with no usable transfer route.
  bool _acceptPlan() {
    switch (plan!.mode) {
      case AppUpdateMode.needsBase:
        phase = AppUpdatePhase.awaitingBase;
        _emit();
        return false;
      case AppUpdateMode.unavailable:
        phase = AppUpdatePhase.unavailable;
        _emit();
        return false;
      case AppUpdateMode.full:
      case AppUpdateMode.incremental:
        return true;
    }
  }

  /// Verifies a picker copy before any incremental transfer and lets users retry rejected files.
  Future<void> selectBase() async {
    if (phase != AppUpdatePhase.awaitingBase) return;
    phase = AppUpdatePhase.validatingBase;
    error = null;
    _emit();
    String? path;
    bool accepted = false;
    try {
      path = await _pickBase();
      if (path != null) {
        plan = await _backend.supplyBase(release!, path);
        accepted = true;
      }
    } on AppUpdateBaseMismatch {
      error =
          '所选安装包不符合要求。请选择版本 ${plan!.baseVersion} 对应当前平台、架构和安装方式的原始安装包，文件内容须与当时安装的包一致。';
    } catch (exception, stack) {
      error = '无法完成安装包校验，请确认文件可读取后重新选择。';
      ClientLogger.w(
        'Update base verification failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
    } finally {
      if (path != null) {
        try {
          await _deleteSelection(path);
        } catch (exception, stack) {
          ClientLogger.w(
            'Update picker cleanup failed',
            tag: 'AppUpdate',
            error: exception,
            stackTrace: stack,
          );
        }
      }
    }
    if (!accepted) {
      phase = AppUpdatePhase.awaitingBase;
      _emit();
      return;
    }
    if (!_acceptPlan()) return;
    try {
      await _download();
    } catch (exception, stack) {
      _fail(exception, stack);
    }
  }

  /// Applies the no-original choice to the same Core plan.
  Future<void> withoutBase() async {
    if (phase != AppUpdatePhase.awaitingBase) return;
    phase = AppUpdatePhase.planning;
    error = null;
    _emit();
    try {
      plan = await _backend.chooseFull(release!);
      if (!_acceptPlan()) return;
      await _download();
    } catch (exception, stack) {
      _fail(exception, stack);
    }
  }

  /// Lets a later update attempt recalculate routes after the acknowledgement is closed.
  void acknowledgeUnavailable() {
    if (phase != AppUpdatePhase.unavailable) return;
    plan = null;
    phase = AppUpdatePhase.available;
    _emit();
  }

  /// Downloads, reconstructs and verifies without invoking the installer.
  Future<void> _download() async {
    if (packageReceipt == null) {
      _stopPreparation();
      _preparationClock.reset();
      preparation = null;
      receivedBytes = 0;
      totalBytes = null;
      phase = AppUpdatePhase.selectingSource;
      _emit();
      packageReceipt = await _backend.download(
        release!,
        (received, total) {
          _stopPreparation();
          preparation = null;
          phase = AppUpdatePhase.downloading;
          receivedBytes = received;
          totalBytes = total;
          _emit();
        },
        onSelectingSource: () {
          _stopPreparation();
          preparation = null;
          phase = AppUpdatePhase.selectingSource;
          receivedBytes = 0;
          totalBytes = null;
          _emit();
        },
        onReconstructing: () {
          preparation = null;
          _startPreparation();
          _emit();
        },
        onPreparationProgress: (value) {
          preparation = value;
          _startPreparation();
          _emit();
        },
      );
    }
    if (_cancelRequested) throw const AppUpdateCancelled();
    _stopPreparation();
    phase = AppUpdatePhase.ready;
    _emit();
  }

  /// Installs only after an explicit confirmation, including the native restart handoff.
  Future<void> confirmInstall() async {
    if (packageReceipt == null || busy) return;
    phase = AppUpdatePhase.preparing;
    error = null;
    _emit();
    try {
      final result = await _backend.install(packageReceipt!);
      restartRequired = result.state == AppUpdateInstallState.restartRequired;
      installMessage = result.details;
      phase = result.state == AppUpdateInstallState.permissionRequired
          ? AppUpdatePhase.awaitingPermission
          : AppUpdatePhase.awaitingInstall;
      _emit();
      if (restartRequired) await restartToInstall();
    } catch (exception, stack) {
      _fail(exception, stack);
    }
  }

  /// Preserves prepared installers and applies typed cancellation and availability outcomes.
  void _fail(Object exception, StackTrace stack) {
    _stopPreparation();
    if (exception is AppUpdatePackageMissing) {
      packageReceipt = null;
      error = exception.toString();
      phase = AppUpdatePhase.failed;
    } else if (exception is AppUpdateCancelled) {
      error = null;
      phase = AppUpdatePhase.available;
      receivedBytes = 0;
      totalBytes = null;
    } else if (exception is AppUpdateUnavailable) {
      phase = AppUpdatePhase.unavailable;
      error = null;
    } else {
      error = exception.toString();
      phase = AppUpdatePhase.failed;
      ClientLogger.w(
        'Application update failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
    }
    _emit();
  }

  /// Cancels active download or reconstruction through the same Core token.
  Future<void> cancelDownload() async {
    if (phase != AppUpdatePhase.selectingSource &&
        phase != AppUpdatePhase.downloading &&
        phase != AppUpdatePhase.reconstructing) {
      return;
    }
    _cancelRequested = true;
    if (_background) _backgroundAttempt = release?.version;
    try {
      await _backend.cancelDownload();
    } catch (exception, stack) {
      error = '取消下载失败：$exception';
      ClientLogger.w(
        'Application update cancellation failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
      _emit();
    }
  }

  Future<void> restartToInstall() async {
    if (!restartRequired || packageReceipt == null || busy) return;
    phase = AppUpdatePhase.installing;
    error = null;
    _emit();
    try {
      await _backend.restartToInstall();
    } catch (exception) {
      error = exception.toString();
      phase = AppUpdatePhase.failed;
      _emit();
    }
  }

  @override
  void dispose() {
    _stopPreparation();
    _disposed = true;
    unawaited(_networkSubscription?.cancel());
    unawaited(cancelDownload());
    super.dispose();
  }
}
