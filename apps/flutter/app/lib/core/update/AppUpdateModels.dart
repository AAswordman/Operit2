// ignore_for_file: file_names

/// A newer package selected by the shared Rust release/channel rules.
class AppUpdateRelease {
  const AppUpdateRelease({
    required this.currentVersion,
    required this.version,
    required this.releasePageUrl,
    this.releaseNotes = '',
  });

  final String currentVersion;
  final String version;
  final Uri releasePageUrl;
  final String releaseNotes;
}

enum AppUpdatePhase {
  idle,
  checking,
  available,
  downloading,
  preparing,
  installing,
  awaitingPermission,
  awaitingInstall,
  failed,
}

enum AppUpdateInstallState {
  permissionRequired,
  installerOpened,
  restartRequired,
  completed,
}

/// The native install_app result determines all follow-up UI; platforms are not inferred.
class AppUpdateInstallResult {
  const AppUpdateInstallResult(this.state, {this.details = ''});
  final AppUpdateInstallState state;
  final String details;
}

typedef AppUpdateProgress = void Function(int received, int? total);

/// Always operates on this installed client, independently of selected devices.
abstract class AppUpdateBackend {
  Future<AppUpdateRelease?> check();
  Future<String> download(AppUpdateRelease release, AppUpdateProgress progress);
  Future<void> cancelDownload();
  Future<AppUpdateInstallResult> install(String receipt);
  Future<void> restartToInstall();
}

class AppUpdateCancelled implements Exception {
  const AppUpdateCancelled();
}

class AppUpdatePackageMissing implements Exception {
  @override
  String toString() => '下载的安装包已被清理，请重新下载';
}
