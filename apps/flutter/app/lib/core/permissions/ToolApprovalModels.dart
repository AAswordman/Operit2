// ignore_for_file: file_names

import 'dart:convert';

class ToolApprovalRequest {
  const ToolApprovalRequest({
    required this.tool,
    required this.description,
    required this.requestedAtMillis,
    this.remoteRequestId,
  });

  factory ToolApprovalRequest.fromJson(Map<String, Object?> json) {
    return ToolApprovalRequest(
      tool: ToolApprovalTool.fromJson(json['tool'] as Map<String, Object?>),
      description: json['description'] as String,
      requestedAtMillis: json['requestedAtMillis'] as int,
      remoteRequestId: json['remoteRequestId'] as String?,
    );
  }

  static ToolApprovalRequest? decode(String? text) {
    if (text == null) {
      return null;
    }
    final value = jsonDecode(text);
    if (value == null) {
      return null;
    }
    return ToolApprovalRequest.fromJson(value as Map<String, Object?>);
  }

  final ToolApprovalTool tool;
  final String description;
  final int requestedAtMillis;
  final String? remoteRequestId;
}

class ToolApprovalTool {
  const ToolApprovalTool({required this.name, required this.parameters});

  factory ToolApprovalTool.fromJson(Map<String, Object?> json) {
    return ToolApprovalTool(
      name: json['name'] as String,
      parameters: (json['parameters'] as List<Object?>)
          .map(
            (item) =>
                ToolApprovalParameter.fromJson(item as Map<String, Object?>),
          )
          .toList(growable: false),
    );
  }

  final String name;
  final List<ToolApprovalParameter> parameters;
}

class ToolApprovalParameter {
  const ToolApprovalParameter({required this.name, required this.value});

  factory ToolApprovalParameter.fromJson(Map<String, Object?> json) {
    return ToolApprovalParameter(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  final String name;
  final String value;
}

enum ToolApprovalResult {
  allow('ALLOW'),
  deny('DENY'),
  alwaysAllow('ALWAYS_ALLOW');

  const ToolApprovalResult(this.wireName);

  final String wireName;
}
