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
  bool restartRequired = false;
  String? installMessage;
  bool _disposed = false;
  bool _cancelRequested = false;

  bool get visible => release != null;
  bool get busy => switch (phase) {
    AppUpdatePhase.planning ||
    AppUpdatePhase.validatingBase ||
    AppUpdatePhase.reconstructing ||
    AppUpdatePhase.downloading ||
    AppUpdatePhase.preparing ||
    AppUpdatePhase.installing => true,
    _ => false,
  };
  double? get progress => totalBytes != null && totalBytes! > 0
      ? (receivedBytes / totalBytes!).clamp(0.0, 1.0)
      : null;

  void _emit() {
    if (!_disposed) notifyListeners();
  }

  /// Requests native discovery; the core owns check caching and release selection.
  Future<void> check({bool force = false}) async {
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
      phase = release == null ? AppUpdatePhase.idle : AppUpdatePhase.available;
      ClientLogger.w(
        'Application update check failed',
        tag: 'AppUpdate',
        error: exception,
        stackTrace: stack,
      );
    }
    _emit();
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
      await _transferAndInstall();
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
      await _transferAndInstall();
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
      await _transferAndInstall();
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

  /// Downloads the selected route and submits only its verified receipt for installation.
  Future<void> _transferAndInstall() async {
    if (packageReceipt == null) {
      receivedBytes = 0;
      totalBytes = null;
      phase = AppUpdatePhase.downloading;
      _emit();
      packageReceipt = await _backend.download(
        release!,
        (received, total) {
          phase = AppUpdatePhase.downloading;
          receivedBytes = received;
          totalBytes = total;
          _emit();
        },
        onReconstructing: () {
          phase = AppUpdatePhase.reconstructing;
          _emit();
        },
      );
    }
    if (_cancelRequested) throw const AppUpdateCancelled();
    phase = AppUpdatePhase.preparing;
    _emit();
    final result = await _backend.install(packageReceipt!);
    restartRequired = result.state == AppUpdateInstallState.restartRequired;
    installMessage = result.details;
    phase = result.state == AppUpdateInstallState.permissionRequired
        ? AppUpdatePhase.awaitingPermission
        : AppUpdatePhase.awaitingInstall;
    _emit();
  }

  /// Preserves prepared installers and applies typed cancellation and availability outcomes.
  void _fail(Object exception, StackTrace stack) {
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
    if (phase != AppUpdatePhase.downloading &&
        phase != AppUpdatePhase.reconstructing) {
      return;
    }
    _cancelRequested = true;
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
    _disposed = true;
    unawaited(cancelDownload());
    super.dispose();
  }
}
