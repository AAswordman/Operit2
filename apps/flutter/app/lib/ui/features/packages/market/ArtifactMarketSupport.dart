// ignore_for_file: file_names

import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';

const String currentAppVersion = '2.0.0';
final Uri coreMarketAuthCompletionRedirectUri = Uri.parse(
  'https://api.operit.app/oauth/github/complete',
);

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
  final value = await clients.bridge.call(
    CoreCallRequest(
      requestId: 'flutter-market-${DateTime.now().microsecondsSinceEpoch}',
      targetPath: CoreObjectPath.parse('application'),
      methodName: 'runCoreCommand',
      args: <String, Object?>{
        'args': <String>[
          'market',
          'install',
          entryId,
          if (versionId?.trim().isNotEmpty == true) versionId!.trim(),
        ],
      },
    ),
  );
  if (value is! Map<Object?, Object?>) {
    throw StateError('Invalid core command output');
  }
  final stderr = value['stderr']?.toString().trim() ?? '';
  if (stderr.isNotEmpty) {
    throw StateError(stderr);
  }
  final stdout = value['stdout']?.toString().trim() ?? '';
  return stdout.isEmpty ? '安装完成' : stdout;
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
