// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Account and upstream restrictions do not establish a model capability result.
enum ModelConnectionTestRestriction { credits, rateLimit }

final _restrictionStatus = RegExp(
  r'(?:^|\b(?:request|connection) failed:\s*|\bHTTP(?:/\d(?:\.\d)?)?\s+)(402|429)(?=\s|:|$)',
  caseSensitive: false,
);

/// Reads the HTTP status preserved in the provider's request error.
ModelConnectionTestRestriction? connectionTestRestriction(String? error) {
  if (error == null) {
    return null;
  }
  return switch (_restrictionStatus.firstMatch(error)?.group(1)) {
    '402' => ModelConnectionTestRestriction.credits,
    '429' => ModelConnectionTestRestriction.rateLimit,
    _ => null,
  };
}

/// Reports whether any check was blocked by account credit or upstream limits.
bool connectionTestHasRestrictions(
  core_proxy.ModelConnectionTestReport report,
) {
  return report.items.any(
    (item) => !item.success && connectionTestRestriction(item.error) != null,
  );
}

/// Reports unsuccessful checks that reached a result outside those restrictions.
bool connectionTestHasFailures(core_proxy.ModelConnectionTestReport report) {
  return report.items.any(
    (item) => !item.success && connectionTestRestriction(item.error) == null,
  );
}

/// Returns whether [report] contains a successful result for [type].
bool connectionTestSucceeded(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelConnectionTestType type,
) {
  return report.items.any((item) => item.type == type && item.success);
}

/// Applies completed capability checks. Skipped and restricted checks do not
/// change the user's selected capabilities.
core_proxy.ModelCapabilities capabilitiesFromConnectionTest(
  core_proxy.ModelConnectionTestReport report,
  core_proxy.ModelCapabilities current,
) {
  var directImage = current.directImage;
  var directAudio = current.directAudio;
  var directVideo = current.directVideo;
  var toolCall = current.toolCall;
  for (final item in report.items) {
    if (!item.success && connectionTestRestriction(item.error) != null) {
      continue;
    }
    switch (item.type) {
      case core_proxy.ModelConnectionTestType.chat:
        break;
      case core_proxy.ModelConnectionTestType.image:
        directImage = item.success;
      case core_proxy.ModelConnectionTestType.audio:
        directAudio = item.success;
      case core_proxy.ModelConnectionTestType.video:
        directVideo = item.success;
      case core_proxy.ModelConnectionTestType.toolCall:
        toolCall = item.success;
    }
  }
  return core_proxy.ModelCapabilities(
    directImage: directImage,
    directAudio: directAudio,
    directVideo: directVideo,
    toolCall: toolCall,
  );
}
