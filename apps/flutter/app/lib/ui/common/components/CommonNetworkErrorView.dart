import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';

class CommonNetworkErrorView extends StatelessWidget {
  const CommonNetworkErrorView({super.key, this.errorDetails, this.errorText});

  final core_proxy.CoreProxyErrorDetails? errorDetails;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final l10n = AppLocalizations.of(context)!;
    final summary = NetworkErrorSummary.fromDetails(
      errorDetails,
      errorText,
      l10n,
      suppressChineseOnlyDetail:
          Localizations.localeOf(context).languageCode == 'ja',
    );

    return Semantics(
      liveRegion: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.errorContainer.withValues(alpha: 0.34),
          borderRadius: BorderRadius.circular(18),
          border: Border.all(color: colorScheme.error.withValues(alpha: 0.18)),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(summary.icon, size: 20, color: colorScheme.error),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      summary.title,
                      style: textTheme.titleSmall?.copyWith(
                        color: colorScheme.onErrorContainer,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      summary.message,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onErrorContainer,
                        height: 1.35,
                      ),
                    ),
                    if (summary.detail != null) ...<Widget>[
                      const SizedBox(height: 6),
                      Text(
                        summary.detail!,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onErrorContainer.withValues(
                            alpha: 0.74,
                          ),
                          height: 1.35,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class NetworkErrorSummary {
  const NetworkErrorSummary({
    required this.title,
    required this.message,
    required this.icon,
    this.detail,
  });

  final String title;
  final String message;
  final String? detail;
  final IconData icon;

  /// Converts structured runtime error details into a visible summary.
  factory NetworkErrorSummary.fromDetails(
    core_proxy.CoreProxyErrorDetails? details,
    String? text,
    AppLocalizations l10n, {
    required bool suppressChineseOnlyDetail,
  }) {
    final statusCode = details?.httpStatus;
    final remoteMessage =
        details?.remoteMessage ??
        details?.stringField('value') ??
        details?.message ??
        text;
    final visibleDetail = suppressChineseOnlyDetail
        ? _hideChineseOnlyDetail(remoteMessage)
        : remoteMessage;

    if (statusCode == 400) {
      return NetworkErrorSummary(
        title: l10n.networkErrorBadRequestTitle,
        message: l10n.networkErrorBadRequestMessage,
        detail: visibleDetail,
        icon: Icons.tune_rounded,
      );
    }

    if (statusCode == 401) {
      return NetworkErrorSummary(
        title: l10n.networkErrorUnauthorizedTitle,
        message: l10n.networkErrorUnauthorizedMessage,
        detail: visibleDetail,
        icon: Icons.key_off_rounded,
      );
    }

    if (statusCode == 403) {
      return NetworkErrorSummary(
        title: l10n.networkErrorForbiddenTitle,
        message: l10n.networkErrorForbiddenMessage,
        detail: visibleDetail,
        icon: Icons.lock_outline_rounded,
      );
    }

    if (statusCode == 404) {
      return NetworkErrorSummary(
        title: l10n.networkErrorNotFoundTitle,
        message: l10n.networkErrorNotFoundMessage,
        detail: visibleDetail,
        icon: Icons.link_off_rounded,
      );
    }

    if (statusCode == 429) {
      return NetworkErrorSummary(
        title: l10n.networkErrorRateLimitedTitle,
        message: l10n.networkErrorRateLimitedMessage,
        detail: visibleDetail,
        icon: Icons.hourglass_top_rounded,
      );
    }

    if (statusCode != null && statusCode >= 500) {
      return NetworkErrorSummary(
        title: l10n.networkErrorServerTitle,
        message: l10n.networkErrorServerMessage,
        detail: visibleDetail,
        icon: Icons.cloud_off_rounded,
      );
    }

    if (details?.variant == 'ModelListFetch') {
      return NetworkErrorSummary(
        title: l10n.networkErrorModelListTitle,
        message: l10n.networkErrorModelListMessage,
        detail: visibleDetail,
        icon: Icons.wifi_off_rounded,
      );
    }

    if (details?.kind == 'network') {
      return NetworkErrorSummary(
        title: l10n.networkErrorConnectionTitle,
        message: l10n.networkErrorConnectionMessage,
        detail: visibleDetail,
        icon: Icons.wifi_off_rounded,
      );
    }

    if (details?.variant == 'ModelAlreadyExists') {
      final duplicateDetails = details!;
      final modelId = duplicateDetails.stringField('modelId')!;
      final providerName = duplicateDetails.stringField('providerName')!;
      return NetworkErrorSummary(
        title: l10n.networkErrorDuplicateModelTitle,
        message: l10n.networkErrorDuplicateModelMessage(modelId, providerName),
        icon: Icons.info_outline_rounded,
      );
    }

    return NetworkErrorSummary(
      title: l10n.networkErrorDefaultTitle,
      message: l10n.networkErrorDefaultMessage,
      detail: visibleDetail,
      icon: Icons.error_outline_rounded,
    );
  }
}

/// Suppresses raw Chinese-only provider/runtime details in localized UI while
/// preserving English and Japanese diagnostics.
String? _hideChineseOnlyDetail(String? detail) {
  if (detail == null || detail.trim().isEmpty) {
    return null;
  }
  final hasHan = RegExp(r'[\u3400-\u9fff]').hasMatch(detail);
  final hasKana = RegExp(r'[\u3040-\u30ff]').hasMatch(detail);
  return hasHan && !hasKana ? null : detail;
}
