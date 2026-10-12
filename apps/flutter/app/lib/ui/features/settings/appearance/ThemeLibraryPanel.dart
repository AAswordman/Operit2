// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit_theme/operit_theme.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/OperitDialog.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../../../theme/ThemeLibraryService.dart';
import '../components/SettingsControlStyles.dart';
import 'ChatAppearancePreview.dart';

enum _ThemeLibraryFilter { all, active, ready, attention }

enum _ThemeLibrarySort { activeFirst, name, size }

enum _ImportDialogChoice { cancel, importOnly, importAndApply }

enum _PreviewStageView { clientScene, chatWidgets }

/// Space owns the packages. Every apply or personal-appearance switch below
/// targets this CoreNode explicitly; delete removes the package from Space.
class ThemeLibraryPanel extends StatefulWidget {
  const ThemeLibraryPanel({super.key, required this.controller});

  final OperitThemeController controller;

  @override
  State<ThemeLibraryPanel> createState() => _ThemeLibraryPanelState();
}

class _ThemeLibraryPanelState extends State<ThemeLibraryPanel> {
  late Future<List<core_proxy.ThemeLibraryItem>> _items;
  final TextEditingController _searchController = TextEditingController();
  _ThemeLibraryFilter _filter = _ThemeLibraryFilter.all;
  _ThemeLibrarySort _sort = _ThemeLibrarySort.activeFirst;
  bool _busy = false;
  String? _busyMessage;
  String? _result;
  String? _error;

  @override
  void initState() {
    super.initState();
    _items = widget.controller.listThemeLibrary();
  }

  @override
  void didUpdateWidget(covariant ThemeLibraryPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.controller, widget.controller)) {
      _items = widget.controller.listThemeLibrary();
    }
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _run(
    String busyMessage,
    Future<void> Function() action,
  ) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _busyMessage = busyMessage;
      _error = null;
      _result = null;
    });
    try {
      await action();
    } catch (error) {
      if (mounted) {
        setState(() => _error = _cleanErrorMessage(error));
      }
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _busyMessage = null;
          _items = widget.controller.listThemeLibrary();
        });
      }
    }
  }

  Future<void> _copyToClipboard(String label, String value) async {
    await Clipboard.setData(ClipboardData(text: value));
    if (!mounted) return;
    setState(() {
      _error = null;
      _result = '已复制$label：${_shortHash(value, head: 12, tail: 10)}';
    });
  }

  Future<void> _import() => _run('正在读取并校验 .optheme 主题包…', () async {
    final input = await widget.controller.library.inspectPickedArchive();
    if (input == null || !mounted) return;
    final choice = await showDialog<_ImportDialogChoice>(
      context: context,
      builder: (_) => _ThemeImportConfirmDialog(input: input),
    );
    if (choice == null || choice == _ImportDialogChoice.cancel || !mounted) {
      return;
    }
    setState(() {
      _busyMessage = choice == _ImportDialogChoice.importAndApply
          ? '正在导入主题包并应用到本节点…'
          : '正在将主题包写入 Space 主题库…';
    });
    final receipt = await widget.controller.library.importArchive(input);
    if (!mounted) return;
    if (choice == _ImportDialogChoice.importAndApply) {
      final applied = await widget.controller.applyLibraryTheme(receipt.entry);
      if (!mounted) return;
      setState(() {
        _result =
            '已导入并应用主题：${applied.entry.displayName} v${applied.entry.version}';
      });
      return;
    }
    setState(() {
      _result = receipt.status == core_proxy.ThemeInstallStatus.alreadyPresent
          ? '主题包「${receipt.entry.displayName} v${receipt.entry.version}」已在库中；本节点当前主题未改变。'
          : '已导入「${receipt.entry.displayName} v${receipt.entry.version}」至 Space 主题库；本节点当前主题未改变。';
    });
  });

  Future<void> _preview(core_proxy.ThemeLibraryEntry entry) async {
    if (_busy) return;
    final selected = widget.controller.selection;
    final isActive =
        selected?.themeId == entry.themeId &&
        selected?.archiveSha256 == entry.archiveSha256;
    var applyAfterPreview = false;

    await _run('正在准备「${entry.displayName}」独立沙箱预览…', () async {
      final visual = await widget.controller.library.prepare(
        entry.themeId,
        entry.archiveSha256,
        expectedVersion: entry.version,
      );
      try {
        if (!mounted) return;
        final shouldApply = await showDialog<bool>(
          context: context,
          builder: (_) => _ThemePreviewDialog(
            visual: visual,
            settings: widget.controller.themePreferenceSnapshot,
            entry: entry,
            isActiveOnNode: isActive,
          ),
        );
        applyAfterPreview = shouldApply == true;
      } finally {
        visual.dispose();
      }
    });

    if (applyAfterPreview && mounted) {
      await _apply(entry, skipConfirm: true);
    }
  }

  Future<void> _apply(
    core_proxy.ThemeLibraryEntry entry, {
    bool skipConfirm = false,
  }) => _run('正在校验并应用「${entry.displayName}」到本节点…', () async {
    if (!skipConfirm) {
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (_) => _ThemeApplyConfirmDialog(entry: entry),
      );
      if (confirmed != true || !mounted) return;
    }
    final receipt = await widget.controller.applyLibraryTheme(entry);
    if (!mounted) return;
    setState(() {
      _result =
          '已在本节点启用主题：${receipt.entry.displayName} v${receipt.entry.version}';
    });
  });

  Future<void> _delete(core_proxy.ThemeLibraryItem item) async {
    final selected = widget.controller.selection;
    final isActive =
        selected?.themeId == item.themeId &&
        selected?.archiveSha256 == item.archiveSha256;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (_) => _ThemeDeleteConfirmDialog(
        item: item,
        isActiveOnNode: isActive,
      ),
    );
    if (confirmed != true || !mounted) return;
    final label = item.entry != null
        ? '${item.entry!.displayName} v${item.entry!.version}'
        : item.themeId;
    await _run('正在从 Space 主题库删除「$label」…', () async {
      await widget.controller.deleteLibraryTheme(
        themeId: item.themeId,
        archiveSha256: item.archiveSha256,
        unselectIfActive: isActive,
      );
      if (!mounted) return;
      setState(() {
        _result = isActive
            ? '已切回个人外观，并从 Space 主题库删除「$label」。'
            : '已从 Space 主题库删除「$label」。';
      });
    });
  }

  Future<void> _personal() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => OperitDialogScaffold(
        title: '切回本节点个人外观',
        icon: const Icon(Icons.person_outline_rounded),
        maxWidth: 440,
        showCloseButton: true,
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('取消'),
          ),
          FilledButton.icon(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            icon: const Icon(Icons.person_outline_rounded, size: 16),
            label: const Text('确认切换'),
          ),
        ],
        child: const Text(
          '这会解除本节点当前绑定的主题包，并恢复您在本节点配置的个人自定义外观。\n\n'
          'Space 主题库中已导入的主题包会被完整保留，随时可再次应用。',
        ),
      ),
    );
    if (confirmed != true || !mounted) return;
    await _run('正在切换至本节点个人外观…', () async {
      await widget.controller.selectPersonalAppearance();
      if (!mounted) return;
      setState(() => _result = '已切回本节点个人自定义外观。');
    });
  }

  Future<void> _showDetails(core_proxy.ThemeLibraryItem item) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _ThemePackageDetailsDialog(
        item: item,
        onCopy: _copyToClipboard,
      ),
    );
  }

  List<core_proxy.ThemeLibraryItem> _filterAndSort(
    List<core_proxy.ThemeLibraryItem> rawItems,
    core_proxy.ThemeSelection? selection,
  ) {
    final query = _searchController.text.trim().toLowerCase();
    final filtered = rawItems.where((item) {
      final isActive =
          selection?.themeId == item.themeId &&
          selection?.archiveSha256 == item.archiveSha256;
      final isReady =
          item.availability == core_proxy.ThemeLibraryAvailability.ready &&
          item.entry != null;
      switch (_filter) {
        case _ThemeLibraryFilter.all:
          break;
        case _ThemeLibraryFilter.active:
          if (!isActive) return false;
        case _ThemeLibraryFilter.ready:
          if (!isReady) return false;
        case _ThemeLibraryFilter.attention:
          if (isReady) return false;
      }
      if (query.isEmpty) return true;
      final entry = item.entry;
      final haystack = <String>[
        item.themeId,
        item.archiveSha256,
        if (entry != null) ...<String>[
          entry.displayName,
          entry.version,
          entry.authorName,
          entry.license,
          ...entry.platforms,
        ],
        if (item.message != null) item.message!,
      ].join(' ').toLowerCase();
      return haystack.contains(query);
    }).toList();

    filtered.sort((left, right) {
      final leftActive =
          selection?.themeId == left.themeId &&
          selection?.archiveSha256 == left.archiveSha256;
      final rightActive =
          selection?.themeId == right.themeId &&
          selection?.archiveSha256 == right.archiveSha256;
      switch (_sort) {
        case _ThemeLibrarySort.activeFirst:
          if (leftActive != rightActive) {
            return leftActive ? -1 : 1;
          }
          final leftName = (left.entry?.displayName ?? left.themeId).toLowerCase();
          final rightName =
              (right.entry?.displayName ?? right.themeId).toLowerCase();
          final byName = leftName.compareTo(rightName);
          if (byName != 0) return byName;
          return (right.entry?.version ?? '').compareTo(left.entry?.version ?? '');
        case _ThemeLibrarySort.name:
          final leftName = (left.entry?.displayName ?? left.themeId).toLowerCase();
          final rightName =
              (right.entry?.displayName ?? right.themeId).toLowerCase();
          return leftName.compareTo(rightName);
        case _ThemeLibrarySort.size:
          final leftSize = left.entry?.archiveBytes ?? 0;
          final rightSize = right.entry?.archiveBytes ?? 0;
          return rightSize.compareTo(leftSize);
      }
    });
    return filtered;
  }

  @override
  Widget build(BuildContext context) {
    final selection = widget.controller.selection;

    return FutureBuilder<List<core_proxy.ThemeLibraryItem>>(
      future: _items,
      builder: (context, snapshot) {
        final rawItems = snapshot.data ?? const <core_proxy.ThemeLibraryItem>[];
        core_proxy.ThemeLibraryEntry? activeEntry;
        if (selection != null) {
          for (final item in rawItems) {
            if (item.themeId == selection.themeId &&
                item.archiveSha256 == selection.archiveSha256 &&
                item.entry != null) {
              activeEntry = item.entry;
              break;
            }
          }
        }

        return SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 28),
          child: Align(
            alignment: Alignment.topCenter,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 1180),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  _NodeThemeOverviewCard(
                    controller: widget.controller,
                    selection: selection,
                    activeEntry: activeEntry,
                    busy: _busy,
                    onImport: _import,
                    onReload: () => _run(
                      '正在刷新主题库并重新校验本节点选择…',
                      () => widget.controller.reloadTheme(),
                    ),
                    onSelectPersonal: _personal,
                    onPreviewActive: activeEntry != null
                        ? () => _preview(activeEntry!)
                        : null,
                    onCopyHash: (hash) => _copyToClipboard('SHA-256', hash),
                  ),
                  if (_busy || _error != null || _result != null) ...<Widget>[
                    const SizedBox(height: 10),
                    _OperationStatusBanner(
                      busy: _busy,
                      busyMessage: _busyMessage,
                      error: _error,
                      result: _result,
                      onDismissError: () => setState(() => _error = null),
                      onDismissResult: () => setState(() => _result = null),
                      onCopyError: _error != null
                          ? () => _copyToClipboard('错误信息', _error!)
                          : null,
                    ),
                  ],
                  const SizedBox(height: 12),
                  _ThemeLibraryToolbar(
                    totalCount: rawItems.length,
                    readyCount: rawItems
                        .where(
                          (item) =>
                              item.availability ==
                                  core_proxy.ThemeLibraryAvailability.ready &&
                              item.entry != null,
                        )
                        .length,
                    attentionCount: rawItems
                        .where(
                          (item) =>
                              item.availability !=
                                  core_proxy.ThemeLibraryAvailability.ready ||
                              item.entry == null,
                        )
                        .length,
                    hasActiveSelection: selection != null,
                    filter: _filter,
                    sort: _sort,
                    searchController: _searchController,
                    onFilterChanged: (next) => setState(() => _filter = next),
                    onSortChanged: (next) => setState(() => _sort = next),
                    onSearchChanged: (_) => setState(() {}),
                    onClearSearch: () {
                      _searchController.clear();
                      setState(() {});
                    },
                  ),
                  const SizedBox(height: 12),
                  _buildLibraryBody(context, snapshot, rawItems, selection),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildLibraryBody(
    BuildContext context,
    AsyncSnapshot<List<core_proxy.ThemeLibraryItem>> state,
    List<core_proxy.ThemeLibraryItem> rawItems,
    core_proxy.ThemeSelection? selection,
  ) {
    final colorScheme = Theme.of(context).colorScheme;
    if (state.hasError) {
      return OperitGlassSurface(
        color: colorScheme.errorContainer.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: colorScheme.error.withValues(alpha: 0.35),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(Icons.error_outline_rounded, color: colorScheme.error),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Space 主题库读取失败',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colorScheme.onErrorContainer,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              SelectableText(
                '${state.error}',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onErrorContainer,
                ),
              ),
              const SizedBox(height: 12),
              FilledButton.tonalIcon(
                onPressed: _busy
                    ? null
                    : () => setState(() {
                        _items = widget.controller.listThemeLibrary();
                      }),
                icon: const Icon(Icons.refresh_rounded, size: 16),
                label: const Text('重试读取'),
              ),
            ],
          ),
        ),
      );
    }

    if (state.connectionState == ConnectionState.waiting && state.data == null) {
      return OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.34),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: const Padding(
          padding: EdgeInsets.symmetric(vertical: 48, horizontal: 20),
          child: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                SizedBox.square(
                  dimension: 28,
                  child: CircularProgressIndicator(strokeWidth: 2.5),
                ),
                SizedBox(height: 12),
                Text('正在加载 Space 主题库…'),
              ],
            ),
          ),
        ),
      );
    }

    if (rawItems.isEmpty) {
      return _EmptyThemeLibraryCard(
        busy: _busy,
        onImport: _import,
      );
    }

    final visibleItems = _filterAndSort(rawItems, selection);
    if (visibleItems.isEmpty) {
      return OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.34),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 36, horizontal: 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                Icons.filter_alt_off_outlined,
                size: 36,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(height: 10),
              Text(
                '没有符合当前筛选或搜索条件的主题包',
                style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                '共 ${rawItems.length} 个已入库主题包被当前过滤条件隐藏。',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 14),
              FilledButton.tonalIcon(
                onPressed: () {
                  _searchController.clear();
                  setState(() => _filter = _ThemeLibraryFilter.all);
                },
                icon: const Icon(Icons.restart_alt_rounded, size: 16),
                label: const Text('重置筛选条件'),
              ),
            ],
          ),
        ),
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final cards = <Widget>[
          for (final item in visibleItems)
            _ThemePackageCard(
              key: ValueKey<String>('${item.themeId}@${item.archiveSha256}'),
              library: widget.controller.library,
              item: item,
              active: selection?.themeId == item.themeId &&
                  selection?.archiveSha256 == item.archiveSha256,
              busy: _busy,
              onPreview: item.entry != null &&
                      item.availability ==
                          core_proxy.ThemeLibraryAvailability.ready
                  ? () => _preview(item.entry!)
                  : null,
              onApply: item.entry != null &&
                      item.availability ==
                          core_proxy.ThemeLibraryAvailability.ready
                  ? () => _apply(item.entry!)
                  : null,
              onShowDetails: () => _showDetails(item),
              onDelete: () => _delete(item),
              onCopyHash: () => _copyToClipboard('SHA-256', item.archiveSha256),
            ),
        ];

        if (constraints.maxWidth >= 1040) {
          final rows = <Widget>[];
          for (var i = 0; i < cards.length; i += 2) {
            rows.add(
              Padding(
                padding: EdgeInsets.only(
                  bottom: i + 2 < cards.length ? 12 : 0,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Expanded(child: cards[i]),
                    const SizedBox(width: 12),
                    Expanded(
                      child: i + 1 < cards.length
                          ? cards[i + 1]
                          : const SizedBox.shrink(),
                    ),
                  ],
                ),
              ),
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: rows,
          );
        }

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            for (var i = 0; i < cards.length; i++) ...<Widget>[
              cards[i],
              if (i + 1 < cards.length) const SizedBox(height: 12),
            ],
          ],
        );
      },
    );
  }
}

/// Rounded-rectangle thumbnail that loads the custom background image
/// declared in the theme package (`background.asset` in `theme.json`).
class _ThemeBackgroundThumbnail extends StatefulWidget {
  const _ThemeBackgroundThumbnail({
    required this.library,
    required this.themeId,
    required this.archiveSha256,
    required this.ready,
    required this.active,
    this.width = 184,
    this.height = 124,
    this.onTap,
  });

  final ThemeLibraryService library;
  final String themeId;
  final String archiveSha256;
  final bool ready;
  final bool active;
  final double width;
  final double height;
  final VoidCallback? onTap;

  @override
  State<_ThemeBackgroundThumbnail> createState() =>
      _ThemeBackgroundThumbnailState();
}

class _ThemeBackgroundThumbnailState extends State<_ThemeBackgroundThumbnail> {
  Future<Uint8List?>? _future;

  @override
  void initState() {
    super.initState();
    _bindFuture();
  }

  @override
  void didUpdateWidget(covariant _ThemeBackgroundThumbnail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.themeId != widget.themeId ||
        oldWidget.archiveSha256 != widget.archiveSha256 ||
        oldWidget.ready != widget.ready ||
        !identical(oldWidget.library, widget.library)) {
      _bindFuture();
    }
  }

  void _bindFuture() {
    if (!widget.ready) {
      _future = null;
      return;
    }
    _future = widget.library.readBackgroundImageBytes(
      themeId: widget.themeId,
      archiveSha256: widget.archiveSha256,
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final radius = BorderRadius.circular(12);
    final resolvedSync =
        widget.library.hasResolvedBackgroundImage(widget.archiveSha256);
    final cachedBytes =
        widget.library.cachedBackgroundImageBytes(widget.archiveSha256);

    Widget buildSurface(Uint8List? bytes, {required bool loading}) {
      return SizedBox(
        width: widget.width,
        height: widget.height,
        child: DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: radius,
            border: Border.all(
              color: widget.active
                  ? colorScheme.primary.withValues(alpha: isDark ? 0.72 : 0.62)
                  : colorScheme.outlineVariant.withValues(alpha: 0.32),
              width: widget.active ? 1.5 : 1.0,
            ),
            boxShadow: <BoxShadow>[
              BoxShadow(
                color: Colors.black.withValues(alpha: isDark ? 0.24 : 0.10),
                blurRadius: 10,
                offset: const Offset(0, 3),
              ),
            ],
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(10.8),
            child: Stack(
              fit: StackFit.expand,
              children: <Widget>[
                if (bytes != null && bytes.isNotEmpty)
                  Image.memory(
                    bytes,
                    fit: BoxFit.cover,
                    width: widget.width,
                    height: widget.height,
                    cacheWidth: 480,
                    filterQuality: FilterQuality.medium,
                    gaplessPlayback: true,
                  )
                else
                  DecoratedBox(
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                        colors: <Color>[
                          colorScheme.primaryContainer.withValues(alpha: 0.55),
                          colorScheme.secondaryContainer.withValues(alpha: 0.45),
                          colorScheme.surfaceContainerHighest.withValues(
                            alpha: 0.65,
                          ),
                        ],
                      ),
                    ),
                    child: Center(
                      child: loading
                          ? SizedBox.square(
                              dimension: 20,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: colorScheme.primary,
                              ),
                            )
                          : Column(
                              mainAxisSize: MainAxisSize.min,
                              children: <Widget>[
                                Icon(
                                  widget.ready
                                      ? Icons.wallpaper_outlined
                                      : Icons.cloud_sync_outlined,
                                  size: 22,
                                  color: colorScheme.onSurfaceVariant,
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  widget.ready ? '纯色无背景图' : '等待同步',
                                  style: textTheme.labelSmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ],
                            ),
                    ),
                  ),
                Positioned(
                  left: 0,
                  right: 0,
                  bottom: 0,
                  height: 36,
                  child: IgnorePointer(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          colors: <Color>[
                            Colors.transparent,
                            Colors.black.withValues(alpha: 0.46),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                if (widget.active)
                  Positioned(
                    left: 7,
                    top: 7,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: colorScheme.primary.withValues(alpha: 0.90),
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 7,
                          vertical: 2.5,
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            Icon(
                              Icons.check_rounded,
                              size: 12,
                              color: colorScheme.onPrimary,
                            ),
                            const SizedBox(width: 3),
                            Text(
                              '使用中',
                              style: textTheme.labelSmall?.copyWith(
                                color: colorScheme.onPrimary,
                                fontWeight: FontWeight.w700,
                                fontSize: 10.5,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                if (widget.onTap != null)
                  Positioned(
                    right: 7,
                    bottom: 6,
                    child: IgnorePointer(
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.52),
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: const Padding(
                          padding: EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 2.5,
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: <Widget>[
                              Icon(
                                Icons.zoom_out_map_rounded,
                                size: 11,
                                color: Colors.white,
                              ),
                              SizedBox(width: 3),
                              Text(
                                '预览',
                                style: TextStyle(
                                  color: Colors.white,
                                  fontSize: 10.5,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),
                if (widget.onTap != null)
                  Material(
                    color: Colors.transparent,
                    child: InkWell(
                      onTap: widget.onTap,
                      child: const SizedBox.expand(),
                    ),
                  ),
              ],
            ),
          ),
        ),
      );
    }

    if (resolvedSync) {
      return buildSurface(cachedBytes, loading: false);
    }
    if (_future == null) {
      return buildSurface(null, loading: false);
    }
    return FutureBuilder<Uint8List?>(
      future: _future,
      builder: (context, snapshot) {
        return buildSurface(
          snapshot.data,
          loading: snapshot.connectionState == ConnectionState.waiting,
        );
      },
    );
  }
}

class _NodeThemeOverviewCard extends StatelessWidget {
  const _NodeThemeOverviewCard({
    required this.controller,
    required this.selection,
    required this.activeEntry,
    required this.busy,
    required this.onImport,
    required this.onReload,
    required this.onSelectPersonal,
    required this.onPreviewActive,
    required this.onCopyHash,
  });

  final OperitThemeController controller;
  final core_proxy.ThemeSelection? selection;
  final core_proxy.ThemeLibraryEntry? activeEntry;
  final bool busy;
  final VoidCallback onImport;
  final VoidCallback onReload;
  final VoidCallback onSelectPersonal;
  final VoidCallback? onPreviewActive;
  final ValueChanged<String> onCopyHash;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final hasPackage = selection != null;

    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(
        alpha: isDark ? 0.42 : 0.48,
      ),
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: hasPackage
            ? colorScheme.primary.withValues(alpha: isDark ? 0.55 : 0.45)
            : colorScheme.outlineVariant.withValues(alpha: 0.22),
        width: hasPackage ? 1.4 : 1.0,
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final isNarrow = constraints.maxWidth < 580;
            final showSegmentIcons = constraints.maxWidth >= 380;

            final modeSelector = SettingsSegmentedSelector<ThemeMode>(
              value: controller.themeMode,
              onChanged: (mode) {
                unawaited(controller.setThemeMode(mode));
              },
              segments: <ButtonSegment<ThemeMode>>[
                ButtonSegment<ThemeMode>(
                  value: ThemeMode.light,
                  icon: showSegmentIcons
                      ? const Icon(Icons.light_mode_outlined, size: 14)
                      : null,
                  label: const Text('亮色', maxLines: 1, softWrap: false),
                ),
                ButtonSegment<ThemeMode>(
                  value: ThemeMode.dark,
                  icon: showSegmentIcons
                      ? const Icon(Icons.dark_mode_outlined, size: 14)
                      : null,
                  label: const Text('暗色', maxLines: 1, softWrap: false),
                ),
                ButtonSegment<ThemeMode>(
                  value: ThemeMode.system,
                  icon: showSegmentIcons
                      ? const Icon(Icons.brightness_auto_outlined, size: 14)
                      : null,
                  label: const Text('系统', maxLines: 1, softWrap: false),
                ),
              ],
            );

            final headerRow = Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                if (hasPackage)
                  _ThemeBackgroundThumbnail(
                    library: controller.library,
                    themeId: selection!.themeId,
                    archiveSha256: selection!.archiveSha256,
                    ready: true,
                    active: true,
                    width: isNarrow ? 108 : 144,
                    height: isNarrow ? 76 : 94,
                    onTap: onPreviewActive,
                  )
                else
                  Container(
                    width: 46,
                    height: 46,
                    decoration: BoxDecoration(
                      color: colorScheme.secondaryContainer.withValues(
                        alpha: 0.65,
                      ),
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: colorScheme.outlineVariant.withValues(
                          alpha: 0.30,
                        ),
                      ),
                    ),
                    child: Icon(
                      Icons.auto_fix_high_rounded,
                      color: colorScheme.onSecondaryContainer,
                      size: 22,
                    ),
                  ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                Wrap(
                                  spacing: 6,
                                  runSpacing: 4,
                                  crossAxisAlignment: WrapCrossAlignment.center,
                                  children: <Widget>[
                                    _StatusTagPill(
                                      icon: hasPackage
                                          ? Icons.check_circle_rounded
                                          : Icons.tune_rounded,
                                      label: hasPackage ? '本节点生效中' : '自由定制模式',
                                      foreground: hasPackage
                                          ? colorScheme.primary
                                          : colorScheme.secondary,
                                      background: hasPackage
                                          ? colorScheme.primary.withValues(
                                              alpha: 0.14,
                                            )
                                          : colorScheme.secondary.withValues(
                                              alpha: 0.14,
                                            ),
                                    ),
                                    if (hasPackage)
                                      SettingsInfoBadge(
                                        label: 'v${selection!.version}',
                                      ),
                                  ],
                                ),
                                const SizedBox(height: 5),
                                Text(
                                  hasPackage
                                      ? (activeEntry?.displayName ??
                                            selection!.themeId)
                                      : '本节点个人自定义外观',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: textTheme.titleMedium?.copyWith(
                                    fontWeight: FontWeight.w800,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          if (!isNarrow) ...<Widget>[
                            const SizedBox(width: 8),
                            modeSelector,
                          ],
                        ],
                      ),
                      const SizedBox(height: 4),
                      if (hasPackage) ...<Widget>[
                        Text(
                          selection!.themeId,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.labelSmall?.copyWith(
                            fontFamily: 'Cascadia Mono',
                            color: colorScheme.onSurfaceVariant,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        if (!isNarrow) ...<Widget>[
                          const SizedBox(height: 4),
                          Wrap(
                            spacing: 8,
                            runSpacing: 4,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: <Widget>[
                              if (activeEntry != null)
                                Text(
                                  '${activeEntry!.authorName} · ${activeEntry!.license} · ${_formatBytes(activeEntry!.archiveBytes)}',
                                  style: textTheme.bodySmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                  ),
                                ),
                              _CopyableHashChip(
                                hash: selection!.archiveSha256,
                                onTap: () => onCopyHash(selection!.archiveSha256),
                              ),
                            ],
                          ),
                        ],
                      ] else
                        Text(
                          '主题包存储于 Space，应用状态仅归属当前 CoreNode，不绑定角色卡或群组。',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            );

            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                headerRow,
                if (isNarrow && hasPackage) ...<Widget>[
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      if (activeEntry != null) ...<Widget>[
                        _MetaBadge(
                          icon: Icons.person_outline_rounded,
                          label: activeEntry!.authorName,
                        ),
                        _MetaBadge(
                          icon: Icons.balance_rounded,
                          label: activeEntry!.license,
                        ),
                        _MetaBadge(
                          icon: Icons.folder_zip_outlined,
                          label: _formatBytes(activeEntry!.archiveBytes),
                        ),
                      ],
                      _CopyableHashChip(
                        hash: selection!.archiveSha256,
                        onTap: () => onCopyHash(selection!.archiveSha256),
                      ),
                    ],
                  ),
                ],
                if (isNarrow) ...<Widget>[
                  const SizedBox(height: 10),
                  SizedBox(
                    width: double.infinity,
                    child: modeSelector,
                  ),
                ],
                const SizedBox(height: 10),
                Divider(
                  height: 1,
                  color: colorScheme.outlineVariant.withValues(alpha: 0.16),
                ),
                const SizedBox(height: 10),
                if (isNarrow)
                  _buildCompactOverviewActions()
                else
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: <Widget>[
                      FilledButton.icon(
                        style: SettingsControlStyles.sectionTextButton(),
                        onPressed: busy ? null : onImport,
                        icon: const Icon(Icons.add_box_outlined, size: 16),
                        label: const Text('导入 .optheme'),
                      ),
                      FilledButton.tonalIcon(
                        style: SettingsControlStyles.sectionTextButton(),
                        onPressed: busy ? null : onReload,
                        icon: const Icon(Icons.refresh_rounded, size: 16),
                        label: const Text('刷新与重验'),
                      ),
                      if (onPreviewActive != null)
                        OutlinedButton.icon(
                          style: SettingsControlStyles.sectionTextButton(),
                          onPressed: busy ? null : onPreviewActive,
                          icon: const Icon(
                            Icons.visibility_outlined,
                            size: 16,
                          ),
                          label: const Text('预览当前主题'),
                        ),
                      if (hasPackage)
                        OutlinedButton.icon(
                          style: SettingsControlStyles.sectionTextButton(),
                          onPressed: busy ? null : onSelectPersonal,
                          icon: const Icon(
                            Icons.person_outline_rounded,
                            size: 16,
                          ),
                          label: const Text('切回个人外观'),
                        ),
                    ],
                  ),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _buildCompactOverviewActions() {
    final hasPackage = selection != null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: FilledButton.icon(
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: busy ? null : onImport,
                icon: const Icon(Icons.add_box_outlined, size: 15),
                label: const Text(
                  '导入 .optheme',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: FilledButton.tonalIcon(
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: busy ? null : onReload,
                icon: const Icon(Icons.refresh_rounded, size: 15),
                label: const Text(
                  '刷新与重验',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
          ],
        ),
        if (onPreviewActive != null || hasPackage) ...<Widget>[
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              if (onPreviewActive != null)
                Expanded(
                  child: OutlinedButton.icon(
                    style: SettingsControlStyles.sectionTextButton(),
                    onPressed: busy ? null : onPreviewActive,
                    icon: const Icon(Icons.visibility_outlined, size: 15),
                    label: const Text(
                      '预览当前主题',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ),
              if (onPreviewActive != null && hasPackage)
                const SizedBox(width: 8),
              if (hasPackage)
                Expanded(
                  child: OutlinedButton.icon(
                    style: SettingsControlStyles.sectionTextButton(),
                    onPressed: busy ? null : onSelectPersonal,
                    icon: const Icon(Icons.person_outline_rounded, size: 15),
                    label: const Text(
                      '切回个人外观',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ),
            ],
          ),
        ],
      ],
    );
  }
}

class _CopyableHashChip extends StatelessWidget {
  const _CopyableHashChip({
    required this.hash,
    required this.onTap,
  });

  final String hash;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Material(
      color: colorScheme.surface.withValues(alpha: isDark ? 0.32 : 0.52),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(7),
        side: BorderSide(
          color: colorScheme.outlineVariant.withValues(alpha: 0.24),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                Icons.fingerprint_rounded,
                size: 13,
                color: colorScheme.primary,
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text(
                  _shortHash(hash, head: 8, tail: 6),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelSmall?.copyWith(
                    fontFamily: 'Cascadia Mono',
                    color: colorScheme.onSurfaceVariant,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                Icons.copy_rounded,
                size: 12,
                color: colorScheme.primary,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _OperationStatusBanner extends StatelessWidget {
  const _OperationStatusBanner({
    required this.busy,
    required this.busyMessage,
    required this.error,
    required this.result,
    required this.onDismissError,
    required this.onDismissResult,
    this.onCopyError,
  });

  final bool busy;
  final String? busyMessage;
  final String? error;
  final String? result;
  final VoidCallback onDismissError;
  final VoidCallback onDismissResult;
  final VoidCallback? onCopyError;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    if (busy) {
      return OperitGlassSurface(
        color: colorScheme.primaryContainer.withValues(alpha: 0.34),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.primary.withValues(alpha: 0.30),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          child: Row(
            children: <Widget>[
              SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(
                  strokeWidth: 2.2,
                  color: colorScheme.primary,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  busyMessage ?? '正在处理主题包操作…',
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurface,
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    }

    if (error != null) {
      return OperitGlassSurface(
        color: colorScheme.errorContainer.withValues(alpha: 0.45),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.error.withValues(alpha: 0.42),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 10, 8, 10),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Icon(
                  Icons.error_outline_rounded,
                  size: 19,
                  color: colorScheme.error,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      '操作未完成',
                      style: textTheme.labelLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colorScheme.error,
                      ),
                    ),
                    const SizedBox(height: 2),
                    SelectableText(
                      error!,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onErrorContainer,
                      ),
                    ),
                  ],
                ),
              ),
              if (onCopyError != null)
                IconButton(
                  visualDensity: VisualDensity.compact,
                  tooltip: '复制错误信息',
                  onPressed: onCopyError,
                  icon: Icon(
                    Icons.copy_rounded,
                    size: 16,
                    color: colorScheme.onErrorContainer,
                  ),
                ),
              IconButton(
                visualDensity: VisualDensity.compact,
                tooltip: '关闭提示',
                onPressed: onDismissError,
                icon: Icon(
                  Icons.close_rounded,
                  size: 18,
                  color: colorScheme.onErrorContainer,
                ),
              ),
            ],
          ),
        ),
      );
    }

    return OperitGlassSurface(
      color: colorScheme.primaryContainer.withValues(alpha: 0.38),
      borderRadius: BorderRadius.circular(12),
      border: Border.all(
        color: colorScheme.primary.withValues(alpha: 0.32),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(14, 8, 8, 8),
        child: Row(
          children: <Widget>[
            Icon(
              Icons.check_circle_outline_rounded,
              size: 19,
              color: colorScheme.primary,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                result ?? '',
                style: textTheme.bodyMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colorScheme.onSurface,
                ),
              ),
            ),
            IconButton(
              visualDensity: VisualDensity.compact,
              tooltip: '关闭提示',
              onPressed: onDismissResult,
              icon: Icon(
                Icons.close_rounded,
                size: 18,
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ThemeLibraryToolbar extends StatelessWidget {
  const _ThemeLibraryToolbar({
    required this.totalCount,
    required this.readyCount,
    required this.attentionCount,
    required this.hasActiveSelection,
    required this.filter,
    required this.sort,
    required this.searchController,
    required this.onFilterChanged,
    required this.onSortChanged,
    required this.onSearchChanged,
    required this.onClearSearch,
  });

  final int totalCount;
  final int readyCount;
  final int attentionCount;
  final bool hasActiveSelection;
  final _ThemeLibraryFilter filter;
  final _ThemeLibrarySort sort;
  final TextEditingController searchController;
  final ValueChanged<_ThemeLibraryFilter> onFilterChanged;
  final ValueChanged<_ThemeLibrarySort> onSortChanged;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onClearSearch;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    Widget buildSortButton() {
      return PopupMenuButton<_ThemeLibrarySort>(
        tooltip: '排序方式',
        initialValue: sort,
        onSelected: onSortChanged,
        itemBuilder: (context) => const <PopupMenuEntry<_ThemeLibrarySort>>[
          PopupMenuItem<_ThemeLibrarySort>(
            value: _ThemeLibrarySort.activeFirst,
            child: Text('当前启用优先'),
          ),
          PopupMenuItem<_ThemeLibrarySort>(
            value: _ThemeLibrarySort.name,
            child: Text('按主题名称排序'),
          ),
          PopupMenuItem<_ThemeLibrarySort>(
            value: _ThemeLibrarySort.size,
            child: Text('按包体积排序'),
          ),
        ],
        child: Container(
          height: 32,
          padding: const EdgeInsets.symmetric(horizontal: 10),
          decoration: BoxDecoration(
            color: colorScheme.surface.withValues(alpha: isDark ? 0.35 : 0.55),
            borderRadius: BorderRadius.circular(9),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.30),
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                Icons.sort_rounded,
                size: 15,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 5),
              Text(
                switch (sort) {
                  _ThemeLibrarySort.activeFirst => '启用优先',
                  _ThemeLibrarySort.name => '按名称',
                  _ThemeLibrarySort.size => '按体积',
                },
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colorScheme.onSurface,
                ),
              ),
            ],
          ),
        ),
      );
    }

    Widget buildSearchField({double? width}) {
      return SizedBox(
        width: width,
        height: 34,
        child: TextField(
          controller: searchController,
          onChanged: onSearchChanged,
          style: Theme.of(context).textTheme.bodySmall,
          decoration: InputDecoration(
            isDense: true,
            hintText: '搜索名称 / ID / 作者 / 哈希…',
            hintStyle: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant.withValues(alpha: 0.72),
            ),
            prefixIcon: Icon(
              Icons.search_rounded,
              size: 16,
              color: colorScheme.onSurfaceVariant,
            ),
            prefixIconConstraints: const BoxConstraints(
              minWidth: 32,
              minHeight: 32,
            ),
            suffixIcon: searchController.text.isNotEmpty
                ? IconButton(
                    visualDensity: VisualDensity.compact,
                    padding: EdgeInsets.zero,
                    onPressed: onClearSearch,
                    icon: Icon(
                      Icons.close_rounded,
                      size: 15,
                      color: colorScheme.onSurfaceVariant,
                    ),
                  )
                : null,
            suffixIconConstraints: const BoxConstraints(
              minWidth: 28,
              minHeight: 28,
            ),
            filled: true,
            fillColor: colorScheme.surface.withValues(
              alpha: isDark ? 0.35 : 0.55,
            ),
            contentPadding: const EdgeInsets.symmetric(
              horizontal: 10,
              vertical: 7,
            ),
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(9),
              borderSide: BorderSide(
                color: colorScheme.outlineVariant.withValues(alpha: 0.30),
              ),
            ),
            enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(9),
              borderSide: BorderSide(
                color: colorScheme.outlineVariant.withValues(alpha: 0.30),
              ),
            ),
          ),
        ),
      );
    }

    final filterChips = <Widget>[
      SettingsToggleChip(
        label: '全部 ($totalCount)',
        selected: filter == _ThemeLibraryFilter.all,
        onSelected: (_) => onFilterChanged(_ThemeLibraryFilter.all),
      ),
      if (hasActiveSelection)
        SettingsToggleChip(
          icon: Icons.check_circle_outline_rounded,
          label: '当前启用',
          selected: filter == _ThemeLibraryFilter.active,
          onSelected: (_) => onFilterChanged(_ThemeLibraryFilter.active),
        ),
      SettingsToggleChip(
        icon: Icons.verified_outlined,
        label: '就绪 ($readyCount)',
        selected: filter == _ThemeLibraryFilter.ready,
        onSelected: (_) => onFilterChanged(_ThemeLibraryFilter.ready),
      ),
      if (attentionCount > 0 || filter == _ThemeLibraryFilter.attention)
        SettingsToggleChip(
          icon: Icons.sync_problem_outlined,
          label: '同步/异常 ($attentionCount)',
          selected: filter == _ThemeLibraryFilter.attention,
          onSelected: (_) => onFilterChanged(_ThemeLibraryFilter.attention),
        ),
    ];

    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(
        alpha: isDark ? 0.34 : 0.40,
      ),
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.18),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final isCompact = constraints.maxWidth < 640;
            if (isCompact) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      Icon(
                        Icons.inventory_2_outlined,
                        size: 18,
                        color: colorScheme.primary,
                      ),
                      const SizedBox(width: 6),
                      Text(
                        'Space 主题库',
                        style: SettingsControlStyles.sectionTitleTextStyle(
                          context,
                        ),
                      ),
                      const SizedBox(width: 8),
                      SettingsInfoBadge(label: '共 $totalCount 个'),
                      const Spacer(),
                      buildSortButton(),
                    ],
                  ),
                  const SizedBox(height: 8),
                  buildSearchField(width: double.infinity),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: filterChips,
                  ),
                ],
              );
            }

            return Wrap(
              spacing: 12,
              runSpacing: 10,
              alignment: WrapAlignment.spaceBetween,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: <Widget>[
                    Icon(
                      Icons.inventory_2_outlined,
                      size: 18,
                      color: colorScheme.primary,
                    ),
                    Text(
                      'Space 主题库',
                      style: SettingsControlStyles.sectionTitleTextStyle(
                        context,
                      ),
                    ),
                    SettingsInfoBadge(label: '共 $totalCount 个'),
                    const SizedBox(width: 2),
                    ...filterChips,
                  ],
                ),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: <Widget>[
                    buildSortButton(),
                    buildSearchField(width: 220),
                  ],
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _ThemePackageCard extends StatelessWidget {
  const _ThemePackageCard({
    super.key,
    required this.library,
    required this.item,
    required this.active,
    required this.busy,
    required this.onPreview,
    required this.onApply,
    required this.onShowDetails,
    required this.onDelete,
    required this.onCopyHash,
  });

  final ThemeLibraryService library;
  final core_proxy.ThemeLibraryItem item;
  final bool active;
  final bool busy;
  final VoidCallback? onPreview;
  final VoidCallback? onApply;
  final VoidCallback onShowDetails;
  final VoidCallback onDelete;
  final VoidCallback onCopyHash;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final entry = item.entry;
    final ready =
        item.availability == core_proxy.ThemeLibraryAvailability.ready &&
        entry != null;
    final waiting =
        item.availability == core_proxy.ThemeLibraryAvailability.waitingForSync;

    final Color borderColor;
    if (active) {
      borderColor = colorScheme.primary.withValues(alpha: isDark ? 0.72 : 0.62);
    } else if (!ready && !waiting) {
      borderColor = colorScheme.error.withValues(alpha: 0.48);
    } else {
      borderColor = colorScheme.outlineVariant.withValues(alpha: 0.22);
    }

    final Color cardTint;
    if (active) {
      cardTint = Color.alphaBlend(
        colorScheme.primary.withValues(alpha: isDark ? 0.12 : 0.08),
        colorScheme.surfaceContainerHighest.withValues(
          alpha: isDark ? 0.44 : 0.52,
        ),
      );
    } else {
      cardTint = colorScheme.surfaceContainerHighest.withValues(
        alpha: isDark ? 0.36 : 0.44,
      );
    }

    return OperitGlassSurface(
      color: cardTint,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: borderColor,
        width: active ? 1.6 : 1.0,
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final isNarrow = constraints.maxWidth < 560;

            if (isNarrow) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      _ThemeBackgroundThumbnail(
                        library: library,
                        themeId: item.themeId,
                        archiveSha256: item.archiveSha256,
                        ready: ready,
                        active: active,
                        width: 108,
                        height: 76,
                        onTap: ready && !busy ? onPreview : null,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Wrap(
                              spacing: 6,
                              runSpacing: 4,
                              crossAxisAlignment: WrapCrossAlignment.center,
                              children: <Widget>[
                                _buildAvailabilityBadge(
                                  context,
                                  active,
                                  ready,
                                  waiting,
                                ),
                                if (entry != null)
                                  SettingsInfoBadge(label: 'v${entry.version}'),
                              ],
                            ),
                            const SizedBox(height: 5),
                            Text(
                              entry?.displayName ?? item.themeId,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: textTheme.titleMedium?.copyWith(
                                fontWeight: FontWeight.w800,
                              ),
                            ),
                            const SizedBox(height: 2),
                            Text(
                              item.themeId,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: textTheme.labelSmall?.copyWith(
                                fontFamily: 'Cascadia Mono',
                                color: colorScheme.onSurfaceVariant,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      if (entry != null) ...<Widget>[
                        _MetaBadge(
                          icon: Icons.person_outline_rounded,
                          label: entry.authorName,
                        ),
                        _MetaBadge(
                          icon: Icons.balance_rounded,
                          label: entry.license,
                        ),
                        _MetaBadge(
                          icon: Icons.folder_zip_outlined,
                          label: _formatBytes(entry.archiveBytes),
                        ),
                        _MetaBadge(
                          icon: Icons.perm_media_outlined,
                          label: _summarizeValidatedFiles(
                            entry.validation.files,
                          ),
                        ),
                      ],
                      _CopyableHashChip(
                        hash: item.archiveSha256,
                        onTap: onCopyHash,
                      ),
                    ],
                  ),
                  if (item.message != null || !ready) ...<Widget>[
                    const SizedBox(height: 8),
                    _buildDiagnosticCallout(context, waiting),
                  ],
                  const SizedBox(height: 10),
                  Divider(
                    height: 1,
                    color: colorScheme.outlineVariant.withValues(alpha: 0.16),
                  ),
                  const SizedBox(height: 10),
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: active
                            ? FilledButton.tonalIcon(
                                style: SettingsControlStyles.sectionTextButton(),
                                onPressed: ready && !busy ? onApply : null,
                                icon: const Icon(
                                  Icons.verified_outlined,
                                  size: 15,
                                ),
                                label: const Text(
                                  '重新校验',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              )
                            : FilledButton.icon(
                                style: SettingsControlStyles.sectionTextButton(),
                                onPressed: ready && !busy ? onApply : null,
                                icon: const Icon(
                                  Icons.check_circle_outline_rounded,
                                  size: 15,
                                ),
                                label: const Text(
                                  '应用主题',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: OutlinedButton.icon(
                          style: SettingsControlStyles.sectionTextButton(),
                          onPressed: ready && !busy ? onPreview : null,
                          icon: const Icon(
                            Icons.visibility_outlined,
                            size: 15,
                          ),
                          label: const Text(
                            '独立预览',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ),
                      const SizedBox(width: 6),
                      OutlinedButton(
                        style: SettingsControlStyles.sectionTextButton(),
                        onPressed: onShowDetails,
                        child: const Text('清单'),
                      ),
                      const SizedBox(width: 2),
                      IconButton(
                        visualDensity: VisualDensity.compact,
                        tooltip: '删除主题包',
                        onPressed: busy ? null : onDelete,
                        icon: Icon(
                          Icons.delete_outline_rounded,
                          size: 18,
                          color: colorScheme.error,
                        ),
                      ),
                    ],
                  ),
                ],
              );
            }

            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                _ThemeBackgroundThumbnail(
                  library: library,
                  themeId: item.themeId,
                  archiveSha256: item.archiveSha256,
                  ready: ready,
                  active: active,
                  width: 184,
                  height: 124,
                  onTap: ready && !busy ? onPreview : null,
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Expanded(
                            child: Wrap(
                              spacing: 8,
                              runSpacing: 4,
                              crossAxisAlignment: WrapCrossAlignment.center,
                              children: <Widget>[
                                Text(
                                  entry?.displayName ?? item.themeId,
                                  style: textTheme.titleMedium?.copyWith(
                                    fontWeight: FontWeight.w800,
                                  ),
                                ),
                                if (entry != null)
                                  SettingsInfoBadge(label: 'v${entry.version}'),
                              ],
                            ),
                          ),
                          const SizedBox(width: 8),
                          _buildAvailabilityBadge(
                            context,
                            active,
                            ready,
                            waiting,
                          ),
                        ],
                      ),
                      const SizedBox(height: 5),
                      Wrap(
                        spacing: 10,
                        runSpacing: 4,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: <Widget>[
                          SelectableText(
                            item.themeId,
                            style: textTheme.labelSmall?.copyWith(
                              fontFamily: 'Cascadia Mono',
                              color: colorScheme.onSurfaceVariant,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          _CopyableHashChip(
                            hash: item.archiveSha256,
                            onTap: onCopyHash,
                          ),
                        ],
                      ),
                      if (entry != null) ...<Widget>[
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 6,
                          runSpacing: 6,
                          children: <Widget>[
                            _MetaBadge(
                              icon: Icons.person_outline_rounded,
                              label: entry.authorName,
                            ),
                            _MetaBadge(
                              icon: Icons.balance_rounded,
                              label: entry.license,
                            ),
                            _MetaBadge(
                              icon: Icons.folder_zip_outlined,
                              label: _formatBytes(entry.archiveBytes),
                            ),
                            _MetaBadge(
                              icon: Icons.perm_media_outlined,
                              label: _summarizeValidatedFiles(
                                entry.validation.files,
                              ),
                            ),
                            _MetaBadge(
                              icon: Icons.devices_outlined,
                              label: _formatPlatforms(entry.platforms),
                            ),
                          ],
                        ),
                      ],
                      if (item.message != null || !ready) ...<Widget>[
                        const SizedBox(height: 8),
                        _buildDiagnosticCallout(context, waiting),
                      ],
                      const SizedBox(height: 10),
                      Divider(
                        height: 1,
                        color: colorScheme.outlineVariant.withValues(
                          alpha: 0.16,
                        ),
                      ),
                      const SizedBox(height: 10),
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: Wrap(
                              spacing: 8,
                              runSpacing: 6,
                              children: <Widget>[
                                if (active)
                                  FilledButton.tonalIcon(
                                    style:
                                        SettingsControlStyles.sectionTextButton(),
                                    onPressed: ready && !busy ? onApply : null,
                                    icon: const Icon(
                                      Icons.verified_outlined,
                                      size: 15,
                                    ),
                                    label: const Text('重新校验应用'),
                                  )
                                else
                                  FilledButton.icon(
                                    style:
                                        SettingsControlStyles.sectionTextButton(),
                                    onPressed: ready && !busy ? onApply : null,
                                    icon: const Icon(
                                      Icons.check_circle_outline_rounded,
                                      size: 15,
                                    ),
                                    label: const Text('应用到本节点'),
                                  ),
                                OutlinedButton.icon(
                                  style:
                                      SettingsControlStyles.sectionTextButton(),
                                  onPressed: ready && !busy ? onPreview : null,
                                  icon: const Icon(
                                    Icons.visibility_outlined,
                                    size: 15,
                                  ),
                                  label: const Text('独立预览'),
                                ),
                                OutlinedButton.icon(
                                  style:
                                      SettingsControlStyles.sectionTextButton(),
                                  onPressed: onShowDetails,
                                  icon: const Icon(
                                    Icons.receipt_long_outlined,
                                    size: 15,
                                  ),
                                  label: const Text('包清单'),
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(width: 8),
                          TextButton.icon(
                            style: SettingsControlStyles.sectionTextButton()
                                .copyWith(
                                  foregroundColor:
                                      WidgetStatePropertyAll<Color>(
                                        colorScheme.error,
                                      ),
                                ),
                            onPressed: busy ? null : onDelete,
                            icon: Icon(
                              Icons.delete_outline_rounded,
                              size: 16,
                              color: colorScheme.error,
                            ),
                            label: const Text('删除'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _buildDiagnosticCallout(BuildContext context, bool waiting) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: waiting
            ? colorScheme.tertiaryContainer.withValues(alpha: 0.35)
            : colorScheme.errorContainer.withValues(alpha: 0.42),
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        item.message ??
            (waiting
                ? '正在等待 Space 同步完整主题包字节，同步完成后可预览与应用。'
                : '该主题包条目校验失败，已禁止预览与应用，可将其从主题库删除。'),
        style: textTheme.bodySmall?.copyWith(
          color: waiting
              ? colorScheme.onTertiaryContainer
              : colorScheme.onErrorContainer,
        ),
      ),
    );
  }

  Widget _buildAvailabilityBadge(
    BuildContext context,
    bool active,
    bool ready,
    bool waiting,
  ) {
    final colorScheme = Theme.of(context).colorScheme;
    if (active) {
      return _StatusTagPill(
        icon: Icons.radio_button_checked_rounded,
        label: '本节点已启用',
        foreground: colorScheme.primary,
        background: colorScheme.primary.withValues(alpha: 0.15),
      );
    }
    if (ready) {
      return _StatusTagPill(
        icon: Icons.verified_outlined,
        label: '就绪可用',
        foreground: colorScheme.secondary,
        background: colorScheme.secondary.withValues(alpha: 0.14),
      );
    }
    if (waiting) {
      return _StatusTagPill(
        icon: Icons.hourglass_top_rounded,
        label: '等待同步',
        foreground: colorScheme.tertiary,
        background: colorScheme.tertiary.withValues(alpha: 0.16),
      );
    }
    return _StatusTagPill(
      icon: Icons.error_outline_rounded,
      label: '条目异常',
      foreground: colorScheme.error,
      background: colorScheme.error.withValues(alpha: 0.15),
    );
  }
}

class _EmptyThemeLibraryCard extends StatelessWidget {
  const _EmptyThemeLibraryCard({
    required this.busy,
    required this.onImport,
  });

  final bool busy;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.34),
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.20),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 42, horizontal: 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Container(
              width: 56,
              height: 56,
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withValues(alpha: 0.55),
                shape: BoxShape.circle,
              ),
              child: Icon(
                Icons.palette_outlined,
                size: 28,
                color: colorScheme.primary,
              ),
            ),
            const SizedBox(height: 14),
            Text(
              'Space 主题库暂无主题包',
              style: textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.w800,
              ),
            ),
            const SizedBox(height: 6),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 480),
              child: Text(
                '您可以导入由 Operit Theme Studio 导出的 .optheme 主题包。'
                '每个主题包均经过 L0–L4 结构、哈希与媒体资产严格校验，支持独立沙箱预览与按节点启用。',
                textAlign: TextAlign.center,
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            const SizedBox(height: 18),
            FilledButton.icon(
              onPressed: busy ? null : onImport,
              icon: const Icon(Icons.add_box_outlined, size: 18),
              label: const Text('导入第一个 .optheme 主题包'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ThemeImportConfirmDialog extends StatelessWidget {
  const _ThemeImportConfirmDialog({required this.input});

  final ThemeArchiveInput input;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final validation = input.validation;

    return OperitDialogScaffold(
      title: '导入主题包预检通过',
      icon: const Icon(Icons.verified_outlined),
      maxWidth: 520,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () =>
              Navigator.of(context).pop(_ImportDialogChoice.cancel),
          child: const Text('取消'),
        ),
        OutlinedButton.icon(
          onPressed: () =>
              Navigator.of(context).pop(_ImportDialogChoice.importOnly),
          icon: const Icon(Icons.inventory_2_outlined, size: 16),
          label: const Text('仅导入到库'),
        ),
        FilledButton.icon(
          onPressed: () =>
              Navigator.of(context).pop(_ImportDialogChoice.importAndApply),
          icon: const Icon(Icons.check_circle_outline_rounded, size: 16),
          label: const Text('导入并立即应用'),
        ),
      ],
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _DialogKeyValueCard(
              rows: <(String, String)>[
                ('文件名称', input.name),
                (
                  '归档体积',
                  '${_formatBytes(validation.archiveBytes)}（${validation.archiveBytes} 字节）',
                ),
                (
                  '展开大小',
                  '${_formatBytes(validation.expandedBytes)}（共 ${validation.files.length} 个校验文件）',
                ),
                ('资产组成', _summarizeValidatedFiles(validation.files)),
                ('包 SHA-256', validation.archiveSha256),
                ('Manifest 摘要', validation.manifestSha256),
                ('Theme 摘要', validation.themeSha256),
              ],
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.45,
                ),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: colorScheme.outlineVariant.withValues(alpha: 0.25),
                ),
              ),
              child: Text(
                '• 选择「仅导入到库」：只将主题包存入 Space 主题库，不改变本节点当前主题。\n'
                '• 选择「导入并立即应用」：入库后立即将该主题应用到当前节点。',
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                  height: 1.45,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ThemeApplyConfirmDialog extends StatelessWidget {
  const _ThemeApplyConfirmDialog({required this.entry});

  final core_proxy.ThemeLibraryEntry entry;

  @override
  Widget build(BuildContext context) {
    return OperitDialogScaffold(
      title: '应用主题包到本节点',
      icon: const Icon(Icons.check_circle_outline_rounded),
      maxWidth: 500,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          onPressed: () => Navigator.of(context).pop(true),
          icon: const Icon(Icons.check_circle_outline_rounded, size: 16),
          label: const Text('确认应用'),
        ),
      ],
      child: _DialogKeyValueCard(
        rows: <(String, String)>[
          ('主题名称', '${entry.displayName} v${entry.version}'),
          ('主题包 ID', entry.themeId),
          ('创作者 / 协议', '${entry.authorName} · ${entry.license}'),
          (
            '包体积',
            '${_formatBytes(entry.archiveBytes)}（展开 ${_formatBytes(entry.expandedBytes)}）',
          ),
          ('SHA-256', entry.archiveSha256),
        ],
      ),
    );
  }
}

class _ThemeDeleteConfirmDialog extends StatelessWidget {
  const _ThemeDeleteConfirmDialog({
    required this.item,
    required this.isActiveOnNode,
  });

  final core_proxy.ThemeLibraryItem item;
  final bool isActiveOnNode;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final entry = item.entry;

    return OperitDialogScaffold(
      title: '从 Space 主题库删除主题包',
      icon: Icon(Icons.delete_outline_rounded, color: colorScheme.error),
      maxWidth: 500,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          style: FilledButton.styleFrom(
            backgroundColor: colorScheme.error,
            foregroundColor: colorScheme.onError,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          icon: const Icon(Icons.delete_outline_rounded, size: 16),
          label: Text(isActiveOnNode ? '切回个人外观并删除' : '确认删除'),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          if (isActiveOnNode) ...<Widget>[
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.errorContainer.withValues(alpha: 0.55),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: colorScheme.error.withValues(alpha: 0.45),
                ),
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Icon(
                    Icons.warning_amber_rounded,
                    size: 18,
                    color: colorScheme.error,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      '该主题包当前正在本节点启用中。确认删除将先自动把本节点切回「个人自定义外观」，再从 Space 主题库彻底移除此主题包。',
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onErrorContainer,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
          ],
          _DialogKeyValueCard(
            rows: <(String, String)>[
              if (entry != null)
                ('主题名称', '${entry.displayName} v${entry.version}'),
              ('主题包 ID', item.themeId),
              if (entry != null)
                ('创作者 / 协议', '${entry.authorName} · ${entry.license}'),
              ('SHA-256', item.archiveSha256),
            ],
          ),
        ],
      ),
    );
  }
}

class _ThemePackageDetailsDialog extends StatelessWidget {
  const _ThemePackageDetailsDialog({
    required this.item,
    required this.onCopy,
  });

  final core_proxy.ThemeLibraryItem item;
  final void Function(String label, String value) onCopy;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final entry = item.entry;
    final maxDialogHeight = (MediaQuery.sizeOf(context).height * 0.85)
        .clamp(420.0, 740.0)
        .toDouble();

    return OperitDialogScaffold(
      title: entry != null
          ? '${entry.displayName} v${entry.version} · 包详情清单'
          : '${item.themeId} · 条目详情',
      icon: const Icon(Icons.receipt_long_outlined),
      maxWidth: 640,
      maxHeight: maxDialogHeight,
      expandContent: false,
      showCloseButton: true,
      actions: <Widget>[
        OutlinedButton.icon(
          onPressed: () => onCopy('归档 SHA-256', item.archiveSha256),
          icon: const Icon(Icons.copy_rounded, size: 16),
          label: const Text('复制归档 SHA-256'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _DialogKeyValueCard(
              rows: <(String, String)>[
                ('主题包 ID', item.themeId),
                if (entry != null) ...<(String, String)>[
                  ('显示名称', entry.displayName),
                  ('版本号', entry.version),
                  ('创作者', entry.authorName),
                  ('开源许可证', entry.license),
                  ('支持平台', _formatPlatforms(entry.platforms)),
                  (
                    '归档体积 / 展开体积',
                    '${_formatBytes(entry.archiveBytes)} / ${_formatBytes(entry.expandedBytes)}',
                  ),
                  ('Manifest SHA-256', entry.manifestSha256),
                  ('Theme JSON SHA-256', entry.themeSha256),
                ],
                ('Archive SHA-256', item.archiveSha256),
              ],
            ),
            if (entry != null && entry.validation.files.isNotEmpty) ...<Widget>[
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  Icon(
                    Icons.verified_user_outlined,
                    size: 16,
                    color: colorScheme.primary,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '封存文件校验列表（${entry.validation.files.length} 项）',
                    style: textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              DecoratedBox(
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest.withValues(
                    alpha: 0.36,
                  ),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.25),
                  ),
                ),
                child: Column(
                  children: <Widget>[
                    for (
                      var i = 0;
                      i < entry.validation.files.length;
                      i++
                    ) ...<Widget>[
                      _ValidatedFileRow(
                        file: entry.validation.files[i],
                        onCopyHash: () => onCopy(
                          entry.validation.files[i].path,
                          entry.validation.files[i].sha256,
                        ),
                      ),
                      if (i + 1 < entry.validation.files.length)
                        Divider(
                          height: 1,
                          color: colorScheme.outlineVariant.withValues(
                            alpha: 0.18,
                          ),
                        ),
                    ],
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _ValidatedFileRow extends StatelessWidget {
  const _ValidatedFileRow({
    required this.file,
    required this.onCopyHash,
  });

  final core_proxy.ThemeFileValidation file;
  final VoidCallback onCopyHash;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Row(
        children: <Widget>[
          Icon(
            _iconForFilePath(file.path),
            size: 16,
            color: colorScheme.primary,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                SelectableText(
                  file.path,
                  style: textTheme.bodySmall?.copyWith(
                    fontFamily: 'Cascadia Mono',
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurface,
                  ),
                ),
                SelectableText(
                  'SHA-256：${_shortHash(file.sha256, head: 16, tail: 14)}',
                  style: textTheme.labelSmall?.copyWith(
                    fontFamily: 'Cascadia Mono',
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          SettingsInfoBadge(label: _formatBytes(file.bytes)),
          const SizedBox(width: 4),
          IconButton(
            visualDensity: VisualDensity.compact,
            tooltip: '复制文件 SHA-256',
            onPressed: onCopyHash,
            icon: Icon(
              Icons.copy_rounded,
              size: 15,
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

/// A failed selected package blocks ordinary app content. This screen permits
/// repair/import, retry or an explicit personal-appearance selection only.
class ThemeLibraryRecoveryScreen extends StatelessWidget {
  const ThemeLibraryRecoveryScreen({super.key, required this.controller});
  final OperitThemeController controller;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 1180),
                child: OperitGlassSurface(
                  color: colorScheme.errorContainer.withValues(alpha: 0.55),
                  borderRadius: BorderRadius.circular(14),
                  border: Border.all(
                    color: colorScheme.error.withValues(alpha: 0.45),
                  ),
                  material: true,
                  child: Padding(
                    padding: const EdgeInsets.all(14),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Icon(
                          Icons.warning_amber_rounded,
                          color: colorScheme.error,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: SelectableText(
                            '本节点当前选择的主题包无法加载，应用内容已进入安全恢复模式（不会自动篡改您的主题选择）。\n'
                            '您可以重新校验、导入修复后的主题包，或显式切回「本节点个人外观」。\n'
                            '原因：${controller.themeLoadError}',
                            style: TextStyle(
                              color: colorScheme.onErrorContainer,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
            Expanded(child: ThemeLibraryPanel(controller: controller)),
          ],
        ),
      ),
    );
  }
}

class _ThemePreviewDialog extends StatefulWidget {
  const _ThemePreviewDialog({
    required this.visual,
    required this.settings,
    required this.entry,
    required this.isActiveOnNode,
  });

  final PreparedThemeVisual visual;
  final ThemePreferenceSnapshot settings;
  final core_proxy.ThemeLibraryEntry entry;
  final bool isActiveOnNode;

  @override
  State<_ThemePreviewDialog> createState() => _ThemePreviewDialogState();
}

class _ThemePreviewDialogState extends State<_ThemePreviewDialog> {
  ThemeMode _mode = ThemeMode.light;
  _PreviewStageView _stageView = _PreviewStageView.clientScene;
  ThemeClientSceneMode _sceneMode = ThemeClientSceneMode.conversation;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final screenSize = MediaQuery.sizeOf(context);
    final dialogWidth = (screenSize.width * 0.92).clamp(360.0, 1140.0);
    final dialogHeight = (screenSize.height * 0.88).clamp(480.0, 820.0);

    final previewSettings = ThemePreferenceSnapshot.fromJson(<String, Object?>{
      ...widget.settings.toJson(),
      'themeMode': _mode == ThemeMode.dark ? 'dark' : 'light',
    });
    final effectiveSnapshot = widget.visual.snapshot(previewSettings);

    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: dialogWidth,
        height: dialogHeight,
        child: Column(
          children: <Widget>[
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHigh,
                border: Border(
                  bottom: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.30),
                  ),
                ),
              ),
              child: Wrap(
                spacing: 12,
                runSpacing: 8,
                alignment: WrapAlignment.spaceBetween,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: <Widget>[
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Icon(
                        Icons.visibility_outlined,
                        size: 18,
                        color: colorScheme.primary,
                      ),
                      const SizedBox(width: 8),
                      Flexible(
                        child: Text(
                          '${widget.entry.displayName} v${widget.entry.version}',
                          style: textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w800,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      const SettingsInfoBadge(label: '独立沙箱预览 · 未应用'),
                    ],
                  ),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      SettingsSegmentedSelector<_PreviewStageView>(
                        value: _stageView,
                        onChanged: (next) => setState(() => _stageView = next),
                        segments: const <ButtonSegment<_PreviewStageView>>[
                          ButtonSegment<_PreviewStageView>(
                            value: _PreviewStageView.clientScene,
                            icon: Icon(Icons.desktop_windows_outlined, size: 15),
                            label: Text('完整界面'),
                          ),
                          ButtonSegment<_PreviewStageView>(
                            value: _PreviewStageView.chatWidgets,
                            icon: Icon(Icons.forum_outlined, size: 15),
                            label: Text('会话组件'),
                          ),
                        ],
                      ),
                      if (_stageView == _PreviewStageView.clientScene)
                        SettingsSegmentedSelector<ThemeClientSceneMode>(
                          value: _sceneMode,
                          onChanged: (next) => setState(() => _sceneMode = next),
                          segments: const <ButtonSegment<ThemeClientSceneMode>>[
                            ButtonSegment<ThemeClientSceneMode>(
                              value: ThemeClientSceneMode.conversation,
                              label: Text('对话'),
                            ),
                            ButtonSegment<ThemeClientSceneMode>(
                              value: ThemeClientSceneMode.empty,
                              label: Text('欢迎页'),
                            ),
                          ],
                        ),
                      SettingsSegmentedSelector<ThemeMode>(
                        value: _mode,
                        onChanged: (next) => setState(() => _mode = next),
                        segments: const <ButtonSegment<ThemeMode>>[
                          ButtonSegment<ThemeMode>(
                            value: ThemeMode.light,
                            icon: Icon(Icons.light_mode_outlined, size: 15),
                            label: Text('亮色'),
                          ),
                          ButtonSegment<ThemeMode>(
                            value: ThemeMode.dark,
                            icon: Icon(Icons.dark_mode_outlined, size: 15),
                            label: Text('暗色'),
                          ),
                        ],
                      ),
                      if (!widget.isActiveOnNode)
                        FilledButton.icon(
                          style: SettingsControlStyles.sectionTextButton(),
                          onPressed: () => Navigator.of(context).pop(true),
                          icon: const Icon(
                            Icons.check_circle_outline_rounded,
                            size: 16,
                          ),
                          label: const Text('应用到本节点'),
                        ),
                      IconButton(
                        visualDensity: VisualDensity.compact,
                        onPressed: () => Navigator.of(context).pop(false),
                        icon: const Icon(Icons.close_rounded),
                        tooltip: '关闭预览',
                      ),
                    ],
                  ),
                ],
              ),
            ),
            Expanded(
              child: OperitThemePreview(
                visual: widget.visual,
                settings: previewSettings,
                child: _stageView == _PreviewStageView.clientScene
                    ? ThemeClientScene(
                        snapshot: effectiveSnapshot,
                        userFontFamily: widget.visual.resources.user.family,
                        aiFontFamily: widget.visual.resources.ai.family,
                        mode: _sceneMode,
                      )
                    : Scaffold(
                        backgroundColor: Colors.transparent,
                        body: Center(
                          child: ConstrainedBox(
                            constraints: const BoxConstraints(maxWidth: 780),
                            child: SingleChildScrollView(
                              padding: const EdgeInsets.all(20),
                              child: const ChatAppearancePreview(
                                showInputPreview: true,
                              ),
                            ),
                          ),
                        ),
                      ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DialogKeyValueCard extends StatelessWidget {
  const _DialogKeyValueCard({required this.rows});

  final List<(String, String)> rows;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.38),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.25),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (var i = 0; i < rows.length; i++) ...<Widget>[
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 5),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    SizedBox(
                      width: 112,
                      child: Text(
                        rows[i].$1,
                        style: textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: SelectableText(
                        rows[i].$2,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w600,
                          fontFamily: rows[i].$1.contains('SHA') ||
                                  rows[i].$1.contains('摘要') ||
                                  rows[i].$1.contains('ID')
                              ? 'Cascadia Mono'
                              : null,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              if (i + 1 < rows.length)
                Divider(
                  height: 1,
                  color: colorScheme.outlineVariant.withValues(alpha: 0.16),
                ),
            ],
          ],
        ),
      ),
    );
  }
}

class _StatusTagPill extends StatelessWidget {
  const _StatusTagPill({
    required this.icon,
    required this.label,
    required this.foreground,
    required this.background,
  });

  final IconData icon;
  final String label;
  final Color foreground;
  final Color background;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: foreground.withValues(alpha: 0.32),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3.5),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 13, color: foreground),
            const SizedBox(width: 4),
            Text(
              label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: foreground,
                fontWeight: FontWeight.w700,
                fontSize: 11.5,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MetaBadge extends StatelessWidget {
  const _MetaBadge({
    required this.icon,
    required this.label,
  });

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: isDark ? 0.30 : 0.48),
        borderRadius: BorderRadius.circular(7),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.22),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3.5),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 13, color: colorScheme.onSurfaceVariant),
            const SizedBox(width: 4),
            Text(
              label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

String _shortHash(String hash, {int head = 12, int tail = 10}) {
  if (hash.length <= head + tail + 3) return hash;
  return '${hash.substring(0, head)}…${hash.substring(hash.length - tail)}';
}

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) {
    return '${(bytes / 1024).toStringAsFixed(1)} KB';
  }
  return '${(bytes / (1024 * 1024)).toStringAsFixed(2)} MB';
}

String _summarizeValidatedFiles(List<core_proxy.ThemeFileValidation> files) {
  var imageCount = 0;
  var fontCount = 0;
  for (final file in files) {
    if (file.path.startsWith('assets/images/')) {
      imageCount++;
    } else if (file.path.startsWith('assets/fonts/')) {
      fontCount++;
    }
  }
  final parts = <String>['${files.length} 个文件'];
  if (imageCount > 0) parts.add('$imageCount 图像/SVG');
  if (fontCount > 0) parts.add('$fontCount 字体');
  return parts.join(' · ');
}

String _formatPlatforms(List<String> platforms) {
  if (platforms.isEmpty) return '全平台';
  if (platforms.length >= 6) return '全平台 (${platforms.length})';
  return platforms.join(' / ');
}

IconData _iconForFilePath(String path) {
  if (path.endsWith('.json')) return Icons.data_object_rounded;
  if (path.endsWith('.svg')) return Icons.category_outlined;
  if (path.startsWith('assets/fonts/')) return Icons.font_download_outlined;
  if (path.startsWith('assets/images/') || path.endsWith('.png')) {
    return Icons.image_outlined;
  }
  return Icons.insert_drive_file_outlined;
}

String _cleanErrorMessage(Object error) {
  final raw = error.toString().trim();
  if (raw.startsWith('StateError: ')) {
    return raw.substring('StateError: '.length);
  }
  if (raw.startsWith('FormatException: ')) {
    return raw.substring('FormatException: '.length);
  }
  return raw;
}
