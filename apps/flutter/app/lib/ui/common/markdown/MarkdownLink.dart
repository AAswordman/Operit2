// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../features/packages/screens/ComposeDslImageSource.dart';
import 'MarkdownFileScope.dart';

/// Supplies the workspace preview opener to Markdown rendered in either pane.
class MarkdownLinkScope extends InheritedWidget {
  const MarkdownLinkScope({
    super.key,
    required this.onOpenFile,
    required super.child,
  });

  final Future<void> Function(String path) onOpenFile;

  static MarkdownLinkScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<MarkdownLinkScope>();

  @override
  bool updateShouldNotify(MarkdownLinkScope oldWidget) =>
      onOpenFile != oldWidget.onOpenFile;
}

/// Extracts a file destination without treating web URLs or anchors as files.
String? markdownFileLinkPath(String source) {
  final value = source.trim();
  if (value.isEmpty || value.startsWith('#') || value.startsWith('//')) {
    return null;
  }
  final uri = Uri.tryParse(value);
  if (uri == null) return null;
  try {
    if (!uri.hasScheme) {
      return uri.path.isEmpty ? null : Uri.decodeComponent(uri.path);
    }
    if (uri.scheme == 'file' && (uri.host.isEmpty || uri.host == 'localhost')) {
      return Uri.decodeComponent(uri.path);
    }
    if (uri.scheme == 'vfs' || uri.scheme == 'operit-vfs') {
      final host = uri.host;
      return Uri.decodeComponent(
        host.isEmpty || host == 'localhost' ? uri.path : '/$host${uri.path}',
      );
    }
    // Retain the existing node-qualified VFS file tool route.
    if (uri.scheme != 'http' && uri.scheme != 'https') {
      return composeDslVfsImagePath(value);
    }
  } on FormatException {
    return null;
  }
  return null;
}

/// Opens a Markdown link. A caller-supplied handler replaces the external opener.
void activateMarkdownLink(String url, void Function(String url)? onLinkClick) {
  final destination = url.trim();
  if (destination.isEmpty) {
    return;
  }
  if (onLinkClick != null) {
    onLinkClick(destination);
    return;
  }
  final uri = Uri.tryParse(destination);
  if (uri == null || (uri.scheme != 'http' && uri.scheme != 'https')) {
    return;
  }
  unawaited(_launchMarkdownLink(uri));
}

Future<void> _launchMarkdownLink(Uri uri) async {
  try {
    final launched = await launchUrl(uri, mode: LaunchMode.externalApplication);
    if (!launched) {
      ClientLogger.w('Cannot open link: $uri', tag: 'MarkdownLink');
    }
  } catch (error, stackTrace) {
    ClientLogger.w(
      'Cannot open link: $uri',
      tag: 'MarkdownLink',
      error: error,
      stackTrace: stackTrace,
    );
  }
}

/// Uses the same file service for downloaded outputs as the chat's renderer.
void openMarkdownFileLink(
  String url,
  GeneratedCoreProxyClients? clients, {
  Future<void> Function(String path)? onOpenFile,
  String? basePath,
}) {
  final filePath = markdownFileLinkPath(url);
  final path = filePath == null
      ? null
      : resolveMarkdownFilePath(filePath, basePath, decodePath: false);
  if (path == null || (clients == null && onOpenFile == null)) {
    activateMarkdownLink(url, null);
    return;
  }
  unawaited(() async {
    try {
      if (onOpenFile != null &&
          !RegExp(r'^[A-Za-z][A-Za-z0-9+.-]*:').hasMatch(path)) {
        await onOpenFile(path);
        return;
      }
      if (clients == null) return;
      final result = await clients.application.aiToolHandler().executeTool(
        tool: core.CoreOperitToolsToolExecutionManagerAiTool(
          name: 'open_file',
          parameters: [
            core.CoreOperitToolsToolExecutionManagerToolParameter(
              name: 'path',
              value: path,
            ),
          ],
        ),
      );
      if (!result.success) {
        throw StateError(result.error ?? 'File could not be opened');
      }
    } catch (error, stackTrace) {
      ClientLogger.w(
        'Cannot open output: $path',
        tag: 'MarkdownLink',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }());
}
