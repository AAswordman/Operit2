// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Returns whether [report] contains a successful result for [type].
bool connectionTestSucceeded(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelConnectionTestType type,
) {
  return report.items.any((item) => item.type == type && item.success);
}

/// Applies connection-test results onto [current] without treating skipped
/// checks as failures.
core_proxy.ModelCapabilities capabilitiesFromConnectionTest(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelCapabilities current,
) {
  return core_proxy.ModelCapabilities(
    directImage: connectionTestCapabilityOrFallback(
      report,
      core_proxy.ModelConnectionTestType.image,
      current.directImage,
    ),
    directAudio: connectionTestCapabilityOrFallback(
      report,
      core_proxy.ModelConnectionTestType.audio,
      current.directAudio,
    ),
    directVideo: connectionTestCapabilityOrFallback(
      report,
      core_proxy.ModelConnectionTestType.video,
      current.directVideo,
    ),
    toolCall: connectionTestCapabilityOrFallback(
      report,
      core_proxy.ModelConnectionTestType.toolCall,
      current.toolCall,
    ),
  );
}

/// Uses the tested result for [type], or [fallback] when that check was skipped.
bool connectionTestCapabilityOrFallback(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelConnectionTestType type,
  bool fallback,
) {
  for (final item in report.items) {
    if (item.type == type) {
      return item.success;
    }
  }
  return fallback;
}
