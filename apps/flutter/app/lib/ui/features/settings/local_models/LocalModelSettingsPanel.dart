// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/components/OperitDialog.dart';
import '../components/SettingsControlStyles.dart';
import '../../../theme/OperitGlassSurface.dart';

class LocalModelSettingsPanel extends StatefulWidget {
  const LocalModelSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  /// Creates mutable local model settings state.
  @override
  State<LocalModelSettingsPanel> createState() =>
      _LocalModelSettingsPanelState();
}

class _LocalModelSettingsPanelState extends State<LocalModelSettingsPanel> {
  Future<_LocalModelSettingsData>? _future;
  final Set<String> _activeOperations = <String>{};
  final Set<String> _pausedOperations = <String>{};
  Timer? _progressTimer;
  List<core_proxy.LocalModelInstallStatus>? _installStatuses;
  bool _statusRefreshRunning = false;

  /// Loads local model state when the panel is created.
  @override
  void initState() {
    super.initState();
    _reload();
    _progressTimer = Timer.periodic(
      const Duration(milliseconds: 500),
      (_) => _refreshInstallStatuses(),
    );
  }

  /// Refreshes installation status snapshots while the settings panel is mounted.
  Future<void> _refreshInstallStatuses() async {
    if (!mounted || _statusRefreshRunning) {
      return;
    }
    _statusRefreshRunning = true;
    try {
      final statuses = await widget.clients.servicesLocalModelService
          .getInstallStatuses();
      if (!mounted) {
        return;
      }
      setState(() {
        _installStatuses = statuses;
      });
    } catch (error) {
      debugPrint('Local model status refresh failed: $error');
    } finally {
      _statusRefreshRunning = false;
    }
  }

  /// Releases the installation status polling timer.
  @override
  void dispose() {
    _progressTimer?.cancel();
    super.dispose();
  }

  /// Reloads catalog, registry, and platform state from the runtime provider.
  void _reload() {
    setState(() {
      _future = _loadData();
    });
  }

  /// Reads local model settings data through the generated runtime client.
  Future<_LocalModelSettingsData> _loadData() async {
    final service = widget.clients.servicesLocalModelService;
    final results = await Future.wait<Object>(<Future<Object>>[
      service.getCatalogStatus(),
      service.getRegistry(),
      service.getPlatformTarget(),
      service.getInstallStatuses(),
    ]);
    return _LocalModelSettingsData(
      catalog: results[0] as List<core_proxy.LocalModelCatalogStatus>,
      registry: results[1] as core_proxy.LocalModelRegistrySnapshot,
      target: results[2] as core_proxy.LocalPlatformTarget,
      installStatuses: results[3] as List<core_proxy.LocalModelInstallStatus>,
    );
  }

  /// Runs one provider operation and refreshes the displayed installation state.
  Future<void> _runOperation(
    String operationKey,
    Future<void> Function() operation,
  ) async {
    if (_activeOperations.contains(operationKey)) {
      return;
    }
    setState(() {
      _activeOperations.add(operationKey);
    });
    try {
      await operation();
      if (!mounted) {
        return;
      }
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      if (_pausedOperations.remove(operationKey)) {
        _reload();
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.localModelsOperationFailed('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() {
          _activeOperations.remove(operationKey);
        });
      }
    }
  }

  /// Updates the preferred model download source.
  Future<void> _setPreferredSource(
    core_proxy.LocalModelSourceKind sourceKind,
  ) {
    return _runOperation('source:${sourceKind.value}', () async {
      await widget.clients.servicesLocalModelService.setPreferredSource(
        sourceKind: sourceKind,
      );
    });
  }

  /// Installs one model and its exact platform engine dependency.
  Future<void> _install(core_proxy.LocalModelManifest manifest) {
    return _runOperation(
      'install:${manifest.id}@${manifest.version}',
      () async {
        await widget.clients.servicesLocalModelService.installModel(
          modelId: manifest.id,
          version: manifest.version,
        );
      },
    );
  }

  /// Removes one custom imported model from the catalog and filesystem.
  Future<void> _removeCustomModel(core_proxy.LocalModelManifest manifest) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await _confirmDelete(
      title: l10n.localModelsRemoveFromCatalog,
      message: l10n.localModelsDeleteModelMessage(manifest.displayName),
    );
    if (!confirmed) {
      return;
    }
    await _runOperation('remove:${manifest.id}@${manifest.version}', () async {
      await widget.clients.servicesLocalModelService.removeCustomModel(
        modelId: manifest.id,
        version: manifest.version,
      );
    });
  }

  /// Opens the Hub explore and import dialog for searching or importing repositories.
  Future<void> _openHubExploreDialog(
    BuildContext context,
    core_proxy.LocalModelSourceKind defaultSource,
  ) async {
    await showDialog<void>(
      context: context,
      builder: (context) => _LocalModelHubDialog(
        clients: widget.clients,
        initialSource: defaultSource,
        onImportSuccess: _reload,
      ),
    );
  }

  /// Pauses one active model download through the existing local model service.
  Future<void> _pauseInstall(core_proxy.LocalModelManifest manifest) async {
    final operationKey = 'install:${manifest.id}@${manifest.version}';
    _pausedOperations.add(operationKey);
    try {
      await widget.clients.servicesLocalModelService.cancelInstall(
        modelId: manifest.id,
        version: manifest.version,
      );
      if (!mounted) {
        return;
      }
      _reload();
    } catch (error) {
      _pausedOperations.remove(operationKey);
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.localModelsOperationFailed('$error'))),
      );
    }
  }

  /// Verifies one installed model and its exact platform engine dependency.
  Future<void> _verify(core_proxy.LocalModelManifest manifest) {
    return _runOperation('verify:${manifest.id}@${manifest.version}', () async {
      await widget.clients.servicesLocalModelService.verifyModel(
        modelId: manifest.id,
        version: manifest.version,
      );
    });
  }

  /// Confirms and deletes one installed local model.
  Future<void> _deleteModel(core_proxy.LocalModelManifest manifest) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await _confirmDelete(
      title: l10n.localModelsDeleteModelTitle,
      message: l10n.localModelsDeleteModelMessage(manifest.displayName),
    );
    if (!confirmed) {
      return;
    }
    final persistedStatus = _findInstallStatus(
      manifest,
      _installStatuses ?? const <core_proxy.LocalModelInstallStatus>[],
    );
    if (persistedStatus != null) {
      if (_isInstallRunning(persistedStatus)) {
        _pausedOperations.add('install:${manifest.id}@${manifest.version}');
        await widget.clients.servicesLocalModelService.cancelInstall(
          modelId: manifest.id,
          version: manifest.version,
        );
        await _waitForInstallStop(manifest);
      }
    }
    await _runOperation('delete:${manifest.id}@${manifest.version}', () async {
      await widget.clients.servicesLocalModelService.deleteModel(
        modelId: manifest.id,
        version: manifest.version,
      );
    });
  }

  /// Waits until one cancelled installation no longer owns its host download files.
  Future<void> _waitForInstallStop(
    core_proxy.LocalModelManifest manifest,
  ) async {
    for (;;) {
      final status = await widget.clients.servicesLocalModelService
          .getInstallStatus(modelId: manifest.id, version: manifest.version);
      if (status == null || !_isInstallRunning(status)) {
        return;
      }
      await Future<void>.delayed(const Duration(milliseconds: 100));
    }
  }

  /// Confirms and deletes one installed platform engine.
  Future<void> _deleteEngine(core_proxy.InstalledLocalEngine engine) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await _confirmDelete(
      title: l10n.localModelsDeleteEngineTitle,
      message: l10n.localModelsDeleteEngineMessage(
        engine.manifest.displayName,
        engine.manifest.version,
      ),
    );
    if (!confirmed) {
      return;
    }
    await _runOperation(
      'engine:${engine.manifest.id}@${engine.manifest.version}',
      () async {
        await widget.clients.servicesLocalModelService.deleteEngine(
          engineId: engine.manifest.id,
          version: engine.manifest.version,
        );
      },
    );
  }

  /// Displays a destructive action confirmation dialog.
  Future<bool> _confirmDelete({
    required String title,
    required String message,
  }) async {
    final l10n = AppLocalizations.of(context)!;
    final result = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton.tonalIcon(
            onPressed: () => Navigator.of(context).pop(true),
            icon: const Icon(Icons.delete_outline),
            label: Text(l10n.delete),
          ),
        ],
      ),
    );
    return result == true;
  }

  /// Builds local model management content for the current runtime target.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final future = _future;
    if (future == null) {
      return const Center(child: M3LoadingIndicator());
    }
    return FutureBuilder<_LocalModelSettingsData>(
      future: future,
      builder: (context, snapshot) {
        if (!snapshot.hasData) {
          if (snapshot.hasError) {
            return Center(
              child: Text(l10n.localModelsLoadFailed('${snapshot.error}')),
            );
          }
          return const Center(child: M3LoadingIndicator());
        }
        final data = snapshot.requireData;
        final preferredSource =
            data.registry.preferredSourceKind ??
            core_proxy.LocalModelSourceKind.huggingFace;
        final modelKinds = data.catalog
            .map((status) => status.manifest.kind)
            .toSet();
        return ListView(
          padding: const EdgeInsets.fromLTRB(20, 18, 20, 32),
          children: <Widget>[
            _buildHeader(context, data.target, preferredSource),
            const SizedBox(height: 20),
            Row(
              children: <Widget>[
                Text(
                  l10n.localModelsCatalog,
                  style: SettingsControlStyles.sectionTitleTextStyle(context),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Text(
                    '${data.catalog.length}',
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            for (final kind in core_proxy.LocalModelKind.values.where(
              modelKinds.contains,
            )) ...<Widget>[
              Padding(
                padding: const EdgeInsets.only(top: 6, bottom: 8),
                child: Row(
                  children: <Widget>[
                    Icon(
                      _modelKindIcon(kind),
                      size: 16,
                      color: Theme.of(context).colorScheme.primary,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      _localModelKindTitle(l10n, kind),
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
              for (final status in data.catalog.where(
                (status) => status.manifest.kind == kind,
              )) ...<Widget>[
                _buildModelItem(
                  context,
                  status,
                  _installStatuses ?? data.installStatuses,
                  data.registry,
                ),
                const SizedBox(height: 10),
              ],
            ],
            const SizedBox(height: 14),
            Text(
              l10n.localModelsInstalledEngines,
              style: SettingsControlStyles.sectionTitleTextStyle(context),
            ),
            const SizedBox(height: 10),
            if (data.registry.installedEngines.isEmpty)
              OperitGlassSurface(
                color: Theme.of(context).colorScheme.surfaceContainerLow.withValues(alpha: 0.6),
                layer: OperitGlassSurfaceLayer.control,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text(
                    l10n.localModelsNoInstalledEngines,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              )
            else
              for (final engine in data.registry.installedEngines) ...<Widget>[
                _buildEngineItem(context, engine),
                const SizedBox(height: 8),
              ],
          ],
        );
      },
    );
  }

  /// Builds the platform provider heading, target summary, and source selector.
  Widget _buildHeader(
    BuildContext context,
    core_proxy.LocalPlatformTarget target,
    core_proxy.LocalModelSourceKind preferredSource,
  ) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colors.surfaceContainerLow.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.35)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Container(
                  width: 44,
                  height: 44,
                  decoration: BoxDecoration(
                    color: colors.primary.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(Icons.memory_outlined, color: colors.primary, size: 24),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        l10n.settingsModelProviderTypeLocalModel,
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Wrap(
                        spacing: 8,
                        runSpacing: 4,
                        children: <Widget>[
                          _buildTagChip(
                            context,
                            icon: Icons.laptop_outlined,
                            label: target.platform.value,
                          ),
                          _buildTagChip(
                            context,
                            icon: Icons.settings_input_component_outlined,
                            label: target.architecture.value,
                          ),
                          _buildTagChip(
                            context,
                            icon: Icons.cloud_download_outlined,
                            label: _sourceKindTitle(l10n, preferredSource),
                            color: colors.primary,
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                IconButton(
                  onPressed: _reload,
                  icon: const Icon(Icons.refresh),
                  tooltip: l10n.refresh,
                ),
              ],
            ),
            const Divider(height: 24),
            Wrap(
              spacing: 12,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                Text(
                  l10n.localModelsDownloadSource,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.onSurfaceVariant,
                  ),
                ),
                SegmentedButton<core_proxy.LocalModelSourceKind>(
                  showSelectedIcon: false,
                  style: const ButtonStyle(
                    visualDensity: VisualDensity.compact,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                  segments: <ButtonSegment<core_proxy.LocalModelSourceKind>>[
                    ButtonSegment<core_proxy.LocalModelSourceKind>(
                      value: core_proxy.LocalModelSourceKind.huggingFace,
                      label: Text(l10n.localModelsSourceHuggingFace),
                    ),
                    ButtonSegment<core_proxy.LocalModelSourceKind>(
                      value: core_proxy.LocalModelSourceKind.modelScope,
                      label: Text(l10n.localModelsSourceModelScope),
                    ),
                    ButtonSegment<core_proxy.LocalModelSourceKind>(
                      value: core_proxy.LocalModelSourceKind.hfMirror,
                      label: Text(l10n.localModelsSourceHfMirror),
                    ),
                  ],
                  selected: <core_proxy.LocalModelSourceKind>{preferredSource},
                  onSelectionChanged: (selected) {
                    if (selected.isNotEmpty) {
                      _setPreferredSource(selected.first);
                    }
                  },
                ),
                const SizedBox(width: 4),
                FilledButton.tonalIcon(
                  onPressed: () => _openHubExploreDialog(context, preferredSource),
                  icon: const Icon(Icons.travel_explore_outlined, size: 18),
                  label: Text(l10n.localModelsExploreHub),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// Builds one catalog model with installation and provider actions.
  Widget _buildModelItem(
    BuildContext context,
    core_proxy.LocalModelCatalogStatus status,
    List<core_proxy.LocalModelInstallStatus> installStatuses,
    core_proxy.LocalModelRegistrySnapshot registry,
  ) {
    final l10n = AppLocalizations.of(context)!;
    final manifest = status.manifest;
    final installed = status.installedModel != null;
    final installStatus = _findInstallStatus(manifest, installStatuses);
    final operationSuffix = '${manifest.id}@${manifest.version}';
    final localOperationActive = _activeOperations.any(
      (operation) => operation.endsWith(operationSuffix),
    );
    final isPaused =
        installStatus?.phase == core_proxy.LocalModelInstallPhase.cancelled;
    final isCancelling =
        installStatus?.phase == core_proxy.LocalModelInstallPhase.cancelling;
    final isBusy =
        localOperationActive ||
        (installStatus != null && _isInstallRunning(installStatus));
    final hasDownloadTask = installStatus != null && !installed;
    final isCustomModel = registry.customModels.any(
      (m) => m.id == manifest.id && m.version == manifest.version,
    );
    final colors = Theme.of(context).colorScheme;

    return OperitGlassSurface(
      color: colors.surfaceContainerLow.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(10),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.35)),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: colors.primary.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(_modelKindIcon(manifest.kind), color: colors.primary, size: 20),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Row(
                        children: <Widget>[
                          Flexible(
                            child: Text(
                              manifest.displayName,
                              style: Theme.of(context).textTheme.titleSmall?.copyWith(
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                          if (isCustomModel) ...<Widget>[
                            const SizedBox(width: 8),
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                              decoration: BoxDecoration(
                                color: colors.secondaryContainer,
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                l10n.localModelsCustomBadge,
                                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                                  color: colors.onSecondaryContainer,
                                  fontSize: 10,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                          ],
                        ],
                      ),
                      const SizedBox(height: 3),
                      Text(
                        _localModelDescription(l10n, manifest),
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                          height: 1.35,
                        ),
                      ),
                    ],
                  ),
                ),
                if (isBusy)
                  const SizedBox(
                    width: 24,
                    height: 24,
                    child: Padding(
                      padding: EdgeInsets.all(2),
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 10),
            if (hasDownloadTask) ...<Widget>[
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: colors.surfaceContainerHighest.withValues(alpha: 0.45),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.2)),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    ClipRRect(
                      borderRadius: BorderRadius.circular(3),
                      child: LinearProgressIndicator(
                        minHeight: 5,
                        value: installStatus.totalBytes == 0
                            ? null
                            : installStatus.downloadedBytes / installStatus.totalBytes,
                      ),
                    ),
                    const SizedBox(height: 6),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            isCancelling
                                ? l10n.localModelsCancelling
                                : isPaused
                                ? l10n.localModelsDownloadPaused(
                                    _formatBytes(installStatus.downloadedBytes),
                                    _formatBytes(installStatus.totalBytes),
                                  )
                                : installStatus.downloadedBytes == installStatus.totalBytes
                                ? l10n.localModelsDownloadInstalling
                                : l10n.localModelsDownloading(
                                    _formatBytes(installStatus.downloadedBytes),
                                    _formatBytes(installStatus.totalBytes),
                                  ),
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: colors.onSurfaceVariant,
                            ),
                          ),
                        ),
                        if (installStatus.totalBytes > 0)
                          Text(
                            '${((installStatus.downloadedBytes / installStatus.totalBytes) * 100).clamp(0, 100).toStringAsFixed(1)}%',
                            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                              fontWeight: FontWeight.w700,
                              color: colors.primary,
                            ),
                          ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 10),
            ],
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: <Widget>[
                _buildTagChip(
                  context,
                  icon: Icons.storage_outlined,
                  label: _formatBytes(_modelByteSize(manifest)),
                ),
                _buildTagChip(
                  context,
                  icon: Icons.gavel_outlined,
                  label: manifest.license,
                ),
                _buildTagChip(
                  context,
                  icon: Icons.translate_outlined,
                  label: manifest.languages.join(' / '),
                ),
                _buildTagChip(
                  context,
                  icon: Icons.cloud_outlined,
                  label: _modelSourcesLabel(l10n, manifest.sources),
                  color: colors.primary,
                ),
                if (installed)
                  SettingsActivePill(label: l10n.localModelsModelInstalled)
                else
                  _buildTagChip(
                    context,
                    icon: Icons.circle_outlined,
                    label: l10n.localModelsModelNotInstalled,
                  ),
                if (status.installedEngine != null)
                  _buildTagChip(
                    context,
                    icon: Icons.developer_board,
                    label: l10n.localModelsEngineInstalled,
                    color: colors.secondary,
                  ),
                if (!status.platformCompatible)
                  _buildTagChip(
                    context,
                    icon: Icons.warning_amber_outlined,
                    label: l10n.localModelsPlatformIncompatible,
                    color: colors.error,
                  ),
              ],
            ),
            const SizedBox(height: 10),
            Row(
              children: <Widget>[
                const Spacer(),
                if (installed) ...<Widget>[
                  IconButton(
                    onPressed: !isBusy ? () => _verify(manifest) : null,
                    icon: const Icon(Icons.verified_outlined),
                    tooltip: l10n.localModelsVerifyModelAndEngine,
                  ),
                  IconButton(
                    onPressed: !isBusy ? () => _deleteModel(manifest) : null,
                    icon: Icon(Icons.delete_outline, color: colors.error),
                    tooltip: l10n.localModelsDeleteModel,
                  ),
                  if (isCustomModel)
                    IconButton(
                      onPressed: !isBusy ? () => _removeCustomModel(manifest) : null,
                      icon: const Icon(Icons.bookmark_remove_outlined),
                      tooltip: l10n.localModelsRemoveFromCatalog,
                    ),
                ] else ...<Widget>[
                  if (isCustomModel && !hasDownloadTask)
                    IconButton(
                      onPressed: !isBusy ? () => _removeCustomModel(manifest) : null,
                      icon: const Icon(Icons.bookmark_remove_outlined),
                      tooltip: l10n.localModelsRemoveFromCatalog,
                    ),
                  if (hasDownloadTask)
                    IconButton(
                      onPressed: isPaused || isCancelling
                          ? null
                          : () => _pauseInstall(manifest),
                      icon: const Icon(Icons.pause),
                      tooltip: l10n.localModelsPauseDownload,
                    ),
                  if (hasDownloadTask)
                    IconButton(
                      onPressed: () => _deleteModel(manifest),
                      icon: Icon(Icons.delete_outline, color: colors.error),
                      tooltip: l10n.localModelsDeleteDownload,
                    ),
                  FilledButton.icon(
                    onPressed:
                        status.platformCompatible &&
                            !localOperationActive &&
                            (!isBusy || isPaused)
                        ? () => _install(manifest)
                        : null,
                    icon: Icon(
                      isPaused ? Icons.play_arrow : Icons.download_outlined,
                      size: 18,
                    ),
                    label: Text(
                      isPaused
                          ? l10n.localModelsResumeDownload
                          : isBusy
                          ? l10n.localModelsInstalling
                          : l10n.localModelsInstall,
                    ),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// Builds one installed engine row with target and deletion controls.
  Widget _buildEngineItem(
    BuildContext context,
    core_proxy.InstalledLocalEngine engine,
  ) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colors.surfaceContainerLow.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.3)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        child: Row(
          children: <Widget>[
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: colors.primary.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(Icons.developer_board_outlined, color: colors.primary, size: 20),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '${engine.manifest.displayName} ${engine.manifest.version}',
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Wrap(
                    spacing: 8,
                    children: <Widget>[
                      _buildTagChip(
                        context,
                        icon: Icons.laptop_outlined,
                        label: engine.artifact.target.platform.value,
                      ),
                      _buildTagChip(
                        context,
                        icon: Icons.settings_input_component_outlined,
                        label: engine.artifact.target.architecture.value,
                      ),
                      _buildTagChip(
                        context,
                        icon: Icons.storage_outlined,
                        label: _formatBytes(engine.artifact.byteSize),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            IconButton(
              onPressed: _activeOperations.isEmpty
                  ? () => _deleteEngine(engine)
                  : null,
              icon: Icon(Icons.delete_outline, color: colors.error),
              tooltip: l10n.localModelsDeleteEngine,
            ),
          ],
        ),
      ),
    );
  }
}

/// Builds a styled chip for tags, size, and metadata.
Widget _buildTagChip(
  BuildContext context, {
  required IconData icon,
  required String label,
  Color? color,
}) {
  final colors = Theme.of(context).colorScheme;
  final effectiveColor = color ?? colors.onSurfaceVariant;
  return Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
    decoration: BoxDecoration(
      color: colors.surfaceContainerHigh.withValues(alpha: 0.55),
      borderRadius: BorderRadius.circular(6),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.2)),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 12, color: effectiveColor),
        const SizedBox(width: 4),
        Text(
          label,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: effectiveColor,
            fontSize: 11,
          ),
        ),
      ],
    ),
  );
}

/// Returns whether one installation status still owns Host download resources.
bool _isInstallRunning(core_proxy.LocalModelInstallStatus status) {
  return switch (status.phase) {
    core_proxy.LocalModelInstallPhase.preparing ||
    core_proxy.LocalModelInstallPhase.engine ||
    core_proxy.LocalModelInstallPhase.model ||
    core_proxy.LocalModelInstallPhase.cancelling => true,
    core_proxy.LocalModelInstallPhase.cancelled ||
    core_proxy.LocalModelInstallPhase.completed ||
    core_proxy.LocalModelInstallPhase.failed => false,
  };
}

/// Returns a compact label listing all download sources declared by one manifest.
String _modelSourcesLabel(
  AppLocalizations l10n,
  List<core_proxy.LocalModelSource> sources,
) {
  final labels = <String>{};
  for (final source in sources) {
    labels.add(_sourceKindTitle(l10n, source.kind));
  }
  return labels.join(' / ');
}

/// Returns the localized title for one model download source kind.
String _sourceKindTitle(
  AppLocalizations l10n,
  core_proxy.LocalModelSourceKind kind,
) {
  return switch (kind) {
    core_proxy.LocalModelSourceKind.huggingFace =>
      l10n.localModelsSourceHuggingFace,
    core_proxy.LocalModelSourceKind.modelScope =>
      l10n.localModelsSourceModelScope,
    core_proxy.LocalModelSourceKind.hfMirror =>
      l10n.localModelsSourceHfMirror,
    core_proxy.LocalModelSourceKind.directHttp =>
      l10n.localModelsSourceDirectHttp,
  };
}

/// Returns the localized category title for one local model kind.
String _localModelKindTitle(
  AppLocalizations l10n,
  core_proxy.LocalModelKind kind,
) {
  return switch (kind) {
    core_proxy.LocalModelKind.speechToText =>
      l10n.localModelsCategorySpeechToText,
    core_proxy.LocalModelKind.textToSpeech =>
      l10n.localModelsCategoryTextToSpeech,
    core_proxy.LocalModelKind.chat => l10n.localModelsCategoryChat,
    core_proxy.LocalModelKind.embedding => l10n.localModelsCategoryEmbedding,
  };
}

/// Returns the localized description for one exact built-in local model or custom description.
String _localModelDescription(
  AppLocalizations l10n,
  core_proxy.LocalModelManifest manifest,
) {
  return switch (manifest.id) {
    'sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20' =>
      l10n.localModelDescriptionSherpaOnnxStreamingStt,
    'vits-zh-aishell3-int8' => l10n.localModelDescriptionSherpaOnnxVitsAishell3,
    'sherpa-onnx-vits-zh-ll' => l10n.localModelDescriptionSherpaOnnxVitsZhLl,
    'matcha-icefall-zh-baker' =>
      l10n.localModelDescriptionSherpaOnnxMatchaBaker,
    'kitten-nano-en-v0_8-int8' =>
      l10n.localModelDescriptionSherpaOnnxKittenNano,
    'sherpa-onnx-web-paraformer-small-zh-en' =>
      l10n.localModelDescriptionSherpaOnnxWebParaformer,
    'sherpa-onnx-web-vits-piper-en-us-libritts-r-medium' =>
      l10n.localModelDescriptionSherpaOnnxWebVitsPiper,
    _ => manifest.description.isNotEmpty ? manifest.description : manifest.displayName,
  };
}

class _LocalModelSettingsData {
  const _LocalModelSettingsData({
    required this.catalog,
    required this.registry,
    required this.target,
    required this.installStatuses,
  });

  final List<core_proxy.LocalModelCatalogStatus> catalog;
  final core_proxy.LocalModelRegistrySnapshot registry;
  final core_proxy.LocalPlatformTarget target;
  final List<core_proxy.LocalModelInstallStatus> installStatuses;
}

/// Finds the active or persisted installation status for one exact model release.
core_proxy.LocalModelInstallStatus? _findInstallStatus(
  core_proxy.LocalModelManifest manifest,
  List<core_proxy.LocalModelInstallStatus> statuses,
) {
  for (final status in statuses) {
    if (status.modelId == manifest.id && status.version == manifest.version) {
      return status;
    }
  }
  return null;
}

/// Returns an icon for one local model capability.
IconData _modelKindIcon(core_proxy.LocalModelKind kind) {
  return switch (kind) {
    core_proxy.LocalModelKind.speechToText => Icons.mic_outlined,
    core_proxy.LocalModelKind.textToSpeech => Icons.record_voice_over_outlined,
    core_proxy.LocalModelKind.chat => Icons.chat_bubble_outline,
    core_proxy.LocalModelKind.embedding => Icons.data_array_outlined,
  };
}

/// Calculates the declared byte size of one model manifest.
int _modelByteSize(core_proxy.LocalModelManifest manifest) {
  var total = 0;
  for (final file in manifest.files) {
    total += file.byteSize;
  }
  return total;
}

/// Formats a byte count using a compact binary unit.
String _formatBytes(int bytes) {
  const kib = 1024;
  const mib = 1024 * kib;
  const gib = 1024 * mib;
  if (bytes >= gib) {
    return '${(bytes / gib).toStringAsFixed(2)} GiB';
  }
  if (bytes >= mib) {
    return '${(bytes / mib).toStringAsFixed(1)} MiB';
  }
  if (bytes >= kib) {
    return '${(bytes / kib).toStringAsFixed(1)} KiB';
  }
  return '$bytes B';
}

/// Dialog for searching Hugging Face / ModelScope repositories or importing by owner/repo.
class _LocalModelHubDialog extends StatefulWidget {
  const _LocalModelHubDialog({
    required this.clients,
    required this.initialSource,
    required this.onImportSuccess,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.LocalModelSourceKind initialSource;
  final VoidCallback onImportSuccess;

  @override
  State<_LocalModelHubDialog> createState() => _LocalModelHubDialogState();
}

class _LocalModelHubDialogState extends State<_LocalModelHubDialog> {
  late core_proxy.LocalModelSourceKind _selectedSource;
  int _activeTab = 0;
  final TextEditingController _searchController = TextEditingController();
  final TextEditingController _directRepoController = TextEditingController();
  final TextEditingController _directRevController = TextEditingController();
  List<core_proxy.LocalModelHubRepoSummary>? _searchResults;
  bool _isSearching = false;
  String? _searchError;
  bool _isImporting = false;
  String? _importingRepo;

  @override
  void initState() {
    super.initState();
    _selectedSource = widget.initialSource;
  }

  @override
  void dispose() {
    _searchController.dispose();
    _directRepoController.dispose();
    _directRevController.dispose();
    super.dispose();
  }

  Future<void> _doSearch() async {
    final query = _searchController.text.trim();
    if (query.isEmpty) {
      return;
    }
    setState(() {
      _isSearching = true;
      _searchError = null;
    });
    try {
      final results = await widget.clients.servicesLocalModelService
          .searchHubModels(query: query, sourceKind: _selectedSource);
      if (!mounted) {
        return;
      }
      setState(() {
        _searchResults = results;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _searchError = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _isSearching = false;
        });
      }
    }
  }

  Future<void> _doImport(String repository, [String? revision]) async {
    final cleanRepo = repository.trim();
    if (cleanRepo.isEmpty) {
      return;
    }
    setState(() {
      _isImporting = true;
      _importingRepo = cleanRepo;
    });
    try {
      await widget.clients.servicesLocalModelService.importModelFromHub(
        repository: cleanRepo,
        revision: revision?.trim().isNotEmpty == true ? revision!.trim() : null,
        sourceKind: _selectedSource,
      );
      if (!mounted) {
        return;
      }
      widget.onImportSuccess();
      final l10n = AppLocalizations.of(context)!;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.localModelsHubImportSuccess)),
      );
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.localModelsOperationFailed('$error'))),
      );
    } finally {
      if (mounted) {
        setState(() {
          _isImporting = false;
          _importingRepo = null;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    return OperitDialogScaffold(
      maxWidth: 680,
      maxHeight: 560,
      title: l10n.localModelsExploreHub,
      icon: const Icon(Icons.travel_explore_outlined),
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              SegmentedButton<int>(
                showSelectedIcon: false,
                style: const ButtonStyle(
                  visualDensity: VisualDensity.compact,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                segments: <ButtonSegment<int>>[
                  ButtonSegment<int>(
                    value: 0,
                    icon: const Icon(Icons.search, size: 16),
                    label: Text(l10n.search),
                  ),
                  ButtonSegment<int>(
                    value: 1,
                    icon: const Icon(Icons.link, size: 16),
                    label: Text(l10n.localModelsHubDirectImportTitle),
                  ),
                ],
                selected: <int>{_activeTab},
                onSelectionChanged: (selected) {
                  if (selected.isNotEmpty) {
                    setState(() {
                      _activeTab = selected.first;
                    });
                  }
                },
              ),
              const Spacer(),
              Text(
                l10n.localModelsDownloadSource,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colors.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              DropdownButton<core_proxy.LocalModelSourceKind>(
                value: _selectedSource,
                underline: const SizedBox.shrink(),
                items: <DropdownMenuItem<core_proxy.LocalModelSourceKind>>[
                  DropdownMenuItem(
                    value: core_proxy.LocalModelSourceKind.huggingFace,
                    child: Text(l10n.localModelsSourceHuggingFace),
                  ),
                  DropdownMenuItem(
                    value: core_proxy.LocalModelSourceKind.modelScope,
                    child: Text(l10n.localModelsSourceModelScope),
                  ),
                  DropdownMenuItem(
                    value: core_proxy.LocalModelSourceKind.hfMirror,
                    child: Text(l10n.localModelsSourceHfMirror),
                  ),
                ],
                onChanged: (source) {
                  if (source != null) {
                    setState(() {
                      _selectedSource = source;
                      _searchResults = null;
                    });
                  }
                },
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (_activeTab == 0) ...<Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: TextField(
                    controller: _searchController,
                    decoration: InputDecoration(
                      hintText: l10n.localModelsHubSearchHint,
                      isDense: true,
                      prefixIcon: const Icon(Icons.search, size: 20),
                      border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                    onSubmitted: (_) => _doSearch(),
                  ),
                ),
                const SizedBox(width: 10),
                FilledButton.icon(
                  onPressed: _isSearching ? null : _doSearch,
                  icon: const Icon(Icons.search, size: 18),
                  label: Text(l10n.search),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Expanded(child: _buildResultsList(context)),
          ] else ...<Widget>[
            OperitGlassSurface(
              color: colors.surfaceContainerLow.withValues(alpha: 0.65),
              layer: OperitGlassSurfaceLayer.control,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.3)),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      l10n.localModelsHubDirectImportPrompt,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: _directRepoController,
                      decoration: InputDecoration(
                        hintText: 'owner/repo / https://modelscope.cn/models/...',
                        isDense: true,
                        prefixIcon: const Icon(Icons.folder_outlined, size: 18),
                        border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                      ),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: _directRevController,
                      decoration: InputDecoration(
                        hintText: 'main / master',
                        isDense: true,
                        prefixIcon: const Icon(Icons.commit_outlined, size: 18),
                        border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                      ),
                    ),
                    const SizedBox(height: 16),
                    Align(
                      alignment: Alignment.centerRight,
                      child: FilledButton.icon(
                        onPressed: _isImporting
                            ? null
                            : () => _doImport(
                                  _directRepoController.text,
                                  _directRevController.text,
                                ),
                        icon: _isImporting
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.download_for_offline_outlined, size: 18),
                        label: Text(
                          _isImporting
                              ? l10n.localModelsHubImporting
                              : l10n.localModelsHubImport,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const Spacer(),
          ],
        ],
      ),
    );
  }

  Widget _buildResultsList(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    if (_isSearching) {
      return const Center(child: M3LoadingIndicator());
    }
    if (_searchError != null) {
      return Center(
        child: Text(
          _searchError!,
          style: TextStyle(color: colors.error),
          textAlign: TextAlign.center,
        ),
      );
    }
    final results = _searchResults;
    if (results == null) {
      return Center(
        child: Text(
          l10n.localModelsHubDirectImportPrompt,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: colors.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
        ),
      );
    }
    if (results.isEmpty) {
      return Center(
        child: Text(
          l10n.localModelsHubNoResults,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: colors.onSurfaceVariant,
          ),
        ),
      );
    }
    return ListView.separated(
      itemCount: results.length,
      separatorBuilder: (_, __) => const SizedBox(height: 8),
      itemBuilder: (context, index) {
        final item = results[index];
        final isThisImporting = _isImporting && _importingRepo == item.repository;
        return OperitGlassSurface(
          color: colors.surfaceContainerLow.withValues(alpha: 0.65),
          layer: OperitGlassSurfaceLayer.control,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.25)),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        item.displayName.isNotEmpty ? item.displayName : item.repository,
                        style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        item.repository,
                        style: TextStyle(color: colors.primary, fontSize: 12),
                      ),
                      const SizedBox(height: 3),
                      Text(
                        item.description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Wrap(
                        spacing: 8,
                        children: <Widget>[
                          _buildTagChip(
                            context,
                            icon: Icons.download_outlined,
                            label: '${item.downloads}',
                          ),
                          _buildTagChip(
                            context,
                            icon: Icons.favorite_border,
                            label: '${item.likes}',
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton.tonal(
                  onPressed: _isImporting ? null : () => _doImport(item.repository, item.revision),
                  child: isThisImporting
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Text(l10n.localModelsHubImport),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
