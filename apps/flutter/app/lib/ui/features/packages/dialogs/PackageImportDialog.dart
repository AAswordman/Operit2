// ignore_for_file: file_names

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../components/PackageTab.dart';

class PackageImportResult {
  const PackageImportResult({required this.message});

  final String message;
}

class PackageImportDialog extends StatefulWidget {
  const PackageImportDialog({
    super.key,
    required this.clients,
    required this.initialTab,
  });

  final GeneratedCoreProxyClients clients;
  final PackageTab initialTab;

  @override
  State<PackageImportDialog> createState() => _PackageImportDialogState();
}

class _PackageImportDialogState extends State<PackageImportDialog> {
  bool _importing = false;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      icon: const Icon(Icons.add),
      title: const Text('导入'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _ImportOption(
              icon: Icons.apps,
              title: '导入插件',
              description: '选择 .toolpkg 插件包。',
              highlighted: widget.initialTab == PackageTab.plugins,
              enabled: !_importing,
              onTap: _importPlugin,
            ),
            const SizedBox(height: 8),
            _ImportOption(
              icon: Icons.extension,
              title: '导入包',
              description: '选择 .toolpkg、.hjson、.js 或 .ts 工具包文件。',
              highlighted: widget.initialTab == PackageTab.packages,
              enabled: !_importing,
              onTap: _importPackage,
            ),
            const SizedBox(height: 8),
            _ImportOption(
              icon: Icons.build,
              title: '导入技能',
              description: '选择技能 .zip 文件。',
              highlighted: widget.initialTab == PackageTab.skills,
              enabled: !_importing,
              onTap: _importSkill,
            ),
            const SizedBox(height: 8),
            _ImportOption(
              icon: Icons.cloud,
              title: '导入 MCP',
              description: '选择 MCP JSON 配置文件。',
              highlighted: widget.initialTab == PackageTab.mcp,
              enabled: !_importing,
              onTap: _importMcp,
            ),
            if (_importing) ...<Widget>[
              const SizedBox(height: 16),
              LinearProgressIndicator(minHeight: 2, color: colorScheme.primary),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _importing ? null : () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }

  Future<void> _importPlugin() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'ToolPkg', extensions: <String>['toolpkg']),
      ],
    );
    if (file == null) {
      return;
    }
    await _runImport(
      () => widget.clients.permissionsPackToolPackageManager
          .addPackageFileFromExternalStorage(filePath: file.path),
    );
  }

  Future<void> _importPackage() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(
          label: 'Operit package',
          extensions: <String>['toolpkg', 'hjson', 'js', 'ts'],
        ),
      ],
    );
    if (file == null) {
      return;
    }
    await _runImport(
      () => widget.clients.permissionsPackToolPackageManager
          .addPackageFileFromExternalStorage(filePath: file.path),
    );
  }

  Future<void> _importSkill() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'Zip', extensions: <String>['zip']),
      ],
    );
    if (file == null) {
      return;
    }
    await _runImport(
      () =>
          widget.clients.skillRepository.importSkillFromZip(zipFile: file.path),
    );
  }

  Future<void> _importMcp() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'JSON', extensions: <String>['json']),
      ],
    );
    if (file == null) {
      return;
    }
    final configJson = await file.readAsString();
    await _runImport(() async {
      final count = await widget.clients.mcpLocalServer.mergeConfigFromJson(
        jsonConfig: configJson,
      );
      return '已导入 $count 个 MCP 服务';
    });
  }

  Future<void> _runImport(Future<String> Function() action) async {
    setState(() {
      _importing = true;
    });
    try {
      final message = await action();
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop(PackageImportResult(message: message));
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _importing = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }
}

class _ImportOption extends StatelessWidget {
  const _ImportOption({
    required this.icon,
    required this.title,
    required this.description,
    required this.highlighted,
    required this.enabled,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String description;
  final bool highlighted;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final backgroundColor = highlighted
        ? colorScheme.primaryContainer.withValues(alpha: 0.52)
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.34);
    final borderColor = highlighted
        ? colorScheme.primary.withValues(alpha: 0.48)
        : colorScheme.outlineVariant.withValues(alpha: 0.34);
    return Material(
      color: backgroundColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: borderColor),
      ),
      child: InkWell(
        onTap: enabled ? onTap : null,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          child: Row(
            children: <Widget>[
              Icon(icon, size: 20),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      title,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      description,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              const Icon(Icons.chevron_right, size: 18),
            ],
          ),
        ),
      ),
    );
  }
}
