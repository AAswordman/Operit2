// ignore_for_file: file_names

import 'dart:typed_data';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';

const String currentAppVersion = '2.0.0+6';
final Uri coreMarketAuthCompletionRedirectUri = Uri.parse(
  'https://api.operit.app/oauth/github/complete',
);

/// Identifies the compatibility bound that rejects one marketplace version.
enum MarketAppVersionCompatibilityKind { belowMinimum, aboveMaximum }

/// Describes why a marketplace entry cannot run on the current client build.
class MarketAppVersionCompatibility {
  const MarketAppVersionCompatibility({
    required this.kind,
    required this.currentAppVersion,
    required this.requiredAppVersion,
  });

  final MarketAppVersionCompatibilityKind kind;
  final String currentAppVersion;
  final String requiredAppVersion;

  /// Returns a user-facing explanation for the rejected compatibility bound.
  String get message => switch (kind) {
    MarketAppVersionCompatibilityKind.belowMinimum =>
      '客户端版本过低：当前版本 $currentAppVersion，'
          '该资源要求至少为 $requiredAppVersion。请更新客户端后再下载。',
    MarketAppVersionCompatibilityKind.aboveMaximum =>
      '客户端版本过高：当前版本 $currentAppVersion，'
          '该资源最高支持到 $requiredAppVersion。请使用受支持的客户端版本。',
  };
}

/// Returns the violated marketplace client-version bound, when one exists.
MarketAppVersionCompatibility? resolveMarketAppVersionCompatibility({
  required String appVersion,
  required String minAppVersion,
  required String? maxAppVersion,
}) {
  final current = _MarketAppVersion.parse(appVersion);
  final minimumValue = minAppVersion.trim();
  if (minimumValue.isNotEmpty) {
    final minimum = _MarketAppVersion.parse(minimumValue);
    if (current.compareTo(minimum) < 0) {
      return MarketAppVersionCompatibility(
        kind: MarketAppVersionCompatibilityKind.belowMinimum,
        currentAppVersion: current.toString(),
        requiredAppVersion: minimum.toString(),
      );
    }
  }
  final maximumValue = maxAppVersion?.trim() ?? '';
  if (maximumValue.isNotEmpty) {
    final maximum = _MarketAppVersion.parse(maximumValue);
    if (current.compareTo(maximum) > 0) {
      return MarketAppVersionCompatibility(
        kind: MarketAppVersionCompatibilityKind.aboveMaximum,
        currentAppVersion: current.toString(),
        requiredAppVersion: maximum.toString(),
      );
    }
  }
  return null;
}

/// Rejects one market entry version when the current client cannot support it.
void ensureMarketAppVersionSupported({
  required String minAppVersion,
  required String? maxAppVersion,
}) {
  final compatibility = resolveMarketAppVersionCompatibility(
    appVersion: currentAppVersion,
    minAppVersion: minAppVersion,
    maxAppVersion: maxAppVersion,
  );
  if (compatibility != null) {
    throw StateError(compatibility.message);
  }
}

/// Rejects one market entry's selected version when the current client cannot support it.
void ensureMarketEntryVersionSupported({
  required MarketEntrySummary entry,
  String? versionId,
}) {
  final normalizedVersionId = versionId?.trim();
  final version = switch (normalizedVersionId) {
    null || '' => entry.latestVersion,
    final selectedVersionId =>
      entry.versions
          .where((candidate) => candidate.id == selectedVersionId)
          .firstOrNull,
  };
  if (version == null) {
    throw StateError('市场条目缺少要安装的版本信息。');
  }
  ensureMarketAppVersionSupported(
    minAppVersion: version.minAppVer,
    maxAppVersion: version.maxAppVer,
  );
}

/// Parses and compares the app-version format used by marketplace metadata.
class _MarketAppVersion implements Comparable<_MarketAppVersion> {
  const _MarketAppVersion({
    required this.major,
    required this.minor,
    required this.patch,
    required this.build,
  });

  factory _MarketAppVersion.parse(String value) {
    final match = RegExp(
      r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$',
    ).firstMatch(value.trim());
    if (match == null) {
      throw FormatException('版本号必须使用 x.y.z 或 x.y.z+n 格式：$value');
    }
    return _MarketAppVersion(
      major: int.parse(match.group(1)!),
      minor: int.parse(match.group(2)!),
      patch: int.parse(match.group(3)!),
      build: int.parse(match.group(4) ?? '0'),
    );
  }

  final int major;
  final int minor;
  final int patch;
  final int build;

  @override
  int compareTo(_MarketAppVersion other) {
    final majorOrder = major.compareTo(other.major);
    if (majorOrder != 0) return majorOrder;
    final minorOrder = minor.compareTo(other.minor);
    if (minorOrder != 0) return minorOrder;
    final patchOrder = patch.compareTo(other.patch);
    if (patchOrder != 0) return patchOrder;
    return build.compareTo(other.build);
  }

  @override
  String toString() =>
      build == 0 ? '$major.$minor.$patch' : '$major.$minor.$patch+$build';
}

String firstNonBlank(Iterable<String> values) {
  for (final value in values) {
    final trimmed = value.trim();
    if (trimmed.isNotEmpty) {
      return trimmed;
    }
  }
  return '';
}

String artifactTypeLabel(String type) {
  return switch (type.trim()) {
    'package' => 'Package',
    'script' => 'Script',
    final value when value.isNotEmpty => value,
    _ => 'Artifact',
  };
}

Future<String> runCoreMarketInstall({
  required GeneratedCoreProxyClients clients,
  required String type,
  required String entryId,
  String? versionId,
}) async {
  final normalizedType = type.trim();
  if (normalizedType.isEmpty) {
    throw StateError('Artifact type is empty');
  }
  final entry = await clients.providersMarketStatsApiService.getEntryById(
    entryId: entryId,
  );
  if (entry.type != normalizedType) {
    throw StateError('Marketplace entry type changed during installation');
  }
  final selectedVersionId = versionId?.trim();
  final targetVersionId = selectedVersionId == null || selectedVersionId.isEmpty
      ? entry.latestVersion?.id.trim()
      : selectedVersionId;
  if (targetVersionId == null || targetVersionId.isEmpty) {
    throw StateError('Marketplace entry has no installable version');
  }
  final asset = entry.assets
      .where(
        (candidate) =>
            candidate.versionId == targetVersionId &&
            candidate.id.trim().isNotEmpty,
      )
      .firstOrNull;
  if (asset == null) {
    throw StateError('Marketplace entry has no downloadable asset for version');
  }
  final fileName = asset.assetName?.trim();
  if (fileName == null || fileName.isEmpty) {
    throw StateError('Marketplace asset has no file name');
  }
  final bytes = await clients.providersMarketStatsApiService.downloadAsset(
    assetId: asset.id,
  );
  final result = await clients.application
      .packageManager()
      .addMarketArtifactBytes(
        bytes: Uint8List.fromList(bytes),
        fileName: fileName,
        expectedSha256: asset.sha256,
      );
  if (!result.toLowerCase().startsWith('successfully imported')) {
    throw StateError(result);
  }
  return result;
}

/// Starts a broker transaction for Flutter's visible market browser.
Future<GitHubOAuthBrokerLoginStart> startCoreMarketAuthLogin({
  required GeneratedCoreProxyClients clients,
}) async {
  final broker = clients.servicesGitHubOAuthBrokerService;
  final start = await broker.startLogin(
    completionRedirectUri: coreMarketAuthCompletionRedirectUri.toString(),
  );
  final authorizationUrl = Uri.tryParse(start.authorizationUrl);
  if (authorizationUrl == null ||
      authorizationUrl.scheme != 'https' ||
      authorizationUrl.host != 'github.com') {
    throw StateError('Invalid GitHub OAuth authorizationUrl');
  }
  return start;
}

/// Claims the GitHub OAuth broker transaction after the visible browser reaches its completion URL.
Future<String> completeCoreMarketAuthLogin({
  required GeneratedCoreProxyClients clients,
  required GitHubOAuthBrokerLoginStart start,
  required Uri completionUrl,
}) async {
  if (!isCoreMarketAuthCompletionUri(completionUrl)) {
    throw StateError('GitHub OAuth callback destination is invalid');
  }
  final broker = clients.servicesGitHubOAuthBrokerService;
  final result = await broker.completeLogin(
    completion: GitHubOAuthBrokerLoginCompletion(
      attemptId: start.attemptId,
      completionUrl: completionUrl.toString(),
    ),
  );
  return result.login;
}

/// Returns whether one browser navigation reached the registered OAuth completion destination.
bool isCoreMarketAuthCompletionUri(Uri uri) {
  return uri.scheme == coreMarketAuthCompletionRedirectUri.scheme &&
      uri.host == coreMarketAuthCompletionRedirectUri.host &&
      uri.port == coreMarketAuthCompletionRedirectUri.port &&
      uri.path == coreMarketAuthCompletionRedirectUri.path;
}

String formatMarketDate(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '-';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}
