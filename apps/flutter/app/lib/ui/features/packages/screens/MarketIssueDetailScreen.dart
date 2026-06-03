// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../market/ArtifactMarketSupport.dart';
import '../market/UnifiedMarketDetailScreen.dart';

class MarketIssueDetailScreen extends StatefulWidget {
  const MarketIssueDetailScreen({
    super.key,
    required this.clients,
    required this.type,
    required this.item,
  });

  final GeneratedCoreProxyClients clients;
  final String type;
  final core_proxy.MarketRankIssueEntryResponse item;

  @override
  State<MarketIssueDetailScreen> createState() =>
      _MarketIssueDetailScreenState();
}

class _MarketIssueDetailScreenState extends State<MarketIssueDetailScreen> {
  final TextEditingController _commentController = TextEditingController();
  bool _communityLoading = true;
  bool _postingComment = false;
  bool _reacting = false;
  bool _installing = false;
  String? _communityError;
  List<core_proxy.GitHubComment> _comments = <core_proxy.GitHubComment>[];
  List<core_proxy.GitHubReaction> _reactions = <core_proxy.GitHubReaction>[];
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _currentUser;

  GeneratedApiMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.apiMarketStatsApiService;

  core_proxy.MarketRankIssueEntryResponse get _item => widget.item;

  Map<String, String> get _metadata =>
      _marketIssueMetadata(widget.item.issue, widget.type);

  @override
  void initState() {
    super.initState();
    _loadCommunity();
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
      final repo = _issueMarketRepository(widget.type);
      final auth = widget.clients.preferencesGitHubAuthPreferences;
      final loggedIn = await auth.isLoggedIn();
      final user = loggedIn ? await auth.getCurrentUserInfo() : null;
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: _item.issue.number,
        page: 1,
        perPage: 50,
      );
      final reactions = await _market.getIssueReactions(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: _item.issue.number,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _currentUser = user;
        _comments = comments;
        _reactions = reactions;
        _communityLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load ${widget.type} community: $error\n$stackTrace',
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _communityError = error.toString();
        _communityLoading = false;
      });
    }
  }

  Future<bool> _postComment() async {
    final body = _commentController.text.trim();
    if (body.isEmpty || _currentUser == null || _postingComment) {
      return false;
    }
    setState(() {
      _postingComment = true;
    });
    try {
      final repo = _issueMarketRepository(widget.type);
      await _market.createIssueComment(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: _item.issue.number,
        body: body,
      );
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: _item.issue.number,
        page: 1,
        perPage: 50,
      );
      if (!mounted) {
        return false;
      }
      setState(() {
        _comments = comments;
        _commentController.clear();
        _postingComment = false;
      });
      return true;
    } catch (error, stackTrace) {
      debugPrint('Failed to post ${widget.type} comment: $error\n$stackTrace');
      if (!mounted) {
        return false;
      }
      setState(() {
        _postingComment = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
      return false;
    }
  }

  void _showCommentDialog() {
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        var posting = false;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final canPost =
                _commentController.text.trim().isNotEmpty && !posting;
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
                  onPressed: posting
                      ? null
                      : () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                TextButton(
                  onPressed: canPost
                      ? () async {
                          setDialogState(() {
                            posting = true;
                          });
                          final posted = await _postComment();
                          if (!dialogContext.mounted) {
                            return;
                          }
                          if (posted) {
                            Navigator.of(dialogContext).pop();
                            return;
                          }
                          setDialogState(() {
                            posting = false;
                          });
                        }
                      : null,
                  child: posting
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('发送'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _react(String content) async {
    if (_currentUser == null || _reacting || _hasReaction(content)) {
      return;
    }
    setState(() {
      _reacting = true;
    });
    try {
      final repo = _issueMarketRepository(widget.type);
      final reaction = await _market.createIssueReaction(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: _item.issue.number,
        content: content,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _reactions = <core_proxy.GitHubReaction>[..._reactions, reaction];
        _reacting = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to react ${widget.type}: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _reacting = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _install() async {
    if (_installing) {
      return;
    }
    setState(() {
      _installing = true;
    });
    try {
      final metadata = _metadata;
      if (widget.type == 'skill') {
        await _installSkill(metadata);
      } else {
        await _installMcp(metadata);
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install ${widget.type}: $error\n$stackTrace');
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
          _installing = false;
        });
      }
    }
  }

  Future<void> _installSkill(Map<String, String> metadata) async {
    final repoUrl = metadata['repositoryUrl']?.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('技能缺少 repositoryUrl');
    }
    final result = await widget.clients.skillRepository
        .importSkillFromGitHubRepo(repoUrl: repoUrl);
    await _market.trackDownload(
      type: 'skill',
      id: _item.id,
      targetUrl: repoUrl,
    );
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }

  Future<void> _installMcp(Map<String, String> metadata) async {
    final repoUrl = metadata['repositoryUrl']?.trim() ?? '';
    final installConfig = metadata['installConfig']?.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('MCP 缺少 repositoryUrl');
    }
    if (installConfig.isEmpty) {
      throw StateError('MCP 缺少 installConfig');
    }
    final result = await widget.clients.mcpRepository
        .installMcpServerWithObjectForFlutter(
          pluginId: _safePackageId(_item.displayTitle),
          repoUrl: repoUrl,
          name: _item.displayTitle,
          description: _item.summaryDescription,
          mcpConfig: installConfig,
        );
    await _market.trackDownload(type: 'mcp', id: _item.id, targetUrl: repoUrl);
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }

  bool _hasReaction(String content) {
    final login = _currentUser?.login;
    if (login == null || login.isEmpty) {
      return false;
    }
    return _reactions.any(
      (reaction) => reaction.content == content && reaction.user.login == login,
    );
  }

  int _reactionCount(String content) {
    if (_reactions.isNotEmpty) {
      return _reactions.where((reaction) => reaction.content == content).length;
    }
    final issueReactions = _item.issue.reactions;
    return switch (content) {
      '+1' => issueReactions?.thumbsUp ?? 0,
      'heart' => issueReactions?.heart ?? 0,
      _ => 0,
    };
  }

  @override
  Widget build(BuildContext context) {
    final metadata = _metadata;
    final repositoryUrl = metadata['repositoryUrl']?.trim() ?? '';
    final installConfig = metadata['installConfig']?.trim() ?? '';
    final typeLabel = widget.type == 'skill' ? 'Skill' : 'MCP';
    return UnifiedMarketDetailScreen(
      title: _item.displayTitle,
      header: UnifiedMarketDetailHeader(
        title: _item.displayTitle,
        fallbackAvatarText: marketDetailInitial(_item.displayTitle),
        participants: <UnifiedMarketDetailParticipant>[
          UnifiedMarketDetailParticipant(
            roleLabel: '作者',
            name: _item.authorLogin,
            avatarUrl: _item.authorAvatarUrl,
            fallbackAvatarText: marketDetailInitial(_item.authorLogin),
          ),
          UnifiedMarketDetailParticipant(
            roleLabel: '分享者',
            name: _item.issue.user.login,
            avatarUrl: _item.issue.user.avatarUrl,
            fallbackAvatarText: marketDetailInitial(_item.issue.user.login),
          ),
        ],
        badges: <String>[
          widget.type,
          if ((_item.updatedAt ?? '').trim().isNotEmpty)
            formatMarketDate(_item.updatedAt ?? ''),
        ],
        metrics: <UnifiedMarketDetailMetric>[
          UnifiedMarketDetailMetric(value: '${_item.downloads}', label: '下载'),
          UnifiedMarketDetailMetric(
            value: '${_reactionCount('+1')}',
            label: '喜欢',
          ),
          UnifiedMarketDetailMetric(
            value: formatMarketDate(_item.issue.createdAt),
            label: '发布',
          ),
        ],
      ),
      overviewChildren: <Widget>[
        if (_item.summaryDescription.trim().isNotEmpty) ...<Widget>[
          ArtifactDetailSectionCard(
            title: '关于',
            child: Text(_item.summaryDescription),
          ),
          const SizedBox(height: 14),
        ],
        if (widget.type == 'mcp' && installConfig.isNotEmpty) ...<Widget>[
          ArtifactDetailSectionCard(
            title: '安装配置',
            child: SelectableText(
              installConfig,
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(fontFamily: 'monospace'),
            ),
          ),
          const SizedBox(height: 18),
        ],
        ArtifactDetailSectionCard(
          title: '元数据',
          child: ArtifactInfoTable(
            rows: <ArtifactInfoRow>[
              ArtifactInfoRow(label: '类型', value: typeLabel),
              ArtifactInfoRow(label: 'Issue', value: '#${_item.issue.number}'),
              ArtifactInfoRow(label: '作者', value: _item.authorLogin),
              ArtifactInfoRow(label: '下载', value: '${_item.downloads}'),
              if ((_item.updatedAt ?? '').trim().isNotEmpty)
                ArtifactInfoRow(
                  label: '更新',
                  value: formatMarketDate(_item.updatedAt ?? ''),
                ),
              if (repositoryUrl.isNotEmpty)
                ArtifactInfoRow(label: '仓库', value: repositoryUrl),
              ArtifactInfoRow(label: '链接', value: _item.issue.htmlUrl),
            ],
          ),
        ),
      ],
      comments: UnifiedMarketDetailCommentsState(
        title: '用户评论',
        commentCount: _comments.length,
        isLoading: _communityLoading,
        errorMessage: _communityError,
        onRetry: _loadCommunity,
        reactions: <Widget>[
          _reactionButton('+1', '喜欢', Icons.thumb_up_outlined),
          _reactionButton('heart', '收藏', Icons.favorite_border),
        ],
        comments: _comments,
        canPost: _currentUser != null,
        isPosting: _postingComment,
        onRequestPost: _showCommentDialog,
        postHint: _currentUser == null ? '登录 GitHub 后评论' : null,
      ),
      primaryAction: UnifiedMarketDetailAction(
        label: _installing ? '安装中' : '安装',
        onPressed: _install,
        enabled: _item.issue.state == 'open' && !_installing,
        isLoading: _installing,
        icon: Icons.download_outlined,
      ),
      secondaryAction: repositoryUrl.isEmpty
          ? null
          : UnifiedMarketDetailAction(
              label: '仓库',
              onPressed: () => _openRepository(repositoryUrl),
              icon: Icons.code,
            ),
    );
  }

  Future<void> _openRepository(String repositoryUrl) async {
    await launchUrl(
      Uri.parse(repositoryUrl),
      mode: LaunchMode.externalApplication,
    );
  }

  Widget _reactionButton(String content, String label, IconData icon) {
    final selected = _hasReaction(content);
    return FilterChip(
      selected: selected,
      avatar: Icon(icon, size: 18),
      label: Text('$label ${_reactionCount(content)}'),
      onSelected: selected || _currentUser == null || _reacting
          ? null
          : (_) => _react(content),
    );
  }
}

class _IssueMarketRepository {
  const _IssueMarketRepository({required this.owner, required this.repo});

  final String owner;
  final String repo;
}

_IssueMarketRepository _issueMarketRepository(String type) {
  return switch (type) {
    'skill' => const _IssueMarketRepository(
      owner: 'AAswordman',
      repo: 'OperitSkillMarket',
    ),
    'mcp' => const _IssueMarketRepository(
      owner: 'AAswordman',
      repo: 'OperitMCPMarket',
    ),
    final value => throw StateError('Unsupported market type: $value'),
  };
}

Map<String, String> _marketIssueMetadata(
  core_proxy.GitHubIssue issue,
  String type,
) {
  final body = issue.body ?? '';
  final prefix = type == 'skill'
      ? '<!-- operit-skill-json: '
      : '<!-- operit-mcp-json: ';
  final start = body.indexOf(prefix);
  if (start < 0) {
    return <String, String>{};
  }
  final jsonStart = start + prefix.length;
  final end = body.indexOf(' -->', jsonStart);
  if (end <= jsonStart) {
    return <String, String>{};
  }
  final decoded = jsonDecode(body.substring(jsonStart, end));
  if (decoded is! Map) {
    return <String, String>{};
  }
  final metadata = decoded.map(
    (key, value) => MapEntry(key.toString(), value?.toString() ?? ''),
  );
  if ((metadata['repositoryUrl'] ?? '').isEmpty &&
      (metadata['repoUrl'] ?? '').isNotEmpty) {
    metadata['repositoryUrl'] = metadata['repoUrl']!;
  }
  if ((metadata['installConfig'] ?? '').isEmpty &&
      (metadata['installCommand'] ?? '').isNotEmpty) {
    metadata['installConfig'] = metadata['installCommand']!;
  }
  return metadata;
}

String _safePackageId(String raw) {
  final normalized = raw
      .trim()
      .replaceAll(RegExp(r'[^a-zA-Z0-9_]'), '_')
      .replaceAll(RegExp(r'_+'), '_')
      .replaceAll(RegExp(r'^_|_$'), '');
  return normalized.isEmpty ? 'market_item' : normalized;
}
