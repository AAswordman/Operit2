// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../theme/OperitGlassSurface.dart';
import 'WorkspaceOverviewModels.dart';

class WorkspaceHomeContent extends StatelessWidget {
  /// Creates the workspace home content surface.
  const WorkspaceHomeContent({
    super.key,
    required this.workspacePath,
    required this.workspaceUsage,
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
    required this.onUnbindWorkspace,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
    required this.onOpenBrowser,
  });

  final String? workspacePath;
  final WorkspaceOverviewUsage workspaceUsage;
  final ValueListenable<int> terminalSessionCountListenable;
  final ValueListenable<int> browserSessionCountListenable;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;
  final VoidCallback onUnbindWorkspace;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;
  final VoidCallback onOpenBrowser;

  /// Builds the workspace home tab with overview and common actions.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final boundWorkspacePath = workspacePath?.trim();
    final boundWorkspaceName = workspaceUsage.workspaceName?.trim();
    final hasBoundWorkspace =
        (boundWorkspacePath != null && boundWorkspacePath.isNotEmpty) ||
        (boundWorkspaceName != null && boundWorkspaceName.isNotEmpty);
    return ListView(
      padding: const EdgeInsets.fromLTRB(14, 14, 14, 14),
      children: <Widget>[
        _WorkspaceStatusSummary(
          workspacePath: boundWorkspacePath,
          workspaceUsage: workspaceUsage,
          terminalSessionCountListenable: terminalSessionCountListenable,
          browserSessionCountListenable: browserSessionCountListenable,
          onOpenFolder: onOpenFolder,
          onAddFolder: onAddFolder,
          onOpenTerminalSessions: onOpenTerminalSessions,
          onOpenBrowserSessions: onOpenBrowserSessions,
        ),
        const SizedBox(height: 8),
        if (hasBoundWorkspace)
          _WorkspacePrimaryAction(
            icon: Icons.link_off,
            title: l10n.workspaceUnbindTitle,
            subtitle: l10n.workspaceUnbindDescription,
            onTap: onUnbindWorkspace,
          ),
        if (!hasBoundWorkspace) ...[
          _WorkspacePrimaryAction(
            icon: Icons.create_new_folder,
            title: l10n.workspaceCreateTitle,
            subtitle: l10n.workspaceCreateDescription,
            onTap: onCreateWorkspace,
          ),
          const SizedBox(height: 8),
          _WorkspacePrimaryAction(
            icon: Icons.folder_open,
            title: l10n.workspaceBindExistingTitle,
            subtitle: l10n.workspaceBindExistingDescription,
            onTap: onChooseExistingWorkspace,
          ),
        ],
        const SizedBox(height: 8),
        _WorkspacePrimaryAction(
          icon: Icons.play_arrow,
          title: l10n.openTerminal,
          subtitle: l10n.openTerminalDescription,
          onTap: onOpenTerminal,
        ),
        const SizedBox(height: 8),
        _WorkspacePrimaryAction(
          icon: Icons.public,
          title: l10n.openBrowser,
          subtitle: l10n.openBrowserDescription,
          onTap: onOpenBrowser,
        ),
      ],
    );
  }
}

class _WorkspaceStatusSummary extends StatelessWidget {
  /// Creates the workspace status summary card.
  const _WorkspaceStatusSummary({
    required this.workspacePath,
    required this.workspaceUsage,
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
  });

  final String? workspacePath;
  final WorkspaceOverviewUsage workspaceUsage;
  final ValueListenable<int> terminalSessionCountListenable;
  final ValueListenable<int> browserSessionCountListenable;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;

  /// Builds the workspace overview card and registered contribution avatar strip.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final normalizedPath = workspacePath?.trim();
    final workspaceName = workspaceUsage.workspaceName?.trim();
    final hasWorkspace =
        normalizedPath != null && normalizedPath.isNotEmpty ||
        workspaceName != null && workspaceName.isNotEmpty;
    final title = workspaceName != null && workspaceName.isNotEmpty
        ? workspaceName
        : '工作区总览';
    final subtitle = hasWorkspace ? '工作区总览' : '当前对话未绑定工作区';
    return OperitGlassSurface(
      color: Colors.transparent,
      transparentAlpha: 0,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.34),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(14, 13, 14, 14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Container(
                  width: 46,
                  height: 46,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: colorScheme.surfaceContainerHighest.withValues(
                      alpha: 0.52,
                    ),
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(
                      color: colorScheme.outlineVariant.withValues(alpha: 0.28),
                    ),
                  ),
                  child: Icon(
                    Icons.work_outline,
                    size: 25,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                      const SizedBox(height: 2),
                      if (hasWorkspace)
                        _WorkspaceContributionAvatarStrip(
                          contributions: workspaceUsage.contributions,
                        )
                      else
                        Text(
                          subtitle,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            _WorkspaceContributionStateNotice(
              contributions: workspaceUsage.contributions,
            ),
            const SizedBox(height: 8),
            ValueListenableBuilder<int>(
              valueListenable: terminalSessionCountListenable,
              builder: (context, terminalSessionCount, child) {
                return ValueListenableBuilder<int>(
                  valueListenable: browserSessionCountListenable,
                  builder: (context, browserSessionCount, child) {
                    return Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          _workspaceSummaryText(workspaceUsage),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 6),
                        Wrap(
                          spacing: 8,
                          runSpacing: 5,
                          children: <Widget>[
                            _WorkspaceSessionButton(
                              icon: Icons.terminal,
                              label: '$terminalSessionCount 个终端',
                              onTap: onOpenTerminalSessions,
                            ),
                            _WorkspaceSessionButton(
                              icon: Icons.public,
                              label: '$browserSessionCount 个浏览器',
                              onTap: onOpenBrowserSessions,
                            ),
                          ],
                        ),
                      ],
                    );
                  },
                );
              },
            ),
            if (hasWorkspace) ...<Widget>[
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      '挂载文件夹',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: colorScheme.onSurface,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  SizedBox.square(
                    dimension: 28,
                    child: IconButton(
                      tooltip: '挂载新文件夹',
                      padding: EdgeInsets.zero,
                      visualDensity: VisualDensity.compact,
                      onPressed: onAddFolder,
                      icon: const Icon(Icons.add, size: 18),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              _WorkspaceMountedFolderStack(
                folders: workspaceUsage.mountedFolders,
                loading: workspaceUsage.mountedFoldersLoading,
                errorMessage: workspaceUsage.mountedFoldersError,
                onOpenFolder: onOpenFolder,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _WorkspaceContributionAvatarStrip extends StatelessWidget {
  /// Creates the compact avatar strip for successfully loaded contribution usage.
  const _WorkspaceContributionAvatarStrip({required this.contributions});

  final WorkspaceContributionsState contributions;

  /// Preserves catalog order for equal counts without assigning unused sections.
  @override
  Widget build(BuildContext context) {
    final state = contributions;
    if (state is! WorkspaceContributionsReady || state.usages.isEmpty) {
      return const SizedBox(height: 2);
    }
    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Wrap(
        spacing: 4,
        runSpacing: 4,
        children: <Widget>[
          for (final usage in state.usages)
            _WorkspaceContributionAvatar(
              key: ValueKey(usage.identity),
              usage: usage,
            ),
        ],
      ),
    );
  }
}

class _WorkspaceContributionStateNotice extends StatelessWidget {
  /// Creates a visible status for pending or failed contribution requests.
  const _WorkspaceContributionStateNotice({required this.contributions});

  final WorkspaceContributionsState contributions;

  /// Displays the real failure instead of presenting an empty successful catalog.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final Widget status;
    switch (contributions) {
      case WorkspaceContributionsReady():
        return const SizedBox.shrink();
      case WorkspaceContributionsLoading():
        status = _WorkspaceInlineState(
          icon: Icons.sync,
          text: '正在读取工作区贡献项',
          progress: true,
          color: colors.onSurfaceVariant,
        );
      case WorkspaceContributionsFailed(:final error):
        status = _WorkspaceInlineState(
          icon: Icons.error_outline,
          text: '工作区贡献项加载失败：$error',
          color: colors.error,
        );
    }
    return Padding(padding: const EdgeInsets.only(top: 8), child: status);
  }
}

class _WorkspaceContributionAvatar extends StatefulWidget {
  /// Creates one plugin-owned preview affordance in the workspace title area.
  const _WorkspaceContributionAvatar({super.key, required this.usage});

  final WorkspaceContributionUsage usage;

  /// Owns the preview request so repeated taps cannot start overlapping dialogs.
  @override
  State<_WorkspaceContributionAvatar> createState() =>
      _WorkspaceContributionAvatarState();
}

class _WorkspaceContributionAvatarState
    extends State<_WorkspaceContributionAvatar> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  bool _presenting = false;

  /// Delegates the untouched preview action to its registered owning route.
  Future<void> _presentPreview() async {
    if (_presenting) {
      return;
    }
    setState(() => _presenting = true);
    try {
      await widget.usage.preview.present(context: context, clients: _clients);
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('贡献项预览失败：$error')));
    } finally {
      if (mounted) {
        setState(() => _presenting = false);
      }
    }
  }

  /// Keeps the original circular avatar dimensions and exposes a labeled action.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final usage = widget.usage;
    final label = '${usage.title} · ${usage.conversationCount} 个对话';
    return Tooltip(
      message: label,
      waitDuration: const Duration(milliseconds: 450),
      child: Semantics(
        button: true,
        label: label,
        enabled: !_presenting,
        child: Material(
          shape: CircleBorder(
            side: BorderSide(
              color: colors.surface.withValues(alpha: 0.90),
              width: 1.4,
            ),
          ),
          color: colors.primaryContainer,
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            customBorder: const CircleBorder(),
            onTap: _presenting ? null : () => unawaited(_presentPreview()),
            child: SizedBox.square(
              dimension: 22,
              child: _WorkspaceContributionAvatarImage(
                avatarUri: usage.avatarUri,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceContributionAvatarImage extends StatefulWidget {
  /// Creates a display-only image backed by the existing runtime asset host.
  const _WorkspaceContributionAvatarImage({required this.avatarUri});

  final String? avatarUri;

  /// Retains the image request until the plugin changes its display URI.
  @override
  State<_WorkspaceContributionAvatarImage> createState() =>
      _WorkspaceContributionAvatarImageState();
}

class _WorkspaceContributionAvatarImageState
    extends State<_WorkspaceContributionAvatarImage> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  Future<Uint8List>? _bytes;

  /// Starts the request for the exact display URI published by the contribution.
  @override
  void initState() {
    super.initState();
    _loadImage();
  }

  /// Replaces image requests when the authoritative display URI changes.
  @override
  void didUpdateWidget(covariant _WorkspaceContributionAvatarImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.avatarUri != widget.avatarUri) {
      _loadImage();
    }
  }

  /// Reads the runtime asset without searching character stores or platform paths.
  Future<Uint8List> _readImage(String uri) async {
    final encoded = await _clients.repositoryRuntimeStorageRepository
        .readBase64(path: uri);
    if (encoded == null) {
      throw StateError('The contribution avatar asset is missing: $uri');
    }
    return base64Decode(encoded);
  }

  /// Uses explicit image absence only when the catalog publishes a null URI.
  void _loadImage() {
    final uri = widget.avatarUri;
    _bytes = uri == null ? null : _readImage(uri);
  }

  /// Displays loading and asset errors without synthesizing an avatar identity.
  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    if (bytes == null) {
      return Icon(
        Icons.extension_outlined,
        size: 14,
        color: Theme.of(context).colorScheme.onPrimaryContainer,
      );
    }
    return FutureBuilder<Uint8List>(
      key: ValueKey(widget.avatarUri),
      future: bytes,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return _imageError(snapshot.error!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const Padding(
            padding: EdgeInsets.all(5),
            child: CircularProgressIndicator(strokeWidth: 1.5),
          );
        }
        return Image.memory(
          data,
          fit: BoxFit.cover,
          width: 22,
          height: 22,
          errorBuilder: (context, error, stackTrace) => _imageError(error),
        );
      },
    );
  }

  /// Makes failed avatar assets explicit rather than substituting another image.
  Widget _imageError(Object error) {
    return Tooltip(
      message: '贡献项头像加载失败：$error',
      child: Icon(
        Icons.broken_image_outlined,
        size: 14,
        color: Theme.of(context).colorScheme.error,
      ),
    );
  }
}

class _WorkspaceSessionButton extends StatelessWidget {
  /// Creates an explicit session list button.
  const _WorkspaceSessionButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  /// Builds a compact text button for session management.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return TextButton.icon(
      onPressed: onTap,
      style: TextButton.styleFrom(
        visualDensity: VisualDensity.compact,
        padding: const EdgeInsetsDirectional.fromSTEB(8, 4, 10, 4),
        minimumSize: Size.zero,
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        foregroundColor: colorScheme.primary,
      ),
      icon: Icon(icon, size: 15),
      label: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
    );
  }
}

/// Keeps host conversation and folder counts visible while contributions load.
String _workspaceSummaryText(WorkspaceOverviewUsage usage) {
  final conversations = usage.conversationCount;
  final conversationText = conversations == null
      ? '对话数未就绪'
      : '$conversations 个对话';
  final contributionText = switch (usage.contributions) {
    WorkspaceContributionsReady(:final usages) => '${usages.length} 个贡献项',
    WorkspaceContributionsLoading() => '贡献项读取中',
    WorkspaceContributionsFailed() => '贡献项加载失败',
  };
  final folderText = usage.mountedFoldersLoading
      ? '文件夹读取中'
      : usage.mountedFoldersError != null
      ? '文件夹加载失败'
      : '${usage.mountedFolders.length} 个文件夹';
  return '$conversationText · $contributionText · $folderText';
}

class _WorkspaceMountedFolderStack extends StatelessWidget {
  /// Creates the visible mounted folder list for the workspace overview.
  const _WorkspaceMountedFolderStack({
    required this.folders,
    required this.loading,
    required this.errorMessage,
    required this.onOpenFolder,
  });

  final List<WorkspaceMountedFolder> folders;
  final bool loading;
  final String? errorMessage;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;

  /// Builds mounted folder rows with complete path text.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final errorText = errorMessage?.trim();
    if (loading) {
      return _WorkspaceInlineState(
        icon: Icons.sync,
        text: '正在读取挂载文件夹',
        progress: true,
        color: theme.colorScheme.onSurfaceVariant,
      );
    }
    if (errorText != null && errorText.isNotEmpty) {
      return _WorkspaceInlineState(
        icon: Icons.error_outline,
        text: errorText,
        color: theme.colorScheme.error,
      );
    }
    if (folders.isEmpty) {
      return _WorkspaceInlineState(
        icon: Icons.folder_off_outlined,
        text: '当前工作区暂无挂载文件夹',
        color: theme.colorScheme.onSurfaceVariant,
      );
    }

    return Column(
      children: List<Widget>.generate(folders.length, (index) {
        final folder = folders[index];
        return Padding(
          padding: EdgeInsets.only(bottom: index == folders.length - 1 ? 0 : 7),
          child: _WorkspaceMountedFolderTile(
            folder: folder,
            onOpenFolder: onOpenFolder,
          ),
        );
      }),
    );
  }
}

class _WorkspaceMountedFolderTile extends StatelessWidget {
  /// Creates one mounted folder row for the workspace overview.
  const _WorkspaceMountedFolderTile({
    required this.folder,
    required this.onOpenFolder,
  });

  final WorkspaceMountedFolder folder;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;

  /// Builds one mounted folder row with full path wrapping.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final path = folder.path.trim();
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: () => onOpenFolder(folder),
        child: DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.24),
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Container(
                  width: 26,
                  height: 26,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: colorScheme.surfaceContainerLowest.withValues(
                      alpha: 0.66,
                    ),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: colorScheme.outlineVariant.withValues(alpha: 0.30),
                    ),
                  ),
                  child: Icon(
                    Icons.folder_open_outlined,
                    size: 15,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 9),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        folder.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 3),
                      Tooltip(
                        message: path,
                        waitDuration: const Duration(milliseconds: 450),
                        child: Text(
                          _pathWithBreakOpportunities(path),
                          softWrap: true,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            height: 1.22,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Icon(
                  Icons.chevron_right,
                  size: 17,
                  color: colorScheme.onSurfaceVariant,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceInlineState extends StatelessWidget {
  /// Creates a compact inline state row for overview subsections.
  const _WorkspaceInlineState({
    required this.icon,
    required this.text,
    required this.color,
    this.progress = false,
  });

  final IconData icon;
  final String text;
  final Color color;
  final bool progress;

  /// Builds one inline status row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: <Widget>[
        if (progress)
          SizedBox.square(
            dimension: 14,
            child: CircularProgressIndicator(strokeWidth: 1.8, color: color),
          )
        else
          Icon(icon, size: 15, color: color),
        const SizedBox(width: 7),
        Expanded(
          child: Text(
            text,
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(color: color),
          ),
        ),
      ],
    );
  }
}

class _WorkspacePrimaryAction extends StatelessWidget {
  /// Creates one primary workspace action row.
  const _WorkspacePrimaryAction({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  /// Builds one primary action row for the workspace home tab.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: Colors.transparent,
      transparentAlpha: 0,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(
        color: theme.colorScheme.outlineVariant.withValues(alpha: 0.30),
      ),
      material: true,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 10),
          child: Row(
            children: <Widget>[
              Container(
                width: 32,
                height: 32,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest.withValues(
                    alpha: 0.46,
                  ),
                  borderRadius: BorderRadius.circular(7),
                  border: Border.all(
                    color: theme.colorScheme.outlineVariant.withValues(
                      alpha: 0.26,
                    ),
                  ),
                ),
                child: Icon(icon, size: 18, color: theme.colorScheme.primary),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurface,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      subtitle,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Icon(
                Icons.chevron_right,
                size: 18,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Adds line-break opportunities after path separators for narrow panels.
String _pathWithBreakOpportunities(String path) {
  return path.replaceAll('/', '/\u200B').replaceAll('\\', '\\\u200B');
}
