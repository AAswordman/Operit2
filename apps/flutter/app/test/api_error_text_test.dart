import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations_en.dart';
import 'package:operit2/l10n/generated/app_localizations_fr.dart';
import 'package:operit2/l10n/generated/app_localizations_zh.dart';
import 'package:operit2/ui/common/ApiErrorText.dart';

/// Verifies status recognition boundaries and lossless error presentation.
void main() {
  final l10n = AppLocalizationsZh();
  test(
    'HTTP 400 reasoning_content guides provider choice in every language',
    () {
      const original =
          'request failed: 400 Bad Request: '
          '{"error":{"message":"Missing reasoning_content field"}}';
      for (final language in [
        l10n,
        AppLocalizationsEn(),
        AppLocalizationsFr(),
      ]) {
        final formatted = apiErrorText(original, language);
        expect(formatted, contains(language.apiErrorReasoningContentTitle));
        expect(formatted, contains(language.apiErrorReasoningContentMessage));
        expect(formatted, contains('DeepSeek'));
        expect(formatted, contains('OpenAI'));
        expect(formatted, endsWith(original));
      }
    },
  );

  test('reasoning guidance requires both HTTP 400 and the specified field', () {
    expect(
      apiErrorText('request failed: 400: invalid model', l10n),
      startsWith('HTTP 400：请求参数有误\n'),
    );
    expect(
      apiErrorText('request failed: 401: reasoning_content', l10n),
      startsWith('HTTP 401：API Key 有误\n'),
    );
    const unstructured =
        '{"error":{"code":400,"message":"Missing reasoning_content"}}';
    expect(apiErrorText(unstructured, l10n), unstructured);
  });

  for (final (status, title) in [
    (400, '请求参数有误'),
    (401, 'API Key 有误'),
    (402, 'API 欠费'),
    (403, '请求被服务器风控拒绝'),
    (404, '接口或模型不存在'),
    (405, '接口不支持此请求方式'),
    (407, '代理验证失败'),
    (408, 'API 请求超时'),
    (409, '请求状态冲突'),
    (413, '请求内容过大'),
    (415, '请求内容格式不受支持'),
    (422, '请求参数有误'),
    (429, '服务器限流 / 风控'),
    (499, '服务器风控 / 请求中断'),
    (500, 'API 服务器内部错误'),
    (502, 'API 网关响应异常'),
    (503, 'API 服务暂时不可用'),
    (504, 'API 请求超时'),
    (521, 'API 源站无法连接'),
    (524, 'API 请求超时'),
    (525, 'API 服务器 TLS 或证书异常'),
    (529, 'API 服务暂时不可用'),
  ]) {
    test('explains HTTP $status and retains the original response verbatim', () {
      final original =
          'request failed: $status: {"error":"upstream detail"}\n trace-id: abc  ';
      final formatted = apiErrorText(original, l10n);
      expect(formatted, startsWith('HTTP $status：$title\n'));
      expect(formatted, endsWith('原始错误：\n$original'));
    });
  }

  for (final original in [
    '401 Unauthorized: invalid key',
    'request failed: 401 Unauthorized: invalid key',
    'connection failed: 401 Unauthorized: invalid key',
    'list_models request failed: 401 {"error":"invalid key"}',
    'HTTP/1.1 401 Unauthorized',
    'HTTP 401: invalid key',
    'internal: model list fetch error: list_models request failed: 401 invalid key',
    'request failed: Codex token refresh failed: 401 Unauthorized: invalid key',
  ]) {
    test('recognizes the explicit provider status in $original', () {
      expect(apiErrorHttpStatus(original), 401);
    });
  }

  for (final original in [
    'request failed: 4290: model code',
    'request failed: {"code":429,"message":"application error"}',
    'model-403 is unavailable',
    'request failed: {"message":"documentation says HTTP 401"}',
    'connection failed: DNS lookup failed',
    '',
  ]) {
    test('does not invent an explanation for $original', () {
      expect(apiErrorText(original, l10n), original);
    });
  }

  test(
    'classifies the complete HTTP error range and preserves raw responses',
    () {
      for (var status = 400; status < 600; status++) {
        final original = 'request failed: $status: body contains HTTP 401';
        expect(apiErrorHttpStatus(original), status);
        expect(
          apiErrorExplanation(status, l10n),
          isNotNull,
          reason: 'HTTP $status',
        );
        expect(apiErrorText(original, l10n), endsWith(original));
      }
      for (final status in [null, 0, 200, 301, 399, 600]) {
        expect(apiErrorExplanation(status, l10n), isNull);
      }
    },
  );
}
