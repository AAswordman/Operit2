// ignore_for_file: file_names

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

const XTypeGroup _rawSnapshotFileTypeGroup = XTypeGroup(
  label: 'Operit snapshot',
  extensions: <String>['opsnapshot', 'zip'],
);

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
    final l10n = AppLocalizations.of(context)!;
    final suggestedName = _rawSnapshotSuggestedName();
    final location = await getSaveLocation(
      acceptedTypeGroups: const <XTypeGroup>[_rawSnapshotFileTypeGroup],
      suggestedName: suggestedName,
    );
    if (location == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final bytes = await widget.clients.application.exportRawSnapshot();
      await XFile.fromData(
        Uint8List.fromList(bytes),
        name: suggestedName,
        mimeType: 'application/zip',
      ).saveTo(location.path);
      if (!mounted) {
        return;
      }
      setState(() {
        _lastSnapshotBytes = bytes.length;
      });
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(location.path))));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataSnapshotExportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _importRawSnapshot() async {
    final l10n = AppLocalizations.of(context)!;
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[_rawSnapshotFileTypeGroup],
    );
    if (file == null) {
      return;
    }
    setState(() => _busy = true);
    late final List<int> bytes;
    late final RawSnapshotManifest manifest;
    try {
      bytes = await file.readAsBytes();
      manifest = await widget.clients.application.inspectRawSnapshot(
        bytes: bytes,
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataSnapshotImportError('$error'))),
      );
      return;
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
    if (!mounted) {
      return;
    }
    final confirmed = await _RawSnapshotRestoreDialog.show(
      context: context,
      manifest: manifest,
      byteCount: bytes.length,
    );
    if (confirmed != true) {
      return;
    }
    setState(() => _busy = true);
    try {
      await widget.clients.application.importRawSnapshot(bytes: bytes);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataSnapshotImported)),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataSnapshotImportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
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
    final colorScheme = Theme.of(context).colorScheme;
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
              title: l10n.settingsDataBackupSection,
              children: <Widget>[
                _SnapshotBackupLine(
                  lastSnapshotBytes: _lastSnapshotBytes,
                  onExport: _busy ? null : _exportRawSnapshot,
                  onImport: _busy ? null : _importRawSnapshot,
                ),
                const Divider(height: 20),
                Theme(
                  data: Theme.of(
                    context,
                  ).copyWith(dividerColor: Colors.transparent),
                  child: ExpansionTile(
                    tilePadding: EdgeInsets.zero,
                    childrenPadding: EdgeInsets.zero,
                    leading: Icon(
                      Icons.tune_outlined,
                      color: colorScheme.primary,
                    ),
                    title: Text(
                      l10n.settingsDataAdvancedBackupOptions,
                      style: const TextStyle(fontWeight: FontWeight.w700),
                    ),
                    subtitle: Text(
                      l10n.settingsDataAdvancedBackupOptionsDescription,
                    ),
                    children: <Widget>[
                      _BackupLine(
                        title: l10n.settingsDataChatHistoriesBackup,
                        subtitle: l10n.settingsDataBackupCount(
                          data.chatHistoryCount,
                        ),
                        description:
                            l10n.settingsDataChatHistoriesBackupDescription,
                        onExport: _busy ? null : _copyChatHistoriesBackup,
                        onImport: _busy ? null : _importChatHistoriesBackup,
                      ),
                      const Divider(height: 20),
                      _BackupLine(
                        title: l10n.settingsDataCharacterCardsBackup,
                        subtitle: l10n.settingsDataBackupCount(
                          data.characterCardCount,
                        ),
                        description:
                            l10n.settingsDataCharacterCardsBackupDescription,
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
                        subtitle: l10n.settingsDataBackupCount(
                          data.modelConfigCount,
                        ),
                        description:
                            l10n.settingsDataModelConfigsBackupDescription,
                        onExport: _busy ? null : _copyModelConfigsBackup,
                        onImport: null,
                      ),
                    ],
                  ),
                ),
              ],
            ),
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

class _SnapshotBackupLine extends StatelessWidget {
  const _SnapshotBackupLine({
    required this.lastSnapshotBytes,
    required this.onExport,
    required this.onImport,
  });

  final int? lastSnapshotBytes;
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
          Text(
            l10n.settingsDataSnapshotBackupTitle,
            style: const TextStyle(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 6),
          Text(
            l10n.settingsDataExportRawSnapshotDescription,
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
          if (lastSnapshotBytes != null) ...<Widget>[
            const SizedBox(height: 8),
            Text(
              l10n.settingsDataSnapshotBytes(lastSnapshotBytes!),
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ],
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton.icon(
                onPressed: onExport,
                icon: const Icon(Icons.download_outlined),
                label: Text(l10n.settingsDataExportRawSnapshot),
              ),
              FilledButton.tonalIcon(
                onPressed: onImport,
                icon: const Icon(Icons.restore_outlined),
                label: Text(l10n.settingsDataImportRawSnapshot),
              ),
            ],
          ),
        ],
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

class _RawSnapshotRestoreDialog extends StatelessWidget {
  const _RawSnapshotRestoreDialog({
    required this.manifest,
    required this.byteCount,
  });

  final RawSnapshotManifest manifest;
  final int byteCount;

  static Future<bool?> show({
    required BuildContext context,
    required RawSnapshotManifest manifest,
    required int byteCount,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) =>
          _RawSnapshotRestoreDialog(manifest: manifest, byteCount: byteCount),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataSnapshotRestoreConfirmTitle),
      content: Text(
        l10n.settingsDataSnapshotRestoreConfirmMessage(
          manifest.formatVersion,
          manifest.includes.length,
          _formatSnapshotCreatedAt(manifest.createdAt),
          byteCount,
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.settingsDataSnapshotRestoreConfirmAction),
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
    this.destructive = false,
  });

  final IconData icon;
  final String title;
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
      trailing: const Icon(Icons.chevron_right),
      onTap: onTap,
    );
  }
}

String _rawSnapshotSuggestedName() {
  final now = DateTime.now();
  return 'operit-snapshot-${now.year}${_twoDigits(now.month)}'
      '${_twoDigits(now.day)}-${_twoDigits(now.hour)}'
      '${_twoDigits(now.minute)}${_twoDigits(now.second)}.opsnapshot';
}

String _formatSnapshotCreatedAt(int millisecondsSinceEpoch) {
  final createdAt = DateTime.fromMillisecondsSinceEpoch(
    millisecondsSinceEpoch,
  ).toLocal();
  return '${createdAt.year}-${_twoDigits(createdAt.month)}'
      '-${_twoDigits(createdAt.day)} ${_twoDigits(createdAt.hour)}'
      ':${_twoDigits(createdAt.minute)}';
}

String _twoDigits(int value) => value.toString().padLeft(2, '0');
