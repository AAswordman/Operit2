// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('本地模型操作失败：$error')));
    } finally {
      if (mounted) {
        setState(() {
          _activeOperations.remove(operationKey);
        });
      }
    }
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('本地模型操作失败：$error')));
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
    final confirmed = await _confirmDelete(
      title: '删除本地模型',
      message: '删除 ${manifest.displayName} 的模型文件？',
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
    final confirmed = await _confirmDelete(
      title: '删除本地引擎',
      message: '删除 ${engine.manifest.displayName} ${engine.manifest.version}？',
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
    final result = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton.tonalIcon(
            onPressed: () => Navigator.of(context).pop(true),
            icon: const Icon(Icons.delete_outline),
            label: const Text('删除'),
          ),
        ],
      ),
    );
    return result == true;
  }

  /// Builds local model management content for the current runtime target.
  @override
  Widget build(BuildContext context) {
    final future = _future;
    if (future == null) {
      return const Center(child: M3LoadingIndicator());
    }
    return FutureBuilder<_LocalModelSettingsData>(
      future: future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Center(child: Text('本地模型状态加载失败：${snapshot.error}'));
        }
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: M3LoadingIndicator());
        }
        final data = snapshot.requireData;
        return ListView(
          padding: const EdgeInsets.fromLTRB(20, 18, 20, 32),
          children: <Widget>[
            _buildHeader(context, data.target),
            const SizedBox(height: 18),
            Text('模型目录', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            for (final status in data.catalog) ...<Widget>[
              _buildModelItem(
                context,
                status,
                _installStatuses ?? data.installStatuses,
              ),
              const SizedBox(height: 10),
            ],
            const SizedBox(height: 10),
            Text('已安装引擎', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            if (data.registry.installedEngines.isEmpty)
              Text(
                '当前平台尚未安装本地推理引擎',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              )
            else
              for (final engine in data.registry.installedEngines)
                _buildEngineItem(context, engine),
          ],
        );
      },
    );
  }

  /// Builds the platform provider heading and target summary.
  Widget _buildHeader(
    BuildContext context,
    core_proxy.LocalPlatformTarget target,
  ) {
    final colors = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(Icons.memory_outlined, color: colors.primary, size: 28),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                'LOCAL_MODEL',
                style: Theme.of(
                  context,
                ).textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 3),
              Text(
                '${target.platform.value} · ${target.architecture.value}',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        IconButton(
          onPressed: _reload,
          icon: const Icon(Icons.refresh),
          tooltip: '刷新',
        ),
      ],
    );
  }

  /// Builds one catalog model with installation and provider actions.
  Widget _buildModelItem(
    BuildContext context,
    core_proxy.LocalModelCatalogStatus status,
    List<core_proxy.LocalModelInstallStatus> installStatuses,
  ) {
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
    final colors = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colors.surfaceContainerLow.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.35)),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Icon(_modelKindIcon(manifest.kind), color: colors.primary),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        manifest.displayName,
                        style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 3),
                      Text(
                        manifest.description,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                if (isBusy)
                  const SizedBox(
                    width: 28,
                    height: 28,
                    child: Padding(
                      padding: EdgeInsets.all(4),
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 10),
            if (hasDownloadTask) ...<Widget>[
              LinearProgressIndicator(
                value: installStatus.totalBytes == 0
                    ? null
                    : installStatus.downloadedBytes / installStatus.totalBytes,
              ),
              const SizedBox(height: 6),
              Text(
                isCancelling
                    ? '正在暂停'
                    : isPaused
                    ? '已暂停 · ${_formatBytes(installStatus.downloadedBytes)} '
                          '/ ${_formatBytes(installStatus.totalBytes)}'
                    : installStatus.downloadedBytes == installStatus.totalBytes
                    ? '下载完成，正在安装'
                    : '下载 ${_formatBytes(installStatus.downloadedBytes)} '
                          '/ ${_formatBytes(installStatus.totalBytes)}',
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
              ),
              const SizedBox(height: 10),
            ],
            Wrap(
              spacing: 12,
              runSpacing: 5,
              children: <Widget>[
                Text(_formatBytes(_modelByteSize(manifest))),
                Text('License: ${manifest.license}'),
                Text(manifest.languages.join(' / ')),
                Text(status.platformCompatible ? '平台兼容' : '平台不兼容'),
                Text(installed ? '模型已安装' : '模型未安装'),
                Text(status.installedEngine != null ? '引擎已安装' : '引擎未安装'),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: <Widget>[
                const Spacer(),
                if (installed) ...<Widget>[
                  IconButton(
                    onPressed: !isBusy ? () => _verify(manifest) : null,
                    icon: const Icon(Icons.verified_outlined),
                    tooltip: '校验模型和引擎',
                  ),
                  IconButton(
                    onPressed: !isBusy ? () => _deleteModel(manifest) : null,
                    icon: const Icon(Icons.delete_outline),
                    tooltip: '删除模型',
                  ),
                ] else ...<Widget>[
                  if (hasDownloadTask)
                    IconButton(
                      onPressed: isPaused || isCancelling
                          ? null
                          : () => _pauseInstall(manifest),
                      icon: const Icon(Icons.pause),
                      tooltip: '暂停下载',
                    ),
                  if (hasDownloadTask)
                    IconButton(
                      onPressed: () => _deleteModel(manifest),
                      icon: const Icon(Icons.delete_outline),
                      tooltip: '删除下载',
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
                    ),
                    label: Text(
                      isPaused
                          ? '继续'
                          : isBusy
                          ? '下载中'
                          : '安装',
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
    final colors = Theme.of(context).colorScheme;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.developer_board_outlined),
      title: Text('${engine.manifest.displayName} ${engine.manifest.version}'),
      subtitle: Text(
        '${engine.artifact.target.platform.value} · '
        '${engine.artifact.target.architecture.value} · '
        '${_formatBytes(engine.artifact.byteSize)}',
      ),
      trailing: IconButton(
        onPressed: _activeOperations.isEmpty
            ? () => _deleteEngine(engine)
            : null,
        icon: Icon(Icons.delete_outline, color: colors.error),
        tooltip: '删除引擎',
      ),
    );
  }
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
