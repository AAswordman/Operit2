// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class DataSettingsPanel extends StatefulWidget {
  const DataSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<DataSettingsPanel> createState() => _DataSettingsPanelState();
}

class _DataSettingsPanelState extends State<DataSettingsPanel> {
  Future<_DataSettingsData>? _future;
  bool _busy = false;
  int? _lastSnapshotBytes;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_DataSettingsData> _load() async {
    final characterCardManager = widget.clients.preferencesCharacterCardManager;
    final characterGroupCardManager =
        widget.clients.preferencesCharacterGroupCardManager;
    final modelConfigManager = widget.clients.preferencesModelConfigManager;
    await characterCardManager.initializeIfNeeded();
    await characterGroupCardManager.initializeIfNeeded();
    await modelConfigManager.initializeIfNeeded();
    return _DataSettingsData(
      coreVersion: await widget.clients.application.coreVersion(),
      inputTokens: await widget.clients.chatRuntimeHolderMain
          .inputTokenCountFlowSnapshot(),
      outputTokens: await widget.clients.chatRuntimeHolderMain
          .outputTokenCountFlowSnapshot(),
      chatHistoryCount:
          (await widget.clients.chatRuntimeHolderMain
                  .chatHistoriesFlowSnapshot())
              .length,
      characterCardCount:
          (await characterCardManager.getAllCharacterCards()).length,
      characterGroupCount:
          (await characterGroupCardManager.getAllCharacterGroupCards()).length,
      modelConfigCount:
          (await modelConfigManager.getAllModelSummaries()).length,
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _updateTokenStatistics() async {
    setState(() => _busy = true);
    await widget.clients.chatRuntimeHolderMain.updateCumulativeStatistics();
    setState(() => _busy = false);
    _reload();
  }

  Future<void> _resetTokenStatistics() async {
    setState(() => _busy = true);
    await widget.clients.chatRuntimeHolderMain.resetTokenStatistics();
    setState(() => _busy = false);
    _reload();
  }

  Future<void> _exportRawSnapshot() async {
    setState(() => _busy = true);
    final bytes = await widget.clients.application.exportRawSnapshot();
    if (!mounted) {
      return;
    }
    setState(() {
      _busy = false;
      _lastSnapshotBytes = bytes.length;
    });
  }

  Future<void> _copyChatHistoriesBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.chatRuntimeHolderMain
          .exportChatHistoriesToJson();
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupCopied(l10n.settingsDataChatHistoriesBackup),
          ),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupCopyError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _importChatHistoriesBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _BackupImportDialog.show(context: context);
    if (jsonText == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final result = await widget.clients.chatRuntimeHolderMain
          .importChatHistoriesFromJson(jsonString: jsonText);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupImportResult(
              result.newValue,
              result.updated,
              result.skipped,
            ),
          ),
        ),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupImportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _copyCharacterCardsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesCharacterCardManager
          .exportAllCharacterCardsToBackupContent();
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupCopied(
              l10n.settingsDataCharacterCardsBackup,
            ),
          ),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupCopyError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _importCharacterCardsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _BackupImportDialog.show(context: context);
    if (jsonText == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final result = await widget.clients.preferencesCharacterCardManager
          .importAllCharacterCardsFromBackupContent(jsonContent: jsonText);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupImportResult(
              result.newValue,
              result.updated,
              result.skipped,
            ),
          ),
        ),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupImportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _copyCharacterGroupsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesCharacterGroupCardManager
          .exportAllCharacterGroupsToBackupContent();
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupCopied(
              l10n.settingsDataCharacterGroupsBackup,
            ),
          ),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupCopyError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _importCharacterGroupsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _BackupImportDialog.show(context: context);
    if (jsonText == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final result = await widget.clients.preferencesCharacterGroupCardManager
          .importAllCharacterGroupsFromBackupContent(jsonContent: jsonText);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupImportResult(
              result.newValue,
              result.updated,
              result.skipped,
            ),
          ),
        ),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupImportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _copyModelConfigsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesModelConfigManager
          .exportAllProviders();
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            l10n.settingsDataBackupCopied(l10n.settingsDataModelConfigsBackup),
          ),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupCopyError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_DataSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsDataRuntimeSection,
              children: <Widget>[
                _InfoLine(
                  label: l10n.settingsDataCoreVersion,
                  value: data.coreVersion,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsDataTokenSection,
              children: <Widget>[
                _InfoLine(
                  label: l10n.settingsDataInputTokens,
                  value: data.inputTokens.toString(),
                ),
                _InfoLine(
                  label: l10n.settingsDataOutputTokens,
                  value: data.outputTokens.toString(),
                ),
                _ActionLine(
                  icon: Icons.refresh,
                  title: l10n.settingsDataRefreshTokenStats,
                  onTap: _busy ? null : _updateTokenStatistics,
                ),
                _ActionLine(
                  icon: Icons.restart_alt,
                  title: l10n.settingsDataResetTokenStats,
                  onTap: _busy ? null : _resetTokenStatistics,
                  destructive: true,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsDataBackupSection,
              children: <Widget>[
                _BackupLine(
                  title: l10n.settingsDataChatHistoriesBackup,
                  subtitle: l10n.settingsDataBackupCount(data.chatHistoryCount),
                  description: l10n.settingsDataChatHistoriesBackupDescription,
                  onExport: _busy ? null : _copyChatHistoriesBackup,
                  onImport: _busy ? null : _importChatHistoriesBackup,
                ),
                const Divider(height: 20),
                _BackupLine(
                  title: l10n.settingsDataCharacterCardsBackup,
                  subtitle: l10n.settingsDataBackupCount(
                    data.characterCardCount,
                  ),
                  description: l10n.settingsDataCharacterCardsBackupDescription,
                  onExport: _busy ? null : _copyCharacterCardsBackup,
                  onImport: _busy ? null : _importCharacterCardsBackup,
                ),
                const Divider(height: 20),
                _BackupLine(
                  title: l10n.settingsDataCharacterGroupsBackup,
                  subtitle: l10n.settingsDataBackupCount(
                    data.characterGroupCount,
                  ),
                  description:
                      l10n.settingsDataCharacterGroupsBackupDescription,
                  onExport: _busy ? null : _copyCharacterGroupsBackup,
                  onImport: _busy ? null : _importCharacterGroupsBackup,
                ),
                const Divider(height: 20),
                _BackupLine(
                  title: l10n.settingsDataModelConfigsBackup,
                  subtitle: l10n.settingsDataBackupCount(data.modelConfigCount),
                  description: l10n.settingsDataModelConfigsBackupDescription,
                  onExport: _busy ? null : _copyModelConfigsBackup,
                  onImport: null,
                ),
                const Divider(height: 20),
                _ActionLine(
                  icon: Icons.archive_outlined,
                  title: l10n.settingsDataExportRawSnapshot,
                  subtitle: l10n.settingsDataExportRawSnapshotDescription,
                  onTap: _busy ? null : _exportRawSnapshot,
                ),
                if (_lastSnapshotBytes != null)
                  Padding(
                    padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
                    child: Text(
                      l10n.settingsDataSnapshotBytes(_lastSnapshotBytes!),
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _DataSettingsData {
  const _DataSettingsData({
    required this.coreVersion,
    required this.inputTokens,
    required this.outputTokens,
    required this.chatHistoryCount,
    required this.characterCardCount,
    required this.characterGroupCount,
    required this.modelConfigCount,
  });

  final String coreVersion;
  final int inputTokens;
  final int outputTokens;
  final int chatHistoryCount;
  final int characterCardCount;
  final int characterGroupCount;
  final int modelConfigCount;
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

class _BackupLine extends StatelessWidget {
  const _BackupLine({
    required this.title,
    required this.subtitle,
    required this.description,
    required this.onExport,
    required this.onImport,
  });

  final String title;
  final String subtitle;
  final String description;
  final VoidCallback? onExport;
  final VoidCallback? onImport;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: const TextStyle(fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      subtitle,
                      style: TextStyle(color: colorScheme.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            description,
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              OutlinedButton.icon(
                onPressed: onExport,
                icon: const Icon(Icons.copy_outlined),
                label: Text(l10n.settingsDataCopyBackupJson),
              ),
              FilledButton.tonalIcon(
                onPressed: onImport,
                icon: const Icon(Icons.upload_file_outlined),
                label: Text(l10n.settingsDataImportBackupJson),
              ),
            ],
          ),
        ],
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
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}

class _BackupImportDialog extends StatefulWidget {
  const _BackupImportDialog();

  static Future<String?> show({required BuildContext context}) {
    return showDialog<String>(
      context: context,
      builder: (context) => const _BackupImportDialog(),
    );
  }

  @override
  State<_BackupImportDialog> createState() => _BackupImportDialogState();
}

class _BackupImportDialogState extends State<_BackupImportDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataImportBackupJson),
      content: SizedBox(
        width: 560,
        child: TextField(
          controller: _controller,
          minLines: 10,
          maxLines: 18,
          decoration: InputDecoration(
            labelText: l10n.settingsDataBackupJsonInput,
            border: const OutlineInputBorder(),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: Text(l10n.settingsDataImportBackupJson),
        ),
      ],
    );
  }
}

class _ActionLine extends StatelessWidget {
  const _ActionLine({
    required this.icon,
    required this.title,
    required this.onTap,
    this.subtitle,
    this.destructive = false,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final VoidCallback? onTap;
  final bool destructive;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color = destructive ? colorScheme.error : colorScheme.primary;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      dense: true,
      visualDensity: VisualDensity.compact,
      leading: Icon(icon, color: color),
      title: Text(title, style: TextStyle(color: destructive ? color : null)),
      subtitle: subtitle == null ? null : Text(subtitle!),
      trailing: const Icon(Icons.chevron_right),
      onTap: onTap,
    );
  }
}
