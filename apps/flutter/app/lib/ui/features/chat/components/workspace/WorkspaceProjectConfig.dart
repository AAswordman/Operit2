// ignore_for_file: file_names

import 'dart:convert';

class WorkspaceProjectConfig {
  const WorkspaceProjectConfig({
    required this.projectType,
    required this.title,
    required this.previewUrl,
  });

  final String projectType;
  final String title;
  final String previewUrl;

  factory WorkspaceProjectConfig.fromJsonText(String text) {
    final json = jsonDecode(text) as Map<String, Object?>;
    final preview = json['preview'] as Map<String, Object?>;
    return WorkspaceProjectConfig(
      projectType: json['projectType'] as String,
      title: json['title'] as String,
      previewUrl: preview['url'] as String,
    );
  }
}
