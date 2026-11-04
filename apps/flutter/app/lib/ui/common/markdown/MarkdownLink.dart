// ignore_for_file: file_names

import 'dart:async';

import 'package:url_launcher/url_launcher.dart';

import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../features/packages/screens/ComposeDslImageSource.dart';

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
void openMarkdownFileLink(String url, GeneratedCoreProxyClients? clients) {
  final path = composeDslVfsImagePath(url);
  if (path == null || clients == null) {
    activateMarkdownLink(url, null);
    return;
  }
  unawaited(() async {
    try {
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
