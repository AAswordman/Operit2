// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';

class WorkspaceSetupContent extends StatefulWidget {
  const WorkspaceSetupContent({
    super.key,
    required this.onCreateDefaultWorkspace,
    required this.onBindWorkspace,
  });

  final Future<void> Function(String? projectType) onCreateDefaultWorkspace;
  final Future<void> Function(String workspace) onBindWorkspace;

  @override
  State<WorkspaceSetupContent> createState() => _WorkspaceSetupContentState();
}

class _WorkspaceSetupContentState extends State<WorkspaceSetupContent> {
  bool _busy = false;
  String? _errorMessage;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Center(
        child: SingleChildScrollView(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Icon(Icons.widgets, size: 48, color: theme.colorScheme.primary),
              const SizedBox(height: 16),
              Text(
                l10n.workspaceSetupTitle,
                style: theme.textTheme.titleLarge?.copyWith(
                  color: theme.colorScheme.onSurface,
                ),
              ),
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Text(
                  l10n.workspaceSetupSubtitle,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
              const SizedBox(height: 24),
              Wrap(
                alignment: WrapAlignment.center,
                spacing: 12,
                runSpacing: 12,
                children: <Widget>[
                  _WorkspaceOption(
                    icon: Icons.create_new_folder,
                    title: l10n.workspaceCreateDefaultTitle,
                    description: l10n.workspaceCreateDefaultDescription,
                    onTap: _showProjectTypeDialog,
                  ),
                  _WorkspaceOption(
                    icon: Icons.folder_open,
                    title: l10n.workspaceBindExistingTitle,
                    description: l10n.workspaceBindExistingDescription,
                    onTap: _pickAndBindWorkspace,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _runWorkspaceAction(Future<void> Function() action) async {
    if (_busy) {
      return;
    }
    setState(() {
      _busy = true;
      _errorMessage = null;
    });
    try {
      await action();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error, stackTrace) {
      debugPrint('Workspace action failed: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
        });
      }
    }
  }

  void _showProjectTypeDialog() {
    _errorMessage = null;
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final l10n = AppLocalizations.of(context)!;
            void syncDialogState(VoidCallback action) {
              setDialogState(action);
              setState(() {});
            }

            return AlertDialog(
              title: Text(l10n.workspaceProjectTypeDialogTitle),
              content: SizedBox(
                width: 420,
                child: SingleChildScrollView(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        l10n.workspaceProjectTypeDialogDescription,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                      if (_errorMessage != null) ...<Widget>[
                        const SizedBox(height: 10),
                        Text(
                          _errorMessage!,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ],
                      const SizedBox(height: 14),
                      for (final project in _workspaceProjectTypes)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 10),
                          child: _ProjectTypeCard(
                            icon: project.icon,
                            title: project.title(l10n),
                            description: project.description(l10n),
                            busy: _busy,
                            onTap: () {
                              syncDialogState(() {
                                _errorMessage = null;
                              });
                              _runWorkspaceAction(() {
                                return widget.onCreateDefaultWorkspace(
                                  project.projectType,
                                );
                              });
                            },
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: _busy
                      ? null
                      : () {
                          Navigator.of(dialogContext).pop();
                        },
                  child: Text(l10n.cancel),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _pickAndBindWorkspace() async {
    if (_busy) {
      return;
    }
    setState(() {
      _errorMessage = null;
    });
    final selectedPath = await getDirectoryPath(canCreateDirectories: false);
    if (selectedPath == null) {
      return;
    }
    await _runWorkspaceAction(() {
      return widget.onBindWorkspace(selectedPath);
    });
  }
}

enum _WorkspaceProjectKind {
  blank,
  office,
  web,
  android,
  flutter,
  node,
  typeScript,
  python,
  java,
  go,
}

class _WorkspaceProjectType {
  const _WorkspaceProjectType({
    required this.icon,
    required this.kind,
    required this.projectType,
  });

  final IconData icon;
  final _WorkspaceProjectKind kind;
  final String? projectType;

  String title(AppLocalizations l10n) {
    return switch (kind) {
      _WorkspaceProjectKind.blank => l10n.workspaceProjectBlankTitle,
      _WorkspaceProjectKind.office => l10n.workspaceProjectOfficeTitle,
      _WorkspaceProjectKind.web => l10n.workspaceProjectWebTitle,
      _WorkspaceProjectKind.android => l10n.workspaceProjectAndroidTitle,
      _WorkspaceProjectKind.flutter => l10n.workspaceProjectFlutterTitle,
      _WorkspaceProjectKind.node => l10n.workspaceProjectNodeTitle,
      _WorkspaceProjectKind.typeScript => l10n.workspaceProjectTypeScriptTitle,
      _WorkspaceProjectKind.python => l10n.workspaceProjectPythonTitle,
      _WorkspaceProjectKind.java => l10n.workspaceProjectJavaTitle,
      _WorkspaceProjectKind.go => l10n.workspaceProjectGoTitle,
    };
  }

  String description(AppLocalizations l10n) {
    return switch (kind) {
      _WorkspaceProjectKind.blank => l10n.workspaceProjectBlankDescription,
      _WorkspaceProjectKind.office => l10n.workspaceProjectOfficeDescription,
      _WorkspaceProjectKind.web => l10n.workspaceProjectWebDescription,
      _WorkspaceProjectKind.android => l10n.workspaceProjectAndroidDescription,
      _WorkspaceProjectKind.flutter => l10n.workspaceProjectFlutterDescription,
      _WorkspaceProjectKind.node => l10n.workspaceProjectNodeDescription,
      _WorkspaceProjectKind.typeScript =>
        l10n.workspaceProjectTypeScriptDescription,
      _WorkspaceProjectKind.python => l10n.workspaceProjectPythonDescription,
      _WorkspaceProjectKind.java => l10n.workspaceProjectJavaDescription,
      _WorkspaceProjectKind.go => l10n.workspaceProjectGoDescription,
    };
  }
}

const List<_WorkspaceProjectType> _workspaceProjectTypes =
    <_WorkspaceProjectType>[
      _WorkspaceProjectType(
        icon: Icons.create_new_folder,
        kind: _WorkspaceProjectKind.blank,
        projectType: 'blank',
      ),
      _WorkspaceProjectType(
        icon: Icons.description,
        kind: _WorkspaceProjectKind.office,
        projectType: 'office',
      ),
      _WorkspaceProjectType(
        icon: Icons.language,
        kind: _WorkspaceProjectKind.web,
        projectType: null,
      ),
      _WorkspaceProjectType(
        icon: Icons.phone_android,
        kind: _WorkspaceProjectKind.android,
        projectType: 'android',
      ),
      _WorkspaceProjectType(
        icon: Icons.widgets,
        kind: _WorkspaceProjectKind.flutter,
        projectType: 'flutter',
      ),
      _WorkspaceProjectType(
        icon: Icons.terminal,
        kind: _WorkspaceProjectKind.node,
        projectType: 'node',
      ),
      _WorkspaceProjectType(
        icon: Icons.code,
        kind: _WorkspaceProjectKind.typeScript,
        projectType: 'typescript',
      ),
      _WorkspaceProjectType(
        icon: Icons.code,
        kind: _WorkspaceProjectKind.python,
        projectType: 'python',
      ),
      _WorkspaceProjectType(
        icon: Icons.settings,
        kind: _WorkspaceProjectKind.java,
        projectType: 'java',
      ),
      _WorkspaceProjectType(
        icon: Icons.build,
        kind: _WorkspaceProjectKind.go,
        projectType: 'go',
      ),
    ];

class _ProjectTypeCard extends StatelessWidget {
  const _ProjectTypeCard({
    required this.icon,
    required this.title,
    required this.description,
    required this.busy,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String description;
  final bool busy;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(
        color: theme.colorScheme.outline.withValues(alpha: 0.18),
      ),
      material: true,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: busy ? null : onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: <Widget>[
              Container(
                width: 48,
                height: 48,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  icon,
                  size: 28,
                  color: theme.colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: theme.textTheme.titleMedium?.copyWith(
                        color: theme.colorScheme.onSurface,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      description,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 10),
              Icon(
                Icons.chevron_right,
                size: 20,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _WorkspaceOption extends StatelessWidget {
  const _WorkspaceOption({
    required this.icon,
    required this.title,
    required this.description,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String description;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SizedBox(
      width: 132,
      height: 132,
      child: OperitGlassSurface(
        color: theme.colorScheme.surfaceContainerHighest.withValues(
          alpha: 0.42,
        ),
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.18),
        ),
        material: true,
        child: InkWell(
          borderRadius: BorderRadius.circular(12),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                Icon(icon, size: 34, color: theme.colorScheme.primary),
                const SizedBox(height: 10),
                Text(
                  title,
                  maxLines: 2,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.titleMedium?.copyWith(
                    color: theme.colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 6),
                SizedBox(
                  height: 34,
                  child: Text(
                    description,
                    textAlign: TextAlign.center,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
