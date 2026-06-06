// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import '../WorkspaceTabModels.dart';
import 'WorkspaceFilePreviewActionBar.dart';

class WorkspaceTextPreview extends StatelessWidget {
  const WorkspaceTextPreview({
    super.key,
    required this.tab,
    required this.onOpenBrowser,
  });

  final WorkspaceTab tab;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        WorkspaceFilePreviewActionBar(
          canOpenInBrowser: tab.absolutePath != null,
          onOpenInBrowser: () => onOpenBrowser(localFilePath: tab.absolutePath),
        ),
        Expanded(
          child: WorkspaceTextBody(
            text: tab.fileContent ?? '',
            monospace: true,
          ),
        ),
      ],
    );
  }
}

class WorkspaceMarkdownPreview extends StatelessWidget {
  const WorkspaceMarkdownPreview({
    super.key,
    required this.tab,
    required this.onOpenBrowser,
  });

  final WorkspaceTab tab;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      children: <Widget>[
        WorkspaceFilePreviewActionBar(
          canOpenInBrowser: tab.absolutePath != null,
          onOpenInBrowser: () => onOpenBrowser(localFilePath: tab.absolutePath),
        ),
        Expanded(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(14),
            child: StreamMarkdownRenderer(
              content: tab.fileContent ?? '',
              isStreaming: false,
              textColor: theme.colorScheme.onSurface,
              backgroundColor: Colors.transparent,
              onLinkClick: (url) => onOpenBrowser(url: url),
            ),
          ),
        ),
      ],
    );
  }
}

class WorkspaceTextBody extends StatelessWidget {
  const WorkspaceTextBody({
    super.key,
    required this.text,
    required this.monospace,
  });

  final String text;
  final bool monospace;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: theme.colorScheme.surface.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.025,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: SelectableText(
          text,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurface,
            fontFamily: monospace ? 'monospace' : null,
            height: 1.45,
          ),
        ),
      ),
    );
  }
}
