// ignore_for_file: file_names

import 'package:flutter/material.dart';

class WorkspaceSetupContent extends StatefulWidget {
  const WorkspaceSetupContent({
    super.key,
    required this.onCreateDefaultWorkspace,
    required this.onBindWorkspace,
  });

  final Future<void> Function(String? projectType) onCreateDefaultWorkspace;
  final Future<void> Function(String workspace, String? workspaceEnv)
  onBindWorkspace;

  @override
  State<WorkspaceSetupContent> createState() => _WorkspaceSetupContentState();
}

class _WorkspaceSetupContentState extends State<WorkspaceSetupContent> {
  final TextEditingController _workspacePathController =
      TextEditingController();
  final TextEditingController _workspaceEnvController = TextEditingController();
  bool _busy = false;
  String? _errorMessage;

  @override
  void dispose() {
    _workspacePathController.dispose();
    _workspaceEnvController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: Padding(
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
                  '设置工作区',
                  style: theme.textTheme.titleLarge?.copyWith(
                    color: theme.colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 8),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Text(
                    '为您的AI项目提供一个专属的文件环境',
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
                      title: '创建默认',
                      description: '在应用内创建新工作区',
                      onTap: _showProjectTypeDialog,
                    ),
                    _WorkspaceOption(
                      icon: Icons.folder_open,
                      title: '选择已有',
                      description: '从设备选择文件夹',
                      onTap: _showBindWorkspaceDialog,
                    ),
                  ],
                ),
              ],
            ),
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
            void syncDialogState(VoidCallback action) {
              setDialogState(action);
              setState(() {});
            }

            return AlertDialog(
              title: const Text('选择语言类型'),
              content: SizedBox(
                width: 420,
                child: SingleChildScrollView(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        '请选择要创建的默认工作区类型',
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
                            title: project.title,
                            description: project.description,
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
                  child: const Text('取消'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  void _showBindWorkspaceDialog() {
    _workspacePathController.clear();
    _workspaceEnvController.clear();
    _errorMessage = null;
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            void syncDialogState(VoidCallback action) {
              setDialogState(action);
              setState(() {});
            }

            return AlertDialog(
              title: const Text('选择已有工作区'),
              content: SizedBox(
                width: 420,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    TextField(
                      controller: _workspacePathController,
                      decoration: const InputDecoration(
                        labelText: '工作区路径',
                        hintText: r'D:\Code\project',
                      ),
                      enabled: !_busy,
                    ),
                    const SizedBox(height: 10),
                    TextField(
                      controller: _workspaceEnvController,
                      decoration: const InputDecoration(
                        labelText: '工作区环境',
                        hintText: '可留空',
                      ),
                      enabled: !_busy,
                    ),
                    if (_errorMessage != null) ...<Widget>[
                      const SizedBox(height: 10),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: Text(
                          _errorMessage!,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: _busy
                      ? null
                      : () {
                          Navigator.of(dialogContext).pop();
                        },
                  child: const Text('取消'),
                ),
                FilledButton(
                  onPressed: _busy
                      ? null
                      : () {
                          final workspace = _workspacePathController.text
                              .trim();
                          if (workspace.isEmpty) {
                            syncDialogState(() {
                              _errorMessage = '请输入工作区路径';
                            });
                            return;
                          }
                          final workspaceEnv = _workspaceEnvController.text
                              .trim();
                          syncDialogState(() {
                            _errorMessage = null;
                          });
                          _runWorkspaceAction(() {
                            return widget.onBindWorkspace(
                              workspace,
                              workspaceEnv.isEmpty ? null : workspaceEnv,
                            );
                          });
                        },
                  child: const Text('绑定'),
                ),
              ],
            );
          },
        );
      },
    );
  }
}

class _WorkspaceProjectType {
  const _WorkspaceProjectType({
    required this.icon,
    required this.title,
    required this.description,
    required this.projectType,
  });

  final IconData icon;
  final String title;
  final String description;
  final String? projectType;
}

const List<_WorkspaceProjectType> _workspaceProjectTypes =
    <_WorkspaceProjectType>[
      _WorkspaceProjectType(
        icon: Icons.create_new_folder,
        title: '空白工作区',
        description: '仅创建一个空的工作区目录，不包含任何模板文件',
        projectType: 'blank',
      ),
      _WorkspaceProjectType(
        icon: Icons.description,
        title: '办公文档',
        description: '用于文档编辑、文件处理和通用办公任务',
        projectType: 'office',
      ),
      _WorkspaceProjectType(
        icon: Icons.language,
        title: 'Web 项目',
        description: '适用于网页开发，支持 HTML/CSS/JavaScript，自动启动本地服务器',
        projectType: null,
      ),
      _WorkspaceProjectType(
        icon: Icons.phone_android,
        title: 'Android 项目',
        description: '适用于 Android 工程开发，包含 Gradle 常用任务快捷按钮',
        projectType: 'android',
      ),
      _WorkspaceProjectType(
        icon: Icons.widgets,
        title: 'Flutter 项目',
        description: '适用于 Flutter 跨平台开发，内置当前稳定版应用模板和常用命令',
        projectType: 'flutter',
      ),
      _WorkspaceProjectType(
        icon: Icons.terminal,
        title: 'Node.js 项目',
        description: '适用于 Node.js 后端开发，提供 npm 命令快捷按钮',
        projectType: 'node',
      ),
      _WorkspaceProjectType(
        icon: Icons.code,
        title: 'TypeScript 项目',
        description: 'TypeScript + pnpm，支持类型安全开发和 tsc watch 实时编译',
        projectType: 'typescript',
      ),
      _WorkspaceProjectType(
        icon: Icons.code,
        title: 'Python 项目',
        description: '适用于 Python 开发，支持 pip 和 HTTP 服务器',
        projectType: 'python',
      ),
      _WorkspaceProjectType(
        icon: Icons.settings,
        title: 'Java 项目',
        description: '适用于 Java 开发，支持 Gradle 和 Maven 构建',
        projectType: 'java',
      ),
      _WorkspaceProjectType(
        icon: Icons.build,
        title: 'Go 项目',
        description: '适用于 Go 开发，提供 go mod 和 build 命令',
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
    return Material(
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
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
      child: Material(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(
            color: theme.colorScheme.outline.withValues(alpha: 0.2),
          ),
        ),
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
