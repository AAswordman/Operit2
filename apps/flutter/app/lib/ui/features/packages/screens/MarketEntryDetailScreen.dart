// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../market/UnifiedMarketDetailScreen.dart';
import '../market/ArtifactMarketSupport.dart';
import 'ArtifactProjectNodeTreeDialog.dart';
import 'ArtifactPublishScreen.dart';
import 'RepoMarketPublishScreen.dart';

class MarketEntryDetailScreen extends StatefulWidget {
  const MarketEntryDetailScreen({
    super.key,
    required this.clients,
    required this.entry,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.MarketEntrySummary entry;

  @override
  State<MarketEntryDetailScreen> createState() => _MarketEntryDetailScreenState();
}

class _MarketEntryDetailScreenState extends State<MarketEntryDetailScreen> {
  final TextEditingController _commentController = TextEditingController();
  String? _currentGithubLogin;
  bool _communityLoading = true;
  bool _postingComment = false;
  bool _reacting = false;
  bool _installing = false;
  bool _openingPublish = false;
  String? _communityError;
  List<core_proxy.MarketComment> _comments = <core_proxy.MarketComment>[];
  List<core_proxy.MarketReactionCount> _reactions = <core_proxy.MarketReactionCount>[];

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _reactions = widget.entry.reactions;
    _loadCommunity();
    _loadCurrentGithubLogin();
  }

  @override
  void dispose() {
    _commentController.dispose();
    super.dispose();
  }

  Future<void> _loadCommunity() async {
    setState(() {
      _communityLoading = true;
      _communityError = null;
    });
    try {
      final page = await _market.getCommentsPage(entryId: widget.entry.id, page: 1);
      if (!mounted) return;
      setState(() {
        _comments = page.items;
        _communityLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market community: $error\n$stackTrace');
      if (!mounted) return;
      setState(() {
        _communityError = error.toString();
        _communityLoading = false;
      });
    }
  }

  Future<void> _loadCurrentGithubLogin() async {
    try {
      final currentUser = await _market.getCurrentGithubUser();
      if (!mounted) return;
      setState(() {
        _currentGithubLogin = currentUser.login.trim().toLowerCase();
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load current GitHub user: $error\n$stackTrace');
    }
  }

  Future<bool> _postComment() async {
    final body = _commentController.text.trim();
    if (body.isEmpty || _postingComment) return false;
    setState(() => _postingComment = true);
    try {
      await _market.createEntryComment(entryId: widget.entry.id, body: body);
      final page = await _market.getCommentsPage(entryId: widget.entry.id, page: 1);
      if (!mounted) return false;
      setState(() {
        _comments = page.items;
        _commentController.clear();
        _postingComment = false;
      });
      return true;
    } catch (error, stackTrace) {
      debugPrint('Failed to post market comment: $error\n$stackTrace');
      if (!mounted) return false;
      setState(() => _postingComment = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error.toString()), behavior: SnackBarBehavior.floating),
      );
      return false;
    }
  }

  Future<void> _react() async {
    if (_reacting) return;
    setState(() => _reacting = true);
    try {
      await _market.createEntryReaction(entryId: widget.entry.id);
      if (!mounted) return;
      setState(() {
        _reactions = _withIncrementedReaction(_reactions, '+1');
        _reacting = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to react market entry: $error\n$stackTrace');
      if (!mounted) return;
      setState(() => _reacting = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error.toString()), behavior: SnackBarBehavior.floating),
      );
    }
  }

  void _showCommentDialog() {
    _commentController.clear();
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        var posting = false;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final canPost = _commentController.text.trim().isNotEmpty && !posting;
            return AlertDialog(
              title: const Text('发表评论'),
              content: TextField(
                controller: _commentController,
                enabled: !posting,
                minLines: 4,
                maxLines: 8,
                autofocus: true,
                onChanged: (_) => setDialogState(() {}),
                decoration: const InputDecoration(
                  hintText: '写下你的评论',
                  border: OutlineInputBorder(),
                ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: posting ? null : () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                TextButton(
                  onPressed: canPost
                      ? () async {
                          setDialogState(() => posting = true);
                          final ok = await _postComment();
                          if (!dialogContext.mounted) return;
                          if (ok) Navigator.of(dialogContext).pop();
                          setDialogState(() => posting = false);
                        }
                      : null,
                  child: posting
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('发布'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  void _showEditCommentDialog(core_proxy.MarketComment comment) {
    final editController = TextEditingController(text: comment.body);
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        var saving = false;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final canSave = editController.text.trim().isNotEmpty && !saving;
            return AlertDialog(
              title: const Text('编辑评论'),
              content: TextField(
                controller: editController,
                enabled: !saving,
                minLines: 4,
                maxLines: 8,
                autofocus: true,
                onChanged: (_) => setDialogState(() {}),
                decoration: const InputDecoration(
                  hintText: '修改你的评论内容',
                  border: OutlineInputBorder(),
                ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: saving ? null : () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                TextButton(
                  onPressed: canSave
                      ? () async {
                          setDialogState(() => saving = true);
                          try {
                            await _market.editEntryComment(
                              commentId: comment.id,
                              body: editController.text.trim(),
                            );
                            final page = await _market.getCommentsPage(
                              entryId: widget.entry.id,
                              page: 1,
                            );
                            if (!mounted) return;
                            setState(() => _comments = page.items);
                            if (dialogContext.mounted) Navigator.of(dialogContext).pop();
                          } catch (error, stackTrace) {
                            debugPrint('Failed to edit market comment: $error\n$stackTrace');
                            if (!dialogContext.mounted) return;
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(
                                content: Text(error.toString()),
                                behavior: SnackBarBehavior.floating,
                              ),
                            );
                            setDialogState(() => saving = false);
                          }
                        }
                      : null,
                  child: saving
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('保存'),
                ),
              ],
            );
          },
        );
      },
    ).whenComplete(editController.dispose);
  }

  void _confirmDeleteComment(core_proxy.MarketComment comment) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        var deleting = false;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            return AlertDialog(
              title: const Text('删除评论'),
              content: const Text('确定删除这条评论吗？删除后评论会从公开列表移除。'),
              actions: <Widget>[
                TextButton(
                  onPressed: deleting ? null : () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                TextButton(
                  onPressed: deleting
                      ? null
                      : () async {
                          setDialogState(() => deleting = true);
                          try {
                            await _market.deleteEntryComment(commentId: comment.id);
                            final page = await _market.getCommentsPage(
                              entryId: widget.entry.id,
                              page: 1,
                            );
                            if (!mounted) return;
                            setState(() => _comments = page.items);
                            if (dialogContext.mounted) Navigator.of(dialogContext).pop();
                          } catch (error, stackTrace) {
                            debugPrint('Failed to delete market comment: $error\n$stackTrace');
                            if (!dialogContext.mounted) return;
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(
                                content: Text(error.toString()),
                                behavior: SnackBarBehavior.floating,
                              ),
                            );
                            setDialogState(() => deleting = false);
                          }
                        },
                  child: deleting
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('删除'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _install() async {
    if (_installing) return;
    final entry = widget.entry;

    // For script/package with artifact, let the user confirm the version carried by this entry.
    if ((entry.type == 'script' || entry.type == 'package') &&
        entry.artifact != null) {
      final version = await showArtifactVersionListDialog(
        context,
        entry: entry,
      );
      if (version == null || !mounted) return;
      setState(() => _installing = true);
      try {
        final result = await runCoreMarketInstall(
          clients: widget.clients,
          type: entry.type,
          entryId: entry.id,
          versionId: version.versionId,
        );
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(result)),
          );
        }
      } catch (error, stackTrace) {
        debugPrint('Failed to install artifact: $error\n$stackTrace');
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(error.toString()),
              behavior: SnackBarBehavior.floating,
            ),
          );
        }
      } finally {
        if (mounted) setState(() => _installing = false);
      }
      return;
    }

    setState(() => _installing = true);
    try {
      if (entry.type == 'skill') {
        final repoUrl = entry.source?.url.trim() ?? '';
        if (repoUrl.isEmpty) throw StateError('技能缺少仓库地址');
        final result = await widget.clients.permissionsSkillRuntimeSkillRepository.importSkillFromGitHubRepo(repoUrl: repoUrl);
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(result)));
        }
      } else if (entry.type == 'mcp') {
        final repoUrl = entry.source?.url.trim() ?? '';
        if (repoUrl.isEmpty) throw StateError('MCP 缺少仓库地址');
        final result = await widget.clients.permissionsMcpRuntimeMcpRepository.installMcpServerWithObjectForFlutter(
          pluginId: _safePackageId(entry.title),
          repoUrl: repoUrl,
          name: entry.title,
          description: entry.description,
          mcpConfig: entry.repoVersion?.installConfig ?? entry.latestVersion?.installConfig ?? '',
        );
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(result)));
        }
      } else {
        throw StateError('请在脚本/包详情页安装资产');
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install market entry: $error\n$stackTrace');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(error.toString()), behavior: SnackBarBehavior.floating),
        );
      }
    } finally {
      if (mounted) setState(() => _installing = false);
    }
  }

  bool _canPublishArtifactVersion(core_proxy.MarketEntrySummary entry) {
    return (entry.type == 'script' || entry.type == 'package') &&
        entry.artifact != null &&
        entry.allowPublicUpdates;
  }

  bool _canPublishRepoVersion(core_proxy.MarketEntrySummary entry) {
    return (entry.type == 'skill' || entry.type == 'mcp') &&
        entry.allowPublicUpdates;
  }

  bool _canPublishVersion(core_proxy.MarketEntrySummary entry) {
    return _canPublishArtifactVersion(entry) || _canPublishRepoVersion(entry);
  }

  Future<bool> _canCurrentUserEditEntry(core_proxy.MarketEntrySummary entry) async {
    final currentUser = await _market.getCurrentGithubUser();
    final publisherLogin = entry.publisher?.login.trim().toLowerCase() ?? '';
    return publisherLogin.isNotEmpty &&
        publisherLogin == currentUser.login.trim().toLowerCase();
  }

  Future<void> _publishVersion() async {
    final entry = widget.entry;
    if (_canPublishArtifactVersion(entry)) {
      await _publishArtifactVersion();
      return;
    }
    if (_canPublishRepoVersion(entry)) {
      await _publishRepoVersion();
    }
  }

  Future<void> _publishArtifactVersion() async {
    if (_openingPublish) {
      return;
    }
    final entry = widget.entry;
    final artifact = entry.artifact;
    if (artifact == null) {
      return;
    }
    setState(() {
      _openingPublish = true;
    });
    try {
      final canEditEntry = await _canCurrentUserEditEntry(entry);
      if (!mounted) {
        return;
      }
      Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) => ArtifactPublishScreen(
            clients: widget.clients,
            publishContext: ArtifactPublishClusterContext(
              entryId: entry.id,
              projectId: artifact.projectId,
              runtimePackageId: artifact.runtimePackageId ?? entry.latestVersion?.runtimePackageId ?? '',
              lockedDisplayName: entry.title,
              canEditEntry: canEditEntry,
            ),
          ),
        ),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to open artifact version publish: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error.toString()), behavior: SnackBarBehavior.floating),
      );
    } finally {
      if (mounted) {
        setState(() {
          _openingPublish = false;
        });
      }
    }
  }

  Future<void> _publishRepoVersion() async {
    if (_openingPublish) {
      return;
    }
    final entry = widget.entry;
    setState(() {
      _openingPublish = true;
    });
    try {
      final canEditEntry = await _canCurrentUserEditEntry(entry);
      if (!mounted) {
        return;
      }
      Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) => RepoMarketPublishScreen(
            clients: widget.clients,
            type: entry.type,
            publishContext: RepoMarketPublishContext(
              entry: entry,
              canEditEntry: canEditEntry,
            ),
          ),
        ),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to open repo version publish: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(error.toString()), behavior: SnackBarBehavior.floating),
      );
    } finally {
      if (mounted) {
        setState(() {
          _openingPublish = false;
        });
      }
    }
  }

  int _reactionCount(String reaction) {
    return _reactions
        .where((count) => count.reaction == reaction)
        .fold<int>(0, (sum, count) => sum + count.total);
  }

  List<core_proxy.MarketReactionCount> _withIncrementedReaction(
    List<core_proxy.MarketReactionCount> reactions,
    String reaction,
  ) {
    var updated = false;
    final next = <core_proxy.MarketReactionCount>[
      for (final count in reactions)
        if (count.reaction == reaction) ...<core_proxy.MarketReactionCount>[
          core_proxy.MarketReactionCount(
            reaction: count.reaction,
            total: count.total + 1,
          ),
        ] else ...<core_proxy.MarketReactionCount>[
          count,
        ],
    ];
    for (final count in reactions) {
      if (count.reaction == reaction) {
        updated = true;
      }
    }
    if (!updated) {
      next.add(core_proxy.MarketReactionCount(reaction: reaction, total: 1));
    }
    return next;
  }

  @override
  Widget build(BuildContext context) {
    final entry = widget.entry;
    return UnifiedMarketDetailScreen(
      title: entry.title,
      header: UnifiedMarketDetailHeader(
        title: entry.title,
        fallbackAvatarText: marketDetailInitial(entry.title),
        participants: <UnifiedMarketDetailParticipant>[
          UnifiedMarketDetailParticipant(
            roleLabel: '作者',
            name: entry.author?.login ?? '',
            avatarUrl: _cleanAvatarUrl(entry.author?.avatar),
            fallbackAvatarText: marketDetailInitial(entry.author?.login ?? ''),
          ),
          UnifiedMarketDetailParticipant(
            roleLabel: '发布者',
            name: entry.publisher?.login ?? '',
            avatarUrl: _cleanAvatarUrl(entry.publisher?.avatar),
            fallbackAvatarText: marketDetailInitial(entry.publisher?.login ?? ''),
          ),
          for (final contributor in entry.contributors)
            UnifiedMarketDetailParticipant(
              roleLabel: '贡献者',
              name: contributor.login,
              avatarUrl: _cleanAvatarUrl(contributor.avatar),
              fallbackAvatarText: marketDetailInitial(contributor.login),
            ),
        ],
        badges: <String>[
          entry.type,
          entry.categoryId ?? '',
          entry.latestVersion?.version ?? '',
          entry.stateCode,
        ].where((value) => value.trim().isNotEmpty).toList(growable: false),
        metrics: <UnifiedMarketDetailMetric>[
          UnifiedMarketDetailMetric(value: '${_reactionCount('+1')}', label: '喜欢'),
          UnifiedMarketDetailMetric(value: '${_entryDownloads(entry)}', label: '下载'),
          UnifiedMarketDetailMetric(value: formatMarketDate(entry.updatedAt), label: '更新'),
        ],
      ),
      overviewChildren: <Widget>[
        if (entry.description.trim().isNotEmpty) ...<Widget>[
          ArtifactDetailSectionCard(title: '简介', child: Text(entry.description)),
          const SizedBox(height: 14),
        ],
        if (entry.detail.trim().isNotEmpty) ...<Widget>[
          ArtifactDetailSectionCard(title: '详情', child: Text(entry.detail)),
          const SizedBox(height: 14),
        ],
        ArtifactDetailSectionCard(
          title: '元数据',
          child: ArtifactInfoTable(rows: _metadataRows(entry)),
        ),
      ],
      comments: UnifiedMarketDetailCommentsState(
        title: '用户评论',
        commentCount: _comments.length,
        isLoading: _communityLoading,
        errorMessage: _communityError,
        onRetry: _loadCommunity,
        reactions: <Widget>[
          FilterChip(
            selected: false,
            avatar: const Icon(Icons.thumb_up_outlined, size: 18),
            label: Text('喜欢 ${_reactionCount('+1')}'),
            onSelected: _reacting ? null : (_) => _react(),
          ),
        ],
        comments: _comments,
        canPost: true,
        isPosting: _postingComment,
        onRequestPost: _showCommentDialog,
        postHint: null,
        canManageComment: _canManageComment,
        onRequestEditComment: _showEditCommentDialog,
        onRequestDeleteComment: _confirmDeleteComment,
      ),
      primaryAction: UnifiedMarketDetailAction(
        label: _installing ? '安装中' : '安装',
        onPressed: _install,
        enabled: !_installing,
        isLoading: _installing,
        icon: Icons.download_outlined,
      ),
      secondaryAction: _canPublishVersion(entry)
          ? UnifiedMarketDetailAction(
              label: _openingPublish ? '打开中' : '发布新版本',
              onPressed: _publishVersion,
              enabled: !_openingPublish,
              isLoading: _openingPublish,
              icon: Icons.add_circle_outline,
            )
          : null,
    );
  }

  bool _canManageComment(core_proxy.MarketComment comment) {
    final current = _currentGithubLogin;
    return current != null &&
        current.isNotEmpty &&
        comment.author.login.trim().toLowerCase() == current;
  }

  List<ArtifactInfoRow> _metadataRows(core_proxy.MarketEntrySummary entry) {
    return <ArtifactInfoRow>[
      ArtifactInfoRow(label: '类型', value: entry.type),
      ArtifactInfoRow(label: 'Entry ID', value: entry.id),
      ArtifactInfoRow(label: '分类', value: entry.categoryId ?? ''),
      ArtifactInfoRow(label: '状态', value: entry.stateCode),
      ArtifactInfoRow(label: '作者', value: entry.author?.login ?? ''),
      ArtifactInfoRow(label: '发布者', value: entry.publisher?.login ?? ''),
      ArtifactInfoRow(label: '来源', value: entry.source?.url ?? ''),
      ArtifactInfoRow(label: '版本', value: entry.latestVersion?.version ?? ''),
      ArtifactInfoRow(label: '格式', value: entry.latestVersion?.formatVer ?? ''),
      ArtifactInfoRow(label: '最低版本', value: entry.latestVersion?.minAppVer ?? ''),
      ArtifactInfoRow(label: '最高版本', value: entry.latestVersion?.maxAppVer ?? ''),
      ArtifactInfoRow(label: '发布', value: formatMarketDate(entry.publishedAt ?? entry.createdAt)),
      ArtifactInfoRow(label: '更新', value: formatMarketDate(entry.updatedAt)),
    ].where((row) => row.value.trim().isNotEmpty).toList(growable: false);
  }
}

String? _cleanAvatarUrl(String? url) {
  if (url == null || url.trim().isEmpty) return null;
  return url;
}

String _safePackageId(String raw) {
  final normalized = raw
      .trim()
      .replaceAll(RegExp(r'[^a-zA-Z0-9_]'), '_')
      .replaceAll(RegExp(r'_+'), '_')
      .replaceAll(RegExp(r'^_|_$'), '');
  return normalized.isEmpty ? 'market_item' : normalized;
}

int _entryDownloads(core_proxy.MarketEntrySummary entry) {
  return entry.downloadCount > entry.downloads ? entry.downloadCount : entry.downloads;
}
