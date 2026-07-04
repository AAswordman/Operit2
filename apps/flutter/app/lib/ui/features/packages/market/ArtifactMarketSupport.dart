// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

const String currentAppVersion = '2.0.0';

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

String formatMarketDate(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '-';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}
