// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../main/screens/OperitScreens.dart';
import '../components/CreatorAccountCard.dart';
import '../components/CreatorEntryCard.dart';
import '../components/ExtensionNavigation.dart';
import '../components/ExtensionTopBar.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../market/ArtifactMarketSupport.dart';
import '../model/GitHubAccountState.dart';
import 'ArtifactPublishScreen.dart';
import 'GitHubOAuthLoginDialog.dart';
import 'RepoMarketPublishScreen.dart';

const String _logTag = 'MarketManage';

/// Notifications listed before the rest fold behind "show all".
const int _collapsedNoticeCount = 5;

/// GitHub account, publishing entry points, review progress and notifications
/// for people who publish to the market.
class MarketCreatorScreen extends StatefulWidget {
  const MarketCreatorScreen({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<MarketCreatorScreen> createState() => _MarketCreatorScreenState();
}

class _MarketCreatorScreenState extends State<MarketCreatorScreen> {
  late final GitHubAccountState _account = GitHubAccountState.of(
    widget.clients,
  );

  /// Login state the lists were last loaded for; null before the first read.
  bool? _wasLoggedIn;
  bool _reloading = false;
  bool _mineLoading = false;
  bool _entriesLoaded = false;
  bool _noticesLoaded = false;
  Object? _entriesError;
  Object? _noticesError;
  String? _busyEntryId;
  StoreKind? _kindFilter;
  bool _showAllNotices = false;
  List<core_proxy.MarketPublisherEntrySummary> _entries =
      const <core_proxy.MarketPublisherEntrySummary>[];
  List<core_proxy.MarketNotification> _notifications =
      const <core_proxy.MarketNotification>[];

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _account.addListener(_onAccountChanged);
    unawaited(_account.refresh());
  }

  @override
  void dispose() {
    _account.removeListener(_onAccountChanged);
    super.dispose();
  }

  /// Loads the lists when somebody signs in and drops them when they sign out.
  void _onAccountChanged() {
    if (!mounted) return;
    final loggedIn = _account.loggedIn;
    final changed = _account.loaded && loggedIn != _wasLoggedIn;
    if (changed) {
      _wasLoggedIn = loggedIn;
    }
    setState(() {
      if (changed && !loggedIn) {
        _entries = const <core_proxy.MarketPublisherEntrySummary>[];
        _notifications = const <core_proxy.MarketNotification>[];
        _entriesLoaded = false;
        _noticesLoaded = false;
        _entriesError = null;
        _noticesError = null;
        _kindFilter = null;
      }
    });
    if (changed && loggedIn) {
      unawaited(_loadMine());
    }
  }

  Future<(T?, Object?)> _attempt<T>(
    Future<T> Function() load,
    String what,
  ) async {
    try {
      return (await load(), null);
    } catch (error, stackTrace) {
      debugPrint('$what failed: $error\n$stackTrace');
      ClientLogger.e(
        '$what failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      return (null, error);
    }
  }

  /// Loads published entries and notifications side by side, so one failing
  /// does not hide the other.
  Future<void> _loadMine() async {
    if (_mineLoading) return;
    setState(() => _mineLoading = true);
    ClientLogger.v(
      'request=getMyEntries,getNotifications',
      tag: _logTag,
      verboseLevel: VerboseLevel.level3,
    );
    final minePending = _attempt(() => _market.getMyEntries(), 'getMyEntries');
    final noticesPending = _attempt(
      () => _market.getNotifications(limit: 50, offset: 0, since: null),
      'getNotifications',
    );
    final (mine, mineError) = await minePending;
    final (notices, noticesError) = await noticesPending;
    ClientLogger.v(
      'response=getMyEntries entries=${mine?.entries.length} '
      'notifications=${notices?.items.length} '
      'entriesError=${mineError != null} noticesError=${noticesError != null}',
      tag: _logTag,
      verboseLevel: VerboseLevel.level3,
    );
    if (!mounted) return;
    setState(() {
      _mineLoading = false;
      _entriesError = mineError;
      _noticesError = noticesError;
      if (mine != null) {
        _entries = <core_proxy.MarketPublisherEntrySummary>[...mine.entries]
          ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
        _entriesLoaded = true;
      }
      if (notices != null) {
        _notifications = notices.items;
        _noticesLoaded = true;
      }
    });
  }

  Future<void> _reload() async {
    setState(() => _reloading = true);
    try {
      await _account.refresh();
      if (_account.loggedIn) {
        await _loadMine();
      }
    } finally {
      if (mounted) {
        setState(() => _reloading = false);
      }
    }
  }

  void _login() {
    unawaited(
      showGitHubOAuthLoginDialog(
        context: context,
        clients: widget.clients,
        onLoginCompleted: _account.refresh,
      ),
    );
  }

  Future<void> _logout() async {
    try {
      await _account.logout();
    } catch (error, stackTrace) {
      debugPrint('Failed to logout GitHub: $error\n$stackTrace');
      ClientLogger.e(
        'logout failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      _showMessage('$error');
    }
  }

  /// Opens a publishing editor; publishing needs a signed-in GitHub account.
  Future<void> _openPublisher(WidgetBuilder builder) async {
    if (!_account.loggedIn) {
      _login();
      return;
    }
    await Navigator.of(context).push(MaterialPageRoute<void>(builder: builder));
    if (mounted && _account.loggedIn) {
      await _loadMine();
    }
  }

  Widget _publisherFor(
    core_proxy.MarketEntrySummary detail, {
    required bool canEditEntry,
  }) {
    if (detail.type == 'skill' ||
        detail.type == 'mcp' ||
        detail.type == 'a2a') {
      return RepoMarketPublishScreen(
        clients: widget.clients,
        type: detail.type,
        publishContext: RepoMarketPublishContext(
          entry: detail,
          canEditEntry: canEditEntry,
        ),
      );
    }
    final artifact = detail.artifact;
    return ArtifactPublishScreen(
      clients: widget.clients,
      publishContext: ArtifactPublishClusterContext(
        entryId: detail.id,
        projectId: artifact?.projectId ?? '',
        runtimePackageId:
            artifact?.runtimePackageId ??
            detail.latestVersion?.runtimePackageId ??
            '',
        lockedDisplayName: detail.title,
        canEditEntry: canEditEntry,
        initialEntry: detail,
      ),
    );
  }

  /// Loads [entry] in full and opens the editor that submits its next version.
  Future<void> _openVersionEditor(
    core_proxy.MarketPublisherEntrySummary entry, {
    required Future<core_proxy.MarketEntrySummary> Function() load,
    required String what,
  }) async {
    setState(() => _busyEntryId = entry.id);
    try {
      final detail = await load();
      if (!mounted) return;
      final canEditEntry = CreatorEntryAccess(entry).owner;
      await Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) =>
              _publisherFor(detail, canEditEntry: canEditEntry),
        ),
      );
      if (mounted) {
        await _loadMine();
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to open $what: $error\n$stackTrace');
      ClientLogger.e(
        '$what failed entryId=${entry.id}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      _showMessage('$error');
    } finally {
      if (mounted) {
        setState(() => _busyEntryId = null);
      }
    }
  }

  /// A listed entry publishes from its public data; a withdrawn one has no
  /// public page, so the owner's private copy is the source.
  Future<void> _publishVersion(core_proxy.MarketPublisherEntrySummary entry) {
    final listed = CreatorEntryAccess(entry).publiclyListed;
    return _openVersionEditor(
      entry,
      load: () => listed
          ? _market.getEntryById(entryId: entry.id)
          : _market.getMyEntryDetail(entryId: entry.id),
      what: 'publish version',
    );
  }

  Future<void> _submitRevision(
    core_proxy.MarketPublisherEntrySummary entry,
  ) async {
    final cooldown = creatorRevisionCooldown(entry.revisionAvailableAt);
    ClientLogger.v(
      'revision_gate click entryId=${entry.id} stateCode=${entry.stateCode} '
      'revisionAvailableAt=${entry.revisionAvailableAt ?? '<null>'} '
      'blocked=${cooldown != null}',
      tag: _logTag,
      verboseLevel: VerboseLevel.level2,
    );
    if (cooldown != null) {
      _showMessage('被打回后的修改版提交仍在冷却中，还需等待 ${formatCreatorCooldown(cooldown)}。');
      return;
    }
    await _openVersionEditor(
      entry,
      load: () => _market.getMyEntryDetail(entryId: entry.id),
      what: 'submit revision',
    );
  }

  Future<void> _withdraw(core_proxy.MarketPublisherEntrySummary entry) async {
    setState(() => _busyEntryId = entry.id);
    try {
      await _market.withdrawEntry(entryId: entry.id);
      if (!mounted) return;
      _showMessage('已移除「${entry.title}」');
      await _loadMine();
    } catch (error, stackTrace) {
      debugPrint('Failed to withdraw entry: $error\n$stackTrace');
      ClientLogger.e(
        'withdraw failed entryId=${entry.id}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      _showMessage('$error');
    } finally {
      if (mounted) {
        setState(() => _busyEntryId = null);
      }
    }
  }

  void _showMessage(String text) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(text), behavior: SnackBarBehavior.floating),
    );
  }

  /// The market answered 401: the saved GitHub token is no longer accepted.
  bool _isUnauthorized(Object? error) =>
      error != null && '$error'.contains('HTTP 401');

  List<CreatorStat> _stats() {
    int? entryCount(bool Function(CreatorEntryAccess access) test) =>
        _entriesLoaded
        ? _entries.where((entry) => test(CreatorEntryAccess(entry))).length
        : null;
    return <CreatorStat>[
      CreatorStat(
        label: '已发布',
        value: entryCount((access) => access.publiclyListed),
      ),
      CreatorStat(
        label: '审核中',
        value: entryCount(
          (access) =>
              access.state == CreatorReviewState.pending ||
              (access.open && access.pendingListing),
        ),
      ),
      CreatorStat(
        label: '需修改',
        value: entryCount((access) => access.canSubmitRevision),
        attention: true,
      ),
      CreatorStat(
        label: '通知',
        value: _noticesLoaded ? _notifications.length : null,
      ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final busy = _reloading || _mineLoading;
    return ExtensionTopBar(
      actions: (context) => <Widget>[
        IconButton(
          tooltip: '刷新',
          onPressed: busy ? null : _reload,
          icon: const Icon(Icons.refresh_rounded),
        ),
      ],
      child: Material(
        type: MaterialType.transparency,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final width = constraints.maxWidth;
            return RefreshIndicator(
              onRefresh: _reload,
              child: ListView(
                physics: const AlwaysScrollableScrollPhysics(),
                padding: storeSliverPadding(
                  width,
                  maxWidth: storeDetailMaxWidth,
                  gutter: width < 600 ? 16 : 24,
                  top: 12,
                  bottom: 48,
                ),
                children: _buildChildren(context),
              ),
            );
          },
        ),
      ),
    );
  }

  List<Widget> _buildChildren(BuildContext context) {
    final loggedIn = _account.loggedIn;
    final unauthorized =
        loggedIn &&
        (_isUnauthorized(_entriesError) || _isUnauthorized(_noticesError));
    var order = 0;
    Widget block(String name, Widget child) => StoreEntrance(
      key: ValueKey<String>(name),
      index: order++,
      child: child,
    );
    return <Widget>[
      block('account', _buildAccount()),
      if (unauthorized) ...<Widget>[
        const SizedBox(height: 12),
        StoreNoticeCard(
          icon: Icons.lock_outline_rounded,
          title: 'GitHub 登录已失效',
          message: '市场没有接受保存的登录凭据，可能已过期或被撤销。重新登录后即可继续。',
          action: FilledButton.tonal(
            onPressed: _login,
            style: FilledButton.styleFrom(
              visualDensity: VisualDensity.compact,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
            child: const Text('重新登录'),
          ),
        ),
      ],
      const SizedBox(height: 24),
      block(
        'publish',
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            const StoreSectionHeader(title: '发布到市场', subtitle: '提交后经过审核才会公开。'),
            _PublishOptions(
              locked: !loggedIn,
              onArtifact: () => _openPublisher(
                (context) => ArtifactPublishScreen(clients: widget.clients),
              ),
              onSkill: () => _openPublisher(
                (context) => RepoMarketPublishScreen(
                  clients: widget.clients,
                  type: 'skill',
                ),
              ),
              onA2a: () => _openPublisher(
                (context) => RepoMarketPublishScreen(
                  clients: widget.clients,
                  type: 'a2a',
                ),
              ),
              onMcp: () => _openPublisher(
                (context) => RepoMarketPublishScreen(
                  clients: widget.clients,
                  type: 'mcp',
                ),
              ),
            ),
          ],
        ),
      ),
      if (loggedIn && !unauthorized) ...<Widget>[
        const SizedBox(height: 24),
        block('mine', _buildMine(context)),
        const SizedBox(height: 24),
        block('notices', _buildNotifications(context)),
      ],
    ];
  }

  Widget _buildAccount() {
    final error = _account.error;
    if (error != null) {
      return StoreNoticeCard(
        icon: Icons.error_outline_rounded,
        tone: StoreNoticeTone.error,
        title: '读取 GitHub 登录状态失败',
        message: '$error',
        action: TextButton(
          onPressed: _account.refresh,
          child: const Text('重试'),
        ),
      );
    }
    return CreatorAccountCard(
      loading: !_account.loaded,
      user: _account.user,
      stats: _stats(),
      onLogin: _login,
      onLogout: _logout,
    );
  }

  Widget _buildMine(BuildContext context) {
    final error = _entriesError;
    final kinds = <StoreKind>[
      for (final kind in StoreKind.values)
        if (_entries.any((entry) => identical(StoreKind.of(entry.type), kind)))
          kind,
    ];
    final selected = _kindFilter;
    final filter = selected != null && kinds.contains(selected)
        ? selected
        : null;
    final visible = filter == null
        ? _entries
        : _entries
              .where((entry) => identical(StoreKind.of(entry.type), filter))
              .toList(growable: false);
    final owned = visible
        .where((entry) => entry.relation == 'owner')
        .toList(growable: false);
    final shared = visible
        .where((entry) => entry.relation != 'owner')
        .toList(growable: false);

    Widget card(core_proxy.MarketPublisherEntrySummary entry) => Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: CreatorEntryCard(
        key: ValueKey<String>('entry:${entry.id}'),
        entry: entry,
        busy: _busyEntryId == entry.id,
        onOpen: () => openExtensionScreen(
          MarketEntryDetailScreenRoute(entryId: entry.id),
        ),
        onPublishVersion: () => _publishVersion(entry),
        onSubmitRevision: () => _submitRevision(entry),
        onWithdraw: () => _withdraw(entry),
      ),
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        StoreSectionHeader(
          title: '我发布的',
          count: _entriesLoaded ? _entries.length : null,
        ),
        if (error != null) ...<Widget>[
          StoreNoticeCard(
            icon: Icons.cloud_off_rounded,
            tone: StoreNoticeTone.error,
            title: '发布记录加载失败',
            message: '$error',
            action: TextButton(
              onPressed: _mineLoading ? null : _loadMine,
              child: const Text('重试'),
            ),
          ),
          const SizedBox(height: 12),
        ],
        if (!_entriesLoaded && error == null)
          const StoreShimmer(
            child: Column(
              children: <Widget>[
                StoreSkeletonBox(height: 132, radius: 12),
                SizedBox(height: 10),
                StoreSkeletonBox(height: 132, radius: 12),
              ],
            ),
          )
        else if (_entriesLoaded && _entries.isEmpty)
          const _EmptyCard(
            icon: Icons.inventory_2_outlined,
            title: '还没有发布过扩展',
            message: '从上面的入口发布第一个扩展，审核进度会显示在这里。',
          )
        else if (_entries.isNotEmpty) ...<Widget>[
          if (kinds.length > 1)
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: <Widget>[
                    StoreFilterChip(
                      label: '全部',
                      icon: Icons.apps_rounded,
                      count: _entries.length,
                      selected: filter == null,
                      onSelected: () => setState(() => _kindFilter = null),
                    ),
                    for (final kind in kinds) ...<Widget>[
                      const SizedBox(width: 8),
                      StoreFilterChip(
                        label: kind.label,
                        icon: kind.icon,
                        count: _entries
                            .where(
                              (entry) =>
                                  identical(StoreKind.of(entry.type), kind),
                            )
                            .length,
                        selected: identical(filter, kind),
                        onSelected: () => setState(() => _kindFilter = kind),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          if (owned.isNotEmpty) ...<Widget>[
            if (shared.isNotEmpty) const _GroupCaption(text: '我拥有的'),
            for (final entry in owned) card(entry),
          ],
          if (shared.isNotEmpty) ...<Widget>[
            const _GroupCaption(text: '我参与提交的'),
            for (final entry in shared) card(entry),
          ],
        ],
      ],
    );
  }

  Widget _buildNotifications(BuildContext context) {
    final error = _noticesError;
    final shown = _showAllNotices
        ? _notifications
        : _notifications.take(_collapsedNoticeCount).toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        StoreSectionHeader(
          title: '通知',
          count: _noticesLoaded ? _notifications.length : null,
        ),
        if (error != null) ...<Widget>[
          StoreNoticeCard(
            icon: Icons.cloud_off_rounded,
            tone: StoreNoticeTone.error,
            title: '通知加载失败',
            message: '$error',
            action: TextButton(
              onPressed: _mineLoading ? null : _loadMine,
              child: const Text('重试'),
            ),
          ),
          const SizedBox(height: 12),
        ],
        if (!_noticesLoaded && error == null)
          const StoreShimmer(child: StoreSkeletonBox(height: 72, radius: 12))
        else if (_noticesLoaded && _notifications.isEmpty)
          const _EmptyCard(
            icon: Icons.notifications_none_rounded,
            title: '暂无通知',
            message: '评论、回复和审核结果会显示在这里。',
          )
        else if (_notifications.isNotEmpty) ...<Widget>[
          StoreGroupedList(
            children: <Widget>[
              for (final notice in shown)
                _NoticeRow(
                  key: ValueKey<String>('notice:${notice.id}'),
                  notice: notice,
                ),
            ],
          ),
          if (_notifications.length > _collapsedNoticeCount)
            Align(
              alignment: Alignment.centerRight,
              child: Padding(
                padding: const EdgeInsets.only(top: 6),
                child: TextButton.icon(
                  onPressed: () =>
                      setState(() => _showAllNotices = !_showAllNotices),
                  iconAlignment: IconAlignment.end,
                  icon: Icon(
                    _showAllNotices
                        ? Icons.expand_less_rounded
                        : Icons.expand_more_rounded,
                    size: 18,
                  ),
                  label: Text(
                    _showAllNotices ? '收起' : '查看全部 ${_notifications.length} 条',
                  ),
                ),
              ),
            ),
        ],
      ],
    );
  }
}

/// Publishing doors: plugin or script package, skill and MCP.
class _PublishOptions extends StatelessWidget {
  const _PublishOptions({
    required this.locked,
    required this.onArtifact,
    required this.onSkill,
    required this.onMcp,
    required this.onA2a,
  });

  /// Nobody is signed in, so every door asks for a login first.
  final bool locked;
  final VoidCallback onArtifact;
  final VoidCallback onSkill;
  final VoidCallback onMcp;
  final VoidCallback onA2a;

  /// Explains the lock on hover and long press while nobody is signed in.
  Widget _lockedHint({required Widget child}) =>
      locked ? Tooltip(message: '登录 GitHub 后可发布', child: child) : child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final options = <(StoreKind, String, String, VoidCallback)>[
      (StoreKind.toolpkg, '发布插件或脚本包', '上传打包好的 ToolPkg、脚本或运行时资源', onArtifact),
      (StoreKind.skill, '发布技能', '提交 GitHub 仓库形式的技能', onSkill),
      (StoreKind.mcp, '发布 MCP', '提交 GitHub 仓库形式的 MCP 服务', onMcp),
      (StoreKind.a2a, '发布 A2A', '提交 Agent 接入配置和能力说明', onA2a),
    ];
    // The glyph, the title line and the arrow share one center line, so the
    // doors line up whatever the length of their descriptions.
    final tiles = <Widget>[
      for (final (kind, title, subtitle, onTap) in options)
        _lockedHint(
          child: StorePressable(
            onTap: onTap,
            padding: const EdgeInsets.all(14),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                StoreRowGlyph(
                  icon: kind.icon,
                  color: locked ? colorScheme.outline : colorScheme.primary,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.only(top: 6),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          title,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          subtitle,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Padding(
                  padding: const EdgeInsets.only(top: 7),
                  child: Icon(
                    locked
                        ? Icons.lock_outline_rounded
                        : Icons.arrow_forward_rounded,
                    size: 18,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 720) {
          // One row of equally tall cards, whichever description wraps.
          return IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                for (
                  var index = 0;
                  index < tiles.length;
                  index += 1
                ) ...<Widget>[
                  if (index > 0) const SizedBox(width: 10),
                  Expanded(child: tiles[index]),
                ],
              ],
            ),
          );
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            for (var index = 0; index < tiles.length; index += 1) ...<Widget>[
              if (index > 0) const SizedBox(height: 10),
              tiles[index],
            ],
          ],
        );
      },
    );
  }
}

class _GroupCaption extends StatelessWidget {
  const _GroupCaption({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 0, 4, 8),
      child: Text(
        text,
        style: theme.textTheme.labelLarge?.copyWith(
          color: theme.colorScheme.primary,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _EmptyCard extends StatelessWidget {
  const _EmptyCard({required this.icon, required this.title, this.message});

  final IconData icon;
  final String title;
  final String? message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final message = this.message;
    return StoreGroupedList(
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 18),
          child: Row(
            children: <Widget>[
              Icon(icon, size: 28, color: colorScheme.onSurfaceVariant),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    if (message != null) ...<Widget>[
                      const SizedBox(height: 2),
                      Text(
                        message,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
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

/// One market notification: what happened, when, and the words the market sent.
class _NoticeRow extends StatelessWidget {
  const _NoticeRow({super.key, required this.notice});

  final core_proxy.MarketNotification notice;

  /// Notifications about public entries open that entry; review outcomes can
  /// concern entries that have no public page, so they stay informational.
  bool get _opensEntry =>
      notice.entryId != null &&
      switch (notice.kind) {
        'comment_new' ||
        'comment_reply' ||
        'review_approved' ||
        'entry_curated' => true,
        _ => false,
      };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final tone = _noticeTone(notice.kind, colorScheme);
    final title = notice.title.trim().isEmpty
        ? tone.label
        : notice.title.trim();
    final body = notice.body.trim();
    final entryId = notice.entryId;
    return InkWell(
      onTap: _opensEntry && entryId != null
          ? () => openExtensionScreen(
              MarketEntryDetailScreenRoute(entryId: entryId),
            )
          : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 11),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            StoreRowGlyph(icon: tone.icon, color: tone.color),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: Text(
                          tone.label,
                          style: theme.textTheme.labelMedium?.copyWith(
                            color: tone.color,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      Text(
                        _relativeTime(notice.createdAt),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  if (body.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 2),
                    Text(
                      body,
                      maxLines: 3,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            if (_opensEntry) ...<Widget>[
              const SizedBox(width: 8),
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Icon(
                  Icons.chevron_right_rounded,
                  size: 18,
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

({IconData icon, String label, Color color}) _noticeTone(
  String kind,
  ColorScheme colorScheme,
) => switch (kind) {
  'comment_new' => (
    icon: Icons.comment_outlined,
    label: '新评论',
    color: colorScheme.primary,
  ),
  'comment_reply' => (
    icon: Icons.reply_rounded,
    label: '回复了你的评论',
    color: colorScheme.primary,
  ),
  'review_approved' => (
    icon: Icons.check_circle_outline_rounded,
    label: '已通过审核',
    color: colorScheme.primary,
  ),
  'review_rejected' => (
    icon: Icons.cancel_outlined,
    label: '未通过审核',
    color: colorScheme.error,
  ),
  'review_changes' => (
    icon: Icons.rate_review_outlined,
    label: '需要修改',
    color: colorScheme.tertiary,
  ),
  'entry_curated' => (
    icon: Icons.star_outline_rounded,
    label: '入选精选',
    color: colorScheme.tertiary,
  ),
  _ => (
    icon: Icons.notifications_outlined,
    label: kind,
    color: colorScheme.onSurfaceVariant,
  ),
};

/// "3 分钟前" style age of a market timestamp.
String _relativeTime(String isoDate) {
  final time = DateTime.tryParse(isoDate.trim());
  if (time == null) {
    return formatMarketDate(isoDate);
  }
  final seconds = DateTime.now().toUtc().difference(time.toUtc()).inSeconds;
  if (seconds < 60) return '刚刚';
  if (seconds < 3600) return '${seconds ~/ 60} 分钟前';
  if (seconds < 86400) return '${seconds ~/ 3600} 小时前';
  if (seconds < 2592000) return '${seconds ~/ 86400} 天前';
  if (seconds < 31104000) return '${seconds ~/ 2592000} 个月前';
  return '${seconds ~/ 31104000} 年前';
}
