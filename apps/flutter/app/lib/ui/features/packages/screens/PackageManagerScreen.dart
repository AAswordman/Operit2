// ignore_for_file: file_names

import 'dart:async';

import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/host/AppToastController.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../common/components/AnimatedLazyIndexedStack.dart';
import '../../../common/components/PageActivityMixin.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../chat/PendingChatDraftHandler.dart';
import '../../settings/components/SettingsControlStyles.dart';
import '../components/ExtensionMotion.dart';
import '../components/ExtensionTopBar.dart';
import '../components/PackageTab.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../dialogs/MCPImportDialog.dart';
import '../dialogs/A2AImportDialog.dart';
import '../dialogs/PackageEnvironmentVariablesDialog.dart';
import '../dialogs/QuickPluginCreatorDialog.dart';
import '../dialogs/SkillImportDialog.dart';
import '../market/PluginCreationIntent.dart';
import '../model/DeviceSpaceState.dart';
import '../model/ExtensionCatalogController.dart';
import 'ExtensionsPane.dart';
import 'MarketPane.dart';

/// Package files that can be imported by dropping them on the page.
const Set<String> _importableExtensions = <String>{
  'toolpkg',
  'js',
  'ts',
  'hjson',
};

class PackageManagerScreen extends StatefulWidget {
  const PackageManagerScreen({
    super.key,
    this.initialTab = PackageTab.extensions,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final PackageTab initialTab;
  final GeneratedCoreProxyClients clients;

  @override
  State<PackageManagerScreen> createState() => _PackageManagerScreenState();
}

enum _AddAction { toolpkg, script, skill, mcp, a2a, create }

enum _MoreAction { add, checkUpdates, environment, refresh }

class _PackageManagerScreenState extends State<PackageManagerScreen>
    with PageActivityMixin<PackageManagerScreen> {
  late final ExtensionCatalogController _controller =
      ExtensionCatalogController.of(widget.clients);
  late final DeviceSpaceState _space = DeviceSpaceState.of(widget.clients);
  late final ValueNotifier<PackageTab> _tab = ValueNotifier<PackageTab>(
    widget.initialTab,
  );
  final ValueNotifier<bool> _searchOpen = ValueNotifier<bool>(false);
  final TextEditingController _searchController = TextEditingController();
  final FocusNode _searchFocus = FocusNode();
  String _query = '';
  bool _active = false;
  bool _dragging = false;
  StreamSubscription<void>? _syncCompletions;
  Timer? _syncReport;

  @override
  void onPageActivityChanged(bool active) {
    if (active == _active) {
      return;
    }
    _active = active;
    if (active) {
      _controller.attach();
      _space.attach();
      HardwareKeyboard.instance.addHandler(_onKey);
      _syncCompletions = _space.syncCompletions.listen((_) {
        // Synced records reach the catalog through its revision flow; read
        // what changed once that refresh has had time to land.
        _syncReport?.cancel();
        _syncReport = Timer(
          const Duration(milliseconds: 1500),
          _reportSyncedChanges,
        );
      });
    } else {
      _controller.detach();
      _space.detach();
      HardwareKeyboard.instance.removeHandler(_onKey);
      _syncCompletions?.cancel();
      _syncCompletions = null;
      _syncReport?.cancel();
      _syncReport = null;
    }
  }

  @override
  void didUpdateWidget(covariant PackageManagerScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialTab != widget.initialTab) {
      _selectTab(widget.initialTab);
    }
  }

  @override
  void dispose() {
    if (_active) {
      _controller.detach();
      _space.detach();
      HardwareKeyboard.instance.removeHandler(_onKey);
    }
    _syncCompletions?.cancel();
    _syncReport?.cancel();
    _tab.dispose();
    _searchOpen.dispose();
    _searchController.dispose();
    _searchFocus.dispose();
    super.dispose();
  }

  void _reportSyncedChanges() {
    final snapshot = _controller.snapshot;
    if (!mounted || snapshot == null) {
      return;
    }
    final changed = snapshot.installed
        .where(
          (entry) => _controller.isRecentlyChanged(
            ExtensionCatalogController.entryKey(entry),
          ),
        )
        .length;
    if (changed == 0) {
      return;
    }
    AppToastController.instance.show(
      '设备空间同步完成，$changed 个扩展有变化',
      actionLabel: '查看',
      onAction: () => _selectTab(PackageTab.extensions),
    );
  }

  void _selectTab(PackageTab tab) {
    if (tab == _tab.value) return;
    setState(() => _tab.value = tab);
  }

  /// Page shortcuts; they never fire while the user types elsewhere.
  bool _onKey(KeyEvent event) {
    if (event is! KeyDownEvent) {
      return false;
    }
    final keyboard = HardwareKeyboard.instance;
    final command = keyboard.isControlPressed || keyboard.isMetaPressed;
    final key = event.logicalKey;
    if (command && key == LogicalKeyboardKey.keyF) {
      _openSearch();
      return true;
    }
    if (command) {
      final tab = switch (key) {
        LogicalKeyboardKey.digit1 => PackageTab.extensions,
        LogicalKeyboardKey.digit2 => PackageTab.market,
        _ => null,
      };
      if (tab != null) {
        _selectTab(tab);
        return true;
      }
    }
    if (key == LogicalKeyboardKey.escape && _searchOpen.value) {
      _closeSearch();
      return true;
    }
    if (event.character == '/' && !command && !_typing()) {
      _openSearch();
      return true;
    }
    return false;
  }

  bool _typing() {
    final focus = FocusManager.instance.primaryFocus?.context;
    return focus != null &&
        (focus.widget is EditableText ||
            focus.findAncestorWidgetOfExactType<EditableText>() != null);
  }

  void _openSearch() {
    if (!_searchOpen.value) {
      setState(() => _searchOpen.value = true);
    }
    _searchFocus.requestFocus();
  }

  void _closeSearch() {
    _searchController.clear();
    setState(() {
      _query = '';
      _searchOpen.value = false;
    });
  }

  void _onSearchChanged(String value) {
    // Wait for IME composition to finish before filtering on partial pinyin.
    if (_searchController.value.composing.isValid) return;
    setState(() => _query = value);
  }

  @override
  Widget build(BuildContext context) {
    final motion = ExtensionMotion.of(context);
    return ExtensionTopBar(
      title: (context) => _TopBarTitle(
        tab: _tab,
        searchOpen: _searchOpen,
        controller: _controller,
        searchController: _searchController,
        searchFocus: _searchFocus,
        onSelect: _selectTab,
        onSearchChanged: _onSearchChanged,
      ),
      actions: (context) {
        // Phones keep the title wide enough for the section switcher.
        final narrow = MediaQuery.sizeOf(context).width < 600;
        return <Widget>[
          ValueListenableBuilder<bool>(
            valueListenable: _searchOpen,
            builder: (context, open, child) => IconButton(
              tooltip: open ? '关闭搜索' : '搜索（Ctrl+F）',
              onPressed: open ? _closeSearch : _openSearch,
              icon: Icon(open ? Icons.close_rounded : Icons.search_rounded),
            ),
          ),
          if (!narrow)
            IconButton(
              tooltip: '添加扩展',
              onPressed: _showAddSheet,
              icon: const Icon(Icons.add_rounded),
            ),
          PopupMenuButton<_MoreAction>(
            tooltip: '更多',
            icon: const Icon(Icons.more_vert_rounded),
            onSelected: _onMoreSelected,
            itemBuilder: (context) => <PopupMenuEntry<_MoreAction>>[
              if (narrow)
                const PopupMenuItem<_MoreAction>(
                  value: _MoreAction.add,
                  child: ListTile(
                    leading: Icon(Icons.add_rounded),
                    title: Text('添加扩展'),
                  ),
                ),
              const PopupMenuItem<_MoreAction>(
                value: _MoreAction.checkUpdates,
                child: ListTile(
                  leading: Icon(Icons.system_update_alt_rounded),
                  title: Text('检查更新'),
                ),
              ),
              const PopupMenuItem<_MoreAction>(
                value: _MoreAction.environment,
                child: ListTile(
                  leading: Icon(Icons.key_outlined),
                  title: Text('环境变量'),
                ),
              ),
              const PopupMenuItem<_MoreAction>(
                value: _MoreAction.refresh,
                child: ListTile(
                  leading: Icon(Icons.refresh_rounded),
                  title: Text('重新加载目录'),
                ),
              ),
            ],
          ),
        ];
      },
      child: Material(
        type: MaterialType.transparency,
        child: DropTarget(
          onDragEntered: (_) => setState(() => _dragging = true),
          onDragExited: (_) => setState(() => _dragging = false),
          onDragDone: (details) {
            setState(() => _dragging = false);
            unawaited(
              _importDropped(
                details.files.map((file) => file.path).toList(growable: false),
              ),
            );
          },
          child: Stack(
            children: <Widget>[
              AnimatedLazyIndexedStack(
                index: _tab.value.index,
                itemCount: PackageTab.values.length,
                duration: const Duration(milliseconds: 240),
                itemBuilder: (context, index) =>
                    switch (PackageTab.values[index]) {
                      PackageTab.extensions => ExtensionsPane(
                        controller: _controller,
                        query: _query,
                        onAdd: _showAddSheet,
                      ),
                      PackageTab.market => MarketPane(
                        controller: _controller,
                        query: _query,
                      ),
                    },
              ),
              Positioned.fill(
                child: IgnorePointer(
                  child: AnimatedOpacity(
                    opacity: _dragging ? 1 : 0,
                    duration: motion.fast,
                    child: const _DropOverlay(),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _onMoreSelected(_MoreAction action) async {
    switch (action) {
      case _MoreAction.add:
        await _showAddSheet();
      case _MoreAction.checkUpdates:
        _selectTab(PackageTab.extensions);
        await _runAction(_controller.checkUpdates);
      case _MoreAction.environment:
        await PackageEnvironmentVariablesDialog.show(
          context: context,
          clients: widget.clients,
        );
      case _MoreAction.refresh:
        await _controller.refresh();
    }
  }

  /// Runs an action and reports a failure in a SnackBar; returns success.
  Future<bool> _runAction(Future<void> Function() action) async {
    try {
      await action();
      return true;
    } catch (error, stackTrace) {
      debugPrint('Extension operation failed: $error\n$stackTrace');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('$error'),
            behavior: SnackBarBehavior.floating,
          ),
        );
      }
      return false;
    }
  }

  /// Imports dropped package files into this device; other files are named back.
  Future<void> _importDropped(List<String> paths) async {
    final rejected = <String>[];
    var imported = false;
    for (final path in paths) {
      final dot = path.lastIndexOf('.');
      final extension = dot < 0 ? '' : path.substring(dot + 1).toLowerCase();
      if (!_importableExtensions.contains(extension)) {
        rejected.add(path.split(RegExp(r'[\\/]')).last);
        continue;
      }
      imported |= await _runAction(
        () => _controller.importPackageFile(
          path,
          scope: defaultExtensionInstallScope,
          requireToolPkg: false,
        ),
      );
    }
    if (!mounted) return;
    if (imported) {
      _selectTab(PackageTab.extensions);
    }
    if (rejected.isNotEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('不能直接导入：${rejected.join('、')}。技能和 MCP 请通过「添加扩展」导入。'),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _showAddSheet() async {
    final action = await showModalBottomSheet<_AddAction>(
      context: context,
      showDragHandle: true,
      isScrollControlled: true,
      constraints: const BoxConstraints(maxWidth: 680),
      builder: (context) => const _AddExtensionSheet(),
    );
    if (action == null || !mounted) return;
    switch (action) {
      case _AddAction.toolpkg:
        await _importFile(
          const XTypeGroup(label: 'ToolPkg', extensions: <String>['toolpkg']),
          requireToolPkg: true,
        );
      case _AddAction.script:
        await _importFile(
          const XTypeGroup(
            label: 'Operit package',
            extensions: <String>['toolpkg', 'hjson', 'js', 'ts'],
          ),
          requireToolPkg: false,
        );
      case _AddAction.skill:
        await _showSkillImportDialog();
      case _AddAction.mcp:
        await _showMcpImportDialog();
      case _AddAction.a2a:
        final added = await showDialog<bool>(
          context: context,
          builder: (context) => A2AImportDialog(clients: widget.clients),
        );
        if (added == true) await _controller.refresh();
      case _AddAction.create:
        await _openQuickPluginCreator();
    }
  }

  /// Imports a picked package into this device; its detail page can move it later.
  Future<void> _importFile(
    XTypeGroup typeGroup, {
    required bool requireToolPkg,
  }) async {
    final file = await openFile(acceptedTypeGroups: <XTypeGroup>[typeGroup]);
    if (!mounted || file == null) return;
    final imported = await _runAction(
      () => _controller.importPackageFile(
        file.path,
        scope: defaultExtensionInstallScope,
        requireToolPkg: requireToolPkg,
      ),
    );
    if (imported && mounted) {
      _selectTab(PackageTab.extensions);
    }
  }

  Future<void> _showMcpImportDialog() async {
    final result = await showDialog<MCPImportResult>(
      context: context,
      builder: (context) => MCPImportDialog(clients: widget.clients),
    );
    if (result == null || !mounted) return;
    _selectTab(PackageTab.extensions);
    await _controller.refresh();
  }

  Future<void> _showSkillImportDialog() async {
    final result = await showDialog<SkillImportResult>(
      context: context,
      builder: (context) => SkillImportDialog(clients: widget.clients),
    );
    if (result == null || !mounted) return;
    _selectTab(PackageTab.extensions);
    await _controller.refresh();
  }

  Future<void> _openQuickPluginCreator() async {
    final requirement = await QuickPluginCreatorDialog.show(
      context: context,
      clients: widget.clients,
    );
    if (requirement == null || !mounted) {
      return;
    }
    PendingChatDraftHandler.setPendingDraft(
      FreshPluginCreationIntent(requirement: requirement).toPrompt(),
    );
    final chatEntry = ScreenRouteRegistry.toEntry(
      screen: const AiChatScreenRoute(),
    );
    AppRouterGateway.resetTo(
      routeId: chatEntry.routeId,
      args: chatEntry.args,
      source: chatEntry.source,
    );
  }
}

/// Section switcher, or the search field while searching; built in the shell.
class _TopBarTitle extends StatelessWidget {
  const _TopBarTitle({
    required this.tab,
    required this.searchOpen,
    required this.controller,
    required this.searchController,
    required this.searchFocus,
    required this.onSelect,
    required this.onSearchChanged,
  });

  final ValueNotifier<PackageTab> tab;
  final ValueNotifier<bool> searchOpen;
  final ExtensionCatalogController controller;
  final TextEditingController searchController;
  final FocusNode searchFocus;
  final ValueChanged<PackageTab> onSelect;
  final ValueChanged<String> onSearchChanged;

  @override
  Widget build(BuildContext context) {
    final motion = ExtensionMotion.of(context);
    return ListenableBuilder(
      listenable: Listenable.merge(<Listenable>[tab, searchOpen, controller]),
      builder: (context, child) {
        final Widget content;
        if (searchOpen.value) {
          content = _SearchField(
            key: const ValueKey<String>('search'),
            controller: searchController,
            focusNode: searchFocus,
            hintText: switch (tab.value) {
              PackageTab.extensions => '搜索已安装的扩展',
              PackageTab.market => '搜索市场和官方扩展',
            },
            onChanged: onSearchChanged,
          );
        } else {
          final snapshot = controller.snapshot;
          final attention = snapshot == null
              ? 0
              : snapshot.installed
                        .where(ExtensionCatalogController.hasProblem)
                        .length +
                    controller.updatableEntries().length +
                    snapshot.diagnostics.length;
          content = LayoutBuilder(
            key: const ValueKey<String>('tabs'),
            builder: (context, constraints) {
              // Phones drop icons and counts; a huge font scale shrinks to fit.
              final compact = constraints.maxWidth < 300;
              return Align(
                alignment: Alignment.centerLeft,
                child: FittedBox(
                  fit: BoxFit.scaleDown,
                  alignment: Alignment.centerLeft,
                  child: StoreSegmentedTabs(
                    segmentWidth: compact ? 54 : 88,
                    selectedIndex: tab.value.index,
                    onSelected: (index) => onSelect(PackageTab.values[index]),
                    labels: <Widget>[
                      _TabLabel(
                        icon: Icons.extension_outlined,
                        label: '扩展',
                        compact: compact,
                        count: snapshot?.installed.length,
                        alert: attention > 0,
                      ),
                      _TabLabel(
                        icon: Icons.storefront_outlined,
                        label: '市场',
                        compact: compact,
                      ),
                    ],
                  ),
                ),
              );
            },
          );
        }
        return AnimatedSwitcher(
          duration: motion.standard,
          switchInCurve: ExtensionMotion.enter,
          switchOutCurve: ExtensionMotion.exit,
          transitionBuilder: (child, animation) => SlideTransition(
            position: Tween<Offset>(
              begin: const Offset(0, 0.25),
              end: Offset.zero,
            ).animate(animation),
            child: FadeTransition(opacity: animation, child: child),
          ),
          child: content,
        );
      },
    );
  }
}

/// Tab label with a count that pops when it grows and an alert dot.
class _TabLabel extends StatelessWidget {
  const _TabLabel({
    required this.icon,
    required this.label,
    required this.compact,
    this.count,
    this.alert = false,
  });

  final IconData icon;
  final String label;
  final bool compact;
  final int? count;
  final bool alert;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final motion = ExtensionMotion.of(context);
    final count = compact ? null : this.count;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (!compact) ...<Widget>[Icon(icon), const SizedBox(width: 5)],
        Text(label),
        if (count != null) ...<Widget>[
          const SizedBox(width: 4),
          TweenAnimationBuilder<double>(
            key: ValueKey<int>(count),
            tween: Tween<double>(begin: 1.35, end: 1),
            duration: motion.emphasized,
            curve: ExtensionMotion.spring,
            builder: (context, scale, child) =>
                Transform.scale(scale: scale, child: child),
            child: Text(
              '$count',
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        ],
        if (alert) ...<Widget>[
          const SizedBox(width: 4),
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: colorScheme.error,
              shape: BoxShape.circle,
            ),
          ),
        ],
      ],
    );
  }
}

class _SearchField extends StatelessWidget {
  const _SearchField({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.hintText,
    required this.onChanged,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final String hintText;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final contentColor = theme.colorScheme.onSurface;
    return TextField(
      controller: controller,
      focusNode: focusNode,
      autofocus: true,
      onChanged: onChanged,
      maxLines: 1,
      textInputAction: TextInputAction.search,
      cursorColor: contentColor,
      style: theme.textTheme.bodyMedium?.copyWith(color: contentColor),
      decoration: InputDecoration(
        hintText: hintText,
        hintStyle: theme.textTheme.bodyMedium?.copyWith(
          color: contentColor.withValues(alpha: 0.6),
        ),
        prefixIcon: Icon(
          Icons.search_rounded,
          color: contentColor.withValues(alpha: 0.8),
        ),
        border: InputBorder.none,
        focusedBorder: InputBorder.none,
        enabledBorder: InputBorder.none,
        isDense: true,
        contentPadding: const EdgeInsets.symmetric(vertical: 12),
      ),
    );
  }
}

class _DropOverlay extends StatelessWidget {
  const _DropOverlay();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.all(12),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.primary.withValues(alpha: 0.07),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: colorScheme.primary.withValues(alpha: 0.55),
            width: 1.4,
          ),
        ),
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                Icons.file_download_outlined,
                size: 36,
                color: colorScheme.primary,
              ),
              const SizedBox(height: 8),
              Text(
                '松手导入到这台设备',
                style: theme.textTheme.titleSmall?.copyWith(
                  color: colorScheme.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                '支持 .toolpkg、.js、.ts、.hjson',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Bottom sheet with every way to add an extension.
class _AddExtensionSheet extends StatelessWidget {
  const _AddExtensionSheet();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final options = <_AddOption>[
      const _AddOption(
        kind: StoreKind.toolpkg,
        title: '导入插件',
        subtitle: '选择 .toolpkg 文件',
        action: _AddAction.toolpkg,
      ),
      const _AddOption(
        kind: StoreKind.script,
        title: '导入脚本包',
        subtitle: '.js、.ts、.hjson 或 .toolpkg',
        action: _AddAction.script,
      ),
      const _AddOption(
        kind: StoreKind.skill,
        title: '添加技能',
        subtitle: 'GitHub 仓库、ZIP 或手写',
        action: _AddAction.skill,
      ),
      const _AddOption(
        kind: StoreKind.mcp,
        title: '添加 MCP 服务',
        subtitle: 'GitHub、ZIP、配置或表单',
        action: _AddAction.mcp,
      ),
      const _AddOption(
        kind: StoreKind.a2a,
        title: '添加 A2A',
        subtitle: '服务地址、Agent Card 或配置文件',
        action: _AddAction.a2a,
      ),
    ];
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              '添加扩展',
              style: SettingsControlStyles.sectionTitleTextStyle(context),
            ),
            const SizedBox(height: 4),
            Text(
              '导入的扩展先装在这台设备上，之后可以在扩展详情里共享到设备空间。'
              '桌面端也可以直接把文件拖进来。',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 16),
            LayoutBuilder(
              builder: (context, constraints) {
                final columns = constraints.maxWidth >= 520 ? 2 : 1;
                final itemWidth =
                    (constraints.maxWidth - (columns - 1) * 10) / columns;
                return Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  children: <Widget>[
                    for (final option in options)
                      SizedBox(width: itemWidth, child: option),
                  ],
                );
              },
            ),
            const SizedBox(height: 10),
            StorePressable(
              onTap: () => Navigator.of(context).pop(_AddAction.create),
              color: theme.colorScheme.primaryContainer.withValues(alpha: 0.6),
              padding: const EdgeInsets.fromLTRB(14, 12, 12, 12),
              child: Row(
                children: <Widget>[
                  StoreRowGlyph(
                    icon: Icons.auto_awesome_rounded,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          '用 AI 创作插件',
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '描述你想要的功能，在对话里生成并安装',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  Icon(
                    Icons.arrow_forward_rounded,
                    color: theme.colorScheme.primary,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AddOption extends StatelessWidget {
  const _AddOption({
    required this.kind,
    required this.title,
    required this.subtitle,
    required this.action,
  });

  final StoreKind kind;
  final String title;
  final String subtitle;
  final _AddAction action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return StorePressable(
      onTap: () => Navigator.of(context).pop(action),
      padding: const EdgeInsets.all(12),
      child: Row(
        children: <Widget>[
          StoreRowGlyph(icon: kind.icon),
          const SizedBox(width: 12),
          Expanded(
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
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
