// ignore_for_file: file_names
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart' show ApplicationUpdatePlan;
import '../logging/ClientLogger.dart';
import '../link/CoreLinkProtocol.dart';
import 'AppUpdateModels.dart';

/// Presentation adapter only. Core and Host own target selection, HTTP, files and installation.
class CoreAppUpdateBackend extends AppUpdateBackend {
  CoreAppUpdateBackend({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;
  final GeneratedCoreProxyClients _clients;

  /// Maps only explicit Core decisions to UI states.
  AppUpdatePlan _plan(ApplicationUpdatePlan value) => AppUpdatePlan(
    mode: switch (value.mode) {
      'full' => AppUpdateMode.full,
      'incremental' => AppUpdateMode.incremental,
      'needs_base' => AppUpdateMode.needsBase,
      'unavailable' => AppUpdateMode.unavailable,
      _ => throw StateError('核心未返回可识别的更新方案'),
    },
    fullAvailable: value.fullAvailable,
    incrementalAvailable: value.incrementalAvailable,
    baseVersion: value.baseVersion,
    fullBytes: value.fullBytes,
    incrementalBytes: value.incrementalBytes,
  );

  /// Requests an authoritative plan before the first transfer.
  @override
  Future<AppUpdatePlan> plan(AppUpdateRelease release) async => _plan(
    await _clients.application.planApplicationUpdate(version: release.version),
  );

  /// Verifies the selected old installer without downloading patch data.
  @override
  Future<AppUpdatePlan> supplyBase(
    AppUpdateRelease release,
    String path,
  ) async {
    try {
      return _plan(
        await _clients.application.supplyApplicationUpdateBase(
          version: release.version,
          sourcePath: path,
        ),
      );
    } on CoreLinkError catch (error) {
      if (error.message.startsWith('UPDATE_BASE_MISMATCH:')) {
        throw const AppUpdateBaseMismatch();
      }
      rethrow;
    }
  }

  /// Uses the existing plan when the user has no original installer.
  @override
  Future<AppUpdatePlan> chooseFull(AppUpdateRelease release) async => _plan(
    await _clients.application.selectApplicationUpdateFull(
      version: release.version,
    ),
  );

  @override
  Future<AppUpdateRelease?> check() async {
    final update = await _clients.application.checkApplicationUpdate();
    if (update == null) return null;
    return AppUpdateRelease(
      currentVersion: update.currentVersion,
      version: update.version,
      releasePageUrl: Uri.parse(update.releasePageUrl),
      releaseNotes: update.releaseNotes,
    );
  }

  @override
  Future<String> download(
    AppUpdateRelease release,
    AppUpdateProgress progress, {
    void Function()? onSelectingSource,
    void Function()? onReconstructing,
  }) async {
    final subscription = _clients.application
        .applicationUpdateProgressFlow()
        .listen(
          (event) {
            if (event.stage == 'selecting_source') {
              onSelectingSource?.call();
              return;
            }
            if (event.stage == 'reconstructing') {
              onReconstructing?.call();
              return;
            }
            if (event.stage != 'downloading') return;
            progress(
              event.receivedBytes,
              event.totalBytes > 0 ? event.totalBytes : null,
            );
          },
          onError: (Object error, StackTrace stack) {
            // Completion/failure still comes from the download call. A broken
            // progress subscription must not escape as an unhandled UI error.
            progress(0, null);
            ClientLogger.w(
              'Application update progress unavailable',
              tag: 'AppUpdate',
              error: error,
              stackTrace: stack,
            );
          },
        );
    try {
      return await _clients.application.downloadApplicationUpdate(
        version: release.version,
      );
    } catch (error) {
      if (error is CoreLinkError &&
          error.message.startsWith('UPDATE_UNAVAILABLE')) {
        throw const AppUpdateUnavailable();
      }
      if (error.toString().contains('UPDATE_CANCELLED')) {
        throw const AppUpdateCancelled();
      }
      rethrow;
    } finally {
      await subscription.cancel();
    }
  }

  @override
  Future<void> cancelDownload() =>
      _clients.application.cancelApplicationUpdateDownload();

  @override
  Future<AppUpdateInstallResult> install(String receipt) async {
    try {
      final result = await _clients.application.installApplicationUpdate(
        receipt: receipt,
      );
      final state = switch (result.installState) {
        'permission_required' => AppUpdateInstallState.permissionRequired,
        'installer_opened' => AppUpdateInstallState.installerOpened,
        'restart_required' => AppUpdateInstallState.restartRequired,
        'completed' => AppUpdateInstallState.completed,
        _ => throw StateError('核心未返回可识别的安装状态'),
      };
      return AppUpdateInstallResult(state, details: result.details);
    } catch (error) {
      if (error.toString().contains('UPDATE_PACKAGE_MISSING')) {
        throw AppUpdatePackageMissing();
      }
      rethrow;
    }
  }

  @override
  Future<void> restartToInstall() => platformCoreProxy.restartApplication();
}
