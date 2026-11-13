// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../settings/components/SettingsControlStyles.dart';
import '../components/ExtensionAvatar.dart';
import '../components/ExtensionMotion.dart';
import '../components/ExtensionNavigation.dart';
import '../components/ExtensionTopBar.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketInstallStateStore.dart';
import '../market/MarketCommentPager.dart';
import 'ArtifactProjectNodeTreeDialog.dart';
import 'ArtifactPublishScreen.dart';
import 'RepoMarketPublishScreen.dart';

class MarketEntryDetailScreen extends StatefulWidget {
  const MarketEntryDetailScreen({
    super.key,
    required this.entryId,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final String entryId;
  final GeneratedCoreProxyClients clients;

  @override
  State<MarketEntryDetailScreen> createState() =>
      _MarketEntryDetailScreenState();
}

class _MarketEntryDetailScreenState extends State<MarketEntryDetailScreen> {
  final TextEditingController _commentController = TextEditingController();
  late final MarketInstallStateStore _installState;
  late core_proxy.MarketEntrySummary? _entry = openedMarketEntry(
    widget.entryId,
  );
  String? _entryError;
  String? _currentGithubLogin;
  String? _currentGithubAuthorId;
  final GlobalKey _communityKey = GlobalKey();
  late final MarketCommentPager _commentPager = MarketCommentPager(
    (page) => _market.getCommentsPage(entryId: widget.entryId, page: page),
  );
  int _commentPage = 1;
  int _commentTotal = 0;
  int _commentPageCount = 1;
  int _communityGeneration = 0;
  bool _commentsExpanded = false;
  String? _expandedCommentId;
  Set<String> _expandedContextIds = {};
  bool _communityLoading = true;
  bool _composerOpen = false;
  core_proxy.MarketComment? _replyTo;
  bool _postingComment = false;
  bool _reacting = false;
  bool _liked = false;
  bool _openingPublish = false;
  String? _communityError;
  List<core_proxy.MarketComment> _comments = <core_proxy.MarketComment>[];
  List<core_proxy.MarketReactionCount> _reactions =
      <core_proxy.MarketReactionCount>[];

  bool get _installing => _installState.isInstalling(widget.entryId);

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _installState = MarketInstallStateStore.of(widget.clients);
    _installState.addListener(_onInstallStateChanged);
    unawaited(_installState.refresh());
    _reactions = _entry?.reactions ?? const <core_proxy.MarketReactionCount>[];
    unawaited(_loadEntry());
    unawaited(_loadCommunity());
    unawaited(_loadCurrentGithubLogin());
  }

  void _onInstallStateChanged() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _installState.removeListener(_onInstallStateChanged);
    _commentController.dispose();
    super.dispose();
  }

  Future<void> _loadEntry() async {
    setState(() => _entryError = null);
    try {
      final entry = await _market.getEntryById(entryId: widget.entryId);
      if (!mounted) return;
      setState(() {
        _entry = entry;
        _reactions = entry.reactions;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market entry: $error\n$stackTrace');
      if (!mounted) return;
      setState(() => _entryError = error.toString());
    }
  }

  Future<void> _loadCommunity({int? page, bool refresh = false}) async {
    final generation = ++_communityGeneration;
    setState(() {
      _communityLoading = true;
      _communityError = null;
      _expandedCommentId = null;
      _expandedContextIds = {};
    });
    try {
      final result = await _commentPager.load(
        page ?? _commentPage,
        refresh: refresh,
      );
      if (!mounted || generation != _communityGeneration) return;
      setState(() {
        _comments = result.items;
        _commentPage = result.page;
        _commentTotal = result.total;
        _commentPageCount = result.pageCount;
        _communityLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market community: $error\n$stackTrace');
      if (!mounted || generation != _communityGeneration) return;
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
        _currentGithubAuthorId = 'gh_${currentUser.id}';
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load current GitHub user: $error\n$stackTrace');
    }
  }

  Future<void> _postComment() async {
    final body = _commentController.text.trim();
    if (body.isEmpty || _postingComment) return;
    setState(() => _postingComment = true);
    try {
      final replyTo = _replyTo;
      if (replyTo == null) {
        await _market.createEntryComment(entryId: widget.entryId, body: body);
      } else {
        await _market.replyEntryComment(
          entryId: widget.entryId,
          body: body,
          parentId: replyTo.id,
        );
      }
      await _loadCommunity(page: 1, refresh: true);
      if (!mounted) return;
      setState(() {
        _commentController.clear();
        _postingComment = false;
        _composerOpen = false;
        _replyTo = null;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to post market comment: $error\n$stackTrace');
      if (!mounted) return;
      setState(() => _postingComment = false);
      _showError(error);
    }
  }

  Future<void> _react() async {
    if (_reacting) return;
    setState(() => _reacting = true);
    try {
      await _market.createEntryReaction(entryId: widget.entryId);
      if (!mounted) return;
      setState(() {
        _reactions = _withIncrementedReaction(_reactions, '+1');
        _reacting = false;
        _liked = true;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to react market entry: $error\n$stackTrace');
      if (!mounted) return;
      setState(() => _reacting = false);
      _showError(error);
    }
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
              content: SizedBox(
                width: 480,
                child: TextField(
                  controller: editController,
                  enabled: !saving,
                  minLines: 4,
                  maxLines: 8,
                  autofocus: true,
                  onChanged: (_) => setDialogState(() {}),
                  decoration: const InputDecoration(hintText: '修改你的评论内容'),
                ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: saving
                      ? null
                      : () => Navigator.of(dialogContext).pop(),
                  child: const Text('取消'),
                ),
                FilledButton(
                  onPressed: canSave
                      ? () async {
                          setDialogState(() => saving = true);
                          try {
                            await _market.editEntryComment(
                              commentId: comment.id,
                              body: editController.text.trim(),
                            );
                            await _loadCommunity(refresh: true);
                            if (!mounted) return;
                            if (dialogContext.mounted) {
                              Navigator.of(dialogContext).pop();
                            }
                          } catch (error, stackTrace) {
                            debugPrint(
                              'Failed to edit market comment: $error\n$stackTrace',
                            );
                            if (!dialogContext.mounted) return;
                            _showError(error);
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

  Future<void> _deleteComment(core_proxy.MarketComment comment) async {
    try {
      await _market.deleteEntryComment(commentId: comment.id);
      await _loadCommunity(refresh: true);
    } catch (error, stackTrace) {
      debugPrint('Failed to delete market comment: $error\n$stackTrace');
      _showError(error);
    }
  }

  /// Installs the latest version, or the version picked from the version list.
  Future<void> _install(
    core_proxy.MarketEntrySummary entry, {
    String? versionId,
  }) async {
    if (_installing) return;
    try {
      await _installState.install(entry, versionId: versionId);
    } catch (error, stackTrace) {
      debugPrint('Failed to install market entry: $error\n$stackTrace');
      _showError(error);
    }
  }

  Future<void> _installOtherVersion(core_proxy.MarketEntrySummary entry) async {
    final version = await showArtifactVersionListDialog(context, entry: entry);
    if (version == null || !mounted) return;
    await _install(entry, versionId: version.versionId);
  }

  bool _hasVersionChoice(core_proxy.MarketEntrySummary entry) {
    return (entry.type == 'script' || entry.type == 'package') &&
        entry.artifact != null;
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

  Future<bool> _canCurrentUserEditEntry(
    core_proxy.MarketEntrySummary entry,
  ) async {
    final currentUser = await _market.getCurrentGithubUser();
    final publisherLogin = entry.publisher?.login.trim().toLowerCase() ?? '';
    return publisherLogin.isNotEmpty &&
        publisherLogin == currentUser.login.trim().toLowerCase();
  }

  Future<void> _publishVersion(core_proxy.MarketEntrySummary entry) async {
    if (_openingPublish) {
      return;
    }
    setState(() => _openingPublish = true);
    try {
      final canEditEntry = await _canCurrentUserEditEntry(entry);
      if (!mounted) {
        return;
      }
      final artifact = entry.artifact;
      final Widget screen;
      if (_canPublishArtifactVersion(entry) && artifact != null) {
        screen = ArtifactPublishScreen(
          clients: widget.clients,
          publishContext: ArtifactPublishClusterContext(
            entryId: entry.id,
            projectId: artifact.projectId,
            runtimePackageId:
                artifact.runtimePackageId ??
                entry.latestVersion?.runtimePackageId ??
                '',
            lockedDisplayName: entry.title,
            canEditEntry: canEditEntry,
            initialEntry: entry,
          ),
        );
      } else {
        screen = RepoMarketPublishScreen(
          clients: widget.clients,
          type: entry.type,
          publishContext: RepoMarketPublishContext(
            entry: entry,
            canEditEntry: canEditEntry,
          ),
        );
      }
      await Navigator.of(
        context,
      ).push(MaterialPageRoute<void>(builder: (context) => screen));
    } catch (error, stackTrace) {
      debugPrint('Failed to open version publish: $error\n$stackTrace');
      _showError(error);
    } finally {
      if (mounted) {
        setState(() => _openingPublish = false);
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
    final existing = reactions.any((count) => count.reaction == reaction);
    return <core_proxy.MarketReactionCount>[
      for (final count in reactions)
        if (count.reaction == reaction)
          core_proxy.MarketReactionCount(
            reaction: count.reaction,
            total: count.total + 1,
          )
        else
          count,
      if (!existing)
        core_proxy.MarketReactionCount(reaction: reaction, total: 1),
    ];
  }

  void _showError(Object error) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
    );
  }

  @override
  Widget build(BuildContext context) {
    final entry = _entry;
    return ExtensionTopBar(
      title: entry == null
          ? null
          : (context) => Text(
              entry.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600),
            ),
      child: entry == null ? _buildPending(context) : _buildEntry(entry),
    );
  }

  Widget _buildPending(BuildContext context) {
    final error = _entryError;
    if (error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: StoreNoticeCard(
              icon: Icons.cloud_off_rounded,
              tone: StoreNoticeTone.error,
              title: '市场条目加载失败',
              message: error,
              action: TextButton(
                onPressed: _loadEntry,
                child: const Text('重试'),
              ),
            ),
          ),
        ),
      );
    }
    return const _DetailSkeleton();
  }

  Widget _buildEntry(core_proxy.MarketEntrySummary entry) {
    final colorScheme = Theme.of(context).colorScheme;
    final kind = StoreKind.of(entry.type);
    final localState = _installState.stateFor(entry);
    final installed = localState == MarketLocalInstallState.installed;
    final latest = entry.latestVersion;
    final latestVersion = latest?.version.trim() ?? '';
    final changelog = latest?.changelog?.trim() ?? '';
    final publisher = marketEntryPublisher(entry);
    final badgeLabel = localState.badgeLabel;
    final category = entry.categoryId?.trim() ?? '';
    final participants = <_Participant>[
      if (entry.author != null && entry.author!.login.trim().isNotEmpty)
        _Participant(role: '作者', author: entry.author!),
      if (entry.publisher != null &&
          entry.publisher!.login.trim().isNotEmpty &&
          entry.publisher!.login != entry.author?.login)
        _Participant(role: '发布者', author: entry.publisher!),
      for (final contributor in entry.contributors)
        _Participant(role: '贡献者', author: contributor),
    ];
    final wide = MediaQuery.sizeOf(context).width >= 600;
    return StoreDetailScaffold(
      header: StoreDetailHeader(
        icon: ExtensionAvatar(
          seed: marketEntryAvatarSeed(entry),
          label: entry.title,
          size: wide ? 88 : 72,
        ),
        title: entry.title,
        subtitle: publisher.isEmpty ? '社区市场' : publisher,
        badges: <Widget>[
          StoreBadge(label: kind.label, icon: kind.icon),
          if (latestVersion.isNotEmpty) StoreBadge(label: 'v$latestVersion'),
          if (entry.featured)
            StoreBadge(
              label: '精选',
              icon: Icons.auto_awesome_rounded,
              color: colorScheme.tertiaryContainer,
              foreground: colorScheme.onTertiaryContainer,
            ),
          if (category.isNotEmpty) StoreBadge(label: category),
          if (badgeLabel != null)
            StoreBadge(
              label: badgeLabel,
              color: colorScheme.primaryContainer,
              foreground: colorScheme.onPrimaryContainer,
            ),
        ],
        actions: <Widget>[
          StoreGetButton(
            label: localState.actionLabel(entry),
            icon: localState.actionIcon,
            prominent: true,
            large: true,
            busy: _installing,
            done: installed,
            onPressed: installed ? null : () => _install(entry),
          ),
          if (_hasVersionChoice(entry))
            OutlinedButton.icon(
              onPressed: _installing ? null : () => _installOtherVersion(entry),
              icon: const Icon(Icons.history_rounded, size: 18),
              label: const Text('其他版本'),
            ),
          if (_canPublishVersion(entry))
            OutlinedButton.icon(
              onPressed: _openingPublish ? null : () => _publishVersion(entry),
              icon: _openingPublish
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.publish_rounded, size: 18),
              label: const Text('发布新版本'),
            ),
        ],
      ),
      children: <Widget>[
        StoreStatsStrip(
          stats: <StoreStat>[
            StoreStat(
              label: '下载',
              value: storeCompactCount(marketEntryDownloads(entry)),
              icon: Icons.download_rounded,
            ),
            StoreStat(
              label: '喜欢',
              value: storeCompactCount(_reactionCount('+1')),
              icon: Icons.favorite_rounded,
            ),
            if (latestVersion.isNotEmpty)
              StoreStat(label: '版本', value: latestVersion),
            StoreStat(label: '更新于', value: formatMarketDate(entry.updatedAt)),
            StoreStat(label: '类型', value: kind.label, icon: kind.icon),
          ],
        ),
        if (entry.description.trim().isNotEmpty)
          StoreDetailSection(
            title: '简介',
            child: StoreExpandableText(text: entry.description),
          ),
        if (entry.detail.trim().isNotEmpty)
          StoreDetailSection(
            title: '详细介绍',
            child: StoreExpandableText(text: entry.detail, maxLines: 8),
          ),
        if (changelog.isNotEmpty)
          StoreDetailSection(
            title: '新版本内容',
            trailing: Text(
              <String>[
                if (latestVersion.isNotEmpty) 'v$latestVersion',
                if (latest?.publishedAt != null)
                  formatMarketDate(latest!.publishedAt!),
              ].join(' · '),
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            child: StoreExpandableText(text: changelog),
          ),
        if (participants.isNotEmpty)
          StoreDetailSection(
            title: '开发者',
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                for (final participant in participants)
                  _ParticipantChip(participant: participant),
              ],
            ),
          ),
        _buildCommunity(context, entry),
        StoreDetailSection(
          title: '信息',
          child: StoreInfoTable(items: _metadataItems(entry)),
        ),
      ],
    );
  }

  Widget _buildCommunity(
    BuildContext context,
    core_proxy.MarketEntrySummary entry,
  ) {
    final theme = Theme.of(context);
    final motion = ExtensionMotion.of(context);
    final Widget body;
    if (_communityLoading) {
      body = StoreShimmer(
        child: Column(
          children: <Widget>[
            for (var index = 0; index < 2; index += 1) ...<Widget>[
              const StoreSkeletonBox(height: 64, radius: 12),
              const SizedBox(height: 8),
            ],
          ],
        ),
      );
    } else if (_communityError != null) {
      body = StoreNoticeCard(
        icon: Icons.cloud_off_rounded,
        tone: StoreNoticeTone.error,
        title: '评论加载失败',
        message: _communityError,
        action: TextButton(
          onPressed: () => _loadCommunity(refresh: true),
          child: const Text('重试'),
        ),
      );
    } else if (_comments.isEmpty) {
      body = StoreGroupedList(
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 18),
            child: Row(
              children: <Widget>[
                Icon(
                  Icons.forum_outlined,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    '还没有评论，来写第一条吧。',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      );
    } else {
      body = StoreGroupedList(
        children: <Widget>[
          for (final comment
              in _comments
                  .take(_commentsExpanded ? MarketCommentPager.pageSize : 3)
                  .where(
                    (comment) =>
                        comment.id == _expandedCommentId ||
                        !_expandedContextIds.contains(comment.id),
                  ))
            _buildComment(entry, comment),
        ],
      );
    }
    return StoreDetailSection(
      key: _communityKey,
      title: '评论与反馈',
      count: _communityLoading ? null : _commentTotal,
      trailing: Wrap(
        spacing: 8,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: <Widget>[
          _LikeButton(
            count: _reactionCount('+1'),
            liked: _liked,
            busy: _reacting,
            onPressed: _react,
          ),
          FilledButton.tonalIcon(
            onPressed: _composerOpen
                ? null
                : () => setState(() {
                    _composerOpen = true;
                    _replyTo = null;
                  }),
            style: SettingsControlStyles.sectionFilledButton(),
            icon: const Icon(Icons.rate_review_outlined, size: 18),
            label: const Text('写评论'),
          ),
        ],
      ),
      child: AnimatedSize(
        duration: motion.standard,
        curve: ExtensionMotion.enter,
        alignment: Alignment.topCenter,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            if (_composerOpen) ...<Widget>[
              _CommentComposer(
                controller: _commentController,
                posting: _postingComment,
                replyTo: _replyTo,
                onCancelReply: () => setState(() => _replyTo = null),
                onCancel: () => setState(() {
                  _composerOpen = false;
                  _replyTo = null;
                }),
                onPost: _postComment,
              ),
              const SizedBox(height: 10),
            ],
            body,
            if (!_communityLoading &&
                _communityError == null &&
                _commentTotal > 3)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: _commentsExpanded
                    ? Column(
                        children: [
                          if (_commentPageCount > 1)
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                TextButton.icon(
                                  onPressed: _commentPage > 1
                                      ? () =>
                                            _changeCommentPage(_commentPage - 1)
                                      : null,
                                  icon: const Icon(
                                    Icons.chevron_left_rounded,
                                    size: 18,
                                  ),
                                  label: const Text('上一页'),
                                ),
                                Text(
                                  '$_commentPage / $_commentPageCount',
                                  style: theme.textTheme.labelLarge,
                                ),
                                TextButton(
                                  onPressed: _commentPage < _commentPageCount
                                      ? () =>
                                            _changeCommentPage(_commentPage + 1)
                                      : null,
                                  child: const Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      Text('下一页'),
                                      Icon(
                                        Icons.chevron_right_rounded,
                                        size: 18,
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                          TextButton.icon(
                            onPressed: _collapseComments,
                            icon: const Icon(
                              Icons.keyboard_arrow_up_rounded,
                              size: 18,
                            ),
                            label: const Text('收起评论'),
                          ),
                        ],
                      )
                    : TextButton.icon(
                        onPressed: () =>
                            setState(() => _commentsExpanded = true),
                        icon: const Icon(Icons.unfold_more_rounded, size: 18),
                        label: Text('查看全部 $_commentTotal 条评论'),
                      ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildComment(
    core_proxy.MarketEntrySummary entry,
    core_proxy.MarketComment comment,
  ) {
    Widget tile({VoidCallback? onDelete}) => StoreCommentTile(
      key: ValueKey(comment.id),
      comment: comment,
      parentComment: _commentPager.cachedComment(comment.parentId),
      loadConversation: () => _commentPager.conversation(comment),
      conversationActive: _expandedCommentId == comment.id,
      onConversationChanged: (comments) => setState(() {
        if (comments == null) {
          if (_expandedCommentId != comment.id) return;
          _expandedCommentId = null;
          _expandedContextIds = {};
        } else {
          _expandedCommentId = comment.id;
          _expandedContextIds = comments.map((item) => item.id).toSet();
        }
      }),
      onReplyTo: _postingComment ? null : _beginReply,
      onReply: _postingComment ? null : () => _beginReply(comment),
      onEdit: _canEditComment(comment)
          ? () => _showEditCommentDialog(comment)
          : null,
      onDelete: onDelete,
    );
    if (!_canDeleteComment(entry, comment)) return tile();
    return StoreInlineConfirm(
      key: ValueKey(comment.id),
      message: '删除后这条评论会从公开列表移除。',
      confirmLabel: '删除评论',
      destructive: true,
      onConfirm: () => _deleteComment(comment),
      trigger: (context, requestConfirm) => tile(onDelete: requestConfirm),
    );
  }

  void _beginReply(core_proxy.MarketComment comment) {
    setState(() {
      _replyTo = comment;
      _composerOpen = true;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _scrollToCommunity();
    });
  }

  void _scrollToCommunity() {
    final target = _communityKey.currentContext;
    if (target != null) {
      Scrollable.ensureVisible(
        target,
        duration: ExtensionMotion.of(context).standard,
      );
    }
  }

  Future<void> _changeCommentPage(int page) async {
    _scrollToCommunity();
    await _loadCommunity(page: page);
    if (mounted) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _scrollToCommunity();
      });
    }
  }

  Future<void> _collapseComments() async {
    _scrollToCommunity();
    setState(() {
      _commentsExpanded = false;
      _expandedCommentId = null;
      _expandedContextIds = {};
    });
    if (_commentPage != 1) await _loadCommunity(page: 1);
    if (mounted) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _scrollToCommunity();
      });
    }
  }

  bool _canEditComment(core_proxy.MarketComment comment) {
    return _sameMarketLogin(comment.author.login, _currentGithubLogin) ||
        _sameMarketAuthorId(comment.author.id, _currentGithubAuthorId);
  }

  bool _canDeleteComment(
    core_proxy.MarketEntrySummary entry,
    core_proxy.MarketComment comment,
  ) {
    if (_canEditComment(comment)) return true;
    final publisher = entry.publisher;
    return _sameMarketLogin(publisher?.login ?? '', _currentGithubLogin) ||
        _sameMarketAuthorId(publisher?.id ?? '', _currentGithubAuthorId);
  }

  bool _sameMarketLogin(String candidate, String? current) {
    final normalizedCandidate = candidate.trim().toLowerCase();
    final normalizedCurrent = current?.trim().toLowerCase() ?? '';
    return normalizedCandidate.isNotEmpty &&
        normalizedCurrent.isNotEmpty &&
        normalizedCandidate == normalizedCurrent;
  }

  bool _sameMarketAuthorId(String candidate, String? current) {
    final normalizedCandidate = candidate.trim().toLowerCase();
    final normalizedCurrent = current?.trim().toLowerCase() ?? '';
    return normalizedCandidate.isNotEmpty &&
        normalizedCurrent.isNotEmpty &&
        normalizedCandidate == normalizedCurrent;
  }

  /// Builds the visible marketplace metadata rows for an entry.
  List<StoreInfoItem> _metadataItems(core_proxy.MarketEntrySummary entry) {
    return <StoreInfoItem>[
      StoreInfoItem('类型', StoreKind.of(entry.type).label),
      StoreInfoItem('条目 ID', entry.id),
      StoreInfoItem('分类', entry.categoryId ?? ''),
      StoreInfoItem('状态', entry.stateCode),
      StoreInfoItem('来源', entry.source?.url ?? ''),
      StoreInfoItem('版本', entry.latestVersion?.version ?? ''),
      StoreInfoItem('格式', entry.latestVersion?.formatVer ?? ''),
      StoreInfoItem('ToolPkg API', entry.latestVersion?.apiVersion ?? ''),
      StoreInfoItem(
        '首次发布',
        formatMarketDate(entry.publishedAt ?? entry.createdAt),
      ),
      StoreInfoItem('最近更新', formatMarketDate(entry.updatedAt)),
    ];
  }
}

/// Inline editor that replaces the old comment dialog.
class _CommentComposer extends StatelessWidget {
  const _CommentComposer({
    required this.controller,
    required this.posting,
    required this.onCancel,
    required this.onPost,
    this.replyTo,
    required this.onCancelReply,
  });

  final core_proxy.MarketComment? replyTo;
  final VoidCallback onCancelReply;
  final TextEditingController controller;
  final bool posting;
  final VoidCallback onCancel;
  final VoidCallback onPost;

  @override
  Widget build(BuildContext context) {
    return StoreGroupedList(
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              if (replyTo != null) ...[
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        '回复 @${replyTo!.author.login}',
                        style: Theme.of(context).textTheme.labelLarge?.copyWith(
                          color: Theme.of(context).colorScheme.primary,
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: '取消回复',
                      onPressed: posting ? null : onCancelReply,
                      icon: const Icon(Icons.close_rounded, size: 18),
                    ),
                  ],
                ),
                Text(
                  replyTo!.body,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 8),
              ],
              TextField(
                controller: controller,
                enabled: !posting,
                minLines: 3,
                maxLines: 8,
                autofocus: true,
                decoration: InputDecoration(
                  hintText: replyTo == null ? '分享你的使用体验' : '写下你的回复',
                ),
              ),
              const SizedBox(height: 8),
              ValueListenableBuilder<TextEditingValue>(
                valueListenable: controller,
                builder: (context, value, child) => Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: <Widget>[
                    TextButton(
                      onPressed: posting ? null : onCancel,
                      style: SettingsControlStyles.sectionTextButton(),
                      child: const Text('取消'),
                    ),
                    const SizedBox(width: 6),
                    FilledButton(
                      onPressed: posting || value.text.trim().isEmpty
                          ? null
                          : onPost,
                      style: SettingsControlStyles.sectionFilledButton(),
                      child: posting
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Text('发布'),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _Participant {
  const _Participant({required this.role, required this.author});

  final String role;
  final core_proxy.MarketAuthor author;
}

class _ParticipantChip extends StatelessWidget {
  const _ParticipantChip({required this.participant});

  final _Participant participant;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.fromLTRB(6, 6, 12, 6),
      decoration: BoxDecoration(
        color: storeCardColor(theme.colorScheme),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: storeCardBorderColor(theme.colorScheme)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          StoreAvatar(
            name: participant.author.login,
            url: participant.author.avatar,
            radius: 14,
          ),
          const SizedBox(width: 8),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Text(
                participant.author.login,
                style: theme.textTheme.labelLarge?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              Text(
                participant.role,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Heart pill whose count rolls when the user likes the entry.
class _LikeButton extends StatelessWidget {
  const _LikeButton({
    required this.count,
    required this.liked,
    required this.busy,
    required this.onPressed,
  });

  final int count;
  final bool liked;
  final bool busy;
  final VoidCallback onPressed;

  static const Color _heart = Color(0xFFF06292);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final motion = ExtensionMotion.of(context);
    final heart = liked ? _heart : theme.colorScheme.onSurfaceVariant;
    return AnimatedContainer(
      duration: motion.standard,
      height: 38,
      decoration: ShapeDecoration(
        color: liked
            ? _heart.withValues(alpha: 0.14)
            : theme.colorScheme.surface.withValues(alpha: 0),
        shape: StadiumBorder(
          side: BorderSide(
            color: liked
                ? _heart.withValues(alpha: 0.4)
                : theme.colorScheme.outlineVariant,
          ),
        ),
      ),
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          customBorder: const StadiumBorder(),
          onTap: busy ? null : onPressed,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                AnimatedScale(
                  scale: busy ? 0.8 : 1,
                  duration: motion.fast,
                  child: Icon(
                    liked
                        ? Icons.favorite_rounded
                        : Icons.favorite_border_rounded,
                    size: 18,
                    color: heart,
                  ),
                ),
                const SizedBox(width: 6),
                AnimatedSwitcher(
                  duration: motion.standard,
                  transitionBuilder: (child, animation) => SlideTransition(
                    position: Tween<Offset>(
                      begin: const Offset(0, 0.6),
                      end: Offset.zero,
                    ).animate(animation),
                    child: FadeTransition(opacity: animation, child: child),
                  ),
                  child: Text(
                    storeCompactCount(count),
                    key: ValueKey<int>(count),
                    style: theme.textTheme.labelLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _DetailSkeleton extends StatelessWidget {
  const _DetailSkeleton();

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: storeDetailMaxWidth),
        child: const Padding(
          padding: EdgeInsets.fromLTRB(24, 16, 24, 0),
          child: StoreShimmer(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    StoreSkeletonBox(width: 72, height: 72, radius: 22),
                    SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          StoreSkeletonBox(width: 200, height: 18),
                          SizedBox(height: 10),
                          StoreSkeletonBox(width: 120, height: 12),
                        ],
                      ),
                    ),
                  ],
                ),
                SizedBox(height: 28),
                StoreSkeletonBox(height: 48),
                SizedBox(height: 24),
                StoreSkeletonBox(height: 120, radius: 12),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
