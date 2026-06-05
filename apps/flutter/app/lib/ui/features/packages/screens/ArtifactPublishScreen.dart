// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math';

import 'package:crypto/crypto.dart' as crypto;
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';

const String _marketOwner = 'AAswordman';
const String _forgeRepoName = 'OperitForge';
const String _metadataPrefix = '<!-- operit-market-json: ';

class ArtifactPublishScreen extends StatefulWidget {
  const ArtifactPublishScreen({
    super.key,
    required this.clients,
    this.publishContext,
  });

  final GeneratedCoreProxyClients clients;
  final ArtifactPublishClusterContext? publishContext;

  @override
  State<ArtifactPublishScreen> createState() => _ArtifactPublishScreenState();
}

class ArtifactPublishClusterContext {
  const ArtifactPublishClusterContext({
    required this.projectId,
    required this.rootNodeId,
    required this.runtimePackageId,
    required this.parentNodeIds,
    required this.lockedDisplayName,
    required this.projectDisplayName,
    required this.projectDescription,
  });

  final String projectId;
  final String rootNodeId;
  final String runtimePackageId;
  final List<String> parentNodeIds;
  final String lockedDisplayName;
  final String projectDisplayName;
  final String projectDescription;
}

class _ArtifactPublishScreenState extends State<ArtifactPublishScreen> {
  final TextEditingController _displayNameController = TextEditingController();
  final TextEditingController _descriptionController = TextEditingController();
  final TextEditingController _versionController = TextEditingController();
  final TextEditingController _minVersionController = TextEditingController();
  final TextEditingController _maxVersionController = TextEditingController();

  bool _loading = true;
  bool _publishing = false;
  bool _selectionProjectLoading = false;
  bool _retryingMarketRegistration = false;
  String? _errorMessage;
  String? _progressMessage;
  String? _selectionLoadError;
  ArtifactPublishClusterContext? _publishContext;
  _PendingMarketRegistration? _pendingMarketRegistration;
  List<core_proxy.PublishablePackageSource> _sources =
      <core_proxy.PublishablePackageSource>[];
  core_proxy.PublishablePackageSource? _selectedSource;

  @override
  void initState() {
    super.initState();
    _publishContext = widget.publishContext;
    _loadSources();
  }

  @override
  void dispose() {
    _displayNameController.dispose();
    _descriptionController.dispose();
    _versionController.dispose();
    _minVersionController.dispose();
    _maxVersionController.dispose();
    super.dispose();
  }

  Future<void> _loadSources() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final loadedSources = await _loadPublishablePackageSources(
        widget.clients,
      );
      final publishContext = _publishContext;
      final sources = publishContext == null
          ? loadedSources
          : loadedSources
                .where(
                  (source) => _sameArtifactRuntimePackageId(
                    source.packageName,
                    publishContext.runtimePackageId,
                  ),
                )
                .toList(growable: false);
      if (!mounted) {
        return;
      }
      setState(() {
        _sources = sources;
        if (sources.isEmpty) {
          _selectedSource = null;
        }
        _loading = false;
      });
      if (sources.isNotEmpty) {
        _selectSource(sources.first);
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to load publishable artifacts: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  void _selectSource(core_proxy.PublishablePackageSource source) {
    final lockedDisplayName = _publishContext?.lockedDisplayName.trim();
    setState(() {
      _selectedSource = source;
      _displayNameController.text = lockedDisplayName?.isNotEmpty == true
          ? lockedDisplayName!
          : source.displayName;
      _descriptionController.text = source.description;
      _versionController.text =
          source.inferredVersion?.trim().isNotEmpty == true
          ? source.inferredVersion!.trim()
          : '1.0.0';
    });
  }

  Future<void> _publish({required bool allowCreateForgeRepo}) async {
    final source = _selectedSource;
    if (source == null || _publishing) {
      return;
    }
    setState(() {
      _publishing = true;
      _errorMessage = null;
      _pendingMarketRegistration = null;
      _progressMessage = '正在检查发布信息';
    });
    try {
      final result = await _publishArtifact(
        clients: widget.clients,
        source: source,
        displayName: _displayNameController.text,
        description: _descriptionController.text,
        version: _versionController.text,
        minSupportedAppVersion: _minVersionController.text,
        maxSupportedAppVersion: _maxVersionController.text,
        publishContext: _publishContext,
        allowCreateForgeRepo: allowCreateForgeRepo,
        onProgress: (message) {
          if (mounted) {
            setState(() {
              _progressMessage = message;
            });
          }
        },
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _publishing = false;
        _progressMessage = null;
      });
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          icon: const Icon(Icons.check_circle_outline),
          title: const Text('发布完成'),
          content: SelectableText(
            '已发布「${result.displayName}」\n'
            '项目簇: ${result.projectId}\n'
            '节点: ${result.nodeId}\n'
            'Release: ${result.releaseTag}\n\n'
            '公共市场需要排期发布，请等待排期完成后查看。',
          ),
          actions: <Widget>[
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('确定'),
            ),
          ],
        ),
      );
    } on _NeedsForgeInitialization catch (request) {
      if (!mounted) {
        return;
      }
      setState(() {
        _publishing = false;
        _progressMessage = null;
      });
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          icon: const Icon(Icons.store_outlined),
          title: const Text('初始化 OperitForge'),
          content: Text(
            '需要在 @${request.publisherLogin} 下创建公开仓库 $_forgeRepoName，用于保存发布资产。',
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('创建并继续'),
            ),
          ],
        ),
      );
      if (confirmed == true) {
        await _publish(allowCreateForgeRepo: true);
      }
    } on _RegistrationRetryRequired catch (request) {
      if (!mounted) {
        return;
      }
      setState(() {
        _publishing = false;
        _progressMessage = null;
        _errorMessage = request.errorMessage;
        _pendingMarketRegistration = _PendingMarketRegistration(
          type: request.type,
          title: request.title,
          payload: request.payload,
          result: request.result,
        );
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to publish artifact: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _publishing = false;
        _progressMessage = null;
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _retryMarketRegistration() async {
    final pending = _pendingMarketRegistration;
    if (pending == null || _retryingMarketRegistration) {
      return;
    }
    setState(() {
      _retryingMarketRegistration = true;
      _errorMessage = null;
      _progressMessage = '正在重新登记市场';
    });
    try {
      await _createMarketIssue(
        clients: widget.clients,
        type: pending.type,
        title: pending.title,
        body: _buildArtifactMarketIssueBody(pending.payload),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _retryingMarketRegistration = false;
        _pendingMarketRegistration = null;
        _progressMessage = null;
      });
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          icon: const Icon(Icons.check_circle_outline),
          title: const Text('发布完成'),
          content: SelectableText(
            '已登记「${pending.result.displayName}」\n'
            '项目簇: ${pending.result.projectId}\n'
            '节点: ${pending.result.nodeId}\n'
            'Release: ${pending.result.releaseTag}\n\n'
            '公共市场需要排期发布，请等待排期完成后查看。',
          ),
          actions: <Widget>[
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('确定'),
            ),
          ],
        ),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to retry market registration: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _retryingMarketRegistration = false;
        _progressMessage = null;
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _openParentSelectionEditor() async {
    final publishContext = _publishContext;
    if (publishContext == null || _selectionProjectLoading) {
      return;
    }
    setState(() {
      _selectionProjectLoading = true;
      _selectionLoadError = null;
    });
    try {
      final project = await widget.clients.apiMarketStatsApiService
          .getArtifactProject(projectId: publishContext.projectId);
      if (!mounted) {
        return;
      }
      setState(() {
        _selectionProjectLoading = false;
      });
      final selected = await showDialog<Set<String>>(
        context: context,
        builder: (context) => _ArtifactParentSelectionDialog(
          project: project,
          selectedNodeIds: publishContext.parentNodeIds.toSet(),
        ),
      );
      if (selected == null || selected.isEmpty || !mounted) {
        return;
      }
      final orderedSelected = project.nodes
          .map((node) => node.nodeId)
          .where(selected.contains)
          .toList(growable: false);
      if (orderedSelected.isEmpty) {
        return;
      }
      setState(() {
        _publishContext = ArtifactPublishClusterContext(
          projectId: publishContext.projectId,
          rootNodeId: publishContext.rootNodeId,
          runtimePackageId: publishContext.runtimePackageId,
          parentNodeIds: orderedSelected,
          lockedDisplayName: publishContext.lockedDisplayName,
          projectDisplayName: publishContext.projectDisplayName,
          projectDescription: publishContext.projectDescription,
        );
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load artifact parent selection: $error\n$stackTrace',
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _selectionProjectLoading = false;
        _selectionLoadError = error.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    final source = _selectedSource;
    final publishContext = _publishContext;
    final isContinuationMode = publishContext != null;
    final lockedDisplayName = publishContext?.lockedDisplayName.trim() ?? '';
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: Text(isContinuationMode ? '发布更新版本' : '发布 Artifact'),
        actions: <Widget>[
          IconButton(
            onPressed: _loading || _publishing ? null : _loadSources,
            icon: const Icon(Icons.refresh),
            tooltip: '刷新',
          ),
        ],
      ),
      body: Builder(
        builder: (context) {
          if (_loading) {
            return const Center(child: CircularProgressIndicator());
          }
          if (error != null && _sources.isEmpty) {
            return EmptyState(
              icon: Icons.error_outline,
              title: '加载失败',
              message: error,
              action: TextButton.icon(
                onPressed: _loadSources,
                icon: const Icon(Icons.refresh),
                label: const Text('刷新'),
              ),
            );
          }
          if (_sources.isEmpty) {
            return EmptyState(
              icon: Icons.inventory_2_outlined,
              title: isContinuationMode
                  ? '没有对应的本地 Artifact'
                  : '没有可发布的本地 Artifact',
              message: isContinuationMode
                  ? '当前是基于版本发布，但本地还没有找到同一运行时包。'
                  : '安装外部 JS/HJSON 包或 ToolPkg 后再发布。',
              scrollable: false,
            );
          }
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 120),
            children: <Widget>[
              if (publishContext != null) ...<Widget>[
                _PublishContinuationPanel(
                  contextInfo: publishContext,
                  isLoading: _selectionProjectLoading,
                  errorMessage: _selectionLoadError,
                  onChangeSelection: _openParentSelectionEditor,
                ),
                const SizedBox(height: 12),
              ],
              DropdownButtonFormField<String>(
                key: ValueKey<String?>(source?.packageName),
                initialValue: source?.packageName,
                decoration: const InputDecoration(
                  labelText: '本地 Artifact',
                  border: OutlineInputBorder(),
                ),
                items: _sources
                    .map(
                      (item) => DropdownMenuItem<String>(
                        value: item.packageName,
                        child: Text(
                          '${item.displayName} · ${_artifactTypeLabel(item.type)}',
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    )
                    .toList(growable: false),
                onChanged: _publishing
                    ? null
                    : (value) {
                        final selected = _sources.firstWhere(
                          (item) => item.packageName == value,
                        );
                        _selectSource(selected);
                      },
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _displayNameController,
                enabled: !_publishing && lockedDisplayName.isEmpty,
                decoration: InputDecoration(
                  labelText: '显示名称',
                  border: const OutlineInputBorder(),
                  helperText: lockedDisplayName.isEmpty
                      ? null
                      : '基于版本发布时，名字沿用来源版本。',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _descriptionController,
                enabled: !_publishing,
                minLines: 3,
                maxLines: 6,
                decoration: const InputDecoration(
                  labelText: '简介',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _versionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: '版本',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _minVersionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: '最低支持版本',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _maxVersionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: '最高支持版本',
                  border: OutlineInputBorder(),
                ),
              ),
              if (error != null) ...<Widget>[
                const SizedBox(height: 12),
                _PublishErrorPanel(message: error),
                if (_pendingMarketRegistration != null) ...<Widget>[
                  const SizedBox(height: 8),
                  OutlinedButton.icon(
                    onPressed: _retryingMarketRegistration
                        ? null
                        : _retryMarketRegistration,
                    icon: _retryingMarketRegistration
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.refresh),
                    label: Text(_retryingMarketRegistration ? '重试中' : '重试市场登记'),
                  ),
                ],
              ],
              if (_progressMessage != null) ...<Widget>[
                const SizedBox(height: 12),
                LinearProgressIndicator(value: _publishing ? null : 0),
                const SizedBox(height: 8),
                Text(_progressMessage!),
              ],
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: _publishing
                    ? null
                    : () => _publish(allowCreateForgeRepo: false),
                icon: _publishing
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.cloud_upload_outlined),
                label: Text(
                  _publishing
                      ? '发布中'
                      : isContinuationMode
                      ? '发布更新版本'
                      : '发布到市场',
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _PublishContinuationPanel extends StatelessWidget {
  const _PublishContinuationPanel({
    required this.contextInfo,
    required this.isLoading,
    required this.errorMessage,
    required this.onChangeSelection,
  });

  final ArtifactPublishClusterContext contextInfo;
  final bool isLoading;
  final String? errorMessage;
  final VoidCallback onChangeSelection;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final parentCount = contextInfo.parentNodeIds
        .where((nodeId) => nodeId.trim().isNotEmpty)
        .length;
    return OperitGlassSurface(
      color: colorScheme.secondaryContainer.withValues(alpha: 0.32),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: colorScheme.secondary.withValues(alpha: 0.14)),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    '选中$parentCount个版本',
                    style: textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                TextButton(
                  onPressed: isLoading ? null : onChangeSelection,
                  child: Text(isLoading ? '加载中' : '更改'),
                ),
              ],
            ),
            if (errorMessage?.trim().isNotEmpty == true) ...<Widget>[
              const SizedBox(height: 6),
              Text(
                errorMessage!.trim(),
                style: textTheme.bodySmall?.copyWith(color: colorScheme.error),
              ),
            ],
            const SizedBox(height: 6),
            if (contextInfo.lockedDisplayName.trim().isNotEmpty)
              Text(
                '插件名字将沿用 ${contextInfo.lockedDisplayName.trim()}',
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            Text(
              '包名和发布名称会自动沿用。',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactParentSelectionDialog extends StatefulWidget {
  const _ArtifactParentSelectionDialog({
    required this.project,
    required this.selectedNodeIds,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final Set<String> selectedNodeIds;

  @override
  State<_ArtifactParentSelectionDialog> createState() =>
      _ArtifactParentSelectionDialogState();
}

class _ArtifactParentSelectionDialogState
    extends State<_ArtifactParentSelectionDialog> {
  late final Set<String> _selectedNodeIds = Set<String>.from(
    widget.selectedNodeIds,
  );

  @override
  Widget build(BuildContext context) {
    final viewport = MediaQuery.sizeOf(context);
    return AlertDialog(
      title: const Text('选择基础版本'),
      content: SizedBox(
        width: min(680.0, viewport.width * 0.9),
        height: min(520.0, viewport.height * 0.68),
        child: ListView.separated(
          itemCount: widget.project.nodes.length,
          separatorBuilder: (context, index) => const Divider(height: 1),
          itemBuilder: (context, index) {
            final node = widget.project.nodes[index];
            final selected = _selectedNodeIds.contains(node.nodeId);
            return CheckboxListTile(
              value: selected,
              onChanged: (value) {
                setState(() {
                  if (value == true) {
                    _selectedNodeIds.add(node.nodeId);
                  } else {
                    _selectedNodeIds.remove(node.nodeId);
                  }
                });
              },
              title: Text(
                node.displayName.trim().isEmpty
                    ? node.nodeId
                    : node.displayName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(
                '${node.version} · ${node.publisherLogin} · ${node.state}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              controlAffinity: ListTileControlAffinity.leading,
            );
          },
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _selectedNodeIds.isEmpty
              ? null
              : () => Navigator.of(context).pop(_selectedNodeIds),
          child: const Text('确定'),
        ),
      ],
    );
  }
}

class _PublishErrorPanel extends StatelessWidget {
  const _PublishErrorPanel({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.errorContainer,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: colorScheme.error.withValues(alpha: 0.18)),
      transparentAlpha: 0.22,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Text(
          message,
          style: TextStyle(color: colorScheme.onErrorContainer),
        ),
      ),
    );
  }
}

extension _PublishablePackageSourceArtifactType
    on core_proxy.PublishablePackageSource {
  String get type => isToolPkg ? 'package' : 'script';
}

class _ForgeRepoInfo {
  const _ForgeRepoInfo({
    required this.ownerLogin,
    required this.repoName,
    required this.htmlUrl,
  });

  final String ownerLogin;
  final String repoName;
  final String htmlUrl;
}

class _GitHubReleaseInfo {
  const _GitHubReleaseInfo({required this.id, required this.assets});

  final int id;
  final List<_GitHubReleaseAssetInfo> assets;
}

class _GitHubReleaseAssetInfo {
  const _GitHubReleaseAssetInfo({
    required this.id,
    required this.name,
    required this.browserDownloadUrl,
  });

  factory _GitHubReleaseAssetInfo.fromJson(Map<String, Object?> json) {
    return _GitHubReleaseAssetInfo(
      id: json['id'] as int,
      name: json['name'] as String,
      browserDownloadUrl: json['browser_download_url'] as String,
    );
  }

  final int id;
  final String name;
  final String browserDownloadUrl;
}

class _PublishResult {
  const _PublishResult({
    required this.displayName,
    required this.projectId,
    required this.nodeId,
    required this.releaseTag,
  });

  final String displayName;
  final String projectId;
  final String nodeId;
  final String releaseTag;
}

class _NeedsForgeInitialization implements Exception {
  const _NeedsForgeInitialization(this.publisherLogin);

  final String publisherLogin;
}

class _PendingMarketRegistration {
  const _PendingMarketRegistration({
    required this.type,
    required this.title,
    required this.payload,
    required this.result,
  });

  final String type;
  final String title;
  final Map<String, Object?> payload;
  final _PublishResult result;
}

class _RegistrationRetryRequired implements Exception {
  const _RegistrationRetryRequired({
    required this.type,
    required this.title,
    required this.payload,
    required this.result,
    required this.errorMessage,
  });

  final String type;
  final String title;
  final Map<String, Object?> payload;
  final _PublishResult result;
  final String errorMessage;
}

Future<List<core_proxy.PublishablePackageSource>>
_loadPublishablePackageSources(GeneratedCoreProxyClients clients) async {
  return clients.permissionsPackToolPackageManager
      .getPublishablePackageSources();
}

Future<_PublishResult> _publishArtifact({
  required GeneratedCoreProxyClients clients,
  required core_proxy.PublishablePackageSource source,
  required String displayName,
  required String description,
  required String version,
  required String minSupportedAppVersion,
  required String maxSupportedAppVersion,
  required ArtifactPublishClusterContext? publishContext,
  required bool allowCreateForgeRepo,
  required ValueChanged<String> onProgress,
}) async {
  final trimmedDisplayName = displayName.trim();
  final trimmedDescription = description.trim();
  if (trimmedDisplayName.isEmpty) {
    throw StateError('插件名称不能为空');
  }
  if (trimmedDescription.isEmpty) {
    throw StateError('简介不能为空');
  }
  final cleanVersion = _normalizeArtifactVersion(version);
  final normalizedMinVersion = _normalizeAppVersionOrNull(
    minSupportedAppVersion,
  );
  final normalizedMaxVersion = _normalizeAppVersionOrNull(
    maxSupportedAppVersion,
  );
  _validateAppVersionRange(normalizedMinVersion, normalizedMaxVersion);

  onProgress('正在读取 GitHub 账号');
  final currentUser = await clients.apiMarketStatsApiService.getCurrentUser();
  final normalizedRuntimePackageId = _normalizeMarketArtifactId(
    source.packageName,
  );
  if (publishContext == null) {
    _validateStandaloneArtifactRuntimePackageId(source.packageName);
    onProgress('正在检查名称和 ID');
    await _ensureFreshPublishIdentityAvailable(
      clients: clients,
      displayName: trimmedDisplayName,
      runtimePackageId: source.packageName,
      normalizedRuntimePackageId: normalizedRuntimePackageId,
    );
  } else {
    final contextRuntimePackageId = publishContext.runtimePackageId.trim();
    if (contextRuntimePackageId.isNotEmpty &&
        !_sameArtifactRuntimePackageId(
          contextRuntimePackageId,
          source.packageName,
        )) {
      throw StateError(
        "Continuation publish must keep runtime package id '$contextRuntimePackageId'",
      );
    }
    await _validateContinuationAuthorDeclaration(
      clients: clients,
      source: source,
      publishContext: publishContext,
    );
  }

  onProgress('正在准备 OperitForge');
  final forgeRepo = await _ensureForgeRepository(
    clients: clients,
    publisherLogin: currentUser.login,
    allowCreateForgeRepo: allowCreateForgeRepo,
  );

  final nodeId = _uuidV4();
  final resolvedDisplayName =
      publishContext?.lockedDisplayName.trim().isNotEmpty == true
      ? publishContext!.lockedDisplayName.trim()
      : trimmedDisplayName;
  final rootNodeId = publishContext?.rootNodeId.trim().isNotEmpty == true
      ? publishContext!.rootNodeId.trim()
      : nodeId;
  final parentNodeIds =
      publishContext?.parentNodeIds
          .map((nodeId) => nodeId.trim())
          .where((nodeId) => nodeId.isNotEmpty)
          .toList(growable: false) ??
      const <String>[];
  final projectId = publishContext?.projectId.trim().isNotEmpty == true
      ? _normalizeMarketArtifactId(publishContext!.projectId)
      : normalizedRuntimePackageId;
  final projectDisplayName =
      publishContext?.projectDisplayName.trim().isNotEmpty == true
      ? publishContext!.projectDisplayName.trim()
      : resolvedDisplayName;
  final projectDescription =
      publishContext?.projectDescription.trim().isNotEmpty == true
      ? publishContext!.projectDescription.trim()
      : trimmedDescription;
  final extension = source.fileExtension.trim().isEmpty
      ? 'bin'
      : source.fileExtension.trim();
  final assetName =
      '$normalizedRuntimePackageId-v$cleanVersion-${nodeId.substring(0, 8)}.$extension';
  final releaseTag =
      '${_artifactReleaseTagPrefix(source.type)}-$normalizedRuntimePackageId-v$cleanVersion-${nodeId.substring(0, 8)}';
  final releaseName = '$resolvedDisplayName v$cleanVersion';
  final releaseBody = _buildReleaseBody(
    type: source.type,
    projectId: projectId,
    runtimePackageId: source.packageName,
    nodeId: nodeId,
    rootNodeId: rootNodeId,
    parentNodeIds: parentNodeIds,
    displayName: resolvedDisplayName,
    version: cleanVersion,
    minSupportedAppVersion: normalizedMinVersion,
    maxSupportedAppVersion: normalizedMaxVersion,
  );

  onProgress('正在创建 Release');
  final release = await _createOrUpdateRelease(
    clients: clients,
    owner: currentUser.login,
    repo: forgeRepo.repoName,
    tagName: releaseTag,
    name: releaseName,
    body: releaseBody,
  );

  onProgress('正在上传资源文件');
  final fileBytes = await XFile(source.sourcePath).readAsBytes();
  final asset = await _uploadReleaseAsset(
    clients: clients,
    owner: currentUser.login,
    repo: forgeRepo.repoName,
    release: release,
    assetName: assetName,
    contentType: _artifactContentType(source.type, extension),
    content: fileBytes,
  );

  final payload = <String, Object?>{
    'type': source.type,
    'projectId': projectId,
    'projectDisplayName': projectDisplayName,
    'projectDescription': projectDescription,
    'runtimePackageId': source.packageName,
    'nodeId': nodeId,
    'rootNodeId': rootNodeId,
    'parentNodeIds': parentNodeIds,
    'publisherLogin': currentUser.login,
    'releaseTag': releaseTag,
    'assetName': asset.name,
    'downloadUrl': asset.browserDownloadUrl,
    'sha256': crypto.sha256.convert(fileBytes).toString(),
    'version': cleanVersion,
    'displayName': resolvedDisplayName,
    'description': trimmedDescription,
    'sourceFileName': source.sourceFileName,
    'minSupportedAppVersion': normalizedMinVersion,
    'maxSupportedAppVersion': normalizedMaxVersion,
    'normalizedId': '',
    'forgeRepo': forgeRepo.repoName,
  };

  onProgress('正在登记市场');
  final result = _PublishResult(
    displayName: resolvedDisplayName,
    projectId: projectId,
    nodeId: nodeId,
    releaseTag: releaseTag,
  );
  try {
    await _createMarketIssue(
      clients: clients,
      type: source.type,
      title: resolvedDisplayName,
      body: _buildArtifactMarketIssueBody(payload),
    );
  } catch (error) {
    throw _RegistrationRetryRequired(
      type: source.type,
      title: resolvedDisplayName,
      payload: payload,
      result: result,
      errorMessage: error.toString(),
    );
  }
  return result;
}

Future<_ForgeRepoInfo> _ensureForgeRepository({
  required GeneratedCoreProxyClients clients,
  required String publisherLogin,
  required bool allowCreateForgeRepo,
}) async {
  final repoUri = Uri.https(
    'api.github.com',
    '/repos/$publisherLogin/$_forgeRepoName',
  );
  final existingResponse = await _githubHttpRequest(
    clients: clients,
    method: 'GET',
    uri: repoUri,
  );
  if (_isSuccess(existingResponse.statusCode)) {
    final repo = jsonDecode(existingResponse.body) as Map<String, Object?>;
    if ((repo['size'] as int? ?? 0) == 0) {
      await _createReadme(
        clients: clients,
        owner: publisherLogin,
        repo: _forgeRepoName,
      );
    }
    return _ForgeRepoInfo(
      ownerLogin: publisherLogin,
      repoName: repo['name'] as String,
      htmlUrl: repo['html_url'] as String,
    );
  }
  if (existingResponse.statusCode != 404) {
    throw StateError(
      'HTTP ${existingResponse.statusCode}: ${_summarizeHttpBody(existingResponse.body)}',
    );
  }
  if (!allowCreateForgeRepo) {
    throw _NeedsForgeInitialization(publisherLogin);
  }
  final created =
      await _githubJsonRequest(
            clients: clients,
            method: 'POST',
            uri: Uri.https('api.github.com', '/user/repos'),
            body: <String, Object?>{
              'name': _forgeRepoName,
              'description':
                  'Operit publish-only artifact repository for release assets.',
              'private': false,
              'auto_init': true,
            },
          )
          as Map<String, Object?>;
  return _ForgeRepoInfo(
    ownerLogin: publisherLogin,
    repoName: created['name'] as String,
    htmlUrl: created['html_url'] as String,
  );
}

Future<void> _createReadme({
  required GeneratedCoreProxyClients clients,
  required String owner,
  required String repo,
}) async {
  await _githubJsonRequest(
    clients: clients,
    method: 'PUT',
    uri: Uri.https('api.github.com', '/repos/$owner/$repo/contents/README.md'),
    body: <String, Object?>{
      'message': 'Initialize OperitForge repository',
      'content': base64Encode(
        utf8.encode(
          '# OperitForge\n\nThis repository stores release assets published from Operit.\n',
        ),
      ),
    },
  );
}

Future<_GitHubReleaseInfo> _createOrUpdateRelease({
  required GeneratedCoreProxyClients clients,
  required String owner,
  required String repo,
  required String tagName,
  required String name,
  required String body,
}) async {
  final tagResponse = await _githubHttpRequest(
    clients: clients,
    method: 'GET',
    uri: Uri.https(
      'api.github.com',
      '/repos/$owner/$repo/releases/tags/$tagName',
    ),
  );
  if (tagResponse.statusCode == 404) {
    final created =
        await _githubJsonRequest(
              clients: clients,
              method: 'POST',
              uri: Uri.https('api.github.com', '/repos/$owner/$repo/releases'),
              body: <String, Object?>{
                'tag_name': tagName,
                'name': name,
                'body': body,
                'draft': false,
                'prerelease': false,
              },
            )
            as Map<String, Object?>;
    return _releaseFromJson(created);
  }
  if (!_isSuccess(tagResponse.statusCode)) {
    throw StateError(
      'HTTP ${tagResponse.statusCode}: ${_summarizeHttpBody(tagResponse.body)}',
    );
  }
  final existing = jsonDecode(tagResponse.body) as Map<String, Object?>;
  final updated =
      await _githubJsonRequest(
            clients: clients,
            method: 'PATCH',
            uri: Uri.https(
              'api.github.com',
              '/repos/$owner/$repo/releases/${existing['id']}',
            ),
            body: <String, Object?>{
              'name': name,
              'body': body,
              'draft': false,
              'prerelease': false,
            },
          )
          as Map<String, Object?>;
  return _releaseFromJson(updated);
}

Future<_GitHubReleaseAssetInfo> _uploadReleaseAsset({
  required GeneratedCoreProxyClients clients,
  required String owner,
  required String repo,
  required _GitHubReleaseInfo release,
  required String assetName,
  required String contentType,
  required List<int> content,
}) async {
  for (final asset in release.assets) {
    if (asset.name.toLowerCase() == assetName.toLowerCase()) {
      await _githubJsonRequest(
        clients: clients,
        method: 'DELETE',
        uri: Uri.https(
          'api.github.com',
          '/repos/$owner/$repo/releases/assets/${asset.id}',
        ),
      );
    }
  }
  final response = await _githubHttpRequest(
    clients: clients,
    method: 'POST',
    uri: Uri.https(
      'uploads.github.com',
      '/repos/$owner/$repo/releases/${release.id}/assets',
      <String, String>{'name': assetName},
    ),
    bodyBytes: content,
    contentType: contentType,
  );
  if (!_isSuccess(response.statusCode)) {
    throw StateError(
      'HTTP ${response.statusCode}: ${_summarizeHttpBody(response.body)}',
    );
  }
  return _GitHubReleaseAssetInfo.fromJson(
    jsonDecode(response.body) as Map<String, Object?>,
  );
}

Future<void> _createMarketIssue({
  required GeneratedCoreProxyClients clients,
  required String type,
  required String title,
  required String body,
}) async {
  await _githubJsonRequest(
    clients: clients,
    method: 'POST',
    uri: Uri.https(
      'api.github.com',
      '/repos/$_marketOwner/${_artifactMarketRepo(type)}/issues',
    ),
    body: <String, Object?>{
      'title': title,
      'body': body,
      'labels': <String>[_artifactMarketLabel(type)],
    },
  );
}

Future<void> _validateContinuationAuthorDeclaration({
  required GeneratedCoreProxyClients clients,
  required core_proxy.PublishablePackageSource source,
  required ArtifactPublishClusterContext publishContext,
}) async {
  final parentNodeIds = publishContext.parentNodeIds
      .map((nodeId) => nodeId.trim())
      .where((nodeId) => nodeId.isNotEmpty)
      .toList(growable: false);
  if (parentNodeIds.isEmpty || !source.hasDeclaredAuthorField) {
    return;
  }
  final project = await clients.apiMarketStatsApiService.getArtifactProject(
    projectId: publishContext.projectId,
  );
  final nodeById = <String, core_proxy.ArtifactProjectNodeResponse>{
    for (final node in project.nodes) node.nodeId: node,
  };
  final publisherLogins = <String>{};
  for (final nodeId in parentNodeIds) {
    final node = nodeById[nodeId];
    if (node == null) {
      throw StateError('找不到前驱节点 `$nodeId`，无法校验作者数量。');
    }
    final publisherLogin = node.publisherLogin.trim().isNotEmpty
        ? node.publisherLogin.trim()
        : node.issue.user.login.trim();
    if (publisherLogin.isNotEmpty) {
      publisherLogins.add(publisherLogin);
    }
  }
  final predecessorPublisherCount = publisherLogins.length;
  if (predecessorPublisherCount > 0 &&
      source.declaredAuthorSlotCount < predecessorPublisherCount) {
    throw StateError(
      '当前作品已声明 author，但数量不足。当前直接前驱节点的 GitHub 发布者共有 '
      '$predecessorPublisherCount 个；author 要么不写，要么至少提供 '
      '$predecessorPublisherCount 个位置。',
    );
  }
}

Future<void> _ensureFreshPublishIdentityAvailable({
  required GeneratedCoreProxyClients clients,
  required String displayName,
  required String runtimePackageId,
  required String normalizedRuntimePackageId,
}) async {
  final normalizedTitle = _normalizePublishTitle(displayName);
  for (final definition in _artifactDefinitions()) {
    final titleIssues = await _githubSearchIssues(
      clients: clients,
      query:
          'repo:$_marketOwner/${definition.repo} is:issue in:title "$displayName"',
    );
    final titleConflict = titleIssues.any(
      (issue) =>
          _normalizePublishTitle(issue['title'] as String) == normalizedTitle,
    );
    if (titleConflict) {
      throw StateError('名字「$displayName」已存在。');
    }
    final runtimeIssues = await _githubSearchIssues(
      clients: clients,
      query:
          'repo:$_marketOwner/${definition.repo} is:issue "$runtimePackageId"',
    );
    final runtimeConflict = runtimeIssues.any((issue) {
      final metadata = _artifactMetadataFromBody(
        issue['body'] as String? ?? '',
      );
      final existingRuntimePackageId =
          metadata['runtimePackageId']?.toString() ?? '';
      final existingProjectId = metadata['projectId']?.toString() ?? '';
      return _sameArtifactRuntimePackageId(
            existingRuntimePackageId,
            runtimePackageId,
          ) ||
          _normalizeMarketArtifactId(existingProjectId) ==
              normalizedRuntimePackageId;
    });
    if (runtimeConflict) {
      throw StateError('ID「$runtimePackageId」已存在。');
    }
  }
}

Future<List<Map<String, Object?>>> _githubSearchIssues({
  required GeneratedCoreProxyClients clients,
  required String query,
}) async {
  final response =
      await _githubJsonRequest(
            clients: clients,
            method: 'GET',
            uri: Uri.https('api.github.com', '/search/issues', <String, String>{
              'q': query,
              'sort': 'updated',
              'order': 'desc',
              'page': '1',
              'per_page': '100',
            }),
          )
          as Map<String, Object?>;
  return (response['items'] as List<Object?>)
      .map((item) => item as Map<String, Object?>)
      .toList(growable: false);
}

Future<Object?> _githubJsonRequest({
  required GeneratedCoreProxyClients clients,
  required String method,
  required Uri uri,
  Object? body,
}) async {
  final response = await _githubHttpRequest(
    clients: clients,
    method: method,
    uri: uri,
    body: body,
  );
  if (!_isSuccess(response.statusCode)) {
    throw StateError(
      'HTTP ${response.statusCode}: ${_summarizeHttpBody(response.body)}',
    );
  }
  if (response.body.trim().isEmpty) {
    return null;
  }
  return jsonDecode(response.body);
}

Future<http.Response> _githubHttpRequest({
  required GeneratedCoreProxyClients clients,
  required String method,
  required Uri uri,
  Object? body,
  List<int>? bodyBytes,
  String? contentType,
}) async {
  final token = await clients.preferencesGitHubAuthPreferences
      .getCurrentAccessToken();
  if (token == null || token.trim().isEmpty) {
    throw StateError('GitHub login required');
  }
  final request = http.Request(method, uri);
  request.headers.addAll(<String, String>{
    'Accept':
        'application/vnd.github+json, application/vnd.github.squirrel-girl-preview+json',
    'Authorization': 'Bearer ${token.trim()}',
    'X-GitHub-Api-Version': '2022-11-28',
  });
  if (body != null) {
    request.headers['Content-Type'] = 'application/json';
    request.body = jsonEncode(body);
  }
  if (bodyBytes != null) {
    request.headers['Content-Type'] = contentType ?? 'application/octet-stream';
    request.bodyBytes = bodyBytes;
  }
  final streamed = await request.send();
  return http.Response.fromStream(streamed);
}

_GitHubReleaseInfo _releaseFromJson(Map<String, Object?> json) {
  return _GitHubReleaseInfo(
    id: json['id'] as int,
    assets: (json['assets'] as List<Object?>)
        .map(
          (item) =>
              _GitHubReleaseAssetInfo.fromJson(item as Map<String, Object?>),
        )
        .toList(growable: false),
  );
}

String _buildReleaseBody({
  required String type,
  required String projectId,
  required String runtimePackageId,
  required String nodeId,
  required String rootNodeId,
  required List<String> parentNodeIds,
  required String displayName,
  required String version,
  required String? minSupportedAppVersion,
  required String? maxSupportedAppVersion,
}) {
  final parentNodeText = parentNodeIds.isEmpty ? '-' : parentNodeIds.join(', ');
  return '''
${_artifactTitleLabel(type)} artifact published by OperitForge.

Project ID: $projectId
Runtime package ID: $runtimePackageId
Node ID: $nodeId
Root node ID: $rootNodeId
Parent node IDs: $parentNodeText
Display name: $displayName
Version: $version
Supported app versions: ${_formatSupportedAppVersions(minSupportedAppVersion, maxSupportedAppVersion)}
''';
}

String _buildArtifactMarketIssueBody(Map<String, Object?> payload) {
  final parentNodeIds = payload['parentNodeIds'] as List<String>;
  final parentNodeText = parentNodeIds.isEmpty ? '-' : parentNodeIds.join(', ');
  final timestamp = DateTime.now()
      .toIso8601String()
      .replaceFirst('T', ' ')
      .substring(0, 19);
  return '''
$_metadataPrefix${jsonEncode(payload)} -->
<!-- operit-parser-version: forge-v3 -->

## ${_artifactTitleLabel(payload['type'] as String)}

${payload['description']}

## Project Cluster

- Project ID: `${payload['projectId']}`
- Runtime package ID: `${payload['runtimePackageId']}`
- Node ID: `${payload['nodeId']}`
- Root node ID: `${payload['rootNodeId']}`
- Parent node IDs: `$parentNodeText`
- Project display name: `${payload['projectDisplayName']}`
- Project description: ${payload['projectDescription']}

## Artifact

- Publisher: `${payload['publisherLogin']}`
- Forge repo: `${payload['forgeRepo']}`
- Release tag: `${payload['releaseTag']}`
- Asset: `${payload['assetName']}`
- SHA-256: `${payload['sha256']}`
- Download: ${payload['downloadUrl']}

## Metadata

| Field | Value |
| --- | --- |
| Type | ${payload['type']} |
| Project ID | ${payload['projectId']} |
| Runtime package ID | ${payload['runtimePackageId']} |
| Node ID | ${payload['nodeId']} |
| Root node ID | ${payload['rootNodeId']} |
| Parent node IDs | $parentNodeText |
| Version | ${payload['version']} |
| Supported app versions | ${_formatSupportedAppVersions(payload['minSupportedAppVersion']?.toString(), payload['maxSupportedAppVersion']?.toString())} |
| Source file | ${payload['sourceFileName']} |
| Updated at | $timestamp |
''';
}

Map<String, Object?> _artifactMetadataFromBody(String body) {
  final start = body.indexOf(_metadataPrefix);
  if (start < 0) {
    return const <String, Object?>{};
  }
  final jsonStart = start + _metadataPrefix.length;
  final end = body.indexOf(' -->', jsonStart);
  if (end <= jsonStart) {
    return const <String, Object?>{};
  }
  final decoded = jsonDecode(body.substring(jsonStart, end));
  if (decoded is Map) {
    return decoded.map((key, value) => MapEntry(key.toString(), value));
  }
  return const <String, Object?>{};
}

String _normalizeArtifactVersion(String value) {
  final normalized = value.trim().replaceFirst(RegExp(r'^[vV]'), '');
  return normalized.isEmpty ? '1.0.0' : normalized;
}

String? _normalizeAppVersionOrNull(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  final match = RegExp(
    r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$',
  ).firstMatch(trimmed);
  if (match == null) {
    throw StateError('版本格式应为 1.2.3 或 1.2.3+4');
  }
  final build = match.group(4);
  return build == null
      ? '${match.group(1)}.${match.group(2)}.${match.group(3)}'
      : '${match.group(1)}.${match.group(2)}.${match.group(3)}+$build';
}

void _validateAppVersionRange(String? minVersion, String? maxVersion) {
  if (minVersion == null || maxVersion == null) {
    return;
  }
  if (_compareAppVersions(minVersion, maxVersion) > 0) {
    throw StateError('最低支持版本不能大于最高支持版本');
  }
}

int _compareAppVersions(String left, String right) {
  final leftParts = _appVersionParts(left);
  final rightParts = _appVersionParts(right);
  for (var index = 0; index < leftParts.length; index += 1) {
    final order = leftParts[index].compareTo(rightParts[index]);
    if (order != 0) {
      return order;
    }
  }
  return 0;
}

List<int> _appVersionParts(String value) {
  final match = RegExp(r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$').firstMatch(value);
  if (match == null) {
    throw StateError('版本格式应为 1.2.3 或 1.2.3+4');
  }
  return <int>[
    int.parse(match.group(1)!),
    int.parse(match.group(2)!),
    int.parse(match.group(3)!),
    int.parse(match.group(4) ?? '0'),
  ];
}

void _validateStandaloneArtifactRuntimePackageId(String runtimePackageId) {
  final trimmed = runtimePackageId.trim();
  if (trimmed.isNotEmpty &&
      _normalizeMarketArtifactId(trimmed) == 'artifact' &&
      trimmed.toLowerCase() != 'artifact') {
    throw StateError('当前包 ID「$runtimePackageId」无法生成稳定的市场项目 ID。');
  }
}

String _normalizeMarketArtifactId(String raw) {
  final normalized = raw
      .trim()
      .toLowerCase()
      .replaceAll(RegExp(r'[^a-z0-9]+'), '-')
      .replaceAll(RegExp(r'-+'), '-')
      .replaceAll(RegExp(r'^-|-$'), '');
  return normalized.isEmpty ? 'artifact' : normalized;
}

bool _sameArtifactRuntimePackageId(String left, String right) {
  final leftValue = left.trim();
  final rightValue = right.trim();
  if (leftValue.isEmpty || rightValue.isEmpty) {
    return false;
  }
  return leftValue.toLowerCase() == rightValue.toLowerCase() ||
      _normalizeMarketArtifactId(leftValue) ==
          _normalizeMarketArtifactId(rightValue);
}

String _normalizePublishTitle(String title) {
  return title.trim().replaceAll(RegExp(r'\s+'), ' ').toLowerCase();
}

String _uuidV4() {
  final random = Random.secure();
  final bytes = List<int>.generate(16, (_) => random.nextInt(256));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  final hex = bytes
      .map((byte) => byte.toRadixString(16).padLeft(2, '0'))
      .join();
  return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-${hex.substring(12, 16)}-${hex.substring(16, 20)}-${hex.substring(20)}';
}

bool _isSuccess(int statusCode) {
  return statusCode >= 200 && statusCode < 300;
}

String _summarizeHttpBody(String body) {
  final trimmed = body.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  if (trimmed.contains('<html') || trimmed.contains('<!DOCTYPE html')) {
    return '[html body omitted]';
  }
  return trimmed.split('\n').first.trim();
}

String _artifactTypeLabel(String type) {
  return switch (type) {
    'package' => 'Package',
    'script' => 'Script',
    final value => value,
  };
}

String _artifactTitleLabel(String type) {
  return type == 'package' ? 'Package' : 'Script';
}

String _artifactReleaseTagPrefix(String type) {
  return type == 'package' ? 'package' : 'script';
}

String _artifactMarketRepo(String type) {
  return type == 'package' ? 'OperitPackageMarket' : 'OperitScriptMarket';
}

String _artifactMarketLabel(String type) {
  return type == 'package' ? 'package-artifact' : 'script-artifact';
}

String _artifactContentType(String type, String extension) {
  if (type == 'package') {
    return 'application/zip';
  }
  return switch (extension.toLowerCase()) {
    'js' => 'application/javascript',
    'ts' => 'text/plain',
    'hjson' => 'application/hjson',
    _ => 'application/octet-stream',
  };
}

String _formatSupportedAppVersions(String? minVersion, String? maxVersion) {
  final minValue = minVersion?.trim() ?? '';
  final maxValue = maxVersion?.trim() ?? '';
  if (minValue.isNotEmpty && maxValue.isNotEmpty) {
    return '$minValue - $maxValue';
  }
  if (minValue.isNotEmpty) {
    return '>= $minValue';
  }
  if (maxValue.isNotEmpty) {
    return '<= $maxValue';
  }
  return '未声明';
}

List<_ArtifactDefinition> _artifactDefinitions() {
  return const <_ArtifactDefinition>[
    _ArtifactDefinition(repo: 'OperitScriptMarket'),
    _ArtifactDefinition(repo: 'OperitPackageMarket'),
  ];
}

class _ArtifactDefinition {
  const _ArtifactDefinition({required this.repo});

  final String repo;
}
