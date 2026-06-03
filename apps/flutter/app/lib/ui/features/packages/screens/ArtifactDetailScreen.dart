// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../market/ArtifactMarketSupport.dart';
import '../market/UnifiedMarketDetailScreen.dart';
import 'ArtifactPublishScreen.dart';

class ArtifactNodeDetailsScreen extends StatefulWidget {
  const ArtifactNodeDetailsScreen({
    super.key,
    required this.clients,
    required this.project,
    required this.node,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ArtifactProjectDetailResponse project;
  final core_proxy.ArtifactProjectNodeResponse node;

  @override
  State<ArtifactNodeDetailsScreen> createState() =>
      _ArtifactNodeDetailsScreenState();
}

class _ArtifactNodeDetailsScreenState extends State<ArtifactNodeDetailsScreen> {
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
      final repo = artifactIssueRepository(widget.node.type);
      final auth = widget.clients.preferencesGitHubAuthPreferences;
      final loggedIn = await auth.isLoggedIn();
      final user = loggedIn ? await auth.getCurrentUserInfo() : null;
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        page: 1,
        perPage: 50,
      );
      final reactions = await _market.getIssueReactions(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
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
      debugPrint('Failed to load artifact community: $error\n$stackTrace');
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
      final repo = artifactIssueRepository(widget.node.type);
      await _market.createIssueComment(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        body: body,
      );
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
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
      debugPrint('Failed to post artifact comment: $error\n$stackTrace');
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
      final repo = artifactIssueRepository(widget.node.type);
      final reaction = await _market.createIssueReaction(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
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
      debugPrint('Failed to react artifact issue: $error\n$stackTrace');
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

  Future<void> _installNode() async {
    if (_installing) {
      return;
    }
    final confirmed = await confirmArtifactNodeCompatibility(
      context: context,
      project: widget.project,
      node: widget.node,
    );
    if (!confirmed) {
      return;
    }
    setState(() {
      _installing = true;
    });
    try {
      final result = await runCoreMarketInstall(
        clients: widget.clients,
        type: widget.node.type,
        projectId: widget.project.projectId,
        nodeId: widget.node.nodeId,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _installing = false;
      });
      if (result.trim().isNotEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
        );
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install artifact node: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _installing = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  void _continuePublishNode() {
    final node = widget.node;
    final navigator = Navigator.of(context);
    navigator.pop();
    navigator.push(
      MaterialPageRoute<void>(
        builder: (context) => ArtifactPublishScreen(
          clients: widget.clients,
          publishContext: ArtifactPublishClusterContext(
            projectId: node.projectId,
            rootNodeId: node.rootNodeId,
            runtimePackageId: node.runtimePackageId,
            parentNodeIds: <String>[node.nodeId],
            lockedDisplayName: artifactNodeTitle(node),
            projectDisplayName: firstNonBlank(<String>[
              widget.project.projectDisplayName,
              node.projectDisplayName,
              artifactNodeTitle(node),
            ]),
            projectDescription: firstNonBlank(<String>[
              widget.project.projectDescription,
              node.projectDescription,
              node.description,
            ]),
          ),
        ),
      ),
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
    final issueReactions = widget.node.issue.reactions;
    return switch (content) {
      '+1' => issueReactions?.thumbsUp ?? 0,
      'heart' => issueReactions?.heart ?? 0,
      'rocket' => issueReactions?.rocket ?? 0,
      _ => 0,
    };
  }

  @override
  Widget build(BuildContext context) {
    final node = widget.node;
    return UnifiedMarketDetailScreen(
      title: artifactNodeTitle(node),
      header: UnifiedMarketDetailHeader(
        title: artifactNodeTitle(node),
        fallbackAvatarText: marketDetailInitial(artifactNodeTitle(node)),
        participants: <UnifiedMarketDetailParticipant>[
          UnifiedMarketDetailParticipant(
            roleLabel: '发布者',
            name: node.publisherLogin.trim().isEmpty
                ? node.issue.user.login
                : node.publisherLogin,
            avatarUrl: node.issue.user.avatarUrl,
            fallbackAvatarText: marketDetailInitial(
              node.publisherLogin.trim().isEmpty
                  ? node.issue.user.login
                  : node.publisherLogin,
            ),
          ),
          UnifiedMarketDetailParticipant(
            roleLabel: '分享者',
            name: node.issue.user.login,
            avatarUrl: node.issue.user.avatarUrl,
            fallbackAvatarText: marketDetailInitial(node.issue.user.login),
          ),
        ],
        badges: <String>[
          artifactTypeLabel(node.type),
          node.version,
          supportedVersionLabel(node),
          node.issue.state == 'open' ? '可用' : '已关闭',
        ],
        metrics: <UnifiedMarketDetailMetric>[
          UnifiedMarketDetailMetric(
            value: '${widget.project.downloads}',
            label: '下载',
          ),
          UnifiedMarketDetailMetric(
            value: '${_reactionCount('+1')}',
            label: '喜欢',
          ),
          UnifiedMarketDetailMetric(
            value: formatMarketDate(node.issue.createdAt),
            label: '发布',
          ),
        ],
      ),
      overviewChildren: <Widget>[
        if (node.description.trim().isNotEmpty) ...<Widget>[
          ArtifactDetailSectionCard(title: '关于', child: Text(node.description)),
          const SizedBox(height: 14),
        ],
        if (!isArtifactNodeCompatible(node)) ...<Widget>[
          ArtifactCompatibilityBanner(project: widget.project, node: node),
          const SizedBox(height: 14),
        ],
        ArtifactDetailSectionCard(
          title: '元数据',
          child: ArtifactInfoTable(rows: _metadataRows()),
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
          _reactionButton('rocket', '推荐', Icons.rocket_launch_outlined),
        ],
        comments: _comments,
        canPost: _currentUser != null,
        isPosting: _postingComment,
        onRequestPost: _showCommentDialog,
        postHint: _currentUser == null ? '登录 GitHub 后评论' : null,
      ),
      primaryAction: UnifiedMarketDetailAction(
        label: _installing ? '下载中' : '下载',
        onPressed: _installNode,
        enabled: !_installing,
        isLoading: _installing,
        icon: Icons.download_outlined,
      ),
      secondaryAction: UnifiedMarketDetailAction(
        label: '发布新版本',
        onPressed: _continuePublishNode,
        enabled: node.runtimePackageId.trim().isNotEmpty,
        icon: Icons.update_outlined,
      ),
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

  List<ArtifactInfoRow> _metadataRows() {
    final node = widget.node;
    return <ArtifactInfoRow>[
      ArtifactInfoRow(label: '类型', value: artifactTypeLabel(node.type)),
      ArtifactInfoRow(label: '版本', value: node.version),
      ArtifactInfoRow(label: '项目簇', value: node.projectId),
      ArtifactInfoRow(label: '节点 ID', value: node.nodeId),
      ArtifactInfoRow(label: '运行时包', value: node.runtimePackageId),
      ArtifactInfoRow(label: '资源文件', value: node.assetName),
      ArtifactInfoRow(label: 'Release', value: node.releaseTag),
      ArtifactInfoRow(label: 'SHA-256', value: node.sha256),
      ArtifactInfoRow(label: '源文件', value: node.sourceFileName),
      ArtifactInfoRow(label: '支持版本', value: supportedVersionLabel(node)),
      const ArtifactInfoRow(label: '当前软件版本', value: currentAppVersion),
      ArtifactInfoRow(
        label: '发布',
        value: formatMarketDate(node.issue.createdAt),
      ),
      ArtifactInfoRow(
        label: '更新',
        value: formatMarketDate(node.issue.updatedAt),
      ),
      ArtifactInfoRow(label: 'Issue', value: '#${node.issue.number}'),
    ].where((row) => row.value.trim().isNotEmpty).toList(growable: false);
  }
}
