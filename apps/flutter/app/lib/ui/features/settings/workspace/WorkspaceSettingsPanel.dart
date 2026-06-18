// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class WorkspaceSettingsPanel extends StatefulWidget {
  const WorkspaceSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<WorkspaceSettingsPanel> createState() => _WorkspaceSettingsPanelState();
}

class _WorkspaceSettingsPanelState extends State<WorkspaceSettingsPanel> {
  Future<_WorkspaceSettingsData>? _future;
  final Set<String> _selectedWorkspaceNames = <String>{};
  bool _deleteInProgress = false;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_WorkspaceSettingsData> _load() async {
    final summary = await widget.clients.repositoryWorkspaceService
        .workspaceManagementSummary();

    return _WorkspaceSettingsData(
      chatHistoryCount: summary.chatHistoryCount,
      boundChatCount: summary.boundChatCount,
      workspaceRoot: summary.workspaceRoot,
      unboundWorkspaces: summary.unboundWorkspaces
          .map(
            (workspace) => _UnboundWorkspaceInfo(
              name: workspace.name,
              fullPath: workspace.fullPath,
            ),
          )
          .toList(growable: false),
    );
  }

  void _reload() {
    setState(() {
      _selectedWorkspaceNames.clear();
      _future = _load();
    });
  }

  Future<void> _confirmDeleteSelected() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        final l10n = AppLocalizations.of(context)!;
        return AlertDialog(
          title: Text(l10n.settingsWorkspaceConfirmDeleteTitle),
          content: Text(
            l10n.settingsWorkspaceDeleteConfirmation(
              _selectedWorkspaceNames.length,
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: _deleteInProgress
                  ? null
                  : () => Navigator.of(context).pop(false),
              child: Text(l10n.cancel),
            ),
            TextButton(
              onPressed: _deleteInProgress
                  ? null
                  : () => Navigator.of(context).pop(true),
              style: TextButton.styleFrom(
                foregroundColor: Theme.of(context).colorScheme.error,
              ),
              child: Text(l10n.delete),
            ),
          ],
        );
      },
    );
    if (confirmed != true) {
      return;
    }
    await _deleteSelectedWorkspaces();
  }

  Future<void> _deleteSelectedWorkspaces() async {
    final l10n = AppLocalizations.of(context)!;
    final targets = Set<String>.from(_selectedWorkspaceNames);
    setState(() => _deleteInProgress = true);
    try {
      await widget.clients.repositoryWorkspaceService
          .deleteUnboundWorkspaces(workspaceNames: targets.toList());
      if (!mounted) {
        return;
      }
      setState(() {
        _selectedWorkspaceNames.clear();
        _future = _load();
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsWorkspaceDeleteFailed('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _deleteInProgress = false);
      }
    }
  }

  void _selectAll(List<_UnboundWorkspaceInfo> workspaces) {
    setState(() {
      _selectedWorkspaceNames
        ..clear()
        ..addAll(workspaces.map((workspace) => workspace.name));
    });
  }

  void _clearSelected() {
    setState(() => _selectedWorkspaceNames.clear());
  }

  void _setWorkspaceSelected(String workspaceName, bool selected) {
    if (_deleteInProgress) {
      return;
    }
    setState(() {
      if (selected) {
        _selectedWorkspaceNames.add(workspaceName);
      } else {
        _selectedWorkspaceNames.remove(workspaceName);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_WorkspaceSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        final data = snapshot.data;
        if (data == null) {
          if (snapshot.hasError) {
            return _WorkspaceLoadError(
              message: l10n.settingsWorkspaceLoadFailed('${snapshot.error}'),
              onRetry: _reload,
            );
          }
          return const M3LoadingPane();
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsWorkspaceBoundOverview,
              children: <Widget>[
                _BodyText(l10n.settingsWorkspaceBoundOverviewDescription),
                _InfoLine(
                  label: l10n.settingsWorkspaceBoundChats,
                  value: '${data.boundChatCount} / ${data.chatHistoryCount}',
                ),
                _InfoLine(
                  label: l10n.settingsWorkspaceInternalRoot,
                  value: data.workspaceRoot,
                ),
              ],
            ),
            _UnboundWorkspaceCard(
              workspaces: data.unboundWorkspaces,
              selectedWorkspaceNames: _selectedWorkspaceNames,
              deleteInProgress: _deleteInProgress,
              onSelectAll: () => _selectAll(data.unboundWorkspaces),
              onClearSelected: _clearSelected,
              onSelectionChange: _setWorkspaceSelected,
              onDeleteSelected: _confirmDeleteSelected,
            ),
          ],
        );
      },
    );
  }
}

class _WorkspaceSettingsData {
  const _WorkspaceSettingsData({
    required this.chatHistoryCount,
    required this.boundChatCount,
    required this.workspaceRoot,
    required this.unboundWorkspaces,
  });

  final int chatHistoryCount;
  final int boundChatCount;
  final String workspaceRoot;
  final List<_UnboundWorkspaceInfo> unboundWorkspaces;
}

class _UnboundWorkspaceInfo {
  const _UnboundWorkspaceInfo({
    required this.name,
    required this.fullPath,
  });

  final String name;
  final String fullPath;
}

class _WorkspaceLoadError extends StatelessWidget {
  const _WorkspaceLoadError({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(Icons.error_outline, color: colorScheme.error, size: 36),
            const SizedBox(height: 12),
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: Text(l10n.settingsWorkspaceRefresh),
            ),
          ],
        ),
      ),
    );
  }
}

class _UnboundWorkspaceCard extends StatelessWidget {
  const _UnboundWorkspaceCard({
    required this.workspaces,
    required this.selectedWorkspaceNames,
    required this.deleteInProgress,
    required this.onSelectAll,
    required this.onClearSelected,
    required this.onSelectionChange,
    required this.onDeleteSelected,
  });

  final List<_UnboundWorkspaceInfo> workspaces;
  final Set<String> selectedWorkspaceNames;
  final bool deleteInProgress;
  final VoidCallback onSelectAll;
  final VoidCallback onClearSelected;
  final void Function(String workspaceName, bool selected) onSelectionChange;
  final VoidCallback onDeleteSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return _SectionCard(
      title: l10n.settingsWorkspaceUnboundTitle,
      children: <Widget>[
        _BodyText(l10n.settingsWorkspaceUnboundSubtitle),
        if (workspaces.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(
              l10n.settingsWorkspaceNoUnbound,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          )
        else ...<Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  l10n.settingsWorkspaceSelectedCount(
                    selectedWorkspaceNames.length,
                    workspaces.length,
                  ),
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
              ),
              TextButton(
                onPressed: deleteInProgress ? null : onSelectAll,
                child: Text(l10n.settingsWorkspaceSelectAllCurrentList),
              ),
              TextButton(
                onPressed: selectedWorkspaceNames.isEmpty || deleteInProgress
                    ? null
                    : onClearSelected,
                child: Text(l10n.settingsWorkspaceClearAll),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.30,
                ),
              ),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 320),
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: workspaces.length,
                  separatorBuilder: (context, index) => Divider(
                    height: 1,
                    color: colorScheme.surface.withValues(alpha: 0.75),
                  ),
                  itemBuilder: (context, index) {
                    final workspace = workspaces[index];
                    return _UnboundWorkspaceRow(
                      workspaceInfo: workspace,
                      selected: selectedWorkspaceNames.contains(
                        workspace.name,
                      ),
                      deleteInProgress: deleteInProgress,
                      onSelectionChange: (selected) =>
                          onSelectionChange(workspace.name, selected),
                    );
                  },
                ),
              ),
            ),
          ),
          const SizedBox(height: 14),
          FilledButton.icon(
            onPressed: selectedWorkspaceNames.isEmpty || deleteInProgress
                ? null
                : onDeleteSelected,
            style: FilledButton.styleFrom(
              backgroundColor: colorScheme.error,
              foregroundColor: colorScheme.onError,
            ),
            icon: deleteInProgress
                ? SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: colorScheme.onError,
                    ),
                  )
                : const Icon(Icons.delete_outline),
            label: Text(
              l10n.settingsWorkspaceDeleteSelected(
                selectedWorkspaceNames.length,
              ),
            ),
          ),
        ],
      ],
    );
  }
}

class _UnboundWorkspaceRow extends StatelessWidget {
  const _UnboundWorkspaceRow({
    required this.workspaceInfo,
    required this.selected,
    required this.deleteInProgress,
    required this.onSelectionChange,
  });

  final _UnboundWorkspaceInfo workspaceInfo;
  final bool selected;
  final bool deleteInProgress;
  final ValueChanged<bool> onSelectionChange;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: deleteInProgress ? null : () => onSelectionChange(!selected),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Row(
          children: <Widget>[
            Checkbox(
              value: selected,
              onChanged: deleteInProgress
                  ? null
                  : (value) => onSelectionChange(value == true),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    workspaceInfo.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontWeight: FontWeight.w700),
                  ),
                  const SizedBox(height: 3),
                  Text(
                    l10n.settingsWorkspaceNotUsedByAnyChat,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                  ),
                  const SizedBox(height: 3),
                  Text(
                    workspaceInfo.fullPath,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            Icon(Icons.folder_outlined, color: colorScheme.onSurfaceVariant),
          ],
        ),
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 6),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _BodyText extends StatelessWidget {
  const _BodyText(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Text(
        text,
        style: TextStyle(color: Theme.of(context).colorScheme.onSurfaceVariant),
      ),
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 9),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label)),
          const SizedBox(width: 12),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.end,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}
