// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../theme/OperitGlassSurface.dart';

class WorkspaceFilePreviewActionBar extends StatelessWidget {
  const WorkspaceFilePreviewActionBar({
    super.key,
    required this.canOpenInBrowser,
    required this.onOpenInBrowser,
    this.openInBrowserTooltip,
    this.canOpenWorkspaceFile = false,
    this.onOpenWorkspaceFile,
  });

  final bool canOpenInBrowser;
  final VoidCallback onOpenInBrowser;
  final String? openInBrowserTooltip;
  final bool canOpenWorkspaceFile;
  final VoidCallback? onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return OperitGlassSurface(
      color: theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.58),
      layer: OperitGlassSurfaceLayer.control,
      border: Border(
        bottom: BorderSide(
          color: theme.colorScheme.outlineVariant.withValues(alpha: 0.36),
        ),
      ),
      child: SizedBox(
        height: 34,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: <Widget>[
            if (canOpenWorkspaceFile)
              IconButton(
                tooltip: l10n.openFile,
                onPressed: onOpenWorkspaceFile,
                icon: const Icon(Icons.open_in_new, size: 18),
                visualDensity: VisualDensity.compact,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints.tightFor(
                  width: 32,
                  height: 32,
                ),
              ),
            IconButton(
              tooltip: openInBrowserTooltip ?? l10n.openInBrowser,
              onPressed: canOpenInBrowser ? onOpenInBrowser : null,
              icon: const Icon(Icons.open_in_browser, size: 18),
              visualDensity: VisualDensity.compact,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints.tightFor(width: 32, height: 32),
            ),
            const SizedBox(width: 6),
          ],
        ),
      ),
    );
  }
}
