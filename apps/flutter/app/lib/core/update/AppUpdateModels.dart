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
  ready,
  preparing,
  installing,
  awaitingPermission,
  awaitingInstall,
  failed,
}

enum AppUpdateMode { full, incremental, needsBase, unavailable }

/// Distinguishes offline status from connections eligible for small transfers only.
enum AppUpdateNetwork { offline, restricted, unmetered }

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

/// Identifies local preparation work reported by Core on every supported host.
enum AppUpdatePreparationPhase {
  verifyingBase,
  verifyingPatch,
  expandingBase,
  verifyingExpandedBase,
  applyingPatch,
  restoringPackage,
  verifyingPackage,
  savingPackage,
}

/// Carries actual work completed in the current step, independently of downloads.
class AppUpdatePreparationProgress {
  /// Stores the counters supplied by the shared update pipeline.
  const AppUpdatePreparationProgress(
    this.phase,
    this.completedBytes,
    this.totalBytes,
  );

  final AppUpdatePreparationPhase phase;
  final int completedBytes;
  final int totalBytes;

  /// Leaves unknown work indeterminate without estimating time or completion.
  double? get fraction =>
      totalBytes > 0 ? (completedBytes / totalBytes).clamp(0.0, 1.0) : null;
}

/// Always operates on this installed client, independently of selected devices.
abstract class AppUpdateBackend {
  /// Emits connectivity and cost from the installed client's normalized Host events.
  Stream<AppUpdateNetwork> get backgroundNetwork => const Stream.empty();

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
    void Function(AppUpdatePreparationProgress)? onPreparationProgress,
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
