// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/components/PageActivityMixin.dart';
import '../../../common/components/OperitDialog.dart';
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

class _LocalModelSettingsPanelState extends State<LocalModelSettingsPanel>
    with PageActivityMixin<LocalModelSettingsPanel> {
  Future<_LocalModelSettingsData>? _future;
  final Set<String> _activeOperations = <String>{};
  final Set<String> _pausedOperations = <String>{};
  Timer? _progressTimer;
  String _catalogQuery = '';
  final _searchController = TextEditingController();
  _CatalogScope _scope = _CatalogScope.all;
  core_proxy.LocalModelKind? _kind;
  List<core_proxy.LocalModelInstallStatus>? _installStatuses;
  bool _statusRefreshRunning = false;

  /// Loads local model state when the panel is created.
  @override
  void initState() {
    super.initState();
    _reload();
  }

  /// Polls installation status only while this settings page is active.
  @override
  void onPageActivityChanged(bool active) {
    _progressTimer?.cancel();
    _progressTimer = null;
    if (!active) {
      return;
    }
    unawaited(_refreshInstallStatuses());
    _progressTimer = Timer.periodic(
      const Duration(milliseconds: 500),
      (_) => _refreshInstallStatuses(),
    );
  }

  /// Refreshes installation status snapshots while the settings panel is mounted.
  Future<void> _refreshInstallStatuses() async {
    if (!mounted || !isPageActive || _statusRefreshRunning) {
      return;
    }
    _statusRefreshRunning = true;
    try {
      final statuses = await widget.clients.servicesLocalModelService
          .getInstallStatuses();
      if (!mounted || !isPageActive) {
        return;
      }
      if (jsonEncode(_installStatuses) == jsonEncode(statuses)) return;
      final finished = (_installStatuses ?? [])
          .where(_isInstallRunning)
          .any(
            (previous) => !statuses.any(
              (current) =>
                  current.modelId == previous.modelId &&
                  current.version == previous.version &&
                  _isInstallRunning(current),
            ),
          );
      setState(() => _installStatuses = statuses);
      if (finished) _reload();
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
    _searchController.dispose();
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
    _installStatuses = results[3] as List<core_proxy.LocalModelInstallStatus>;
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
  Future<void> _setPreferredSource(core_proxy.LocalModelSourceKind sourceKind) {
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
  Future<void> _removeCustomModel(
    core_proxy.LocalModelManifest manifest,
  ) async {
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
      if (manifest.kind == core_proxy.LocalModelKind.chat) {
        await widget.clients.servicesLocalModelService.unloadChatModel();
      }
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

  Future<void> _openChatInference(
    core_proxy.LocalModelCatalogStatus status,
  ) async {
    await showDialog<void>(
      context: context,
      builder: (context) =>
          _LocalChatInferenceDialog(clients: widget.clients, status: status),
    );
  }

  /// Keeps catalog controls visible while model rows are built on demand.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_LocalModelSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (!snapshot.hasData) {
          if (snapshot.hasError) {
            return Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(l10n.localModelsLoadFailed('${snapshot.error}')),
                    const SizedBox(height: 12),
                    OutlinedButton.icon(
                      onPressed: _reload,
                      icon: const Icon(Icons.refresh),
                      label: Text(l10n.retry),
                    ),
                  ],
                ),
              ),
            );
          }
          return const Center(child: M3LoadingIndicator());
        }
        final data = snapshot.requireData;
        final statuses = _installStatuses ?? data.installStatuses;
        final source =
            data.registry.preferredSourceKind ??
            core_proxy.LocalModelSourceKind.huggingFace;
        bool matchesScope(core_proxy.LocalModelCatalogStatus status) {
          return switch (_scope) {
            _CatalogScope.all => true,
            _CatalogScope.installed => status.installedModel != null,
            _CatalogScope.downloads =>
              status.installedModel == null &&
                  statuses.any(
                    (task) =>
                        task.modelId == status.manifest.id &&
                        task.version == status.manifest.version &&
                        task.phase !=
                            core_proxy.LocalModelInstallPhase.completed,
                  ),
          };
        }

        final scoped = data.catalog.where(matchesScope).toList();
        final query = _catalogQuery.trim().toLowerCase();
        final visible = scoped.where((status) {
          final m = status.manifest;
          final searchable = [
            m.displayName,
            m.id,
            m.engine.value,
            _localEngineTitle(m.engine),
            _localModelDescription(l10n, m),
            ...m.languages,
            ...m.tags,
          ].join(' ').toLowerCase();
          return (_kind == null || m.kind == _kind) &&
              searchable.contains(query);
        }).toList();
        final colors = Theme.of(context).colorScheme;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  LayoutBuilder(
                    builder: (context, constraints) => Row(
                      children: [
                        Expanded(
                          child: Text(
                            l10n.localModelsCatalog,
                            style: Theme.of(context).textTheme.titleMedium
                                ?.copyWith(fontWeight: FontWeight.w700),
                          ),
                        ),
                        PopupMenuButton<core_proxy.LocalModelSourceKind>(
                          tooltip:
                              '${l10n.localModelsDownloadSource}: ${_sourceKindTitle(l10n, source)}',
                          icon: const Icon(
                            Icons.cloud_download_outlined,
                            size: 20,
                          ),
                          initialValue: source,
                          onSelected: _setPreferredSource,
                          itemBuilder: (_) => [
                            for (final kind in [
                              core_proxy.LocalModelSourceKind.huggingFace,
                              core_proxy.LocalModelSourceKind.modelScope,
                              core_proxy.LocalModelSourceKind.hfMirror,
                            ])
                              CheckedPopupMenuItem(
                                value: kind,
                                checked: source == kind,
                                child: Text(_sourceKindTitle(l10n, kind)),
                              ),
                          ],
                        ),
                        IconButton(
                          onPressed: _reload,
                          icon: const Icon(Icons.refresh, size: 20),
                          tooltip: l10n.refresh,
                        ),
                        const SizedBox(width: 4),
                        if (constraints.maxWidth < 320)
                          IconButton.filled(
                            tooltip: l10n.localModelsAddModel,
                            onPressed: () =>
                                _openHubExploreDialog(context, source),
                            icon: const Icon(Icons.add),
                          )
                        else
                          FilledButton.icon(
                            onPressed: () =>
                                _openHubExploreDialog(context, source),
                            icon: const Icon(Icons.add, size: 18),
                            label: Text(l10n.localModelsAddModel),
                          ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _searchController,
                    onChanged: (value) => setState(() => _catalogQuery = value),
                    decoration: InputDecoration(
                      isDense: true,
                      prefixIcon: const Icon(Icons.search, size: 20),
                      hintText: l10n.localModelsSearchCatalog,
                      suffixIcon: _catalogQuery.isEmpty
                          ? null
                          : IconButton(
                              tooltip: l10n.clear,
                              icon: const Icon(Icons.close, size: 18),
                              onPressed: () {
                                _searchController.clear();
                                setState(() => _catalogQuery = '');
                              },
                            ),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: [
                        for (final scope in _CatalogScope.values)
                          Padding(
                            padding: const EdgeInsets.only(right: 8),
                            child: ChoiceChip(
                              label: Text(switch (scope) {
                                _CatalogScope.all => l10n.localModelsFilterAll,
                                _CatalogScope.installed =>
                                  l10n.localModelsFilterInstalled,
                                _CatalogScope.downloads =>
                                  l10n.localModelsFilterDownloads,
                              }),
                              selected: _scope == scope,
                              showCheckmark: false,
                              onSelected: (_) => setState(() => _scope = scope),
                            ),
                          ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 4),
                  SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: [
                        for (final kind in <core_proxy.LocalModelKind?>[
                          null,
                          core_proxy.LocalModelKind.chat,
                          ...core_proxy.LocalModelKind.values.where(
                            (kind) => kind != core_proxy.LocalModelKind.chat,
                          ),
                        ])
                          Padding(
                            padding: const EdgeInsets.only(right: 6),
                            child: FilterChip(
                              label: Text(
                                kind == null
                                    ? l10n.localModelsAllTypes
                                    : _localModelKindTitle(l10n, kind),
                              ),
                              selected: _kind == kind,
                              showCheckmark: false,
                              onSelected: (_) => setState(() => _kind = kind),
                            ),
                          ),
                      ],
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 10),
                    child: Text(
                      l10n.localModelsResultCount(
                        visible.length,
                        data.catalog.length,
                      ),
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: ListView.builder(
                key: ValueKey('$_scope:$_kind:$_catalogQuery'),
                padding: const EdgeInsets.fromLTRB(20, 12, 20, 24),
                itemCount: (visible.isEmpty ? 1 : visible.length) + 1,
                itemBuilder: (context, index) {
                  if (index == (visible.isEmpty ? 1 : visible.length)) {
                    return Padding(
                      padding: const EdgeInsets.only(top: 16),
                      child: ExpansionTile(
                        tilePadding: EdgeInsets.zero,
                        leading: const Icon(
                          Icons.developer_board_outlined,
                          size: 20,
                        ),
                        title: Text(
                          l10n.localModelsInstalledEngines,
                          style: Theme.of(context).textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          '${data.target.platform.value} · ${data.target.architecture.value} · ${data.registry.installedEngines.length}',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                        children: [
                          if (data.registry.installedEngines.isEmpty)
                            Padding(
                              padding: const EdgeInsets.all(16),
                              child: Text(l10n.localModelsNoInstalledEngines),
                            )
                          else
                            for (final engine in data.registry.installedEngines)
                              Padding(
                                padding: const EdgeInsets.only(bottom: 8),
                                child: _buildEngineItem(context, engine),
                              ),
                        ],
                      ),
                    );
                  }
                  if (visible.isEmpty) {
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 36),
                      child: Column(
                        children: [
                          Icon(
                            Icons.search_off_outlined,
                            size: 36,
                            color: colors.onSurfaceVariant,
                          ),
                          const SizedBox(height: 12),
                          Text(l10n.localModelsNoMatches),
                          const SizedBox(height: 12),
                          TextButton(
                            onPressed: () {
                              _searchController.clear();
                              setState(() {
                                _catalogQuery = '';
                                _scope = _CatalogScope.all;
                                _kind = null;
                              });
                            },
                            child: Text(l10n.localModelsResetFilters),
                          ),
                        ],
                      ),
                    );
                  }
                  final status = visible[index];
                  return Padding(
                    key: ValueKey(
                      '${status.manifest.id}@${status.manifest.version}',
                    ),
                    padding: const EdgeInsets.only(bottom: 8),
                    child: _buildModelItem(
                      context,
                      status,
                      statuses,
                      data.registry,
                    ),
                  );
                },
              ),
            ),
          ],
        );
      },
    );
  }

  /// Shows secondary metadata without crowding the model list.
  Future<void> _showModelDetails(
    core_proxy.LocalModelCatalogStatus status,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final m = status.manifest;
    await showDialog<void>(
      context: context,
      builder: (context) => OperitDialogScaffold(
        title: m.displayName,
        showCloseButton: true,
        expandContent: false,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            SelectableText(_localModelDescription(l10n, m)),
            const SizedBox(height: 16),
            SelectableText(
              '${m.id} @ ${m.version}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _buildTagChip(
                  context,
                  icon: Icons.storage_outlined,
                  label: _formatBytes(_modelByteSize(m)),
                ),
                if (m.license.isNotEmpty)
                  _buildTagChip(
                    context,
                    icon: Icons.gavel_outlined,
                    label: m.license,
                  ),
                if (m.languages.isNotEmpty)
                  _buildTagChip(
                    context,
                    icon: Icons.translate_outlined,
                    label: m.languages.join(' / '),
                  ),
                if (m.sources.isNotEmpty)
                  _buildTagChip(
                    context,
                    icon: Icons.cloud_outlined,
                    label: _modelSourcesLabel(l10n, m.sources),
                  ),
                if (status.installedEngine != null)
                  _buildTagChip(
                    context,
                    icon: Icons.developer_board_outlined,
                    label: l10n.localModelsEngineInstalled,
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
    final isFailed =
        installStatus?.phase == core_proxy.LocalModelInstallPhase.failed;
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

    final stateLabel = !status.platformCompatible
        ? l10n.localModelsPlatformIncompatible
        : installed
        ? l10n.localModelsModelInstalled
        : isBusy
        ? l10n.localModelsInstalling
        : isFailed
        ? l10n.localModelsDownloadFailed
        : isPaused
        ? l10n.localModelsDownloadPausedLabel
        : l10n.localModelsModelNotInstalled;
    final stateColor = !status.platformCompatible || isFailed
        ? colors.error
        : installed
        ? colors.primary
        : colors.onSurfaceVariant;
    final primaryAction = installed
        ? manifest.kind == core_proxy.LocalModelKind.chat
              ? OutlinedButton.icon(
                  onPressed: !isBusy && status.platformCompatible
                      ? () => _openChatInference(status)
                      : null,
                  icon: const Icon(Icons.play_arrow, size: 18),
                  label: Text(l10n.localModelsRunChat),
                )
              : null
        : FilledButton.tonalIcon(
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
                  : isFailed
                  ? l10n.retry
                  : isBusy
                  ? l10n.localModelsInstalling
                  : l10n.localModelsInstall,
            ),
          );
    final menu = PopupMenuButton<_ModelAction>(
      tooltip: l10n.localModelsMoreActions,
      onSelected: (action) {
        switch (action) {
          case _ModelAction.details:
            unawaited(_showModelDetails(status));
          case _ModelAction.verify:
            unawaited(_verify(manifest));
          case _ModelAction.delete:
            unawaited(_deleteModel(manifest));
          case _ModelAction.remove:
            unawaited(_removeCustomModel(manifest));
        }
      },
      itemBuilder: (_) => [
        PopupMenuItem(
          value: _ModelAction.details,
          child: Text(l10n.localModelsDetails),
        ),
        if (installed)
          PopupMenuItem(
            value: _ModelAction.verify,
            enabled: !isBusy,
            child: Text(l10n.localModelsVerifyModelAndEngine),
          ),
        if (installed || hasDownloadTask)
          PopupMenuItem(
            value: _ModelAction.delete,
            enabled: !installed || !isBusy,
            child: Text(
              installed
                  ? l10n.localModelsDeleteModel
                  : l10n.localModelsDeleteDownload,
              style: TextStyle(color: colors.error),
            ),
          ),
        if (isCustomModel)
          PopupMenuItem(
            value: _ModelAction.remove,
            enabled: !isBusy,
            child: Text(l10n.localModelsRemoveFromCatalog),
          ),
      ],
    );
    return OperitGlassSurface(
      color: colors.surfaceContainerLow.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.35)),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            LayoutBuilder(
              builder: (context, constraints) {
                final compact = constraints.maxWidth < 520;
                final actions = Wrap(
                  alignment: WrapAlignment.end,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 4,
                  runSpacing: 4,
                  children: [
                    if (hasDownloadTask && _isInstallRunning(installStatus))
                      IconButton(
                        onPressed: isCancelling
                            ? null
                            : () => _pauseInstall(manifest),
                        icon: const Icon(Icons.pause),
                        tooltip: l10n.localModelsPauseDownload,
                      ),
                    ?primaryAction,
                    menu,
                  ],
                );
                final summary = Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      manifest.displayName,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      _localModelDescription(l10n, manifest),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 10,
                      runSpacing: 4,
                      children: [
                        Text(
                          stateLabel,
                          style: Theme.of(
                            context,
                          ).textTheme.labelSmall?.copyWith(color: stateColor),
                        ),
                        Text(
                          '${_localModelKindTitle(l10n, manifest.kind)} · ${_localEngineTitle(manifest.engine)}',
                          style: Theme.of(context).textTheme.labelSmall
                              ?.copyWith(color: colors.onSurfaceVariant),
                        ),
                        if (_modelByteSize(manifest) > 0)
                          Text(
                            _formatBytes(_modelByteSize(manifest)),
                            style: Theme.of(context).textTheme.labelSmall,
                          ),
                        if (isCustomModel)
                          Text(
                            l10n.localModelsCustomBadge,
                            style: Theme.of(context).textTheme.labelSmall,
                          ),
                      ],
                    ),
                  ],
                );
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            color: colors.primary.withValues(alpha: 0.08),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: Icon(
                            _modelKindIcon(manifest.kind),
                            color: colors.primary,
                            size: 20,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(child: summary),
                        if (!compact) ...[const SizedBox(width: 12), actions],
                      ],
                    ),
                    if (compact) ...[
                      const SizedBox(height: 8),
                      Align(alignment: Alignment.centerRight, child: actions),
                    ],
                  ],
                );
              },
            ),
            if (hasDownloadTask) const SizedBox(height: 12),
            if (hasDownloadTask) ...<Widget>[
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: colors.surfaceContainerHighest.withValues(alpha: 0.45),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: colors.outlineVariant.withValues(alpha: 0.2),
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    if (isFailed && installStatus.error != null) ...[
                      SelectableText(
                        installStatus.error!,
                        style: TextStyle(color: colors.error),
                      ),
                      const SizedBox(height: 8),
                    ],
                    ClipRRect(
                      borderRadius: BorderRadius.circular(3),
                      child: LinearProgressIndicator(
                        minHeight: 5,
                        value: installStatus.totalBytes == 0
                            ? (isBusy ? null : 0)
                            : (installStatus.downloadedBytes /
                                      installStatus.totalBytes)
                                  .clamp(0.0, 1.0),
                      ),
                    ),
                    const SizedBox(height: 6),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            isFailed
                                ? l10n.localModelsDownloadFailed
                                : isCancelling
                                ? l10n.localModelsCancelling
                                : isPaused
                                ? l10n.localModelsDownloadPaused(
                                    _formatBytes(installStatus.downloadedBytes),
                                    _formatBytes(installStatus.totalBytes),
                                  )
                                : installStatus.downloadedBytes ==
                                      installStatus.totalBytes
                                ? l10n.localModelsDownloadInstalling
                                : l10n.localModelsDownloading(
                                    _formatBytes(installStatus.downloadedBytes),
                                    _formatBytes(installStatus.totalBytes),
                                  ),
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: colors.onSurfaceVariant),
                          ),
                        ),
                        if (installStatus.totalBytes > 0)
                          Text(
                            '${((installStatus.downloadedBytes / installStatus.totalBytes) * 100).clamp(0, 100).toStringAsFixed(1)}%',
                            style: Theme.of(context).textTheme.labelSmall
                                ?.copyWith(
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
              child: Icon(
                Icons.developer_board_outlined,
                color: colors.primary,
                size: 20,
              ),
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
        Flexible(
          child: Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: effectiveColor,
              fontSize: 11,
            ),
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
    core_proxy.LocalModelSourceKind.hfMirror => l10n.localModelsSourceHfMirror,
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
    _ =>
      manifest.description.isNotEmpty
          ? manifest.description
          : manifest.displayName,
  };
}

String _localEngineTitle(core_proxy.LocalEngineKind engine) => switch (engine) {
  core_proxy.LocalEngineKind.llamaCpp => 'llama.cpp',
  core_proxy.LocalEngineKind.mnn => 'MNN',
  core_proxy.LocalEngineKind.sherpaOnnx => 'sherpa-onnx',
  core_proxy.LocalEngineKind.sherpaNcnn => 'sherpa-ncnn',
  core_proxy.LocalEngineKind.piper => 'Piper',
  core_proxy.LocalEngineKind.onnxRuntime => 'ONNX Runtime',
};

enum _CatalogScope { all, installed, downloads }

enum _ModelAction { details, verify, delete, remove }

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
  int _searchEpoch = 0;
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
    if (query.isEmpty || _isSearching || _isImporting) {
      return;
    }
    final epoch = ++_searchEpoch;
    setState(() {
      _isSearching = true;
      _searchError = null;
    });
    try {
      final results = await widget.clients.servicesLocalModelService
          .searchHubModels(query: query, sourceKind: _selectedSource);
      if (!mounted || epoch != _searchEpoch) {
        return;
      }
      setState(() {
        _searchResults = results;
      });
    } catch (error) {
      if (!mounted || epoch != _searchEpoch) {
        return;
      }
      setState(() {
        _searchError = error.toString();
      });
    } finally {
      if (mounted && epoch == _searchEpoch) {
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.localModelsHubImportSuccess)));
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
          SizedBox(
            width: double.infinity,
            child: SegmentedButton<int>(
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
                  label: Text(l10n.localModelsHubRepositoryTab),
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
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<core_proxy.LocalModelSourceKind>(
            initialValue: _selectedSource,
            isExpanded: true,
            decoration: InputDecoration(
              labelText: l10n.localModelsDownloadSource,
              isDense: true,
              border: const OutlineInputBorder(),
            ),
            items: [
              for (final kind in [
                core_proxy.LocalModelSourceKind.huggingFace,
                core_proxy.LocalModelSourceKind.modelScope,
                core_proxy.LocalModelSourceKind.hfMirror,
              ])
                DropdownMenuItem(
                  value: kind,
                  child: Text(_sourceKindTitle(l10n, kind)),
                ),
            ],
            onChanged: _isImporting
                ? null
                : (source) {
                    if (source == null) return;
                    setState(() {
                      _selectedSource = source;
                      _searchEpoch++;
                      _isSearching = false;
                      _searchResults = null;
                      _searchError = null;
                    });
                  },
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
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
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
            Expanded(
              child: SingleChildScrollView(
                child: OperitGlassSurface(
                  color: colors.surfaceContainerLow.withValues(alpha: 0.65),
                  layer: OperitGlassSurfaceLayer.control,
                  borderRadius: BorderRadius.circular(10),
                  border: Border.all(
                    color: colors.outlineVariant.withValues(alpha: 0.3),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          l10n.localModelsHubDirectImportPrompt,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: colors.onSurfaceVariant),
                        ),
                        const SizedBox(height: 12),
                        TextField(
                          controller: _directRepoController,
                          decoration: InputDecoration(
                            hintText:
                                'owner/repo / https://modelscope.cn/models/...',
                            isDense: true,
                            prefixIcon: const Icon(
                              Icons.folder_outlined,
                              size: 18,
                            ),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                            ),
                          ),
                        ),
                        const SizedBox(height: 12),
                        TextField(
                          controller: _directRevController,
                          decoration: InputDecoration(
                            hintText: 'main / master',
                            isDense: true,
                            prefixIcon: const Icon(
                              Icons.commit_outlined,
                              size: 18,
                            ),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                            ),
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
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                    ),
                                  )
                                : const Icon(
                                    Icons.download_for_offline_outlined,
                                    size: 18,
                                  ),
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
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colors.onSurfaceVariant),
          textAlign: TextAlign.center,
        ),
      );
    }
    if (results.isEmpty) {
      return Center(
        child: Text(
          l10n.localModelsHubNoResults,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colors.onSurfaceVariant),
        ),
      );
    }
    return ListView.separated(
      itemCount: results.length,
      separatorBuilder: (_, _) => const SizedBox(height: 8),
      itemBuilder: (context, index) {
        final item = results[index];
        final isThisImporting =
            _isImporting && _importingRepo == item.repository;
        return OperitGlassSurface(
          color: colors.surfaceContainerLow.withValues(alpha: 0.65),
          layer: OperitGlassSurfaceLayer.control,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: colors.outlineVariant.withValues(alpha: 0.25),
          ),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        item.displayName.isNotEmpty
                            ? item.displayName
                            : item.repository,
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
                  onPressed: _isImporting
                      ? null
                      : () => _doImport(item.repository, item.revision),
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

class _LocalChatInferenceDialog extends StatefulWidget {
  const _LocalChatInferenceDialog({
    required this.clients,
    required this.status,
  });
  final GeneratedCoreProxyClients clients;
  final core_proxy.LocalModelCatalogStatus status;
  @override
  State<_LocalChatInferenceDialog> createState() =>
      _LocalChatInferenceDialogState();
}

class _LocalChatInferenceDialogState extends State<_LocalChatInferenceDialog> {
  final _prompt = TextEditingController();
  final _threads = TextEditingController(text: '4');
  final _context = TextEditingController(text: '2048');
  final _batch = TextEditingController(text: '512');
  bool _busy = false;
  bool _loading = true;
  String? _error;
  String? _output;
  core_proxy.LocalModelRuntimeSettings? _saved;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final model = await widget.clients.preferencesModelConfigManager
          .getModelProfile(providerId: 'LOCAL_MODEL', modelId: _binding);
      if (!mounted) return;
      _saved = model.localRuntime;
      _threads.text =
          '${_isMnn ? _saved!.mnnThreadCount : _saved!.llamaThreadCount}';
      _context.text = '${_saved!.llamaContextSize}';
      _batch.text = '${_saved!.llamaBatchSize}';
    } catch (error) {
      if (mounted) _error = '$error';
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  String get _binding =>
      '${widget.status.manifest.id}@${widget.status.manifest.version}';
  bool get _isMnn =>
      widget.status.manifest.engine == core_proxy.LocalEngineKind.mnn;

  core_proxy.LocalModelRuntimeSettings _settings() {
    final threads = int.tryParse(_threads.text);
    final contextSize = int.tryParse(_context.text);
    final batch = int.tryParse(_batch.text);
    if (threads == null ||
        threads < 1 ||
        threads > 256 ||
        contextSize == null ||
        contextSize < 128 ||
        contextSize > 1048576 ||
        batch == null ||
        batch < 1 ||
        batch > 65536) {
      throw FormatException(
        AppLocalizations.of(context)!.localModelsInvalidSettings,
      );
    }
    return core_proxy.LocalModelRuntimeSettings.fromJson(<String, Object?>{
      ...?_saved?.toJson(),
      'inferenceBindingKey': widget.status.inferenceBindingKey ?? '',
      'mnnForwardType': 0,
      'mnnThreadCount': threads,
      'llamaThreadCount': threads,
      'llamaContextSize': contextSize,
      'llamaBatchSize': batch,
      'llamaUBatchSize': batch,
      'llamaGpuLayers': 0,
      'llamaUseMmap': true,
      'llamaFlashAttention': false,
      'llamaKvUnified': true,
      'llamaOffloadKqv': false,
    });
  }

  Future<void> _run() async {
    if (_prompt.text.trim().isEmpty) return;
    setState(() {
      _busy = true;
      _error = null;
      _output = null;
    });
    try {
      final result = await widget.clients.servicesLocalModelService
          .runChatInference(
            modelId: widget.status.manifest.id,
            version: widget.status.manifest.version,
            prompt: _prompt.text,
            settings: _settings(),
          );
      if (mounted) setState(() => _output = result.text);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _useForChat() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final settings = _settings();
      final profile = await widget.clients.preferencesModelConfigManager
          .getModelProfile(providerId: 'LOCAL_MODEL', modelId: _binding);
      await widget.clients.preferencesModelConfigManager.updateModelProfile(
        providerId: 'LOCAL_MODEL',
        model: core_proxy.ModelProfile.fromJson(<String, Object?>{
          ...profile.toJson(),
          'localRuntime': settings.toJson(),
          'contextOverride': <String, Object?>{
            'maxContextLength': settings.llamaContextSize / 1000,
          },
        }),
      );
      await widget.clients.preferencesFunctionalConfigManager
          .setModelForFunction(
            functionType: core_proxy.FunctionType.chat,
            providerId: 'LOCAL_MODEL',
            modelId: _binding,
          );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(AppLocalizations.of(context)!.localModelsChatActivated),
        ),
      );
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _unload() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.clients.servicesLocalModelService.unloadChatModel();
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  void dispose() {
    _prompt.dispose();
    _threads.dispose();
    _context.dispose();
    _batch.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.status.manifest.displayName),
      content: SizedBox(
        width: 640,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Text(l10n.localModelsCpuBackend),
              const SizedBox(height: 16),
              TextField(
                controller: _threads,
                enabled: !_busy && !_loading,
                keyboardType: TextInputType.number,
                decoration: InputDecoration(
                  labelText: l10n.localModelsThreads,
                  border: const OutlineInputBorder(),
                ),
              ),
              if (!_isMnn) ...<Widget>[
                const SizedBox(height: 12),
                TextField(
                  controller: _context,
                  enabled: !_busy && !_loading,
                  keyboardType: TextInputType.number,
                  decoration: InputDecoration(
                    labelText: l10n.localModelsContextSize,
                    border: const OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _batch,
                  enabled: !_busy && !_loading,
                  keyboardType: TextInputType.number,
                  decoration: InputDecoration(
                    labelText: l10n.localModelsBatchSize,
                    border: const OutlineInputBorder(),
                  ),
                ),
              ],
              const SizedBox(height: 16),
              TextField(
                controller: _prompt,
                enabled: !_busy && !_loading,
                minLines: 3,
                maxLines: 6,
                decoration: InputDecoration(
                  labelText: l10n.localModelsPrompt,
                  border: const OutlineInputBorder(),
                ),
              ),
              if (_busy || _loading) ...<Widget>[
                const SizedBox(height: 16),
                const LinearProgressIndicator(),
                const SizedBox(height: 8),
                Text(l10n.localModelsGenerating),
              ],
              if (_error != null) ...<Widget>[
                const SizedBox(height: 16),
                SelectableText(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              if (_output != null) ...<Widget>[
                const SizedBox(height: 16),
                SelectableText(_output!),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy || _loading ? null : _unload,
          child: Text(l10n.localModelsUnload),
        ),
        TextButton(
          onPressed: _busy || _loading ? null : _useForChat,
          child: Text(l10n.localModelsUseForChat),
        ),
        FilledButton.icon(
          onPressed: _busy || _loading ? null : _run,
          icon: const Icon(Icons.play_arrow),
          label: Text(l10n.localModelsGenerate),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.close),
        ),
      ],
    );
  }
}
