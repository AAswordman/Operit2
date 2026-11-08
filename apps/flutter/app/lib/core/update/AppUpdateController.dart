// ignore_for_file: file_names
import 'dart:async';

import 'package:flutter/foundation.dart';

import '../logging/ClientLogger.dart';
import 'AppUpdateModels.dart';
import 'CoreAppUpdateBackend.dart';

/// Owns one check/download across expanded and collapsed sidebars and dialogs.
class AppUpdateController extends ChangeNotifier {
  AppUpdateController({required AppUpdateBackend backend}) : _backend = backend;

  static final instance = AppUpdateController(backend: CoreAppUpdateBackend());
  final AppUpdateBackend _backend;
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

  /// Called only after the download dialog has been painted.
  Future<void> start() async {
    final selected = release;
    if (selected == null || busy || phase == AppUpdatePhase.checking) return;
    error = null;
    _cancelRequested = false;
    restartRequired = false;
    try {
      if (packageReceipt == null) {
        receivedBytes = 0;
        totalBytes = null;
        phase = AppUpdatePhase.downloading;
        _emit();
        packageReceipt = await _backend.download(selected, (received, total) {
          receivedBytes = received;
          totalBytes = total;
          _emit();
        });
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
    } on AppUpdatePackageMissing catch (exception) {
      packageReceipt = null;
      error = exception.toString();
      phase = AppUpdatePhase.failed;
    } on AppUpdateCancelled {
      error = null;
      phase = AppUpdatePhase.available;
      receivedBytes = 0;
      totalBytes = null;
    } catch (exception, stack) {
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

  Future<void> cancelDownload() async {
    if (phase != AppUpdatePhase.downloading) return;
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
