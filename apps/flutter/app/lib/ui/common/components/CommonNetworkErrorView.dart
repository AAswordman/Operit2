// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';
import '../ApiErrorText.dart';

class CommonNetworkErrorView extends StatelessWidget {
  /// Creates a model configuration error with its original details.
  const CommonNetworkErrorView({super.key, this.errorDetails, this.errorText});

  final core_proxy.CoreProxyErrorDetails? errorDetails;
  final String? errorText;

  /// Builds a readable explanation followed by selectable original error text.
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
                      SelectableText(
                        l10n.apiErrorOriginal(summary.detail!),
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
  /// Creates an explanation and optional original error details.
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
    AppLocalizations l10n,
  ) {
    final original = text ?? details?.message;
    final statusCode = switch (details?.httpStatus) {
      final int status => status,
      null => apiErrorHttpStatus(original ?? ''),
    };
    final httpExplanation = apiErrorExplanation(
      statusCode,
      l10n,
      original: original,
    );
    final explanation = switch ((
      httpExplanation,
      details?.variant,
      details?.kind,
    )) {
      (final ApiErrorExplanation http, _, _) => (
        title: l10n.apiErrorHttpTitle(statusCode!, http.title),
        message: http.message,
        category: http.category,
      ),
      (_, 'ModelAlreadyExists', _) => (
        title: l10n.apiErrorModelExistsTitle,
        message: l10n.apiErrorModelExistsMessage(
          details!.stringField('modelId')!,
          details.stringField('providerName')!,
        ),
        category: ApiErrorCategory.info,
      ),
      (_, 'ModelListFetch', _) => (
        title: l10n.apiErrorModelListTitle,
        message: l10n.apiErrorModelListMessage,
        category: ApiErrorCategory.network,
      ),
      (_, _, 'network') => (
        title: l10n.apiErrorNetworkTitle,
        message: l10n.apiErrorNetworkMessage,
        category: ApiErrorCategory.network,
      ),
      _ => (
        title: l10n.apiErrorModelConfigTitle,
        message: l10n.apiErrorModelConfigMessage,
        category: ApiErrorCategory.unknown,
      ),
    };
    return NetworkErrorSummary(
      title: explanation.title,
      message: explanation.message,
      detail: explanation.category == ApiErrorCategory.info ? null : original,
      icon: switch (explanation.category) {
        ApiErrorCategory.request => Icons.tune_rounded,
        ApiErrorCategory.key => Icons.key_off_rounded,
        ApiErrorCategory.billing => Icons.account_balance_wallet_outlined,
        ApiErrorCategory.access => Icons.lock_outline_rounded,
        ApiErrorCategory.address => Icons.link_off_rounded,
        ApiErrorCategory.timeout => Icons.timer_outlined,
        ApiErrorCategory.limit => Icons.hourglass_top_rounded,
        ApiErrorCategory.server => Icons.cloud_off_rounded,
        ApiErrorCategory.network => Icons.wifi_off_rounded,
        ApiErrorCategory.info => Icons.info_outline_rounded,
        ApiErrorCategory.unknown => Icons.error_outline_rounded,
      },
    );
  }
}
