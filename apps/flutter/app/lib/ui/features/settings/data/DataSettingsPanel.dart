// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';

import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:flutter/material.dart';

import '../../../../core/backup/ChatBackupService.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/bridge/PlatformCoreProxy.dart';
import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/logging/DiagnosticLogExporter.dart';
import '../../../../core/host/FileSaveService.dart';
import '../../../../core/host/SelectedFileInput.dart' show SelectedFileInput;
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../core/snapshot/SnapshotImportUploader.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/StorageDirectorySelectionError.dart';
import '../../../common/ChatBackupRestorePanel.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

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
    this.backupService,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;
  final DiagnosticLogExporter? diagnosticLogExporter;
  final ChatBackupService? backupService;

  @override
  State<DataSettingsPanel> createState() => _DataSettingsPanelState();
}

class _DataSettingsPanelState extends State<DataSettingsPanel> {
  late Future<_DataSettingsData> _future;
  bool _busy = false;
  bool _exportingLogs = false;
  String? _activeTransfer;
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

  /// Loads runtime details independently of backup and file transfer actions.
  Future<_DataSettingsData> _load() async {
    final storagePaths = await RuntimeBootstrapManager.instance
        .localRuntimeStorageBasePaths();
    return _DataSettingsData(
      coreVersion: await widget.clients.application.coreVersion(),
      storagePaths: storagePaths,
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
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

  /// Saves a streamed raw snapshot and releases its host-owned staging file.
  Future<void> _exportRawSnapshot() {
    final l10n = AppLocalizations.of(context)!;
    final suggestedName = _rawSnapshotSuggestedName();
    return _runTransfer(
      l10n.settingsDataExportRawSnapshot,
      l10n.settingsDataSnapshotExportError,
      () async {
        StagedArchiveFile? exported;
        try {
          final savedPath = await FileSaveService.saveGeneratedFile(
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
          if (savedPath != null && mounted)
            _showMessage(l10n.savedTo(savedPath));
        } finally {
          final file = exported;
          if (file != null) {
            await widget.clients.servicesArchiveTransferManager
                .discardArchiveUpload(archiveId: file.archive.archiveId);
          }
        }
      },
    );
  }

  /// Locks data actions for the picker, review, and execution of one transfer.
  Future<void> _runTransfer(
    String title,
    String Function(String) errorMessage,
    Future<void> Function() action,
  ) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _activeTransfer = title;
    });
    try {
      await action();
    } catch (error, stackTrace) {
      if (ClientLogger.isInitialized) {
        ClientLogger.e(
          'Data transfer failed',
          tag: 'DataSettings',
          error: error,
          stackTrace: stackTrace,
        );
      }
      if (mounted) _showMessage(errorMessage('$error'));
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _activeTransfer = null;
          _operit1ImportProgress = null;
        });
      }
    }
  }

  /// Pauses progress while waiting for the user to approve the transfer.
  Future<bool> _confirmTransfer(Future<bool?> Function() showReview) async {
    final title = _activeTransfer;
    setState(() {
      _activeTransfer = null;
      _operit1ImportProgress = null;
    });
    final confirmed = await showReview();
    if (confirmed != true || !mounted) return false;
    setState(() => _activeTransfer = title);
    return true;
  }

  /// Reviews a portable archive and restarts the application after replacing runtime data.
  Future<void> _importRawSnapshot() {
    final l10n = AppLocalizations.of(context)!;
    return _runTransfer(
      l10n.settingsDataImportRawSnapshot,
      l10n.settingsDataSnapshotImportError,
      () async {
        final file = await SelectedFileInput.pickSnapshot();
        if (file == null) return;
        if (!mounted) {
          await file.close();
          return;
        }
        final session = await SnapshotImportUploader(
          widget.clients,
        ).stage(file);
        var restored = false;
        try {
          final manifest = await session.completeRaw();
          if (!mounted) return;
          final confirmed = await _confirmTransfer(
            () => _RawSnapshotRestoreDialog.show(
              context: context,
              manifest: manifest,
              byteCount: session.byteLength,
            ),
          );
          if (!confirmed) return;
          await session.commitRaw();
          restored = true;
        } finally {
          try {
            await session.discard();
          } finally {
            if (restored) await platformCoreProxy.restartApplication();
          }
        }
        if (mounted) _showMessage(l10n.settingsDataSnapshotImported);
      },
    );
  }

  /// Shows progress while the selected Operit1 file is read and migrated.
  void _setOperit1ImportProgress({
    required String title,
    required String detail,
    required double progress,
  }) {
    if (!mounted) return;
    setState(() {
      _operit1ImportProgress = Operit1SnapshotImportProgress(
        stage: '',
        title: title,
        detail: detail,
        progress: progress,
        active: true,
      );
    });
  }

  /// Selects, previews, and imports a complete Operit1 snapshot into the current data.
  Future<void> _importOperit1Snapshot() {
    final l10n = AppLocalizations.of(context)!;
    return _runTransfer(
      l10n.settingsDataImportOperit1Snapshot,
      l10n.settingsDataOperit1SnapshotImportError,
      () async {
        final file = await SelectedFileInput.pickSnapshot();
        if (file == null) return;
        if (!mounted) {
          await file.close();
          return;
        }
        var lastUploadPercent = -1;
        final session = await SnapshotImportUploader(widget.clients).stage(
          file,
          onProgress: (uploadedBytes, totalBytes) {
            final percent = totalBytes == 0
                ? 100
                : 100 * uploadedBytes ~/ totalBytes;
            if (percent == lastUploadPercent) return;
            lastUploadPercent = percent;
            _setOperit1ImportProgress(
              title: l10n.settingsDataReadingSnapshot,
              detail: file.name,
              progress: 0.18 * percent / 100,
            );
          },
        );
        try {
          _setOperit1ImportProgress(
            title: l10n.settingsDataPreparingPreview,
            detail: file.name,
            progress: 0.20,
          );
          final preview = await session.completeOperit1();
          if (!mounted) return;
          final confirmed = await _confirmTransfer(
            () => _Operit1SnapshotImportDialog.show(
              context: context,
              fileName: file.name,
              preview: preview,
              byteCount: session.byteLength,
            ),
          );
          if (!confirmed) return;
          _subscribeOperit1ImportProgress();
          final result = await session.commitOperit1();
          if (mounted) {
            _showMessage(
              l10n.settingsDataOperit1ImportResult(
                result.importedChats,
                result.importedMessages,
              ),
            );
          }
        } finally {
          try {
            await _operit1ImportProgressSubscription?.cancel();
            _operit1ImportProgressSubscription = null;
          } finally {
            await session.discard();
          }
        }
      },
    );
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

  /// Exports only the selected data type in its portable JSON format.
  Future<void> _exportJson({required bool chat}) {
    final l10n = AppLocalizations.of(context)!;
    final title = chat
        ? l10n.settingsDataExportChatJson
        : l10n.settingsDataExportModelJson;
    return _runTransfer(title, l10n.settingsDataBackupExportError, () async {
      final jsonText = chat
          ? await widget.clients.chatRuntimeHolderMain
                .exportChatHistoriesToJson()
          : await widget.clients.preferencesModelConfigManager
                .exportAllProviders();
      if (!mounted) return;
      final savedPath = await _saveBackupJson(
        jsonText: jsonText,
        suggestedName: _backupJsonSuggestedName(
          chat ? 'chat-histories' : 'model-configs',
        ),
      );
      if (savedPath != null && mounted) _showMessage(l10n.savedTo(savedPath));
    });
  }

  /// Confirms which JSON file will update matching records before applying it.
  Future<void> _importJson({required bool chat}) {
    final l10n = AppLocalizations.of(context)!;
    final title = chat
        ? l10n.settingsDataImportChatJson
        : l10n.settingsDataImportModelJson;
    final description = chat
        ? l10n.settingsDataImportChatJsonDescription
        : l10n.settingsDataImportModelJsonDescription;
    return _runTransfer(title, l10n.settingsDataBackupImportError, () async {
      final file = await openFile(
        acceptedTypeGroups: const [_backupJsonFileTypeGroup],
      );
      if (file == null || !mounted) return;
      final confirmed = await _confirmTransfer(
        () => showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(title),
            scrollable: true,
            content: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(file.name, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 8),
                Text(description),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: Text(l10n.settingsDataImportAction),
              ),
            ],
          ),
        ),
      );
      if (!confirmed) return;
      final jsonText = await file.readAsString();
      if (!mounted) return;
      final (added, updated, skipped) = chat
          ? await _importChatJson(jsonText)
          : await _importModelJson(jsonText);
      if (mounted)
        _showMessage(
          l10n.settingsDataBackupImportResult(added, updated, skipped),
        );
    });
  }

  /// Applies a chat archive and returns its change counts.
  Future<(int, int, int)> _importChatJson(String jsonText) async {
    final result = await widget.clients.chatRuntimeHolderMain
        .importChatHistoriesFromJson(jsonString: jsonText);
    return (result.newValue, result.updated, result.skipped);
  }

  /// Applies a model configuration archive and returns its change counts.
  Future<(int, int, int)> _importModelJson(String jsonText) async {
    final result = await widget.clients.preferencesModelConfigManager
        .importAllProvidersFromBackupContent(jsonContent: jsonText);
    return (result.newValue, result.updated, result.skipped);
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

  /// Resolves the service that owns the scheduled backup destination.
  ChatBackupService get _backupService =>
      widget.backupService ?? ChatBackupService.instance;

  /// Runs one explicit database action at a time from the settings panel.
  Future<void> _runBackupAction(Future<void> Function() action) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await action();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// Shows the saved destination and the latest successful database copy.
  Widget _buildBackupCard(AppLocalizations l10n) => ListenableBuilder(
    listenable: _backupService,
    builder: (context, _) {
      final service = _backupService;
      final directory = service.directory;
      final ready = directory != null && !service.needsDirectory;
      final lastSuccess = service.lastSuccess?.toLocal();
      final material = MaterialLocalizations.of(context);
      final theme = Theme.of(context);
      return _SectionCard(
        title: l10n.settingsDataDatabaseBackup,
        children: [
          Text(l10n.settingsDataDatabaseBackupDescription),
          const SizedBox(height: 12),
          Text(
            ready
                ? l10n.settingsDataBackupScheduled
                : l10n.settingsDataBackupUnavailable,
            style: theme.textTheme.titleSmall,
          ),
          if (directory != null) SelectableText(directory.label),
          if (lastSuccess != null)
            Text(
              l10n.chatBackupLastSuccess(
                '${material.formatMediumDate(lastSuccess)} · '
                '${material.formatTimeOfDay(TimeOfDay.fromDateTime(lastSuccess))}',
              ),
            )
          else if (ready)
            Text(l10n.chatBackupNever),
          if (service.error != null)
            SelectableText(
              service.error!,
              style: TextStyle(color: theme.colorScheme.error),
            ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              if (ready)
                FilledButton.icon(
                  onPressed: _busy || service.backingUp
                      ? null
                      : () => _runBackupAction(() async {
                          await service.checkAndBackup(force: true);
                          if (mounted && service.error == null) {
                            _showMessage(l10n.settingsDataBackupSaved);
                          }
                        }),
                  icon: const Icon(Icons.backup_outlined),
                  label: Text(l10n.settingsDataBackupNow),
                ),
              OutlinedButton.icon(
                onPressed: _busy || service.backingUp
                    ? null
                    : () => _runBackupAction(() async {
                        await service.selectDirectory();
                      }),
                icon: const Icon(Icons.folder_open_outlined),
                label: Text(
                  ready ? l10n.chatBackupChange : l10n.chatBackupChoose,
                ),
              ),
            ],
          ),
          if (service.backingUp) ...[
            const SizedBox(height: 12),
            const LinearProgressIndicator(),
          ],
        ],
      );
    },
  );

  /// Shows application feedback for completed or failed operations.
  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  /// Shows a named operation with feedback next to the action that started it.
  Widget _action({
    required IconData icon,
    required String title,
    String? description,
    required VoidCallback onTap,
    Widget? progress,
  }) {
    final working = _activeTransfer == title;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ListTile(
          contentPadding: EdgeInsets.zero,
          leading: Icon(icon, color: Theme.of(context).colorScheme.primary),
          title: Text(title),
          subtitle: description == null ? null : Text(description),
          trailing: working
              ? const M3LoadingIndicator(size: 20)
              : const Icon(Icons.chevron_right),
          onTap: _busy ? null : onTap,
        ),
        if (working && progress != null)
          Padding(padding: const EdgeInsets.only(bottom: 12), child: progress),
      ],
    );
  }

  /// Places backup, recovery, and data import before storage and support tools.
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<_DataSettingsData>(
      future: _future,
      builder: _buildPage,
    );
  }

  /// Keeps backup and transfer controls available while the runtime details load.
  Widget _buildPage(
    BuildContext context,
    AsyncSnapshot<_DataSettingsData> runtime,
  ) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final operit1Progress = _operit1ImportProgress;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760),
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: [
            _buildBackupCard(l10n),
            _SectionCard(
              title: l10n.settingsDataRecoverySection,
              children: [
                _action(
                  icon: Icons.restore_outlined,
                  title: l10n.chatRestoreTitle,
                  description: l10n.settingsDataRestoreDatabaseDescription,
                  onTap: () => _runBackupAction(() async {
                    await ChatBackupRestorePanel.show(context, _backupService);
                  }),
                ),
                const Divider(height: 1),
                _action(
                  icon: Icons.folder_zip_outlined,
                  title: l10n.settingsDataImportRawSnapshot,
                  description: l10n.settingsDataRestoreArchiveDescription,
                  onTap: _importRawSnapshot,
                ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsDataImportSection,
              children: [
                _action(
                  icon: Icons.move_to_inbox_outlined,
                  title: l10n.settingsDataImportOperit1Snapshot,
                  description: l10n.settingsDataImportOperit1Description,
                  onTap: _importOperit1Snapshot,
                  progress: operit1Progress == null
                      ? null
                      : _Operit1SnapshotImportProgressPanel(
                          progress: operit1Progress,
                        ),
                ),
                const Divider(height: 1),
                _action(
                  icon: Icons.chat_bubble_outline,
                  title: l10n.settingsDataImportChatJson,
                  description: l10n.settingsDataImportChatJsonDescription,
                  onTap: () => _importJson(chat: true),
                ),
                const Divider(height: 1),
                _action(
                  icon: Icons.tune_outlined,
                  title: l10n.settingsDataImportModelJson,
                  description: l10n.settingsDataImportModelJsonDescription,
                  onTap: () => _importJson(chat: false),
                ),
              ],
            ),
            _SectionCard(
              children: [
                ExpansionTile(
                  tilePadding: EdgeInsets.zero,
                  shape: const Border(),
                  collapsedShape: const Border(),
                  title: Text(
                    l10n.settingsDataExportSection,
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
                  ),
                  expandedCrossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _action(
                      icon: Icons.folder_zip_outlined,
                      title: l10n.settingsDataExportRawSnapshot,
                      description:
                          l10n.settingsDataExportRawSnapshotDescription,
                      onTap: _exportRawSnapshot,
                    ),
                    _action(
                      icon: Icons.chat_bubble_outline,
                      title: l10n.settingsDataExportChatJson,
                      onTap: () => _exportJson(chat: true),
                    ),
                    _action(
                      icon: Icons.tune_outlined,
                      title: l10n.settingsDataExportModelJson,
                      onTap: () => _exportJson(chat: false),
                    ),
                  ],
                ),
              ],
            ),
            _buildRuntimeCard(l10n, runtime),
            _buildDiagnosticLogsCard(l10n, colorScheme),
          ],
        ),
      ),
    );
  }

  /// Displays runtime details without delaying display of backup and recovery.
  Widget _buildRuntimeCard(
    AppLocalizations l10n,
    AsyncSnapshot<_DataSettingsData> runtime,
  ) {
    final data = runtime.data;
    return _SectionCard(
      title: l10n.settingsDataRuntimeSection,
      children: [
        if (data != null) ...[
          _InfoLine(
            label: l10n.settingsDataCoreVersion,
            value: data.coreVersion,
          ),
          const Divider(height: 20),
          _StorageLocationLine(
            paths: data.storagePaths,
            onChange: _busy ? null : _changeStorageLocation,
          ),
        ] else if (runtime.hasError)
          Text(l10n.settingsDataOverviewLoadError('${runtime.error}'))
        else
          const LinearProgressIndicator(),
      ],
    );
  }
}

class _DataSettingsData {
  /// Holds the current runtime version and local storage paths.
  const _DataSettingsData({
    required this.coreVersion,
    required this.storagePaths,
  });

  final String coreVersion;
  final RuntimeStoragePaths storagePaths;
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

class _SectionCard extends StatelessWidget {
  const _SectionCard({this.title, required this.children});

  final String? title;
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
              if (title != null) ...[
                Text(
                  title!,
                  style: SettingsControlStyles.sectionTitleTextStyle(context),
                ),
                const SizedBox(height: 8),
              ],
              ...children,
            ],
          ),
        ),
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
      scrollable: true,
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
      scrollable: true,
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
