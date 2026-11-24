// ignore_for_file: file_names

import '../../l10n/generated/app_localizations.dart';

final _apiHttpStatus = RegExp(
  r'^\s*(?:[A-Za-z_][\w.]*:\s*)*'
  r'(?:(?:(?:list_models )?request failed|connection failed|model list fetch error|connection test error|'
  r'Codex (?:device authorization|device token polling|token refresh|token exchange) failed):\s*)*'
  r'(?:HTTP(?:/\d(?:\.\d)?)?\s+)?([45]\d{2})(?=\s|:|$)',
  caseSensitive: false,
);

/// Reads an explicit HTTP status from the provider's original request error.
int? apiErrorHttpStatus(String error) {
  final match = _apiHttpStatus.firstMatch(error);
  return match == null ? null : int.parse(match.group(1)!);
}

/// Presentation categories shared by HTTP and structured provider errors.
enum ApiErrorCategory {
  request,
  key,
  billing,
  access,
  address,
  timeout,
  limit,
  server,
  network,
  info,
  unknown,
}

/// A single explanation shape used by every API error surface.
typedef ApiErrorExplanation = ({
  String title,
  String message,
  ApiErrorCategory category,
});

/// Explains HTTP failures, including the explicit reasoning-content 400 rule.
ApiErrorExplanation? apiErrorExplanation(
  int? status,
  AppLocalizations l10n, {
  String? original,
}) {
  if (status == 400 &&
      original != null &&
      original.contains('reasoning_content')) {
    return (
      title: l10n.apiErrorReasoningContentTitle,
      message: l10n.apiErrorReasoningContentMessage,
      category: ApiErrorCategory.request,
    );
  }
  final description = switch (status) {
    400 || 422 => (
      l10n.apiErrorInvalidParametersTitle,
      l10n.apiErrorInvalidParametersMessage,
      ApiErrorCategory.request,
    ),
    401 => (
      l10n.apiErrorInvalidKeyTitle,
      l10n.apiErrorInvalidKeyMessage,
      ApiErrorCategory.key,
    ),
    402 => (
      l10n.apiErrorBillingTitle,
      l10n.apiErrorBillingMessage,
      ApiErrorCategory.billing,
    ),
    403 => (
      l10n.apiErrorBlockedTitle,
      l10n.apiErrorBlockedMessage,
      ApiErrorCategory.access,
    ),
    404 || 410 => (
      l10n.apiErrorNotFoundTitle,
      l10n.apiErrorNotFoundMessage,
      ApiErrorCategory.address,
    ),
    405 || 501 => (
      l10n.apiErrorUnsupportedMethodTitle,
      l10n.apiErrorUnsupportedMethodMessage,
      ApiErrorCategory.request,
    ),
    406 || 415 => (
      l10n.apiErrorUnsupportedContentTitle,
      l10n.apiErrorUnsupportedContentMessage,
      ApiErrorCategory.request,
    ),
    407 => (
      l10n.apiErrorProxyAuthTitle,
      l10n.apiErrorProxyAuthMessage,
      ApiErrorCategory.key,
    ),
    408 || 504 || 522 || 524 => (
      l10n.apiErrorTimeoutTitle,
      l10n.apiErrorTimeoutMessage,
      ApiErrorCategory.timeout,
    ),
    409 => (
      l10n.apiErrorConflictTitle,
      l10n.apiErrorConflictMessage,
      ApiErrorCategory.request,
    ),
    411 => (
      l10n.apiErrorLengthRequiredTitle,
      l10n.apiErrorLengthRequiredMessage,
      ApiErrorCategory.request,
    ),
    413 || 414 || 431 => (
      l10n.apiErrorTooLargeTitle,
      l10n.apiErrorTooLargeMessage,
      ApiErrorCategory.request,
    ),
    423 => (
      l10n.apiErrorLockedTitle,
      l10n.apiErrorLockedMessage,
      ApiErrorCategory.access,
    ),
    425 => (
      l10n.apiErrorTooEarlyTitle,
      l10n.apiErrorTooEarlyMessage,
      ApiErrorCategory.limit,
    ),
    426 => (
      l10n.apiErrorUpgradeRequiredTitle,
      l10n.apiErrorUpgradeRequiredMessage,
      ApiErrorCategory.request,
    ),
    429 => (
      l10n.apiErrorRateLimitedTitle,
      l10n.apiErrorRateLimitedMessage,
      ApiErrorCategory.limit,
    ),
    451 => (
      l10n.apiErrorLegalRestrictionTitle,
      l10n.apiErrorLegalRestrictionMessage,
      ApiErrorCategory.access,
    ),
    499 => (
      l10n.apiErrorInterruptedTitle,
      l10n.apiErrorInterruptedMessage,
      ApiErrorCategory.network,
    ),
    500 => (
      l10n.apiErrorInternalTitle,
      l10n.apiErrorInternalMessage,
      ApiErrorCategory.server,
    ),
    502 || 520 => (
      l10n.apiErrorGatewayTitle,
      l10n.apiErrorGatewayMessage,
      ApiErrorCategory.server,
    ),
    503 || 529 => (
      l10n.apiErrorUnavailableTitle,
      l10n.apiErrorUnavailableMessage,
      ApiErrorCategory.server,
    ),
    521 || 523 => (
      l10n.apiErrorOriginTitle,
      l10n.apiErrorOriginMessage,
      ApiErrorCategory.server,
    ),
    525 || 526 => (
      l10n.apiErrorTlsTitle,
      l10n.apiErrorTlsMessage,
      ApiErrorCategory.server,
    ),
    507 => (
      l10n.apiErrorStorageTitle,
      l10n.apiErrorStorageMessage,
      ApiErrorCategory.server,
    ),
    final int code when code >= 400 && code < 500 => (
      l10n.apiErrorClientTitle,
      l10n.apiErrorClientMessage,
      ApiErrorCategory.request,
    ),
    final int code when code >= 500 && code < 600 => (
      l10n.apiErrorServerTitle,
      l10n.apiErrorServerMessage,
      ApiErrorCategory.server,
    ),
    _ => null,
  };
  if (description == null) return null;
  final (title, message, category) = description;
  return (title: title, message: message, category: category);
}

/// Prepends a recognized explanation while preserving the entire original error.
String apiErrorText(String original, AppLocalizations l10n) {
  final status = apiErrorHttpStatus(original);
  if (status == null) return original;
  final explanation = apiErrorExplanation(status, l10n, original: original);
  if (explanation == null) return original;
  return '${l10n.apiErrorHttpTitle(status, explanation.title)}\n'
      '${explanation.message}\n\n${l10n.apiErrorOriginal(original)}';
}
