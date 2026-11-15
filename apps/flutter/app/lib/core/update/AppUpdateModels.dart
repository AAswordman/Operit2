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
  planning,
  awaitingBase,
  validatingBase,
  unavailable,
  selectingSource,
  downloading,
  reconstructing,
  preparing,
  installing,
  awaitingPermission,
  awaitingInstall,
  failed,
}

enum AppUpdateMode { full, incremental, needsBase, unavailable }

/// Contains the availability and transfer decision calculated by Core.
class AppUpdatePlan {
  /// Stores presentation data for one selected target version.
  const AppUpdatePlan({
    required this.mode,
    required this.fullAvailable,
    required this.incrementalAvailable,
    this.baseVersion,
    this.fullBytes,
    this.incrementalBytes,
  });
  final AppUpdateMode mode;
  final bool fullAvailable;
  final bool incrementalAvailable;
  final String? baseVersion;
  final int? fullBytes;
  final int? incrementalBytes;
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

  /// Calculates both routes before any installer or patch download.
  Future<AppUpdatePlan> plan(AppUpdateRelease release);

  /// Verifies and retains an exact original for the existing plan.
  Future<AppUpdatePlan> supplyBase(AppUpdateRelease release, String path);

  /// Applies the user's no-original choice to the existing plan.
  Future<AppUpdatePlan> chooseFull(AppUpdateRelease release);
  Future<String> download(
    AppUpdateRelease release,
    AppUpdateProgress progress, {
    void Function()? onSelectingSource,
    void Function()? onReconstructing,
  });
  Future<void> cancelDownload();
  Future<AppUpdateInstallResult> install(String receipt);
  Future<void> restartToInstall();
}

class AppUpdateUnavailable implements Exception {
  /// Identifies a transfer route removed after discovery.
  const AppUpdateUnavailable();
}

class AppUpdateBaseMismatch implements Exception {
  /// Identifies an original that does not match the authenticated patch base.
  const AppUpdateBaseMismatch();
}

class AppUpdateCancelled implements Exception {
  const AppUpdateCancelled();
}

class AppUpdatePackageMissing implements Exception {
  @override
  String toString() => '下载的安装包已被清理，请重新下载';
}
