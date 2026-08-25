// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitFormStyles.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';

const String _forgeRepoName = 'OperitForge';
const List<String> _artifactMarketTypes = <String>['script', 'package'];

enum _ArtifactPublishAssetSource { directUpload, githubReleaseAsset }

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
    required this.entryId,
    required this.projectId,
    required this.runtimePackageId,
    required this.lockedDisplayName,
    this.canEditEntry = false,
    this.initialEntry,
  });

  final String entryId;
  final String projectId;
  final String runtimePackageId;
  final String lockedDisplayName;
  final bool canEditEntry;
  final core_proxy.MarketEntrySummary? initialEntry;
}

class _ArtifactPublishScreenState extends State<ArtifactPublishScreen> {
  final TextEditingController _displayNameController = TextEditingController();
  final TextEditingController _descriptionController = TextEditingController();
  final TextEditingController _detailController = TextEditingController();
  final TextEditingController _versionController = TextEditingController();
  final TextEditingController _minVersionController = TextEditingController();
  final TextEditingController _maxVersionController = TextEditingController();
  final TextEditingController _githubRepositoryController =
      TextEditingController();

  bool _loading = true;
  bool _publishing = false;
  bool _retryingMarketRegistration = false;
  bool _allowPublicUpdates = true;
  bool _minifyArtifact = false;
  bool _loadingGitHubReleaseCatalog = false;
  String? _errorMessage;
  String? _progressMessage;
  String? _githubReleaseCatalogError;
  String? _selectedGitHubReleaseTag;
  String? _selectedGitHubReleaseAssetName;
  _ArtifactPublishAssetSource _assetSource =
      _ArtifactPublishAssetSource.directUpload;
  _GitHubReleaseCatalog? _githubReleaseCatalog;
  ArtifactPublishClusterContext? _publishContext;
  _PendingMarketRegistration? _pendingMarketRegistration;
  List<core_proxy.MarketCategoryInfo> _categories =
      <core_proxy.MarketCategoryInfo>[];
  String? _selectedCategoryId;
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
    _detailController.dispose();
    _versionController.dispose();
    _minVersionController.dispose();
    _maxVersionController.dispose();
    _githubRepositoryController.dispose();
    super.dispose();
  }

  Future<void> _loadSources() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final manifest = await widget.clients.providersMarketStatsApiService
          .getManifest();
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
        _categories = manifest.categories;
        _selectedCategoryId =
            _selectedCategoryId ??
            _publishContext?.initialEntry?.categoryId ??
            _defaultCategoryId(manifest.categories);
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
    final context = _publishContext;
    final initialEntry = context?.initialEntry;
    final lockedDisplayName = context?.lockedDisplayName.trim();
    setState(() {
      _selectedSource = source;
      _displayNameController.text =
          initialEntry?.title.trim().isNotEmpty == true
          ? initialEntry!.title
          : lockedDisplayName?.isNotEmpty == true
          ? lockedDisplayName!
          : source.displayName;
      _descriptionController.text =
          initialEntry?.description.trim().isNotEmpty == true
          ? initialEntry!.description
          : source.description;
      _detailController.text = initialEntry?.detail.trim().isNotEmpty == true
          ? initialEntry!.detail
          : source.description;
      _versionController.text =
          source.inferredVersion?.trim().isNotEmpty == true
          ? source.inferredVersion!.trim()
          : '1.0.0';
      _minVersionController.text = initialEntry?.latestVersion?.minAppVer ?? '';
      _maxVersionController.text = initialEntry?.latestVersion?.maxAppVer ?? '';
      _allowPublicUpdates = initialEntry?.allowPublicUpdates ?? true;
    });
  }

  /// Loads selectable releases from the author-maintained GitHub repository.
  Future<void> _loadGitHubReleaseCatalog() async {
    final repositoryUrl = _githubRepositoryController.text.trim();
    setState(() {
      _loadingGitHubReleaseCatalog = true;
      _githubReleaseCatalogError = null;
      _githubReleaseCatalog = null;
      _selectedGitHubReleaseTag = null;
      _selectedGitHubReleaseAssetName = null;
    });
    try {
      final catalog = await _loadGitHubReleaseCatalogFromRepository(
        clients: widget.clients,
        repositoryUrl: repositoryUrl,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _githubReleaseCatalog = catalog;
        _loadingGitHubReleaseCatalog = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load GitHub release catalog: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _githubReleaseCatalogError = error.toString();
        _loadingGitHubReleaseCatalog = false;
      });
    }
  }

  /// Resolves the selected resource source into the immutable publish reference.
  _PublishArtifactSource? _selectedPublishAssetSource() {
    if (_assetSource == _ArtifactPublishAssetSource.directUpload) {
      return _DirectUploadArtifactSource(minifyArtifact: _minifyArtifact);
    }
    final catalog = _githubReleaseCatalog;
    final selectedReleaseTag = _selectedGitHubReleaseTag;
    final selectedAssetName = _selectedGitHubReleaseAssetName;
    if (catalog == null ||
        selectedReleaseTag == null ||
        selectedAssetName == null) {
      return null;
    }
    return _GitHubReleaseArtifactSource(
      owner: catalog.repository.owner,
      repository: catalog.repository.repository,
      releaseTag: selectedReleaseTag,
      assetName: selectedAssetName,
    );
  }

  Future<void> _publish({required bool allowCreateForgeRepo}) async {
    final source = _selectedSource;
    if (source == null || _publishing) {
      return;
    }
    final publishAssetSource = _selectedPublishAssetSource();
    if (publishAssetSource == null) {
      setState(() {
        _errorMessage = 'GitHub Release とそのアセットを選択してください。';
      });
      return;
    }
    setState(() {
      _publishing = true;
      _errorMessage = null;
      _pendingMarketRegistration = null;
      _progressMessage = '公開情報を確認中';
    });
    try {
      final result = await _publishArtifact(
        clients: widget.clients,
        source: source,
        displayName: _displayNameController.text,
        description: _descriptionController.text,
        detail: _detailController.text,
        categoryId: _selectedCategoryId ?? '',
        allowPublicUpdates: _allowPublicUpdates,
        publishAssetSource: publishAssetSource,
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
          title: const Text('公開が完了しました'),
          content: SelectableText(
            '「${result.displayName}」を公開しました\n'
            'プロジェクト ID: ${result.projectId}\n'
            'Entry ID: ${result.entryId}\n'
            'Version ID: ${result.versionId}\n'
            'Release: ${result.releaseTag}\n\n'
            '公開マーケットでは公開の順番待ちが必要です。完了後に確認してください。',
          ),
          actions: <Widget>[
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('確認'),
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
          title: const Text('OperitForge を初期化'),
          content: Text(
            '公開アセットを保存するため、@${request.publisherLogin} に公開リポジトリ $_forgeRepoName を作成する必要があります。',
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('キャンセル'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('作成して続行'),
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
      _progressMessage = 'マーケットへの登録を再試行中';
    });
    try {
      final response = await _registerMarketEntry(
        clients: widget.clients,
        type: pending.type,
        title: pending.title,
        publishContext: _publishContext,
        payload: pending.payload,
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
          title: const Text('公開が完了しました'),
          content: SelectableText(
            '「${pending.result.displayName}」を登録しました\n'
            'プロジェクト ID: ${pending.result.projectId}\n'
            'Entry ID: ${response.entryId}\n'
            'Version ID: ${response.versionId}\n'
            'Release: ${pending.result.releaseTag}\n\n'
            '公開マーケットでは公開の順番待ちが必要です。完了後に確認してください。',
          ),
          actions: <Widget>[
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('確認'),
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

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    final source = _selectedSource;
    final publishContext = _publishContext;
    final isContinuationMode = publishContext != null;
    final lockedDisplayName = publishContext?.lockedDisplayName.trim() ?? '';
    final isDisplayNameLocked =
        lockedDisplayName.isNotEmpty && !(publishContext?.canEditEntry ?? true);
    final githubReleaseCatalog = _githubReleaseCatalog;
    final selectedGitHubRelease = _findGitHubRelease(
      githubReleaseCatalog?.releases ?? const <_GitHubReleaseInfo>[],
      _selectedGitHubReleaseTag,
    );
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: Text(isContinuationMode ? '更新バージョンを公開' : 'Artifact を公開'),
        actions: <Widget>[
          IconButton(
            onPressed: _loading || _publishing ? null : _loadSources,
            icon: const Icon(Icons.refresh),
            tooltip: '更新',
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
              title: '読み込みに失敗しました',
              message: error,
              action: TextButton.icon(
                onPressed: _loadSources,
                icon: const Icon(Icons.refresh),
                label: const Text('更新'),
              ),
            );
          }
          if (_sources.isEmpty) {
            return EmptyState(
              icon: Icons.inventory_2_outlined,
              title: isContinuationMode
                  ? '対応するローカル Artifact がありません'
                  : '公開できるローカル Artifact がありません',
              message: isContinuationMode
                  ? '現在は既存バージョンを基に公開しますが、同じランタイムパッケージがローカルに見つかりません。'
                  : '外部 JS/HJSON パッケージまたは ToolPkg をインストールしてから公開してください。',
              scrollable: false,
            );
          }
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 120),
            children: <Widget>[
              if (publishContext != null) ...<Widget>[
                _PublishContinuationPanel(contextInfo: publishContext),
                const SizedBox(height: 12),
              ],
              DropdownButtonFormField<String>(
                key: ValueKey<String?>(source?.packageName),
                initialValue: source?.packageName,
                style: OperitFormStyles.dropdownTextStyle(context),
                decoration: const InputDecoration(
                  labelText: 'ローカル Artifact',
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
              Text('公開アセットのソース', style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 8),
              SegmentedButton<_ArtifactPublishAssetSource>(
                segments: const <ButtonSegment<_ArtifactPublishAssetSource>>[
                  ButtonSegment<_ArtifactPublishAssetSource>(
                    value: _ArtifactPublishAssetSource.directUpload,
                    icon: Icon(Icons.cloud_upload_outlined),
                    label: Text('ローカルプラグインを直接公開'),
                  ),
                  ButtonSegment<_ArtifactPublishAssetSource>(
                    value: _ArtifactPublishAssetSource.githubReleaseAsset,
                    icon: Icon(Icons.link_outlined),
                    label: Text('GitHub Release アセットを参照'),
                  ),
                ],
                selected: <_ArtifactPublishAssetSource>{_assetSource},
                onSelectionChanged: _publishing
                    ? null
                    : (selected) {
                        setState(() {
                          _assetSource = selected.single;
                          _errorMessage = null;
                        });
                      },
              ),
              if (_assetSource ==
                  _ArtifactPublishAssetSource.githubReleaseAsset) ...<Widget>[
                const SizedBox(height: 12),
                TextField(
                  controller: _githubRepositoryController,
                  enabled: !_publishing,
                  keyboardType: TextInputType.url,
                  decoration: const InputDecoration(
                    labelText: 'GitHub リポジトリ URL',
                    hintText: 'https://github.com/owner/repository',
                    border: OutlineInputBorder(),
                  ),
                  onChanged: (_) {
                    setState(() {
                      _githubReleaseCatalog = null;
                      _githubReleaseCatalogError = null;
                      _selectedGitHubReleaseTag = null;
                      _selectedGitHubReleaseAssetName = null;
                    });
                  },
                ),
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  onPressed: _publishing || _loadingGitHubReleaseCatalog
                      ? null
                      : _loadGitHubReleaseCatalog,
                  icon: _loadingGitHubReleaseCatalog
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.refresh),
                  label: Text(
                    _loadingGitHubReleaseCatalog
                        ? 'Release を読み込み中'
                        : 'Release を読み込む',
                  ),
                ),
                if (_githubReleaseCatalogError != null) ...<Widget>[
                  const SizedBox(height: 8),
                  Text(
                    _githubReleaseCatalogError!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
                if (githubReleaseCatalog != null) ...<Widget>[
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    key: ValueKey<String?>(_selectedGitHubReleaseTag),
                    initialValue: _selectedGitHubReleaseTag,
                    isExpanded: true,
                    style: OperitFormStyles.dropdownTextStyle(context),
                    decoration: const InputDecoration(
                      labelText: 'GitHub Release',
                      border: OutlineInputBorder(),
                    ),
                    items: githubReleaseCatalog.releases
                        .map(
                          (release) => DropdownMenuItem<String>(
                            value: release.tagName,
                            child: Text(
                              release.name == null || release.name!.isEmpty
                                  ? release.tagName
                                  : '${release.name!} · ${release.tagName}',
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        )
                        .toList(growable: false),
                    onChanged: _publishing
                        ? null
                        : (value) {
                            setState(() {
                              _selectedGitHubReleaseTag = value;
                              _selectedGitHubReleaseAssetName = null;
                            });
                          },
                  ),
                ],
                if (selectedGitHubRelease != null) ...<Widget>[
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    key: ValueKey<String?>(_selectedGitHubReleaseAssetName),
                    initialValue: _selectedGitHubReleaseAssetName,
                    isExpanded: true,
                    style: OperitFormStyles.dropdownTextStyle(context),
                    decoration: const InputDecoration(
                      labelText: 'Release アセット',
                      border: OutlineInputBorder(),
                    ),
                    items: selectedGitHubRelease.assets
                        .map(
                          (asset) => DropdownMenuItem<String>(
                            value: asset.name,
                            child: Text(
                              asset.name,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        )
                        .toList(growable: false),
                    onChanged: _publishing
                        ? null
                        : (value) {
                            setState(() {
                              _selectedGitHubReleaseAssetName = value;
                            });
                          },
                  ),
                ],
              ],
              const SizedBox(height: 12),
              TextField(
                controller: _displayNameController,
                enabled: !_publishing && !isDisplayNameLocked,
                decoration: InputDecoration(
                  labelText: '表示名',
                  border: const OutlineInputBorder(),
                  helperText: !isDisplayNameLocked
                      ? null
                      : '既存バージョンを基に公開する場合、名前は元のバージョンから引き継がれます。',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _descriptionController,
                enabled: !_publishing,
                minLines: 2,
                maxLines: 4,
                decoration: const InputDecoration(
                  labelText: '概要',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _detailController,
                enabled: !_publishing,
                minLines: 5,
                maxLines: 12,
                decoration: const InputDecoration(
                  labelText: '詳細',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                initialValue: _selectedCategoryId,
                style: OperitFormStyles.dropdownTextStyle(context),
                decoration: const InputDecoration(
                  labelText: 'カテゴリ',
                  border: OutlineInputBorder(),
                ),
                items: _categories
                    .map(
                      (category) => DropdownMenuItem<String>(
                        value: category.id,
                        child: Text(category.name),
                      ),
                    )
                    .toList(growable: false),
                onChanged: _publishing
                    ? null
                    : (value) {
                        setState(() {
                          _selectedCategoryId = value;
                        });
                      },
              ),
              const SizedBox(height: 12),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                value: _allowPublicUpdates,
                onChanged: _publishing
                    ? null
                    : (value) {
                        setState(() {
                          _allowPublicUpdates = value;
                        });
                      },
                title: const Text('すべてのユーザーによる新バージョンの公開を許可'),
                subtitle: const Text(
                  '有効にすると、ログイン済みのユーザーがこのプラグインの新バージョンを提出できます。',
                ),
              ),
              const SizedBox(height: 12),
              if (_assetSource ==
                  _ArtifactPublishAssetSource.directUpload) ...<Widget>[
                const SizedBox(height: 12),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  value: _minifyArtifact,
                  onChanged: _publishing
                      ? null
                      : (value) {
                          setState(() {
                            _minifyArtifact = value;
                          });
                        },
                  title: const Text('プラグインスクリプトを圧縮・難読化'),
                  subtitle: const Text(
                    '有効にすると実行可能な JavaScript を圧縮します。生成物はそのままインポートできます。',
                  ),
                ),
              ],
              const SizedBox(height: 12),
              TextField(
                controller: _versionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: 'バージョン',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _minVersionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: '最低対応バージョン',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _maxVersionController,
                enabled: !_publishing,
                decoration: const InputDecoration(
                  labelText: '最高対応バージョン',
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
                    label: Text(
                      _retryingMarketRegistration ? '再試行中' : 'マーケット登録を再試行',
                    ),
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
                      ? '公開中'
                      : isContinuationMode
                      ? '更新バージョンを公開'
                      : 'マーケットに公開',
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
  const _PublishContinuationPanel({required this.contextInfo});

  final ArtifactPublishClusterContext contextInfo;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
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
            Text(
              '既存 Artifact の新バージョンを公開',
              style: textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'Entry ID: ${contextInfo.entryId}',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            if (contextInfo.lockedDisplayName.trim().isNotEmpty) ...<Widget>[
              const SizedBox(height: 4),
              Text(
                'プラグイン名は ${contextInfo.lockedDisplayName.trim()} を引き継ぎます',
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ],
        ),
      ),
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

abstract class _PublishArtifactSource {
  const _PublishArtifactSource();
}

class _DirectUploadArtifactSource extends _PublishArtifactSource {
  const _DirectUploadArtifactSource({required this.minifyArtifact});

  final bool minifyArtifact;
}

class _GitHubReleaseArtifactSource extends _PublishArtifactSource {
  const _GitHubReleaseArtifactSource({
    required this.owner,
    required this.repository,
    required this.releaseTag,
    required this.assetName,
  });

  final String owner;
  final String repository;
  final String releaseTag;
  final String assetName;
}

class _GitHubReleaseRepository {
  const _GitHubReleaseRepository({
    required this.owner,
    required this.repository,
  });

  final String owner;
  final String repository;
}

class _GitHubReleaseCatalog {
  const _GitHubReleaseCatalog({
    required this.repository,
    required this.releases,
  });

  final _GitHubReleaseRepository repository;
  final List<_GitHubReleaseInfo> releases;
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
  const _GitHubReleaseInfo({
    required this.id,
    required this.tagName,
    required this.name,
    required this.assets,
  });

  final int id;
  final String tagName;
  final String? name;
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

class _ResolvedReleaseAsset {
  const _ResolvedReleaseAsset({
    required this.owner,
    required this.repository,
    required this.releaseTag,
    required this.assetName,
    required this.downloadUrl,
    required this.sha256,
  });

  final String owner;
  final String repository;
  final String releaseTag;
  final String assetName;
  final String downloadUrl;
  final String sha256;
}

class _PublishResult {
  const _PublishResult({
    required this.displayName,
    required this.projectId,
    required this.entryId,
    required this.versionId,
    required this.releaseTag,
  });

  final String displayName;
  final String projectId;
  final String entryId;
  final String versionId;
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
  return clients.application.packageManager().getPublishablePackageSources();
}

/// Loads an author repository's releases and assets for market registration.
Future<_GitHubReleaseCatalog> _loadGitHubReleaseCatalogFromRepository({
  required GeneratedCoreProxyClients clients,
  required String repositoryUrl,
}) async {
  final repository = _parseGitHubReleaseRepositoryUrl(repositoryUrl);
  final response = await _githubJsonRequest(
    clients: clients,
    method: 'GET',
    uri: Uri.https(
      'api.github.com',
      '/repos/${repository.owner}/${repository.repository}/releases',
      <String, String>{'per_page': '100'},
    ),
  );
  if (response is! List<Object?>) {
    throw StateError('GitHub Release response is invalid.');
  }
  return _GitHubReleaseCatalog(
    repository: repository,
    releases: response
        .map((item) => _releaseFromJson(item as Map<String, Object?>))
        .toList(growable: false),
  );
}

/// Reloads one selected release before registering its immutable asset.
Future<_GitHubReleaseInfo> _loadGitHubReleaseByTag({
  required GeneratedCoreProxyClients clients,
  required String owner,
  required String repository,
  required String tagName,
}) async {
  final response = await _githubJsonRequest(
    clients: clients,
    method: 'GET',
    uri: Uri.https(
      'api.github.com',
      '/repos/$owner/$repository/releases/tags/$tagName',
    ),
  );
  return _releaseFromJson(response as Map<String, Object?>);
}

/// Downloads a selected GitHub asset so its digest can be compared to the local file.
Future<Uint8List> _downloadGitHubReleaseAsset({
  required GeneratedCoreProxyClients clients,
  required _GitHubReleaseAssetInfo asset,
}) async {
  final response = await _githubHttpRequest(
    clients: clients,
    method: 'GET',
    uri: Uri.parse(asset.browserDownloadUrl),
  );
  if (!_isSuccess(response.statusCode)) {
    throw StateError(
      'HTTP ${response.statusCode}: ${_summarizeHttpBody(response.body)}',
    );
  }
  return Uint8List.fromList(response.bodyBytes);
}

/// Parses a canonical GitHub repository URL entered by the publisher.
_GitHubReleaseRepository _parseGitHubReleaseRepositoryUrl(
  String repositoryUrl,
) {
  final uri = Uri.tryParse(repositoryUrl.trim());
  if (uri == null || uri.scheme.toLowerCase() != 'https') {
    throw StateError('GitHub リポジトリ URL は HTTPS を使用する必要があります。');
  }
  if (uri.host.toLowerCase() != 'github.com' &&
      uri.host.toLowerCase() != 'www.github.com') {
    throw StateError('GitHub リポジトリ URL は github.com を指す必要があります。');
  }
  final segments = uri.pathSegments
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  if (segments.length < 2) {
    throw StateError('GitHub リポジトリ URL には owner と repository が必要です。');
  }
  final owner = segments[0];
  final repository = segments[1].endsWith('.git')
      ? segments[1].substring(0, segments[1].length - 4)
      : segments[1];
  final segmentPattern = RegExp(r'^[A-Za-z0-9_.-]+$');
  if (!segmentPattern.hasMatch(owner) || !segmentPattern.hasMatch(repository)) {
    throw StateError('GitHub リポジトリ URL に無効な owner または repository が含まれています。');
  }
  return _GitHubReleaseRepository(owner: owner, repository: repository);
}

/// Finds the release selected in the loaded repository catalog.
_GitHubReleaseInfo? _findGitHubRelease(
  List<_GitHubReleaseInfo> releases,
  String? tagName,
) {
  if (tagName == null) {
    return null;
  }
  for (final release in releases) {
    if (release.tagName == tagName) {
      return release;
    }
  }
  return null;
}

/// Finds the asset selected from the loaded GitHub release.
_GitHubReleaseAssetInfo? _findGitHubReleaseAsset(
  List<_GitHubReleaseAssetInfo> assets,
  String? assetName,
) {
  if (assetName == null) {
    return null;
  }
  for (final asset in assets) {
    if (asset.name == assetName) {
      return asset;
    }
  }
  return null;
}

/// Requires the named asset to remain present in the release reloaded for publishing.
_GitHubReleaseAssetInfo _requireGitHubReleaseAsset({
  required _GitHubReleaseInfo release,
  required String assetName,
}) {
  final asset = _findGitHubReleaseAsset(release.assets, assetName);
  if (asset == null) {
    throw StateError('選択した GitHub Release アセットが存在しません。');
  }
  return asset;
}

/// Validates local publish metadata, resolves the selected resource, and registers it in the market.
Future<_PublishResult> _publishArtifact({
  required GeneratedCoreProxyClients clients,
  required core_proxy.PublishablePackageSource source,
  required String displayName,
  required String description,
  required String detail,
  required String categoryId,
  required bool allowPublicUpdates,
  required _PublishArtifactSource publishAssetSource,
  required String version,
  required String minSupportedAppVersion,
  required String maxSupportedAppVersion,
  required ArtifactPublishClusterContext? publishContext,
  required bool allowCreateForgeRepo,
  required ValueChanged<String> onProgress,
}) async {
  final trimmedDisplayName = displayName.trim();
  final trimmedDescription = description.trim();
  final trimmedDetail = detail.trim();
  final trimmedCategoryId = categoryId.trim();
  if (trimmedDisplayName.isEmpty) {
    throw StateError('プラグイン名は必須です');
  }
  if (trimmedDescription.isEmpty) {
    throw StateError('概要は必須です');
  }
  if (trimmedDetail.isEmpty) {
    throw StateError('詳細は必須です');
  }
  if (trimmedCategoryId.isEmpty) {
    throw StateError('カテゴリは必須です');
  }
  final cleanVersion = _normalizeArtifactVersion(version);
  final normalizedMinVersion = _normalizeAppVersionOrNull(
    minSupportedAppVersion,
  );
  final normalizedMaxVersion = _normalizeAppVersionOrNull(
    maxSupportedAppVersion,
  );
  _validateAppVersionRange(normalizedMinVersion, normalizedMaxVersion);

  onProgress('GitHub アカウントを読み込み中');
  final currentUser = await clients.providersMarketStatsApiService
      .getCurrentGithubUser();
  final normalizedRuntimePackageId = _normalizeMarketArtifactId(
    source.packageName,
  );
  if (publishContext == null) {
    _validateStandaloneArtifactRuntimePackageId(source.packageName);
    onProgress('名前と ID を確認中');
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
  }

  final resolvedDisplayName =
      publishContext?.lockedDisplayName.trim().isNotEmpty == true &&
          publishContext?.canEditEntry != true
      ? publishContext!.lockedDisplayName.trim()
      : trimmedDisplayName;
  final projectId = publishContext?.projectId.trim().isNotEmpty == true
      ? _normalizeMarketArtifactId(publishContext!.projectId)
      : normalizedRuntimePackageId;
  final extension = source.fileExtension.trim().isEmpty
      ? 'bin'
      : source.fileExtension.trim();
  late final _ResolvedReleaseAsset resolvedAsset;
  if (publishAssetSource is _DirectUploadArtifactSource) {
    onProgress('OperitForge を準備中');
    final forgeRepo = await _ensureForgeRepository(
      clients: clients,
      publisherLogin: currentUser.login,
      allowCreateForgeRepo: allowCreateForgeRepo,
    );
    final assetName = '$normalizedRuntimePackageId-v$cleanVersion.$extension';
    final releaseTag =
        '${_artifactReleaseTagPrefix(source.type)}-$normalizedRuntimePackageId-v$cleanVersion';
    onProgress('Release を作成中');
    final release = await _createOrUpdateRelease(
      clients: clients,
      owner: currentUser.login,
      repo: forgeRepo.repoName,
      tagName: releaseTag,
      name: '$resolvedDisplayName v$cleanVersion',
      body: _buildReleaseBody(
        type: source.type,
        projectId: projectId,
        runtimePackageId: source.packageName,
        displayName: resolvedDisplayName,
        version: cleanVersion,
        minSupportedAppVersion: normalizedMinVersion,
        maxSupportedAppVersion: normalizedMaxVersion,
      ),
    );
    onProgress(
      publishAssetSource.minifyArtifact ? 'プラグインスクリプトを圧縮・処理中' : 'プラグインアセットを処理中',
    );
    final Uint8List fileBytes = await clients.application
        .packageManager()
        .protectArtifactFile(
          sourcePath: source.sourcePath,
          isToolPkg: source.isToolPkg,
          packageId: source.packageName,
          version: cleanVersion,
          author: <String>[currentUser.login],
          minifyArtifact: publishAssetSource.minifyArtifact,
        );
    onProgress('アセットをアップロード中');
    final asset = await _uploadReleaseAsset(
      clients: clients,
      owner: currentUser.login,
      repo: forgeRepo.repoName,
      release: release,
      assetName: assetName,
      contentType: _artifactContentType(source.type, extension),
      content: fileBytes,
    );
    resolvedAsset = _ResolvedReleaseAsset(
      owner: currentUser.login,
      repository: forgeRepo.repoName,
      releaseTag: releaseTag,
      assetName: asset.name,
      downloadUrl: asset.browserDownloadUrl,
      sha256: crypto.sha256.convert(fileBytes).toString(),
    );
  } else if (publishAssetSource is _GitHubReleaseArtifactSource) {
    onProgress('GitHub Release アセットを照合中');
    final release = await _loadGitHubReleaseByTag(
      clients: clients,
      owner: publishAssetSource.owner,
      repository: publishAssetSource.repository,
      tagName: publishAssetSource.releaseTag,
    );
    final asset = _requireGitHubReleaseAsset(
      release: release,
      assetName: publishAssetSource.assetName,
    );
    final remoteBytes = await _downloadGitHubReleaseAsset(
      clients: clients,
      asset: asset,
    );
    final localBytes = await XFile(source.sourcePath).readAsBytes();
    final remoteSha256 = crypto.sha256.convert(remoteBytes).toString();
    if (remoteSha256 != crypto.sha256.convert(localBytes).toString()) {
      throw StateError('選択した GitHub Release アセットがローカルのプラグインファイルと一致しません。');
    }
    resolvedAsset = _ResolvedReleaseAsset(
      owner: publishAssetSource.owner,
      repository: publishAssetSource.repository,
      releaseTag: release.tagName,
      assetName: asset.name,
      downloadUrl: asset.browserDownloadUrl,
      sha256: remoteSha256,
    );
  } else {
    throw StateError('不明な公開アセットのソースです。');
  }

  final payload = <String, Object?>{
    'projectId': projectId,
    'detail': trimmedDetail,
    'categoryId': trimmedCategoryId,
    'allowPublicUpdates': allowPublicUpdates,
    'runtimePackageId': source.packageName,
    'publisherLogin': currentUser.login,
    'releaseOwner': resolvedAsset.owner,
    'releaseRepository': resolvedAsset.repository,
    'releaseTag': resolvedAsset.releaseTag,
    'assetName': resolvedAsset.assetName,
    'downloadUrl': resolvedAsset.downloadUrl,
    'sha256': resolvedAsset.sha256,
    'version': cleanVersion,
    'displayName': resolvedDisplayName,
    'description': trimmedDescription,
    'sourceFileName': source.sourceFileName,
    'minSupportedAppVersion': normalizedMinVersion,
    'maxSupportedAppVersion': normalizedMaxVersion,
  };

  onProgress('マーケットに登録中');
  final result = _PublishResult(
    displayName: resolvedDisplayName,
    projectId: projectId,
    entryId: '',
    versionId: '',
    releaseTag: resolvedAsset.releaseTag,
  );
  try {
    final response = await _registerMarketEntry(
      clients: clients,
      type: source.type,
      title: resolvedDisplayName,
      publishContext: publishContext,
      payload: payload,
    );
    return _PublishResult(
      displayName: resolvedDisplayName,
      projectId: projectId,
      entryId: response.entryId,
      versionId: response.versionId,
      releaseTag: resolvedAsset.releaseTag,
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

/// Registers the resolved GitHub release asset as a new entry or version.
Future<core_proxy.MarketPublishResponse> _registerMarketEntry({
  required GeneratedCoreProxyClients clients,
  required String type,
  required String title,
  required ArtifactPublishClusterContext? publishContext,
  required Map<String, Object?> payload,
}) async {
  final owner = payload['releaseOwner']?.toString() ?? '';
  final repo = payload['releaseRepository']?.toString() ?? '';
  final releaseTag = payload['releaseTag']?.toString() ?? '';
  final assetName = payload['assetName']?.toString() ?? '';
  final sha256 = payload['sha256']?.toString() ?? '';
  final description = payload['description']?.toString() ?? '';
  final detail = payload['detail']?.toString() ?? '';
  final categoryId = payload['categoryId']?.toString() ?? '';
  final version = payload['version']?.toString() ?? '';
  final minAppVer = payload['minSupportedAppVersion']?.toString() ?? '';
  final maxAppVer = _emptyToNull(payload['maxSupportedAppVersion']?.toString());
  final projectId = payload['projectId']?.toString() ?? '';
  final runtimePackageId = payload['runtimePackageId']?.toString() ?? '';
  final assetUrl = payload['downloadUrl']?.toString() ?? '';
  if (publishContext != null) {
    final canPatchEntry = publishContext.canEditEntry;
    return clients.providersMarketStatsApiService.publishArtifactVersion(
      entryId: publishContext.entryId,
      version: version,
      formatVer: _artifactMarketFormatVersion(type),
      minAppVer: minAppVer,
      maxAppVer: maxAppVer,
      changelog: null,
      projectId: projectId,
      runtimePackageId: runtimePackageId,
      assetKind: 'github_release_asset',
      assetUrl: assetUrl,
      ghOwner: owner,
      ghRepo: repo,
      ghReleaseTag: releaseTag,
      assetName: assetName,
      sha256: sha256,
      entryTitle: canPatchEntry ? title : null,
      entryDescription: canPatchEntry ? description : null,
      entryDetail: canPatchEntry ? detail : null,
      entryCategoryId: canPatchEntry ? categoryId : null,
      entryAllowPublicUpdates: canPatchEntry
          ? payload['allowPublicUpdates'] == true
          : null,
    );
  }
  return clients.providersMarketStatsApiService.publishArtifact(
    type: type,
    title: title,
    description: description,
    detail: detail,
    categoryId: categoryId,
    allowPublicUpdates: payload['allowPublicUpdates'] == true,
    version: version,
    formatVer: _artifactMarketFormatVersion(type),
    minAppVer: minAppVer,
    maxAppVer: maxAppVer,
    changelog: null,
    projectId: projectId,
    runtimePackageId: runtimePackageId,
    assetKind: 'github_release_asset',
    assetUrl: assetUrl,
    ghOwner: owner,
    ghRepo: repo,
    ghReleaseTag: releaseTag,
    assetName: assetName,
    sha256: sha256,
  );
}

Future<void> _ensureFreshPublishIdentityAvailable({
  required GeneratedCoreProxyClients clients,
  required String displayName,
  required String runtimePackageId,
  required String normalizedRuntimePackageId,
}) async {
  final normalizedTitle = _normalizePublishTitle(displayName);
  for (final type in _artifactMarketTypes) {
    final entries = await _loadPublishedEntriesByType(clients, type);
    final titleConflict = entries.any(
      (entry) => _normalizePublishTitle(entry.title) == normalizedTitle,
    );
    if (titleConflict) {
      throw StateError('名前「$displayName」はすでに存在します。');
    }
    final runtimeConflict = entries.any((entry) {
      final existingRuntimePackageId = entry.artifact?.runtimePackageId ?? '';
      final existingProjectId = entry.artifact?.projectId ?? entry.id;
      return _sameArtifactRuntimePackageId(
            existingRuntimePackageId,
            runtimePackageId,
          ) ||
          _normalizeMarketArtifactId(existingProjectId) ==
              normalizedRuntimePackageId;
    });
    if (runtimeConflict) {
      throw StateError('ID「$runtimePackageId」はすでに存在します。');
    }
  }
}

Future<List<core_proxy.MarketEntrySummary>> _loadPublishedEntriesByType(
  GeneratedCoreProxyClients clients,
  String type,
) async {
  final market = clients.providersMarketStatsApiService;
  final entries = <core_proxy.MarketEntrySummary>[];
  var page = 1;
  while (true) {
    final list = await market.getTypePage(
      type: type,
      sort: 'updated',
      page: page,
    );
    entries.addAll(list.items);
    if (page >= _marketPageCount(list.total, list.pageSize)) {
      break;
    }
    page += 1;
  }
  return entries;
}

int _marketPageCount(int total, int pageSize) {
  final size = pageSize <= 0 ? 100 : pageSize;
  final count = (total + size - 1) ~/ size;
  return count <= 0 ? 1 : count;
}

String? _defaultCategoryId(List<core_proxy.MarketCategoryInfo> categories) {
  return categories.isEmpty ? null : categories.first.id;
}

String? _emptyToNull(String? value) {
  final trimmed = value?.trim() ?? '';
  return trimmed.isEmpty ? null : trimmed;
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

/// Converts GitHub's release JSON response into the publish-screen model.
_GitHubReleaseInfo _releaseFromJson(Map<String, Object?> json) {
  return _GitHubReleaseInfo(
    id: json['id'] as int,
    tagName: json['tag_name'] as String,
    name: json['name'] as String?,
    assets: (json['assets'] as List<Object?>)
        .map(
          (item) =>
              _GitHubReleaseAssetInfo.fromJson(item as Map<String, Object?>),
        )
        .toList(growable: false),
  );
}

/// Builds the ordinary release notes for assets uploaded through OperitForge.
String _buildReleaseBody({
  required String type,
  required String projectId,
  required String runtimePackageId,
  required String displayName,
  required String version,
  required String? minSupportedAppVersion,
  required String? maxSupportedAppVersion,
}) {
  final lines = <String>[
    '${_artifactTitleLabel(type)} artifact published by OperitForge.',
    '',
    'Project ID: $projectId',
    'Runtime package ID: $runtimePackageId',
    'Display name: $displayName',
    'Version: $version',
    'Supported app versions: ${_formatSupportedAppVersions(minSupportedAppVersion, maxSupportedAppVersion)}',
    '',
  ];
  return lines.join('\n');
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
    throw StateError('バージョン形式は 1.2.3 または 1.2.3+4 にしてください');
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
    throw StateError('最低対応バージョンを最高対応バージョンより大きくすることはできません');
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
    throw StateError('バージョン形式は 1.2.3 または 1.2.3+4 にしてください');
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
    throw StateError(
      '現在のパッケージ ID「$runtimePackageId」から安定したマーケットプロジェクト ID を生成できません。',
    );
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

/// Returns the marketplace format version required for each artifact type.
String _artifactMarketFormatVersion(String type) {
  return type == 'package' ? 'toolpkg_v2' : 'script_v2';
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
  return '未指定';
}
