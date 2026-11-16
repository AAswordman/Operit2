// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Reads raw bytes through the owning Core's file service.
/// Core resolves the original path against its filesystem and storage roots.
Future<Uint8List> readRuntimeFileBytes(
  GeneratedCoreProxyClients clients,
  String path,
) async {
  final result = await clients.application.aiToolHandler().executeTool(
    tool: core_proxy.CoreOperitToolsToolExecutionManagerAiTool(
      name: 'read_file_binary',
      parameters: [
        core_proxy.CoreOperitToolsToolExecutionManagerToolParameter(
          name: 'path',
          value: path,
        ),
      ],
    ),
  );
  if (!result.success) {
    throw StateError(
      'Failed to read file $path: ${result.error ?? 'Core file read failed'}',
    );
  }
  final data = result.result.value;
  if (data is! core_proxy.BinaryFileContentData) {
    throw StateError('Core did not return binary file content for $path');
  }
  try {
    return base64Decode(data.contentBase64);
  } on FormatException catch (error) {
    throw FormatException(
      'Invalid Base64 file content for $path: ${error.message}',
    );
  }
}
