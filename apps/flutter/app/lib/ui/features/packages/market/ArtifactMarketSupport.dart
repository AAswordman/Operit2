// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';

/// Minimum client version recorded for every marketplace publication.
const String marketMinimumAppVersion = '2.0.0';

/// Maximum client version recorded for every marketplace publication.
const String marketMaximumAppVersion = '2.99.99';
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

/// Starts a broker transaction for one Flutter market browser surface.
Future<GitHubOAuthBrokerLoginStart> startCoreMarketAuthLogin({
  required GeneratedCoreProxyClients clients,
  Uri? completionRedirectUri,
}) async {
  final redirectUri =
      completionRedirectUri ?? coreMarketAuthCompletionRedirectUri;
  final broker = clients.servicesGitHubOAuthBrokerService;
  final start = await broker.startLogin(
    completionRedirectUri: redirectUri.toString(),
  );
  final authorizationUrl = Uri.tryParse(start.authorizationUrl);
  if (authorizationUrl == null ||
      authorizationUrl.scheme != 'https' ||
      authorizationUrl.host != 'github.com') {
    throw StateError('Invalid GitHub OAuth authorizationUrl');
  }
  return start;
}

/// Claims the GitHub OAuth broker transaction after the browser reaches its completion URL.
Future<String> completeCoreMarketAuthLogin({
  required GeneratedCoreProxyClients clients,
  required GitHubOAuthBrokerLoginStart start,
  required Uri completionUrl,
  Uri? completionRedirectUri,
}) async {
  final redirectUri =
      completionRedirectUri ?? coreMarketAuthCompletionRedirectUri;
  if (!isMarketAuthCompletionUri(completionUrl, redirectUri: redirectUri)) {
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

/// Returns whether one browser navigation reached the default OAuth completion destination.
bool isCoreMarketAuthCompletionUri(Uri uri) {
  return isMarketAuthCompletionUri(
    uri,
    redirectUri: coreMarketAuthCompletionRedirectUri,
  );
}

/// Returns whether a browser URL reached the callback destination for one login.
bool isMarketAuthCompletionUri(Uri uri, {required Uri redirectUri}) {
  return uri.scheme == redirectUri.scheme &&
      uri.host == redirectUri.host &&
      uri.port == redirectUri.port &&
      uri.path == redirectUri.path;
}

String formatMarketDate(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '-';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}

const _marketCategoryChinese = <String, (String, String)>{
  'search_research': ('搜索与研究', '搜索引擎、研究工具与知识检索'),
  'dev_code': ('开发工具', '代码编辑、调试与开发工具'),
  'automation_workflow': ('自动化', '任务自动化与工作流处理'),
  'docs_knowledge': ('文档与知识', '文档处理与知识管理'),
  'media_content': ('媒体与内容', '图像、视频、音频处理与内容创作'),
  'chat_communication': ('聊天与通信', '聊天工具、消息收发与通信集成'),
  'integration_api': ('服务与接口', '第三方服务集成与 API 工具'),
  'system_data': ('系统与数据', '文件系统、数据管理与系统工具'),
  'business_productivity': ('商务与效率', '业务工作流与效率工具'),
  'life_entertainment': ('生活与娱乐', '日常生活工具、娱乐与趣味应用'),
  'other': ('其他', '未分类或其他类型的扩展'),
};

/// Localizes the fixed market taxonomy while preserving unknown server categories.
({String name, String description}) marketCategoryText(
  MarketCategoryInfo category, {
  required String languageCode,
}) {
  final chinese = languageCode == 'zh'
      ? _marketCategoryChinese[category.id]
      : null;
  return (
    name: chinese?.$1 ?? category.name,
    description: chinese?.$2 ?? category.description?.trim() ?? '',
  );
}
