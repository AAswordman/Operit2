// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

const String currentAppVersion = '2.0.0';

String artifactNodeTitle(core_proxy.ArtifactProjectNodeResponse node) {
  return node.displayName.trim().isEmpty ? node.nodeId : node.displayName;
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

class ArtifactIssueRepository {
  const ArtifactIssueRepository({
    required this.type,
    required this.owner,
    required this.repo,
    required this.label,
  });

  final String type;
  final String owner;
  final String repo;
  final String label;
}

ArtifactIssueRepository artifactIssueRepository(String type) {
  return switch (type.trim()) {
    'package' => const ArtifactIssueRepository(
      type: 'package',
      owner: 'AAswordman',
      repo: 'OperitPackageMarket',
      label: 'package-artifact',
    ),
    'script' => const ArtifactIssueRepository(
      type: 'script',
      owner: 'AAswordman',
      repo: 'OperitScriptMarket',
      label: 'script-artifact',
    ),
    final value => throw StateError('Unsupported artifact type: $value'),
  };
}

Future<bool> confirmArtifactNodeCompatibility({
  required BuildContext context,
  required core_proxy.ArtifactProjectDetailResponse project,
  required core_proxy.ArtifactProjectNodeResponse node,
}) async {
  if (isArtifactNodeCompatible(node)) {
    return true;
  }
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('当前软件版本可能不兼容'),
      content: Text(unsupportedArtifactVersionMessage(project, node)),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('仍然继续下载'),
        ),
      ],
    ),
  );
  return confirmed == true;
}

Future<String> runCoreMarketInstall({
  required GeneratedCoreProxyClients clients,
  required String type,
  required String projectId,
  required String nodeId,
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
          normalizedType,
          projectId,
          nodeId,
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

String unsupportedArtifactVersionMessage(
  core_proxy.ArtifactProjectDetailResponse project,
  core_proxy.ArtifactProjectNodeResponse node,
) {
  final name = firstNonBlank(<String>[
    node.displayName,
    node.projectDisplayName,
    project.projectDisplayName,
    node.nodeId,
  ]);
  return '「$name」声明支持的软件版本为 ${supportedVersionLabel(node)}，当前软件版本是 $currentAppVersion。继续下载仍可能失败或不可用。';
}

String supportedVersionLabel(core_proxy.ArtifactProjectNodeResponse node) {
  try {
    final minVersion = _normalizeAppVersionOrNull(node.minSupportedAppVersion);
    final maxVersion = _normalizeAppVersionOrNull(node.maxSupportedAppVersion);
    if (minVersion != null && maxVersion != null) {
      return '$minVersion - $maxVersion';
    }
    if (minVersion != null) {
      return '>= $minVersion';
    }
    if (maxVersion != null) {
      return '<= $maxVersion';
    }
    return 'Any';
  } catch (error, stackTrace) {
    debugPrint(
      'Failed to format supported versions for node=${node.nodeId}: $error\n$stackTrace',
    );
    return 'Invalid';
  }
}

bool isArtifactNodeCompatible(core_proxy.ArtifactProjectNodeResponse node) {
  try {
    return _isAppVersionSupported(
      appVersion: currentAppVersion,
      minSupportedAppVersion: node.minSupportedAppVersion,
      maxSupportedAppVersion: node.maxSupportedAppVersion,
    );
  } catch (error, stackTrace) {
    debugPrint(
      'Failed to evaluate compatibility for node=${node.nodeId}: $error\n$stackTrace',
    );
    return false;
  }
}

bool _isAppVersionSupported({
  required String appVersion,
  required String? minSupportedAppVersion,
  required String? maxSupportedAppVersion,
}) {
  final normalizedCurrent = _normalizeAppVersionOrNull(appVersion);
  if (normalizedCurrent == null) {
    return true;
  }
  final normalizedMin = _normalizeAppVersionOrNull(minSupportedAppVersion);
  final normalizedMax = _normalizeAppVersionOrNull(maxSupportedAppVersion);
  if (normalizedMin != null &&
      _compareAppVersions(normalizedCurrent, normalizedMin) < 0) {
    return false;
  }
  if (normalizedMax != null &&
      _compareAppVersions(normalizedCurrent, normalizedMax) > 0) {
    return false;
  }
  return true;
}

String? _normalizeAppVersionOrNull(String? value) {
  final trimmed = value?.trim() ?? '';
  if (trimmed.isEmpty) {
    return null;
  }
  final match = RegExp(
    r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$',
  ).firstMatch(trimmed);
  if (match == null) {
    return null;
  }
  final build = match.group(4);
  return build == null
      ? '${match.group(1)}.${match.group(2)}.${match.group(3)}'
      : '${match.group(1)}.${match.group(2)}.${match.group(3)}+$build';
}

int _compareAppVersions(String left, String right) {
  final leftParts = _appVersionParts(left);
  final rightParts = _appVersionParts(right);
  for (var index = 0; index < leftParts.length; index += 1) {
    final order = leftParts[index].compareTo(rightParts[index]);
    if (order != 0) {
      return order;
    }
  }
  return 0;
}

List<int> _appVersionParts(String value) {
  final match = RegExp(r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$').firstMatch(value);
  if (match == null) {
    throw StateError('版本格式应为 1.2.3 或 1.2.3+4');
  }
  return <int>[
    int.parse(match.group(1)!),
    int.parse(match.group(2)!),
    int.parse(match.group(3)!),
    int.parse(match.group(4) ?? '0'),
  ];
}

String formatMarketDate(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '-';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}
