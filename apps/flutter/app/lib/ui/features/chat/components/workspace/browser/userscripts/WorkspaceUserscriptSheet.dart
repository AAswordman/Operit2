// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import 'WorkspaceUserscriptModels.dart';
import 'WorkspaceUserscriptStore.dart';

class WorkspaceUserscriptSheet extends StatefulWidget {
  const WorkspaceUserscriptSheet({
    super.key,
    required this.store,
    required this.onChanged,
    required this.onReadWorkspaceTextFile,
    required this.onLoadMenuCommands,
    required this.onRunMenuCommand,
  });

  final WorkspaceUserscriptStore store;
  final VoidCallback onChanged;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<List<WorkspaceUserscriptMenuCommand>> Function()
  onLoadMenuCommands;
  final Future<void> Function(int index) onRunMenuCommand;

  @override
  State<WorkspaceUserscriptSheet> createState() =>
      _WorkspaceUserscriptSheetState();
}

class _WorkspaceUserscriptSheetState extends State<WorkspaceUserscriptSheet> {
  final TextEditingController _sourceController = TextEditingController();
  final TextEditingController _urlController = TextEditingController();
  final TextEditingController _workspacePathController =
      TextEditingController();
  bool _showInstall = false;
  bool _installingFromUrl = false;
  bool _installingFromWorkspace = false;
  final Set<String> _checkingUpdateIds = <String>{};
  List<WorkspaceUserscriptMenuCommand> _menuCommands =
      const <WorkspaceUserscriptMenuCommand>[];

  @override
  void initState() {
    super.initState();
    _loadMenuCommands();
  }

  @override
  void dispose() {
    _sourceController.dispose();
    _urlController.dispose();
    _workspacePathController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scripts = widget.store.items;
    final logs = widget.store.logs;
    final pageRuns = widget.store.pageRuns;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Row(
              children: <Widget>[
                Text('脚本', style: Theme.of(context).textTheme.titleMedium),
                const Spacer(),
                TextButton.icon(
                  onPressed: () => setState(() => _showInstall = !_showInstall),
                  icon: const Icon(Icons.add),
                  label: const Text('安装'),
                ),
              ],
            ),
            if (_showInstall) ...<Widget>[
              const SizedBox(height: 8),
              TextField(
                controller: _urlController,
                decoration: InputDecoration(
                  hintText: 'https://example.com/script.user.js',
                  border: const OutlineInputBorder(),
                  suffixIcon: IconButton(
                    tooltip: '从 URL 安装',
                    onPressed: _installingFromUrl ? null : _installFromUrl,
                    icon: _installingFromUrl
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.download_outlined),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: _workspacePathController,
                decoration: InputDecoration(
                  hintText: 'scripts/example.user.js',
                  border: const OutlineInputBorder(),
                  suffixIcon: IconButton(
                    tooltip: '从工作区文件安装',
                    onPressed: _installingFromWorkspace
                        ? null
                        : _installFromWorkspaceFile,
                    icon: _installingFromWorkspace
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.file_open_outlined),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: _sourceController,
                minLines: 4,
                maxLines: 8,
                decoration: const InputDecoration(
                  hintText: '粘贴 .user.js 内容',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton(
                  onPressed: _installFromSource,
                  child: const Text('安装脚本'),
                ),
              ),
            ],
            const SizedBox(height: 8),
            Flexible(
              child: ListView(
                shrinkWrap: true,
                children: <Widget>[
                  _UserscriptMenuCommandSection(
                    commands: _menuCommands,
                    onRefresh: _loadMenuCommands,
                    onRun: (command) async {
                      await widget.onRunMenuCommand(command.index);
                      await _loadMenuCommands();
                    },
                  ),
                  _UserscriptPageRunSection(pageRuns: pageRuns),
                  if (scripts.isEmpty)
                    const Padding(
                      padding: EdgeInsets.all(18),
                      child: Text('还没有安装脚本'),
                    )
                  else
                    for (final script in scripts)
                      SwitchListTile(
                        value: script.enabled,
                        onChanged: (value) {
                          widget.store.setEnabled(script.id, value);
                          widget.onChanged();
                          setState(() {});
                        },
                        title: Text(script.metadata.name),
                        subtitle: Text(
                          [
                            if (script.metadata.version.isNotEmpty)
                              script.metadata.version,
                            if (script.metadata.matches.isNotEmpty)
                              script.metadata.matches.join(', '),
                            if (script.knownGrants.isNotEmpty)
                              script.knownGrants.join(', '),
                            if (script.unknownGrants.isNotEmpty)
                              '未知：${script.unknownGrants.join(', ')}',
                          ].join(' | '),
                        ),
                        secondary: _UserscriptItemActions(
                          checkingUpdate: _checkingUpdateIds.contains(
                            script.id,
                          ),
                          canCheckUpdate: widget.store.hasUpdateSource(script),
                          onCheckUpdate: () => _checkUpdate(script.id),
                          onRemove: () {
                            widget.store.remove(script.id);
                            widget.onChanged();
                            setState(() {});
                          },
                        ),
                      ),
                  if (logs.isNotEmpty) ...<Widget>[
                    const Divider(),
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                      child: Text(
                        '日志',
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                    ),
                    for (final log in logs.take(20))
                      ListTile(
                        dense: true,
                        leading: const Icon(Icons.notes, size: 18),
                        title: Text(log.scriptName),
                        subtitle: Text(log.message),
                      ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _installFromUrl() async {
    final url = _urlController.text.trim();
    if (url.isEmpty) {
      return;
    }
    setState(() => _installingFromUrl = true);
    final response = await http.get(Uri.parse(url));
    await _previewAndConfirmSource(response.body, sourceUrl: url);
    if (!mounted) {
      return;
    }
    setState(() {
      _installingFromUrl = false;
    });
  }

  Future<void> _installFromSource() async {
    final source = _sourceController.text;
    if (source.trim().isEmpty) {
      return;
    }
    await _previewAndConfirmSource(source);
  }

  Future<void> _installFromWorkspaceFile() async {
    final path = _workspacePathController.text.trim();
    if (path.isEmpty) {
      return;
    }
    setState(() => _installingFromWorkspace = true);
    final source = await widget.onReadWorkspaceTextFile(path);
    await _previewAndConfirmSource(source, sourceUrl: path);
    if (!mounted) {
      return;
    }
    setState(() => _installingFromWorkspace = false);
  }

  Future<void> _previewAndConfirmSource(
    String source, {
    String? sourceUrl,
  }) async {
    final preview = widget.store.createInstallPreview(
      source,
      sourceUrl: sourceUrl,
    );
    if (!mounted) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return _UserscriptInstallPreviewDialog(preview: preview);
      },
    );
    if (confirmed != true) {
      return;
    }
    final item = widget.store.installFromPreview(preview);
    await widget.store.refreshDependencies(item.id);
    _sourceController.clear();
    _urlController.clear();
    _workspacePathController.clear();
    widget.onChanged();
    if (!mounted) {
      return;
    }
    setState(() => _showInstall = false);
  }

  Future<void> _loadMenuCommands() async {
    final commands = await widget.onLoadMenuCommands();
    if (!mounted) {
      return;
    }
    setState(() {
      _menuCommands = commands;
    });
  }

  Future<void> _checkUpdate(String id) async {
    setState(() {
      _checkingUpdateIds.add(id);
    });
    await widget.store.checkAndInstallUpdate(id);
    widget.onChanged();
    if (!mounted) {
      return;
    }
    setState(() {
      _checkingUpdateIds.remove(id);
    });
  }
}

class _UserscriptItemActions extends StatelessWidget {
  const _UserscriptItemActions({
    required this.checkingUpdate,
    required this.canCheckUpdate,
    required this.onCheckUpdate,
    required this.onRemove,
  });

  final bool checkingUpdate;
  final bool canCheckUpdate;
  final VoidCallback onCheckUpdate;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 74,
      child: Row(
        children: <Widget>[
          IconButton(
            tooltip: '检查更新',
            onPressed: canCheckUpdate && !checkingUpdate ? onCheckUpdate : null,
            icon: checkingUpdate
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.system_update_alt, size: 18),
            visualDensity: VisualDensity.compact,
            constraints: const BoxConstraints.tightFor(width: 34, height: 34),
            padding: EdgeInsets.zero,
          ),
          IconButton(
            tooltip: '删除',
            onPressed: onRemove,
            icon: const Icon(Icons.delete_outline, size: 18),
            visualDensity: VisualDensity.compact,
            constraints: const BoxConstraints.tightFor(width: 34, height: 34),
            padding: EdgeInsets.zero,
          ),
        ],
      ),
    );
  }
}

class _UserscriptPageRunSection extends StatelessWidget {
  const _UserscriptPageRunSection({required this.pageRuns});

  final List<WorkspaceUserscriptPageRun> pageRuns;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colorScheme.outlineVariant),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('页面运行状态', style: theme.textTheme.labelLarge),
              const SizedBox(height: 6),
              if (pageRuns.isEmpty)
                Text(
                  '当前还没有页面运行记录',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                )
              else
                for (final run in pageRuns.take(8))
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: Icon(
                      run.status == 'error'
                          ? Icons.error_outline
                          : Icons.check_circle_outline,
                      size: 20,
                      color: run.status == 'error'
                          ? theme.colorScheme.error
                          : theme.colorScheme.primary,
                    ),
                    title: Text(run.scriptName),
                    subtitle: Text(run.message),
                  ),
            ],
          ),
        ),
      ),
    );
  }
}

class _UserscriptMenuCommandSection extends StatelessWidget {
  const _UserscriptMenuCommandSection({
    required this.commands,
    required this.onRefresh,
    required this.onRun,
  });

  final List<WorkspaceUserscriptMenuCommand> commands;
  final Future<void> Function() onRefresh;
  final Future<void> Function(WorkspaceUserscriptMenuCommand command) onRun;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colorScheme.outlineVariant),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 8, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Text('当前页面菜单', style: theme.textTheme.labelLarge),
                  const Spacer(),
                  IconButton(
                    tooltip: '刷新菜单',
                    onPressed: onRefresh,
                    icon: const Icon(Icons.refresh, size: 18),
                    visualDensity: VisualDensity.compact,
                    constraints: const BoxConstraints.tightFor(
                      width: 32,
                      height: 32,
                    ),
                  ),
                ],
              ),
              if (commands.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Text(
                    '当前页面没有脚本菜单',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                )
              else
                for (final command in commands)
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.play_arrow_outlined, size: 20),
                    title: Text(command.caption),
                    subtitle: Text(command.scriptName),
                    onTap: () => onRun(command),
                  ),
            ],
          ),
        ),
      ),
    );
  }
}

class _UserscriptInstallPreviewDialog extends StatelessWidget {
  const _UserscriptInstallPreviewDialog({required this.preview});

  final WorkspaceUserscriptInstallPreview preview;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final metadata = preview.metadata;
    return AlertDialog(
      title: const Text('安装脚本'),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                metadata.name,
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              if (metadata.description.isNotEmpty) ...<Widget>[
                const SizedBox(height: 6),
                Text(metadata.description),
              ],
              const SizedBox(height: 12),
              _PreviewLine(label: '版本', value: metadata.version),
              _PreviewLine(label: '命名空间', value: metadata.namespace),
              if (metadata.author.isNotEmpty)
                _PreviewLine(label: '作者', value: metadata.author),
              if (preview.sourceUrl != null)
                _PreviewLine(label: '来源', value: preview.sourceUrl!),
              const SizedBox(height: 12),
              _PreviewChips(label: '匹配', values: metadata.matches),
              _PreviewChips(label: '包含', values: metadata.includes),
              _PreviewChips(label: '排除', values: metadata.excludes),
              _PreviewChips(label: '连接', values: metadata.connects),
              _PreviewChips(label: '能力', values: preview.knownGrants),
              _PreviewChips(label: '未知能力', values: preview.unknownGrants),
              if (preview.blockedReasons.isNotEmpty) ...<Widget>[
                const SizedBox(height: 10),
                DecoratedBox(
                  decoration: BoxDecoration(
                    color: theme.colorScheme.errorContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(10),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          '需要注意',
                          style: theme.textTheme.labelLarge?.copyWith(
                            color: theme.colorScheme.onErrorContainer,
                          ),
                        ),
                        const SizedBox(height: 6),
                        for (final reason in preview.blockedReasons)
                          Text(
                            reason,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onErrorContainer,
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('安装'),
        ),
      ],
    );
  }
}

class _PreviewLine extends StatelessWidget {
  const _PreviewLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    if (value.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Text('$label：$value', style: theme.textTheme.bodySmall),
    );
  }
}

class _PreviewChips extends StatelessWidget {
  const _PreviewChips({required this.label, required this.values});

  final String label;
  final List<String> values;

  @override
  Widget build(BuildContext context) {
    if (values.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(label, style: theme.textTheme.labelMedium),
          const SizedBox(height: 4),
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: <Widget>[
              for (final value in values)
                Chip(label: Text(value), visualDensity: VisualDensity.compact),
            ],
          ),
        ],
      ),
    );
  }
}
