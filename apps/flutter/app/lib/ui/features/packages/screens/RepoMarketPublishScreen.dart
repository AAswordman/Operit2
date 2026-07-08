// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitFormStyles.dart';
import '../components/EmptyState.dart';

class RepoMarketPublishContext {
  const RepoMarketPublishContext({
    required this.entry,
    required this.canEditEntry,
  });

  final core_proxy.MarketEntrySummary entry;
  final bool canEditEntry;
}

class RepoMarketPublishScreen extends StatefulWidget {
  const RepoMarketPublishScreen({
    super.key,
    required this.type,
    this.publishContext,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final String type;
  final RepoMarketPublishContext? publishContext;
  final GeneratedCoreProxyClients clients;

  @override
  State<RepoMarketPublishScreen> createState() =>
      _RepoMarketPublishScreenState();
}

class _RepoMarketPublishScreenState extends State<RepoMarketPublishScreen> {
  final TextEditingController _titleController = TextEditingController();
  final TextEditingController _descriptionController = TextEditingController();
  final TextEditingController _detailController = TextEditingController();
  final TextEditingController _sourceUrlController = TextEditingController();
  final TextEditingController _refNameController =
      TextEditingController(text: 'main');
  final TextEditingController _installConfigController =
      TextEditingController();
  final TextEditingController _versionController =
      TextEditingController(text: '1.0.0');
  final TextEditingController _formatVerController = TextEditingController();
  final TextEditingController _minAppVerController = TextEditingController();
  final TextEditingController _maxAppVerController = TextEditingController();
  final TextEditingController _changelogController = TextEditingController();

  bool _loading = true;
  bool _publishing = false;
  bool _allowPublicUpdates = true;
  String _refType = 'branch';
  String? _categoryId;
  String? _errorMessage;
  List<core_proxy.MarketCategoryInfo> _categories =
      <core_proxy.MarketCategoryInfo>[];

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  bool get _isContinuationMode => widget.publishContext != null;

  bool get _canEditEntry => widget.publishContext?.canEditEntry ?? true;

  @override
  void initState() {
    super.initState();
    final context = widget.publishContext;
    if (context == null) {
      _formatVerController.text = '${widget.type}_v2';
    } else {
      _seedFromEntry(context.entry);
    }
    _loadManifest();
  }

  @override
  void dispose() {
    _titleController.dispose();
    _descriptionController.dispose();
    _detailController.dispose();
    _sourceUrlController.dispose();
    _refNameController.dispose();
    _installConfigController.dispose();
    _versionController.dispose();
    _formatVerController.dispose();
    _minAppVerController.dispose();
    _maxAppVerController.dispose();
    _changelogController.dispose();
    super.dispose();
  }

  void _seedFromEntry(core_proxy.MarketEntrySummary entry) {
    final repoVersion = entry.repoVersion;
    final latestVersion = entry.latestVersion;
    _titleController.text = entry.title;
    _descriptionController.text = entry.description;
    _detailController.text = entry.detail;
    _sourceUrlController.text = entry.source?.url.trim() ?? '';
    _refType = repoVersion?.refType.trim().isNotEmpty == true
        ? repoVersion!.refType.trim()
        : 'branch';
    _refNameController.text = repoVersion?.refName.trim().isNotEmpty == true
        ? repoVersion!.refName.trim()
        : 'main';
    _installConfigController.text =
        repoVersion?.installConfig ??
        latestVersion?.installConfig ??
        '';
    _versionController.clear();
    _formatVerController.text = latestVersion?.formatVer.trim().isNotEmpty == true
        ? latestVersion!.formatVer.trim()
        : '${entry.type}_v2';
    _minAppVerController.text = latestVersion?.minAppVer ?? '';
    _maxAppVerController.text = latestVersion?.maxAppVer ?? '';
    _changelogController.clear();
    _allowPublicUpdates = entry.allowPublicUpdates;
    _categoryId = entry.categoryId;
  }

  Future<void> _loadManifest() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final manifest = await _market.getManifest();
      if (!mounted) {
        return;
      }
      setState(() {
        _categories = manifest.categories;
        _categoryId ??=
            manifest.categories.isEmpty ? null : manifest.categories.first.id;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market manifest: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _publish() async {
    if (_publishing) {
      return;
    }
    final title = _titleController.text.trim();
    final description = _descriptionController.text.trim();
    final detail = _detailController.text.trim();
    final categoryId = _categoryId?.trim() ?? '';
    final sourceUrl = _sourceUrlController.text.trim();
    final refName = _refNameController.text.trim();
    final installConfig = _installConfigController.text.trim();
    final version = _versionController.text.trim();
    final formatVer = _formatVerController.text.trim();
    final minAppVer = _minAppVerController.text.trim();
    final maxAppVer = _emptyToNull(_maxAppVerController.text);
    final changelog = _emptyToNull(_changelogController.text);
    final missing = <String>[
      if (_canEditEntry && title.isEmpty) '名称',
      if (_canEditEntry && description.isEmpty) '简介',
      if (_canEditEntry && detail.isEmpty) '详情',
      if (_canEditEntry && categoryId.isEmpty) '分类',
      if (!_isContinuationMode && sourceUrl.isEmpty) 'GitHub 地址',
      if (refName.isEmpty) '引用名称',
      if (installConfig.isEmpty) '安装配置',
      if (version.isEmpty) '版本号',
      if (formatVer.isEmpty) '格式版本',
      if (minAppVer.isEmpty) '最低支持版本',
    ];
    if (missing.isNotEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('请填写：${missing.join('、')}'),
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    setState(() {
      _publishing = true;
    });
    try {
      final response = _isContinuationMode
          ? await _market.publishRepoVersion(
              entryId: widget.publishContext!.entry.id,
              version: version,
              formatVer: formatVer,
              minAppVer: minAppVer,
              maxAppVer: maxAppVer,
              changelog: changelog,
              refType: _refType,
              refName: refName,
              installConfig: installConfig,
              entryTitle: _canEditEntry ? title : null,
              entryDescription: _canEditEntry ? description : null,
              entryDetail: _canEditEntry ? detail : null,
              entryCategoryId: _canEditEntry ? categoryId : null,
              entryAllowPublicUpdates:
                  _canEditEntry ? _allowPublicUpdates : null,
            )
          : await _market.publishRepoEntry(
              type: widget.type,
              title: title,
              description: description,
              detail: detail,
              categoryId: categoryId,
              allowPublicUpdates: _allowPublicUpdates,
              sourceUrl: sourceUrl,
              refType: _refType,
              refName: refName,
              installConfig: installConfig,
              version: version,
              formatVer: formatVer,
              minAppVer: minAppVer,
              maxAppVer: maxAppVer,
              changelog: changelog,
            );
      if (!mounted) {
        return;
      }
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(_isContinuationMode ? '新版本已提交' : '发布已提交'),
          content: SelectableText(
            'Entry ID: ${response.entryId}\n'
            'Version ID: ${response.versionId}\n\n'
            '审核通过后会进入公开市场。',
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('知道了'),
            ),
          ],
        ),
      );
      if (mounted) {
        Navigator.of(context).pop();
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to publish ${widget.type}: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } finally {
      if (mounted) {
        setState(() {
          _publishing = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    final entryFieldsEnabled = !_publishing && _canEditEntry;
    final versionFieldsEnabled = !_publishing;
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: Text(
          _isContinuationMode
              ? '发布 ${_typeLabel(widget.type)} 新版本'
              : '发布 ${_typeLabel(widget.type)}',
        ),
      ),
      body: Builder(
        builder: (context) {
          if (_loading) {
            return const M3LoadingPane();
          }
          if (error != null) {
            return EmptyState(
              icon: Icons.error_outline,
              title: '加载失败',
              message: error,
              action: TextButton.icon(
                onPressed: _loadManifest,
                icon: const Icon(Icons.refresh),
                label: const Text('刷新'),
              ),
            );
          }
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 120),
            children: <Widget>[
              if (_isContinuationMode)
                _PublishModeNotice(canEditEntry: _canEditEntry),
              if (_isContinuationMode) const SizedBox(height: 12),
              TextField(
                controller: _titleController,
                enabled: entryFieldsEnabled,
                decoration: const InputDecoration(
                  labelText: '名称',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _descriptionController,
                enabled: entryFieldsEnabled,
                minLines: 2,
                maxLines: 4,
                decoration: const InputDecoration(
                  labelText: '简介',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _detailController,
                enabled: entryFieldsEnabled,
                minLines: 5,
                maxLines: 12,
                decoration: const InputDecoration(
                  labelText: '详情',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                initialValue: _categoryId,
                style: OperitFormStyles.dropdownTextStyle(context),
                decoration: const InputDecoration(
                  labelText: '分类',
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
                onChanged: entryFieldsEnabled
                    ? (value) => setState(() => _categoryId = value)
                    : null,
              ),
              const SizedBox(height: 12),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                value: _allowPublicUpdates,
                onChanged: entryFieldsEnabled
                    ? (value) => setState(() => _allowPublicUpdates = value)
                    : null,
                title: const Text('允许所有人发布新版本'),
                subtitle: const Text('开启后，登录用户可为该插件提交新版本。'),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _sourceUrlController,
                enabled: !_publishing && !_isContinuationMode,
                decoration: const InputDecoration(
                  labelText: 'GitHub 地址',
                  hintText: 'https://github.com/owner/repo/tree/main/path',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                initialValue: _refType,
                style: OperitFormStyles.dropdownTextStyle(context),
                decoration: const InputDecoration(
                  labelText: '引用类型',
                  border: OutlineInputBorder(),
                ),
                items: const <DropdownMenuItem<String>>[
                  DropdownMenuItem(value: 'branch', child: Text('branch')),
                  DropdownMenuItem(value: 'tag', child: Text('tag')),
                ],
                onChanged: versionFieldsEnabled
                    ? (value) {
                        if (value != null) {
                          setState(() => _refType = value);
                        }
                      }
                    : null,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _refNameController,
                enabled: versionFieldsEnabled,
                decoration: const InputDecoration(
                  labelText: '引用名称',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _installConfigController,
                enabled: versionFieldsEnabled,
                minLines: 5,
                maxLines: 12,
                decoration: const InputDecoration(
                  labelText: '安装配置 JSON',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _versionController,
                enabled: versionFieldsEnabled,
                decoration: InputDecoration(
                  labelText: _isContinuationMode ? '新版本号' : '版本号',
                  border: const OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _formatVerController,
                enabled: versionFieldsEnabled,
                decoration: const InputDecoration(
                  labelText: '格式版本',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _minAppVerController,
                enabled: versionFieldsEnabled,
                decoration: const InputDecoration(
                  labelText: '最低支持版本',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _maxAppVerController,
                enabled: versionFieldsEnabled,
                decoration: const InputDecoration(
                  labelText: '最高支持版本（可选）',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _changelogController,
                enabled: versionFieldsEnabled,
                minLines: 2,
                maxLines: 5,
                decoration: const InputDecoration(
                  labelText: '更新说明（可选）',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 18),
              FilledButton.icon(
                onPressed: _publishing ? null : _publish,
                icon: _publishing
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.cloud_upload_outlined),
                label: Text(
                  _publishing
                      ? '提交中'
                      : _isContinuationMode
                      ? '提交新版本'
                      : '提交发布',
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _PublishModeNotice extends StatelessWidget {
  const _PublishModeNotice({required this.canEditEntry});

  final bool canEditEntry;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.55),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(
              canEditEntry
                  ? Icons.admin_panel_settings_outlined
                  : Icons.group_add_outlined,
              color: colorScheme.primary,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                canEditEntry
                    ? '你是最初发布者，本次提交可同时更新名称、简介、详情、分类和公开协作开关。'
                    : '你将作为贡献者提交版本内容，不能修改该条目的简介、详情、分类或协作开关。',
              ),
            ),
          ],
        ),
      ),
    );
  }
}

String _typeLabel(String type) {
  return switch (type) {
    'skill' => 'Skill',
    'mcp' => 'MCP',
    final value => value,
  };
}

String? _emptyToNull(String value) {
  final trimmed = value.trim();
  return trimmed.isEmpty ? null : trimmed;
}
