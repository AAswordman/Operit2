// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../settings/components/SettingsControlStyles.dart';
import '../components/ExtensionEntryTiles.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../model/ExtensionCatalogController.dart';

enum ExtensionGrouping { kind, location, status }

/// "我的扩展": what needs attention first, then every installed extension
/// grouped by kind, location or status.
class ExtensionsPane extends StatefulWidget {
  const ExtensionsPane({
    super.key,
    required this.controller,
    required this.query,
    required this.onAdd,
  });

  final ExtensionCatalogController controller;
  final String query;
  final VoidCallback onAdd;

  @override
  State<ExtensionsPane> createState() => _ExtensionsPaneState();
}

class _Group {
  const _Group({
    required this.id,
    required this.title,
    required this.icon,
    required this.entries,
    this.subtitle,
  });

  final String id;
  final String title;
  final IconData icon;
  final String? subtitle;
  final List<core_proxy.ExtensionCatalogEntry> entries;
}

class _ExtensionsPaneState extends State<ExtensionsPane> {
  ExtensionGrouping _grouping = ExtensionGrouping.kind;
  final Set<String> _revealed = <String>{};

  ExtensionCatalogController get _controller => widget.controller;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onChanged);
  }

  @override
  void dispose() {
    _controller.removeListener(_onChanged);
    super.dispose();
  }

  void _onChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  void _setGrouping(ExtensionGrouping grouping) {
    if (grouping == _grouping) return;
    setState(() {
      _grouping = grouping;
      // Regrouping deals the rows out again.
      _revealed.clear();
    });
  }

  Future<void> _run(Future<void> Function() action) async {
    try {
      await action();
    } catch (error, stackTrace) {
      debugPrint('Extension action failed: $error\n$stackTrace');
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    }
  }

  List<_Group> _groups(List<core_proxy.ExtensionCatalogEntry> entries) {
    List<core_proxy.ExtensionCatalogEntry> where(
      bool Function(core_proxy.ExtensionCatalogEntry entry) test,
    ) => entries.where(test).toList(growable: false);
    final groups = switch (_grouping) {
      ExtensionGrouping.kind => <_Group>[
        for (final kind in StoreKind.values)
          _Group(
            id: kind.id,
            title: kind.label,
            icon: kind.icon,
            subtitle: kind == StoreKind.mcp
                ? _mcpSummary(where((entry) => entry.kind == 'mcp'))
                : null,
            entries: where((entry) => entry.kind == kind.id),
          ),
      ],
      ExtensionGrouping.location => <_Group>[
        _Group(
          id: 'space',
          title: storeScopeLabel('space'),
          icon: storeScopeIcon('space'),
          subtitle: '同步到设备空间的所有设备，那里的对话都能用',
          entries: where((entry) => entry.scope == 'space'),
        ),
        _Group(
          id: 'device',
          title: storeScopeLabel('device'),
          icon: storeScopeIcon('device'),
          subtitle: '只在这台设备上，只有在这里运行的对话能用',
          entries: where((entry) => entry.scope == 'device'),
        ),
        _Group(
          id: 'localOnly',
          title: storeScopeLabel('localOnly'),
          icon: storeScopeIcon('localOnly'),
          subtitle: '本地命令型 MCP，只能在这台设备上运行',
          entries: where((entry) => entry.scope == 'localOnly'),
        ),
      ],
      ExtensionGrouping.status => <_Group>[
        _Group(
          id: 'problem',
          title: '需要处理',
          icon: Icons.error_outline_rounded,
          entries: where(ExtensionCatalogController.hasProblem),
        ),
        _Group(
          id: 'enabled',
          title: '已启用',
          icon: Icons.toggle_on_outlined,
          entries: where(
            (entry) =>
                !ExtensionCatalogController.hasProblem(entry) &&
                ExtensionCatalogController.isEnabled(entry),
          ),
        ),
        _Group(
          id: 'disabled',
          title: '已停用',
          icon: Icons.toggle_off_outlined,
          entries: where(
            (entry) =>
                !ExtensionCatalogController.hasProblem(entry) &&
                !ExtensionCatalogController.isEnabled(entry),
          ),
        ),
      ],
    };
    return groups.where((group) => group.entries.isNotEmpty).toList();
  }

  static String? _mcpSummary(List<core_proxy.ExtensionCatalogEntry> servers) {
    if (servers.isEmpty) return null;
    final running = servers
        .where((entry) => entry.runtimeState == 'running')
        .length;
    final failed = servers
        .where((entry) => entry.runtimeState.startsWith('error:'))
        .length;
    return <String>[
      '$running 个运行中',
      if (servers.length - running - failed > 0)
        '${servers.length - running - failed} 个未运行',
      if (failed > 0) '$failed 个启动失败',
    ].join(' · ');
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = _controller.snapshot;
    if (snapshot == null) {
      final error = _controller.error;
      if (error != null) {
        return Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 560),
              child: StoreNoticeCard(
                icon: Icons.error_outline_rounded,
                tone: StoreNoticeTone.error,
                title: '扩展目录加载失败',
                message: error,
                action: TextButton(
                  onPressed: _controller.refresh,
                  child: const Text('重试'),
                ),
              ),
            ),
          ),
        );
      }
      return const _LibrarySkeleton();
    }

    final query = widget.query.trim().toLowerCase();
    final visible = snapshot.installed
        .where(
          (entry) =>
              query.isEmpty ||
              <String>[
                entry.displayName,
                entry.id,
                entry.description,
                ...entry.authors,
              ].any((value) => value.toLowerCase().contains(query)),
        )
        .toList(growable: false);
    final problems = snapshot.installed
        .where(ExtensionCatalogController.hasProblem)
        .toList(growable: false);
    final updates = _controller.updatableEntries();
    final enabledCount = snapshot.installed
        .where(ExtensionCatalogController.isEnabled)
        .length;
    final groups = _groups(visible);
    final showAttention =
        query.isEmpty &&
        (problems.isNotEmpty ||
            updates.isNotEmpty ||
            snapshot.diagnostics.isNotEmpty);

    return RefreshIndicator(
      onRefresh: _controller.refresh,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final padding = storeSliverPadding(
            width,
            maxWidth: 1080,
            gutter: width < 600 ? 16 : 24,
          );
          final grid = width >= 1024;
          var order = 0;
          return CustomScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            slivers: <Widget>[
              SliverPadding(
                padding: padding.copyWith(top: 12),
                sliver: SliverToBoxAdapter(
                  child: _SummaryBar(
                    total: snapshot.installed.length,
                    enabled: enabledCount,
                    checking: _controller.checkingUpdates,
                    lastCheck: _controller.lastUpdateCheck,
                    updates: updates.length,
                    onCheck: () => _run(_controller.checkUpdates),
                  ),
                ),
              ),
              if (showAttention)
                SliverPadding(
                  padding: padding.copyWith(top: 16),
                  sliver: SliverToBoxAdapter(
                    child: _AttentionBlock(
                      controller: _controller,
                      problems: problems,
                      updates: updates,
                      diagnostics: snapshot.diagnostics,
                      onUpdateAll: () => _run(_controller.updateAll),
                      onShowDiagnostics: _showDiagnostics,
                      run: _run,
                    ),
                  ),
                ),
              SliverPadding(
                padding: padding.copyWith(top: 20, bottom: 4),
                sliver: SliverToBoxAdapter(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: SettingsSegmentedSelector<ExtensionGrouping>(
                      value: _grouping,
                      onChanged: _setGrouping,
                      segments: const <ButtonSegment<ExtensionGrouping>>[
                        ButtonSegment<ExtensionGrouping>(
                          value: ExtensionGrouping.kind,
                          icon: Icon(Icons.category_outlined, size: 16),
                          label: Text('按类型'),
                        ),
                        ButtonSegment<ExtensionGrouping>(
                          value: ExtensionGrouping.location,
                          icon: Icon(Icons.hub_outlined, size: 16),
                          label: Text('按位置'),
                        ),
                        ButtonSegment<ExtensionGrouping>(
                          value: ExtensionGrouping.status,
                          icon: Icon(Icons.toggle_on_outlined, size: 16),
                          label: Text('按状态'),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              if (visible.isEmpty)
                SliverPadding(
                  padding: padding.copyWith(top: 32),
                  sliver: SliverToBoxAdapter(
                    child: _EmptyLibrary(
                      query: widget.query.trim(),
                      onAdd: widget.onAdd,
                    ),
                  ),
                )
              else
                for (final group in groups) ...<Widget>[
                  SliverPadding(
                    padding: padding.copyWith(top: 22),
                    sliver: SliverToBoxAdapter(
                      child: StoreSectionHeader(
                        icon: group.icon,
                        title: group.title,
                        count: group.entries.length,
                        subtitle: group.subtitle,
                      ),
                    ),
                  ),
                  if (grid)
                    SliverPadding(
                      padding: padding,
                      sliver: SliverGrid(
                        gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
                          maxCrossAxisExtent: 400,
                          mainAxisExtent: 148 * storeTextScale(context),
                          crossAxisSpacing: 12,
                          mainAxisSpacing: 12,
                        ),
                        delegate: SliverChildListDelegate(<Widget>[
                          for (final entry in group.entries)
                            _entrance(
                              entry,
                              order++,
                              ExtensionCard(
                                controller: _controller,
                                entry: entry,
                                showKind:
                                    _grouping != ExtensionGrouping.kind,
                                showScope:
                                    _grouping != ExtensionGrouping.location,
                              ),
                            ),
                        ]),
                      ),
                    )
                  else
                    _groupedSliver(padding, <Widget>[
                      for (final entry in group.entries)
                        _entrance(
                          entry,
                          order++,
                          ExtensionListRow(
                            controller: _controller,
                            entry: entry,
                            showKind: _grouping != ExtensionGrouping.kind,
                            showScope:
                                _grouping != ExtensionGrouping.location,
                            showEnabledState: false,
                            trailing: ExtensionEnableSwitch(
                              controller: _controller,
                              entry: entry,
                            ),
                          ),
                        ),
                    ]),
                ],
              const SliverToBoxAdapter(child: SizedBox(height: 56)),
            ],
          );
        },
      ),
    );
  }

  Widget _entrance(
    core_proxy.ExtensionCatalogEntry entry,
    int order,
    Widget child,
  ) {
    final key = ExtensionCatalogController.entryKey(entry);
    return StoreEntrance(
      key: ValueKey<String>('${_grouping.name}:$key'),
      index: order,
      animate: _revealed.add(key),
      child: child,
    );
  }

  /// A settings-style card around a lazily built list.
  Widget _groupedSliver(EdgeInsets padding, List<Widget> rows) {
    final colorScheme = Theme.of(context).colorScheme;
    return SliverPadding(
      padding: padding,
      sliver: DecoratedSliver(
        decoration: BoxDecoration(
          color: storeCardColor(colorScheme),
          borderRadius: storeCardRadius,
          border: Border.all(color: storeCardBorderColor(colorScheme)),
        ),
        sliver: SliverList.separated(
          itemCount: rows.length,
          itemBuilder: (context, index) => Material(
            type: MaterialType.transparency,
            child: rows[index],
          ),
          separatorBuilder: (context, index) => Divider(
            height: 1,
            thickness: 1,
            indent: 66,
            endIndent: 14,
            color: colorScheme.outlineVariant.withValues(alpha: 0.4),
          ),
        ),
      ),
    );
  }

  Future<void> _showDiagnostics() {
    return showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      isScrollControlled: true,
      constraints: const BoxConstraints(maxWidth: 720),
      builder: (context) => ListenableBuilder(
        listenable: _controller,
        builder: (context, child) {
          final theme = Theme.of(context);
          final diagnostics =
              _controller.snapshot?.diagnostics ??
              const <core_proxy.ExtensionCatalogDiagnostic>[];
          return SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '加载失败的文件',
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '这些文件没能加载，所以不会出现在列表里。',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 14),
                  if (diagnostics.isEmpty)
                    Text('已经全部处理完了。', style: theme.textTheme.bodyMedium)
                  else
                    Flexible(
                      child: SingleChildScrollView(
                        child: StoreGroupedList(
                          children: <Widget>[
                            for (final diagnostic in diagnostics)
                              StoreListRow(
                                leading: StoreRowGlyph(
                                  icon: Icons.error_outline_rounded,
                                  color: theme.colorScheme.error,
                                ),
                                title: diagnostic.displayName,
                                subtitle:
                                    '${diagnostic.message}\n${diagnostic.sourcePath}',
                                trailing: _diagnosticAction(diagnostic),
                              ),
                          ],
                        ),
                      ),
                    ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _diagnosticAction(core_proxy.ExtensionCatalogDiagnostic diagnostic) {
    if (!diagnostic.canDelete) {
      return Tooltip(
        message: diagnostic.packageKind == 'skill'
            ? '技能扫描错误会在目录刷新时重新计算'
            : '此加载错误没有可删除的安装来源',
        child: const Icon(Icons.info_outline_rounded),
      );
    }
    if (_controller.isBusy('diagnostic:${diagnostic.sourcePath}')) {
      return const SizedBox.square(
        dimension: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      );
    }
    return IconButton(
      tooltip: '删除失败来源',
      onPressed: () => _run(() => _controller.deleteDiagnostic(diagnostic)),
      icon: const Icon(Icons.delete_outline_rounded),
    );
  }
}

/// Counts and the update check on one line.
class _SummaryBar extends StatelessWidget {
  const _SummaryBar({
    required this.total,
    required this.enabled,
    required this.checking,
    required this.lastCheck,
    required this.updates,
    required this.onCheck,
  });

  final int total;
  final int enabled;
  final bool checking;
  final DateTime? lastCheck;
  final int updates;
  final VoidCallback onCheck;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final lastCheck = this.lastCheck;
    return Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 12,
      runSpacing: 8,
      children: <Widget>[
        Text.rich(
          TextSpan(
            children: <InlineSpan>[
              TextSpan(
                text: '$total',
                style: theme.textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                  color: colorScheme.onSurface,
                ),
              ),
              const TextSpan(text: ' 个扩展 · '),
              TextSpan(
                text: '$enabled',
                style: TextStyle(
                  color: colorScheme.primary,
                  fontWeight: FontWeight.w700,
                ),
              ),
              const TextSpan(text: ' 个已启用'),
            ],
          ),
          style: theme.textTheme.bodyMedium?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (lastCheck != null && !checking)
              Padding(
                padding: const EdgeInsets.only(right: 8),
                child: Text(
                  updates > 0 ? '发现 $updates 个更新' : '都是最新版本',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            StoreGetButton(
              label: '检查更新',
              icon: Icons.refresh_rounded,
              busy: checking,
              onPressed: onCheck,
            ),
          ],
        ),
      ],
    );
  }
}

/// Problems, updates and load failures, each with its fix in place.
class _AttentionBlock extends StatelessWidget {
  const _AttentionBlock({
    required this.controller,
    required this.problems,
    required this.updates,
    required this.diagnostics,
    required this.onUpdateAll,
    required this.onShowDiagnostics,
    required this.run,
  });

  final ExtensionCatalogController controller;
  final List<core_proxy.ExtensionCatalogEntry> problems;
  final List<core_proxy.ExtensionCatalogEntry> updates;
  final List<core_proxy.ExtensionCatalogDiagnostic> diagnostics;
  final VoidCallback onUpdateAll;
  final VoidCallback onShowDiagnostics;
  final Future<void> Function(Future<void> Function() action) run;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final count = problems.length + updates.length + diagnostics.length;
    final updating = updates.any(
      (entry) => controller.isBusy(ExtensionCatalogController.entryKey(entry)),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        StoreSectionHeader(
          icon: Icons.notifications_active_outlined,
          title: '需要处理',
          count: count,
          trailing: updates.length > 1
              ? TextButton.icon(
                  onPressed: updating ? null : onUpdateAll,
                  style: SettingsControlStyles.sectionTextButton(),
                  icon: const Icon(Icons.system_update_alt_rounded, size: 18),
                  label: const Text('全部更新'),
                )
              : null,
        ),
        StoreGroupedList(
          children: <Widget>[
            for (final entry in problems)
              ExtensionListRow(
                controller: controller,
                entry: entry,
                trailing: Icon(
                  Icons.chevron_right_rounded,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            for (final entry in updates)
              ExtensionListRow(
                controller: controller,
                entry: entry,
                trailing: StoreGetButton(
                  label: '更新',
                  prominent: true,
                  busy: controller.isBusy(
                    ExtensionCatalogController.entryKey(entry),
                  ),
                  onPressed: () => run(() => controller.update(entry)),
                ),
              ),
            if (diagnostics.isNotEmpty)
              StoreListRow(
                leading: StoreRowGlyph(
                  icon: Icons.report_gmailerrorred_rounded,
                  color: theme.colorScheme.error,
                ),
                title: '${diagnostics.length} 个文件加载失败',
                subtitle: '查看加载原因，可删除损坏的安装来源',
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: onShowDiagnostics,
              ),
          ],
        ),
      ],
    );
  }
}

class _EmptyLibrary extends StatelessWidget {
  const _EmptyLibrary({required this.query, required this.onAdd});

  final String query;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(
            query.isEmpty
                ? Icons.extension_outlined
                : Icons.search_off_rounded,
            size: 40,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 12),
          Text(
            query.isEmpty ? '还没有安装扩展' : '没有找到「$query」',
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            query.isEmpty ? '从市场获取，或导入自己的插件、技能和 MCP。' : '换个关键词试试。',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          if (query.isEmpty) ...<Widget>[
            const SizedBox(height: 14),
            FilledButton.tonalIcon(
              onPressed: onAdd,
              style: SettingsControlStyles.sectionFilledButton(),
              icon: const Icon(Icons.add_rounded, size: 18),
              label: const Text('添加扩展'),
            ),
          ],
        ],
      ),
    );
  }
}

class _LibrarySkeleton extends StatelessWidget {
  const _LibrarySkeleton();

  @override
  Widget build(BuildContext context) {
    // Scrollable so a large font scale cannot overflow the fixed layout.
    return ListView(
      physics: const NeverScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
      children: <Widget>[
        StoreShimmer(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              const StoreSkeletonBox(width: 200, height: 26),
              const SizedBox(height: 24),
              const StoreSkeletonBox(width: 240, height: 32, radius: 10),
              const SizedBox(height: 24),
              for (var index = 0; index < 5; index += 1)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 10),
                  child: Row(
                    children: <Widget>[
                      StoreSkeletonBox(width: 40, height: 40, radius: 12),
                      SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            StoreSkeletonBox(width: 150, height: 14),
                            SizedBox(height: 8),
                            StoreSkeletonBox(width: 210, height: 11),
                          ],
                        ),
                      ),
                      SizedBox(width: 12),
                      StoreSkeletonBox(width: 44, height: 26, radius: 13),
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
