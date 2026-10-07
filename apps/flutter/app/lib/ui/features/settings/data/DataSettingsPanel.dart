// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';

import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/logging/DiagnosticLogExporter.dart';
import '../../../../core/host/FileSaveService.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../core/snapshot/SnapshotImportUploader.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/StorageDirectorySelectionError.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';
import 'UsageStatisticsDetailScreen.dart';

const XTypeGroup _rawSnapshotFileTypeGroup = XTypeGroup(
  label: 'Operit snapshot',
  extensions: <String>['opsnapshot', 'zip'],
);

const XTypeGroup _backupJsonFileTypeGroup = XTypeGroup(
  label: 'Operit JSON backup',
  extensions: <String>['json'],
);

const XTypeGroup _diagnosticLogFileTypeGroup = XTypeGroup(
  label: 'Diagnostic log',
  extensions: <String>['log', 'txt'],
);

const String _operit1SnapshotImportLogTag = 'Operit1SnapshotImport';

class DataSettingsPanel extends StatefulWidget {
  const DataSettingsPanel({
    super.key,
    GeneratedCoreProxyClients? clients,
    this.diagnosticLogExporter,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;
  final DiagnosticLogExporter? diagnosticLogExporter;

  @override
  State<DataSettingsPanel> createState() => _DataSettingsPanelState();
}

class _DataSettingsPanelState extends State<DataSettingsPanel> {
  Future<_DataSettingsData>? _future;
  bool _busy = false;
  bool _exportingLogs = false;
  int? _lastSnapshotBytes;
  StreamSubscription<Operit1SnapshotImportProgress>?
  _operit1ImportProgressSubscription;
  Operit1SnapshotImportProgress? _operit1ImportProgress;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  /// Stops observing import progress when the settings panel is removed.
  @override
  void dispose() {
    _operit1ImportProgressSubscription?.cancel();
    super.dispose();
  }

  /// Observes Runtime stages for the active Operit1 snapshot import.
  void _subscribeOperit1ImportProgress() {
    if (_operit1ImportProgressSubscription != null) {
      return;
    }
    _operit1ImportProgressSubscription = widget
        .clients
        .servicesSnapshotImportManager
        .operit1SnapshotImportProgressFlow()
        .listen(
          (progress) {
            if (!mounted) {
              return;
            }
            setState(() => _operit1ImportProgress = progress);
          },
          onError: (Object error, StackTrace stackTrace) {
            ClientLogger.e(
              'settings snapshot progress observation failed',
              tag: _operit1SnapshotImportLogTag,
              error: error,
              stackTrace: stackTrace,
            );
            _operit1ImportProgressSubscription = null;
          },
          onDone: () {
            // Core watch streams are one-shot and close after the terminal
            // event. Clear the handle so a later import can subscribe again.
            _operit1ImportProgressSubscription = null;
          },
        );
  }

  Future<_DataSettingsData> _load() async {
    final characterCardManager = widget.clients.preferencesCharacterCardManager;
    final characterGroupCardManager =
        widget.clients.preferencesCharacterGroupCardManager;
    final modelConfigManager = widget.clients.preferencesModelConfigManager;
    final storagePaths = await RuntimeBootstrapManager.instance
        .localRuntimeStorageBasePaths();
    return _DataSettingsData(
      coreVersion: await widget.clients.application.coreVersion(),
      storagePaths: storagePaths,
      inputTokens: await widget.clients.chatRuntimeHolderMain
          .inputTokenCountFlow()
          .first,
      outputTokens: await widget.clients.chatRuntimeHolderMain
          .outputTokenCountFlow()
          .first,
      chatHistoryCount:
          (await widget.clients.chatRuntimeHolderMain.chatHistoriesFlow().first)
              .length,
      characterCardCount:
          (await characterCardManager.getAllCharacterCards()).length,
      characterGroupCount:
          (await characterGroupCardManager.getAllCharacterGroupCards()).length,
      modelConfigCount:
          (await modelConfigManager.getAllModelSummaries()).length,
      persistThinkingParts: await widget
          .clients
          .preferencesThinkingPersistencePreferences
          .persistThinkingParts(),
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  /// Persists the thinking-persistence switch and refreshes the panel.
  Future<void> _setPersistThinkingParts(bool value) async {
    setState(() => _busy = true);
    try {
      await widget.clients.preferencesThinkingPersistencePreferences
          .savePersistThinkingParts(persist: value);
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
    _reload();
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

  Future<void> _openDetailedStatistics() {
    return UsageStatisticsDetailScreen.open(
      context: context,
      clients: widget.clients,
    );
  }

  /// Lets the user select and migrate the local storage location.
  Future<void> _changeStorageLocation() async {
    final l10n = AppLocalizations.of(context)!;
    final currentPaths = await RuntimeBootstrapManager.instance
        .localRuntimeStorageBasePaths();
    if (!mounted) {
      return;
    }
    final selection = await _StorageLocationEditDialog.show(
      context: context,
      initialPaths: currentPaths,
    );
    if (selection == null) {
      return;
    }
    final basePaths = await RuntimeBootstrapManager.instance
        .localRuntimeStoragePathsForRoots(
          selection.runtimeRoot,
          selection.workspaceRoot,
        );
    final identityPaths = await RuntimeBootstrapManager.instance
        .localRuntimeStoragePathsForIdentityRoots(
          basePaths.runtimeRoot,
          basePaths.workspaceRoot,
        );
    if (!mounted) {
      return;
    }
    final confirmed = await _StorageLocationConfirmDialog.show(
      context: context,
      paths: identityPaths,
    );
    if (confirmed != true) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _runStorageMigrate(identityPaths);
      await RuntimeBootstrapManager.instance.persistMigratedLocalRuntimeStorage(
        basePaths.runtimeRoot,
        basePaths.workspaceRoot,
      );
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.settingsDataStorageChanged)));
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataStorageChangeError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Runs the core storage migration command for the selected roots.
  Future<void> _runStorageMigrate(RuntimeStoragePaths paths) async {
    final value = await widget.clients.application.runCoreCommand(
      args: <String>[
        'storage',
        'migrate',
        '--runtime',
        paths.runtimeRoot,
        '--workspace',
        paths.workspaceRoot,
      ],
    );
    if (value is! Map<Object?, Object?>) {
      throw StateError('Invalid core command output');
    }
    final stderr = value['stderr']?.toString().trim() ?? '';
    if (stderr.isNotEmpty) {
      throw StateError(stderr);
    }
  }

  /// Builds the suggested file name for one JSON backup.
  String _backupJsonSuggestedName(String slug) {
    return 'operit-$slug-${DateTime.now().millisecondsSinceEpoch}.json';
  }

  /// Writes one JSON backup to a user-selected file.
  Future<String?> _saveBackupJson({
    required String jsonText,
    required String suggestedName,
  }) {
    return FileSaveService.saveBytes(
      bytes: Uint8List.fromList(utf8.encode(jsonText)),
      name: suggestedName,
      mimeType: 'application/json',
      acceptedTypeGroups: const <XTypeGroup>[_backupJsonFileTypeGroup],
    );
  }

  /// Reads one JSON backup from a user-selected file.
  Future<String?> _readBackupJson() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[_backupJsonFileTypeGroup],
    );
    if (file == null) {
      return null;
    }
    return file.readAsString();
  }

  /// Saves a streamed raw snapshot and releases its host-owned staging file.
  Future<void> _exportRawSnapshot() async {
    final l10n = AppLocalizations.of(context)!;
    final suggestedName = _rawSnapshotSuggestedName();
    setState(() => _busy = true);
    StagedArchiveFile? exported;
    try {
      final savedPath = await FileSaveService.saveGeneratedFile(
        // Generate the snapshot only after the registered output implementation is ready.
        generate: () async {
          final file = await widget.clients.servicesSnapshotImportManager
              .exportRawSnapshot();
          exported = file;
          return XFile(
            file.fileReference,
            name: suggestedName,
            mimeType: 'application/zip',
            length: file.archive.byteLength,
          );
        },
        name: suggestedName,
        acceptedTypeGroups: const <XTypeGroup>[_rawSnapshotFileTypeGroup],
      );
      if (savedPath == null || !mounted) {
        return;
      }
      setState(() {
        _lastSnapshotBytes = exported!.archive.byteLength;
      });
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(savedPath))));
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(l10n.settingsDataSnapshotExportError('$error')),
          ),
        );
      }
    } finally {
      try {
        final file = exported;
        if (file != null) {
          await widget.clients.servicesArchiveTransferManager
              .discardArchiveUpload(archiveId: file.archive.archiveId);
        }
      } finally {
        if (mounted) {
          setState(() => _busy = false);
        }
      }
    }
  }

  Future<void> _importRawSnapshot() async {
    final l10n = AppLocalizations.of(context)!;
    final file = await SnapshotImportFile.pick();
    if (file == null) {
      return;
    }
    setState(() => _busy = true);
    SnapshotImportSession? stagedSession;
    late final RawSnapshotManifest manifest;
    try {
      stagedSession = await SnapshotImportUploader(widget.clients).stage(file);
      manifest = await stagedSession.completeRaw();
    } catch (error) {
      final session = stagedSession;
      if (session != null) {
        await session.discard();
      }
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
    final session = stagedSession;
    if (!mounted) {
      await session.discard();
      return;
    }
    final confirmed = await _RawSnapshotRestoreDialog.show(
      context: context,
      manifest: manifest,
      byteCount: session.byteLength,
    );
    if (confirmed != true) {
      await session.discard();
      return;
    }
    setState(() => _busy = true);
    try {
      await session.commitRaw();
      await session.discard();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataSnapshotImported)),
      );
      _reload();
    } catch (error) {
      await session.discard();
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

  /// Publishes a local progress state for the file-selection and staging phase.
  void _setOperit1ImportProgress({
    required String stage,
    required String title,
    required String detail,
    required double progress,
  }) {
    if (!mounted) {
      return;
    }
    setState(() {
      _operit1ImportProgress = Operit1SnapshotImportProgress(
        stage: stage,
        title: title,
        detail: detail,
        progress: progress.clamp(0.0, 1.0).toDouble(),
        active: true,
      );
    });
  }

  /// Selects, previews, confirms, and imports a complete Operit1 snapshot.
  Future<void> _importOperit1Snapshot() async {
    final l10n = AppLocalizations.of(context)!;
    final file = await SnapshotImportFile.pick();
    if (file == null) {
      return;
    }
    if (!mounted) {
      await file.close();
      return;
    }
    setState(() {
      _busy = true;
      _operit1ImportProgress = const Operit1SnapshotImportProgress(
        stage: 'prepare',
        title: '准备导入',
        detail: '正在准备 Operit1 快照。',
        progress: 0.0,
        active: true,
      );
    });
    ClientLogger.i(
      'settings snapshot selected name=${file.name} bytes=${file.byteLength}',
      tag: _operit1SnapshotImportLogTag,
    );
    SnapshotImportSession? stagedSession;
    late final Operit1SnapshotPreview preview;
    var lastUploadPercent = -1;
    try {
      stagedSession = await SnapshotImportUploader(widget.clients).stage(
        file,
        onProgress: (uploadedBytes, totalBytes) {
          final uploadPercent = totalBytes <= 0
              ? 100
              : (uploadedBytes * 100 / totalBytes).floor().clamp(0, 100);
          if (uploadPercent == lastUploadPercent) {
            return;
          }
          lastUploadPercent = uploadPercent;
          _setOperit1ImportProgress(
            stage: 'upload',
            title: '上传快照',
            detail: '正在读取并上传快照文件（$uploadPercent%）。',
            progress: 0.18 * uploadPercent / 100,
          );
        },
      );
      _setOperit1ImportProgress(
        stage: 'inspect',
        title: '检查快照',
        detail: '正在检查 Operit1 快照内容，请稍候。',
        progress: 0.20,
      );
      ClientLogger.i(
        'settings snapshot upload completed bytes=${stagedSession.byteLength}; inspection started',
        tag: _operit1SnapshotImportLogTag,
      );
      preview = await stagedSession.completeOperit1();
      ClientLogger.i(
        'settings snapshot inspection completed format=${preview.formatVersion} configs=${preview.modelConfig.configs.length} chats=${preview.chatCount} messages=${preview.messageCount}',
        tag: _operit1SnapshotImportLogTag,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'settings snapshot selection, upload, or inspection failed',
        tag: _operit1SnapshotImportLogTag,
        error: error,
        stackTrace: stackTrace,
      );
      final session = stagedSession;
      if (session != null) {
        await session.discard();
      }
      if (mounted) {
        setState(() => _operit1ImportProgress = null);
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsDataOperit1SnapshotImportError('$error')),
        ),
      );
      return;
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
    final session = stagedSession;
    if (!mounted) {
      await session.discard();
      return;
    }
    setState(() => _operit1ImportProgress = null);
    final confirmed = await _Operit1SnapshotImportDialog.show(
      context: context,
      fileName: file.name,
      preview: preview,
      byteCount: session.byteLength,
    );
    if (confirmed != true) {
      await session.discard();
      return;
    }
    setState(() {
      _busy = true;
      _operit1ImportProgress = const Operit1SnapshotImportProgress(
        stage: 'start',
        title: '开始导入',
        detail: '正在启动 Operit1 数据迁移。',
        progress: 0.02,
        active: true,
      );
    });
    try {
      ClientLogger.i(
        'settings snapshot import started bytes=${session.byteLength}',
        tag: _operit1SnapshotImportLogTag,
      );
      _subscribeOperit1ImportProgress();
      final result = await session.commitOperit1();
      ClientLogger.i(
        'settings snapshot import completed chats=${result.importedChats} messages=${result.importedMessages} memories=${result.importedMemories} files=${result.importedFiles + result.importedExternalFiles + result.importedWorkspaceFiles}',
        tag: _operit1SnapshotImportLogTag,
      );
      await session.discard();
      if (mounted) {
        setState(() => _operit1ImportProgress = null);
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataOperit1SnapshotImported)),
      );
      _reload();
    } catch (error, stackTrace) {
      ClientLogger.e(
        'settings snapshot import failed',
        tag: _operit1SnapshotImportLogTag,
        error: error,
        stackTrace: stackTrace,
      );
      await session.discard();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsDataOperit1SnapshotImportError('$error')),
        ),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Exports merged Core/client diagnostics through the platform save dialog.
  Future<void> _exportDiagnosticLogs() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _busy = true;
      _exportingLogs = true;
    });
    try {
      final exporter =
          widget.diagnosticLogExporter ??
          DiagnosticLogExporter(clients: widget.clients);
      final exported = await exporter.collect();
      if (!mounted) {
        return;
      }
      final savedPath = await FileSaveService.saveBytes(
        bytes: Uint8List.fromList(utf8.encode(exported.text)),
        name: 'operit-diagnostics-${DateTime.now().millisecondsSinceEpoch}.log',
        mimeType: 'text/plain',
        acceptedTypeGroups: const <XTypeGroup>[_diagnosticLogFileTypeGroup],
      );
      if (savedPath == null || !mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            exported.isPartial
                ? l10n.settingsDataLogsExportPartial(savedPath)
                : l10n.savedTo(savedPath),
          ),
        ),
      );
    } catch (error, stackTrace) {
      if (ClientLogger.isInitialized) {
        ClientLogger.e(
          'Diagnostic log export failed',
          tag: 'DiagnosticLogExport',
          error: error,
          stackTrace: stackTrace,
        );
      }
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.settingsDataLogsExportError('$error'))),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _exportingLogs = false;
        });
      }
    }
  }

  /// Exports all chat histories to a JSON file.
  Future<void> _exportChatHistoriesBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.chatRuntimeHolderMain
          .exportChatHistoriesToJson();
      final savedPath = await _saveBackupJson(
        jsonText: jsonText,
        suggestedName: _backupJsonSuggestedName('chat-histories'),
      );
      if (savedPath == null) {
        return;
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(savedPath))));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupExportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Imports all chat histories from a JSON file.
  Future<void> _importChatHistoriesBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _readBackupJson();
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

  /// Exports all character cards to a JSON file.
  Future<void> _exportCharacterCardsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesCharacterCardManager
          .exportAllCharacterCardsToBackupContent();
      final savedPath = await _saveBackupJson(
        jsonText: jsonText,
        suggestedName: _backupJsonSuggestedName('character-cards'),
      );
      if (savedPath == null) {
        return;
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(savedPath))));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupExportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Imports all character cards from a JSON file.
  Future<void> _importCharacterCardsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _readBackupJson();
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

  /// Exports all character groups to a JSON file.
  Future<void> _exportCharacterGroupsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesCharacterGroupCardManager
          .exportAllCharacterGroupsToBackupContent();
      final savedPath = await _saveBackupJson(
        jsonText: jsonText,
        suggestedName: _backupJsonSuggestedName('character-groups'),
      );
      if (savedPath == null) {
        return;
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(savedPath))));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupExportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Imports all character groups from a JSON file.
  Future<void> _importCharacterGroupsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _readBackupJson();
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

  /// Imports model configurations from a JSON file.
  Future<void> _importModelConfigsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _readBackupJson();
    if (jsonText == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final result = await widget.clients.preferencesModelConfigManager
          .importAllProvidersFromBackupContent(jsonContent: jsonText);
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

  /// Exports all model configurations to a JSON file.
  Future<void> _exportModelConfigsBackup() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final jsonText = await widget.clients.preferencesModelConfigManager
          .exportAllProviders();
      final savedPath = await _saveBackupJson(
        jsonText: jsonText,
        suggestedName: _backupJsonSuggestedName('model-configs'),
      );
      if (savedPath == null) {
        return;
      }
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.savedTo(savedPath))));
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsDataBackupExportError('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Keeps diagnostics usable even if the overview cannot connect to Core.
  Widget _buildDiagnosticLogsCard(
    AppLocalizations l10n,
    ColorScheme colorScheme,
  ) {
    return _SectionCard(
      title: l10n.settingsDataLogsSection,
      children: <Widget>[
        Text(
          l10n.settingsDataLogsDescription,
          style: TextStyle(color: colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 12),
        FilledButton.tonalIcon(
          onPressed: _busy ? null : _exportDiagnosticLogs,
          icon: _exportingLogs
              ? const M3LoadingIndicator(size: 20)
              : const Icon(Icons.download_outlined),
          label: Text(
            _exportingLogs
                ? l10n.settingsDataLogsExporting
                : l10n.settingsDataExportLogs,
          ),
        ),
      ],
    );
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
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
            children: <Widget>[
              _buildDiagnosticLogsCard(l10n, colorScheme),
              if (snapshot.hasError)
                _SectionCard(
                  title: l10n.settingsDataRuntimeSection,
                  children: <Widget>[
                    Text(
                      l10n.settingsDataOverviewLoadError('${snapshot.error}'),
                    ),
                  ],
                )
              else
                const SizedBox(height: 160, child: M3LoadingPane()),
            ],
          );
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _buildDiagnosticLogsCard(l10n, colorScheme),
            _SectionCard(
              title: l10n.settingsDataBackupSection,
              children: <Widget>[
                _SnapshotBackupLine(
                  lastSnapshotBytes: _lastSnapshotBytes,
                  onExport: _busy ? null : _exportRawSnapshot,
                  onImport: _busy ? null : _importRawSnapshot,
                  onImportOperit1: _busy ? null : _importOperit1Snapshot,
                  operit1ImportProgress: _operit1ImportProgress,
                ),
                const Divider(height: 20),
                Theme(
                  data: Theme.of(
                    context,
                  ).copyWith(dividerColor: Colors.transparent),
                  child: ExpansionTile(
                    tilePadding: EdgeInsets.zero,
                    childrenPadding: EdgeInsets.zero,
                    shape: const Border(),
                    collapsedShape: const Border(),
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
                        onExport: _busy ? null : _exportChatHistoriesBackup,
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
                        onExport: _busy ? null : _exportCharacterCardsBackup,
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
                        onExport: _busy ? null : _exportCharacterGroupsBackup,
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
                        onExport: _busy ? null : _exportModelConfigsBackup,
                        onImport: _busy ? null : _importModelConfigsBackup,
                      ),
                    ],
                  ),
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsDataPrivacySection,
              children: <Widget>[
                _ThinkingPersistenceLine(
                  value: data.persistThinkingParts,
                  onChanged:
                      _busy ? null : (value) => _setPersistThinkingParts(value),
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
                const Divider(height: 20),
                _StorageLocationLine(
                  paths: data.storagePaths,
                  onChange: _busy ? null : _changeStorageLocation,
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
                const SizedBox(height: 8),
                FilledButton.tonalIcon(
                  onPressed: _busy ? null : _openDetailedStatistics,
                  icon: const Icon(Icons.query_stats_outlined),
                  label: Text(l10n.settingsDataOpenDetailedStats),
                ),
                const SizedBox(height: 8),
                Text(
                  l10n.settingsDataOpenDetailedStatsDescription,
                  style: TextStyle(color: colorScheme.onSurfaceVariant),
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
    required this.storagePaths,
    required this.inputTokens,
    required this.outputTokens,
    required this.chatHistoryCount,
    required this.characterCardCount,
    required this.characterGroupCount,
    required this.modelConfigCount,
    required this.persistThinkingParts,
  });

  final String coreVersion;
  final RuntimeStoragePaths storagePaths;
  final int inputTokens;
  final int outputTokens;
  final int chatHistoryCount;
  final int characterCardCount;
  final int characterGroupCount;
  final int modelConfigCount;
  final bool persistThinkingParts;
}

/// Renders the thinking-persistence switch in the privacy section.
class _ThinkingPersistenceLine extends StatelessWidget {
  const _ThinkingPersistenceLine({
    required this.value,
    required this.onChanged,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  l10n.settingsDataThinkingPersistenceTitle,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 6),
                Text(
                  l10n.settingsDataThinkingPersistenceDescription,
                  style: TextStyle(color: colorScheme.onSurfaceVariant),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          Switch(value: value, onChanged: onChanged),
        ],
      ),
    );
  }
}

class _StorageLocationLine extends StatelessWidget {
  const _StorageLocationLine({required this.paths, required this.onChange});

  final RuntimeStoragePaths paths;
  final VoidCallback? onChange;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        ListTile(
          contentPadding: EdgeInsets.zero,
          leading: Icon(Icons.folder_copy_outlined, color: colorScheme.primary),
          title: Text(
            l10n.settingsDataStorageSection,
            style: const TextStyle(fontWeight: FontWeight.w700),
          ),
          subtitle: Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(l10n.settingsDataStorageDescription),
          ),
          trailing: FilledButton.tonalIcon(
            onPressed: onChange,
            icon: const Icon(Icons.drive_folder_upload_outlined),
            label: Text(l10n.settingsDataChooseStorageRoots),
          ),
        ),
        _InfoLine(
          label: l10n.settingsDataRuntimeRoot,
          value: paths.runtimeRoot,
        ),
        _InfoLine(
          label: l10n.settingsDataWorkspaceRoot,
          value: paths.workspaceRoot,
        ),
      ],
    );
  }
}

class _SnapshotBackupLine extends StatelessWidget {
  const _SnapshotBackupLine({
    required this.lastSnapshotBytes,
    required this.onExport,
    required this.onImport,
    required this.onImportOperit1,
    required this.operit1ImportProgress,
  });

  final int? lastSnapshotBytes;
  final VoidCallback? onExport;
  final VoidCallback? onImport;
  final VoidCallback? onImportOperit1;
  final Operit1SnapshotImportProgress? operit1ImportProgress;

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
              FilledButton.tonalIcon(
                onPressed: onImportOperit1,
                icon: const Icon(Icons.move_to_inbox_outlined),
                label: Text(l10n.settingsDataImportOperit1Snapshot),
              ),
            ],
          ),
          if (operit1ImportProgress != null) ...<Widget>[
            const SizedBox(height: 16),
            _Operit1SnapshotImportProgressPanel(
              progress: operit1ImportProgress!,
            ),
          ],
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
                icon: const Icon(Icons.download_outlined),
                label: Text(l10n.settingsDataExportBackupJson),
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

class _StorageLocationConfirmDialog extends StatelessWidget {
  const _StorageLocationConfirmDialog({required this.paths});

  final RuntimeStoragePaths paths;

  /// Shows a confirmation dialog for the storage migration targets.
  static Future<bool?> show({
    required BuildContext context,
    required RuntimeStoragePaths paths,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) => _StorageLocationConfirmDialog(paths: paths),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataStorageConfirmTitle),
      content: SizedBox(
        width: 560,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(l10n.settingsDataStorageConfirmMessage),
            const SizedBox(height: 16),
            _InfoLine(
              label: l10n.settingsDataRuntimeRoot,
              value: paths.runtimeRoot,
            ),
            _InfoLine(
              label: l10n.settingsDataWorkspaceRoot,
              value: paths.workspaceRoot,
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.settingsDataStorageConfirmAction),
        ),
      ],
    );
  }
}

class _StorageRootSelection {
  const _StorageRootSelection({
    required this.runtimeRoot,
    required this.workspaceRoot,
  });

  final String runtimeRoot;
  final String workspaceRoot;
}

class _StorageLocationEditDialog extends StatefulWidget {
  const _StorageLocationEditDialog({required this.initialPaths});

  final RuntimeStoragePaths initialPaths;

  /// Shows the editable runtime and workspace root dialog.
  static Future<_StorageRootSelection?> show({
    required BuildContext context,
    required RuntimeStoragePaths initialPaths,
  }) {
    return showDialog<_StorageRootSelection>(
      context: context,
      builder: (context) =>
          _StorageLocationEditDialog(initialPaths: initialPaths),
    );
  }

  /// Creates the state for the editable storage root dialog.
  @override
  State<_StorageLocationEditDialog> createState() =>
      _StorageLocationEditDialogState();
}

class _StorageLocationEditDialogState
    extends State<_StorageLocationEditDialog> {
  late final TextEditingController _runtimeRootController;
  late final TextEditingController _workspaceRootController;
  String? _errorText;

  /// Initializes the editable roots from the active configuration.
  @override
  void initState() {
    super.initState();
    _runtimeRootController = TextEditingController(
      text: widget.initialPaths.runtimeRoot,
    );
    _workspaceRootController = TextEditingController(
      text: widget.initialPaths.workspaceRoot,
    );
  }

  /// Releases the storage path text controllers.
  @override
  void dispose() {
    _runtimeRootController.dispose();
    _workspaceRootController.dispose();
    super.dispose();
  }

  /// Selects a new runtime root directory.
  Future<void> _selectRuntimeRoot() =>
      _selectStorageRoot(_runtimeRootController);

  /// Selects a new workspace root directory.
  Future<void> _selectWorkspaceRoot() =>
      _selectStorageRoot(_workspaceRootController);

  /// Keeps picker failures in the storage UI instead of the unhandled zone.
  Future<void> _selectStorageRoot(TextEditingController controller) async {
    try {
      final path = await OperitFolderAccess.pickDirectory();
      if (!mounted || path == null || path.trim().isEmpty) {
        return;
      }
      setState(() {
        controller.text = path.trim();
        _errorText = null;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _errorText = storageDirectorySelectionErrorMessage(
          AppLocalizations.of(context)!,
          error,
        );
      });
    }
  }

  /// Returns the explicitly entered runtime and workspace roots.
  void _submit() {
    final l10n = AppLocalizations.of(context)!;
    final runtimeRoot = _runtimeRootController.text.trim();
    final workspaceRoot = _workspaceRootController.text.trim();
    if (runtimeRoot.isEmpty || workspaceRoot.isEmpty) {
      setState(() {
        _errorText = l10n.settingsDataStorageRootsRequired;
      });
      return;
    }
    Navigator.of(context).pop(
      _StorageRootSelection(
        runtimeRoot: runtimeRoot,
        workspaceRoot: workspaceRoot,
      ),
    );
  }

  /// Builds the editable runtime and workspace root dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataEditStorageRootsTitle),
      content: SizedBox(
        width: 620,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _StorageRootEditField(
              controller: _runtimeRootController,
              label: l10n.settingsDataRuntimeRoot,
              onBrowse: _selectRuntimeRoot,
            ),
            const SizedBox(height: 14),
            _StorageRootEditField(
              controller: _workspaceRootController,
              label: l10n.settingsDataWorkspaceRoot,
              onBrowse: _selectWorkspaceRoot,
            ),
            if (_errorText != null) ...<Widget>[
              const SizedBox(height: 12),
              Text(
                _errorText!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _submit, child: Text(l10n.ok)),
      ],
    );
  }
}

class _StorageRootEditField extends StatelessWidget {
  const _StorageRootEditField({
    required this.controller,
    required this.label,
    required this.onBrowse,
  });

  final TextEditingController controller;
  final String label;
  final VoidCallback onBrowse;

  /// Builds one editable storage root with a directory picker.
  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      decoration: InputDecoration(
        labelText: label,
        border: const OutlineInputBorder(),
        suffixIcon: IconButton(
          onPressed: onBrowse,
          tooltip: label,
          icon: const Icon(Icons.folder_open_outlined),
        ),
      ),
      autocorrect: false,
      enableSuggestions: false,
      style: const TextStyle(
        fontFamily: 'monospace',
        fontSize: 13,
        letterSpacing: 0,
      ),
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

class _Operit1SnapshotImportDialog extends StatelessWidget {
  const _Operit1SnapshotImportDialog({
    required this.fileName,
    required this.preview,
    required this.byteCount,
  });

  final String fileName;
  final Operit1SnapshotPreview preview;
  final int byteCount;

  /// Shows the pre-import summary for an Operit1 snapshot.
  static Future<bool?> show({
    required BuildContext context,
    required String fileName,
    required Operit1SnapshotPreview preview,
    required int byteCount,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) => _Operit1SnapshotImportDialog(
        fileName: fileName,
        preview: preview,
        byteCount: byteCount,
      ),
    );
  }

  /// Builds the Operit1 import confirmation dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataImportOperit1Snapshot),
      content: SizedBox(
        width: 560,
        child: Text(
          l10n.settingsDataOperit1SnapshotImportConfirmMessage(
            fileName,
            preview.formatVersion,
            preview.modelConfig.chatModelId ?? '-',
            preview.chatCount,
            preview.messageCount,
            preview.importedFileCount + preview.importedExternalFileCount,
            byteCount,
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.settingsDataOperit1SnapshotImportAction),
        ),
      ],
    );
  }
}

class _Operit1SnapshotImportProgressPanel extends StatelessWidget {
  const _Operit1SnapshotImportProgressPanel({required this.progress});

  final Operit1SnapshotImportProgress progress;

  /// Builds the active Operit1 import stage and its completion percentage.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final value = progress.progress.clamp(0.0, 1.0).toDouble();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                progress.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: textTheme.labelLarge?.copyWith(
                  color: colorScheme.onSurface,
                  fontWeight: FontWeight.w800,
                  letterSpacing: 0,
                ),
              ),
            ),
            const SizedBox(width: 12),
            Text(
              '${(value * 100).round()}%',
              style: textTheme.labelMedium?.copyWith(
                color: colorScheme.primary,
                fontWeight: FontWeight.w800,
                letterSpacing: 0,
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: LinearProgressIndicator(
            value: value,
            minHeight: 6,
            backgroundColor: colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.72,
            ),
          ),
        ),
        const SizedBox(height: 8),
        Text(
          progress.detail,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
            height: 1.35,
          ),
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
