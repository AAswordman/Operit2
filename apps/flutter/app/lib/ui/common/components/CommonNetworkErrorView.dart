import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class CommonNetworkErrorView extends StatelessWidget {
  const CommonNetworkErrorView({
    super.key,
    this.errorDetails,
    this.errorText,
  });

  final core_proxy.CoreProxyErrorDetails? errorDetails;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final summary = NetworkErrorSummary.fromDetails(errorDetails, errorText);

    return Semantics(
      liveRegion: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.errorContainer.withValues(alpha: 0.34),
          borderRadius: BorderRadius.circular(18),
          border: Border.all(
            color: colorScheme.error.withValues(alpha: 0.18),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(
                summary.icon,
                size: 20,
                color: colorScheme.error,
              ),
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

  factory NetworkErrorSummary.fromDetails(
    core_proxy.CoreProxyErrorDetails? details,
    String? text,
  ) {
    final statusCode = details?.httpStatus;
    final remoteMessage =
        details?.remoteMessage ??
        details?.stringField('value') ??
        details?.message ??
        text;

    if (statusCode == 400) {
      return NetworkErrorSummary(
        title: '请求参数有误',
        message: '服务端拒绝了这次模型列表请求，请检查服务地址和供应商是否匹配。',
        detail: remoteMessage,
        icon: Icons.tune_rounded,
      );
    }

    if (statusCode == 401) {
      return NetworkErrorSummary(
        title: '密钥验证失败',
        message: '访问密钥没有通过供应商验证，请重新粘贴完整密钥后再拉取模型。',
        detail: remoteMessage,
        icon: Icons.key_off_rounded,
      );
    }

    if (statusCode == 403) {
      return NetworkErrorSummary(
        title: '没有访问权限',
        message: '当前密钥无权访问该供应商接口，请确认账号权限和模型服务开通状态。',
        detail: remoteMessage,
        icon: Icons.lock_outline_rounded,
      );
    }

    if (statusCode == 404) {
      return NetworkErrorSummary(
        title: '服务地址不可用',
        message: '当前服务地址没有找到模型列表接口，请检查地址路径是否正确。',
        detail: remoteMessage,
        icon: Icons.link_off_rounded,
      );
    }

    if (statusCode == 429) {
      return NetworkErrorSummary(
        title: '请求过于频繁',
        message: '供应商限制了当前请求频率，请稍后再拉取模型。',
        detail: remoteMessage,
        icon: Icons.hourglass_top_rounded,
      );
    }

    if (statusCode != null && statusCode >= 500) {
      return NetworkErrorSummary(
        title: '供应商服务异常',
        message: '供应商暂时无法处理模型列表请求，请稍后再试。',
        detail: remoteMessage,
        icon: Icons.cloud_off_rounded,
      );
    }

    if (details?.variant == 'ModelListFetch') {
      return NetworkErrorSummary(
        title: '模型列表拉取失败',
        message: '没有拿到供应商返回的模型列表，请检查服务地址、访问密钥和网络连接。',
        detail: remoteMessage,
        icon: Icons.wifi_off_rounded,
      );
    }

    if (details?.kind == 'network') {
      return NetworkErrorSummary(
        title: '网络连接失败',
        message: '无法连接到模型供应商，请检查网络连接和服务地址。',
        detail: remoteMessage,
        icon: Icons.wifi_off_rounded,
      );
    }

    return NetworkErrorSummary(
      title: '模型配置失败',
      message: '拉取可用模型时出现异常，请检查供应商、服务地址和访问密钥。',
      detail: remoteMessage,
      icon: Icons.error_outline_rounded,
    );
  }
}
