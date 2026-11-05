// ignore_for_file: file_names
import 'package:flutter/widgets.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Resolves historical links against the directory saved on their message.
String resolveMarkdownFilePath(String source, String? base) {
  final value = source.trim();
  if (value.isEmpty || base == null || base.isEmpty) return value;
  if (value.startsWith('#') ||
      value.startsWith('/') ||
      RegExp(r'^[A-Za-z][A-Za-z0-9+.-]*:').hasMatch(value)) {
    return value;
  }
  final relative = Uri.tryParse(value);
  if (relative == null) return value;
  final parts = base.split('/').where((p) => p.isNotEmpty).toList();
  for (final part in Uri.decodeComponent(relative.path).split('/')) {
    if (part.isEmpty || part == '.') continue;
    if (part == '..') {
      if (parts.isEmpty) return value;
      parts.removeLast();
    } else {
      parts.add(part);
    }
  }
  return '/${parts.join('/')}';
}

/// Returns the directory context owned by this message, independent of live text.
({String? basePath, String? nodeId}) markdownFileContextForParts(
  List<core.MessagePart> parts,
) {
  for (final part in parts) {
    final base = part.attributes['file_base_path']?.trim();
    if (base != null && base.isNotEmpty) {
      return (basePath: base, nodeId: part.attributes['file_node_id']);
    }
  }
  return (basePath: null, nodeId: null);
}

class MarkdownFileScope extends InheritedWidget {
  const MarkdownFileScope({
    super.key,
    required this.basePath,
    required this.nodeId,
    required this.clients,
    required super.child,
  });
  factory MarkdownFileScope.forParts({
    Key? key,
    required List<core.MessagePart> parts,
    required GeneratedCoreProxyClients clients,
    required Widget child,
    String? fallbackBasePath,
  }) {
    final context = markdownFileContextForParts(parts);
    return MarkdownFileScope(
      key: key,
      basePath: context.basePath ?? fallbackBasePath,
      nodeId: context.nodeId,
      clients: clients,
      child: child,
    );
  }
  final String? basePath;
  final String? nodeId;
  final GeneratedCoreProxyClients clients;
  static MarkdownFileScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<MarkdownFileScope>();
  String resolve(String source) => resolveMarkdownFilePath(source, basePath);
  String? sourcePath(String path) =>
      nodeId == null || !path.startsWith('/') ? null : '$nodeId:$path';
  @override
  bool updateShouldNotify(MarkdownFileScope oldWidget) =>
      basePath != oldWidget.basePath ||
      nodeId != oldWidget.nodeId ||
      clients != oldWidget.clients;
}
