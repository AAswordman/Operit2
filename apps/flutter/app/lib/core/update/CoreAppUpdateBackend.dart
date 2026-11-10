// ignore_for_file: file_names
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../logging/ClientLogger.dart';
import 'AppUpdateModels.dart';

/// Presentation adapter only. Core and Host own target selection, HTTP, files and installation.
class CoreAppUpdateBackend extends AppUpdateBackend {
  CoreAppUpdateBackend({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;
  final GeneratedCoreProxyClients _clients;

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
    AppUpdateProgress progress,
  ) async {
    final subscription = _clients.application
        .applicationUpdateProgressFlow()
        .listen(
          (event) {
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
