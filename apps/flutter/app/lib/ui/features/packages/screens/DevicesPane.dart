// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/PageActivityMixin.dart';
import '../../../common/device/DeviceSphere.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../settings/components/SettingsControlStyles.dart';
import '../../settings/models/SettingsModels.dart';
import '../components/DeviceSpaceVisuals.dart';
import '../components/ExtensionAvatar.dart';
import '../components/ExtensionEntryTiles.dart';
import '../components/ExtensionMotion.dart';
import '../components/ExtensionNavigation.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../model/DeviceSpaceState.dart';
import '../model/ExtensionCatalogController.dart';

/// Where extensions live across the device space: the members, the sync with
/// each of them, and which extensions every device shares.
class DevicesPane extends StatefulWidget {
  const DevicesPane({super.key, required this.controller, required this.query});

  final ExtensionCatalogController controller;
  final String query;

  @override
  State<DevicesPane> createState() => _DevicesPaneState();
}

class _DevicesPaneState extends State<DevicesPane>
    with PageActivityMixin<DevicesPane> {
  late final DeviceSpaceState _space = DeviceSpaceState.of(
    widget.controller.clients,
  );
  bool _attached = false;
  core_proxy.ExtensionCatalogEntry? _dropped;
  String? _dropTarget;

  ExtensionCatalogController get _controller => widget.controller;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onChanged);
    _space.addListener(_onChanged);
  }

  @override
  void onPageActivityChanged(bool active) {
    if (active && !_attached) {
      _space.attach();
      _attached = true;
    } else if (!active && _attached) {
      _space.detach();
      _attached = false;
    }
  }

  @override
  void dispose() {
    if (_attached) {
      _space.detach();
    }
    _controller.removeListener(_onChanged);
    _space.removeListener(_onChanged);
    super.dispose();
  }

  void _onChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _move(core_proxy.ExtensionCatalogEntry entry, String scope) async {
    try {
      await _controller.moveScope(entry, scope);
    } catch (error, stackTrace) {
      debugPrint('Failed to move extension scope: $error\n$stackTrace');
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    }
  }

  String _moveMessage(core_proxy.ExtensionCatalogEntry entry, String target) {
    final name = '「${entry.displayName}」';
    if (target == 'space') {
      final count = _space.devices.length;
      return count > 1
          ? '共享后$name会同步到设备空间的 $count 台设备，启用状态和配置里的密钥也会一起同步。'
          : '共享后，之后加入设备空间的设备都会同步$name，配置里的密钥也会一起同步。';
    }
    return '移回本机后，设备空间里的其他设备会移除$name和它的配置。';
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = _controller.snapshot;
    final query = widget.query.trim().toLowerCase();
    final entries = (snapshot?.installed ??
            const <core_proxy.ExtensionCatalogEntry>[])
        .where(
          (entry) =>
              query.isEmpty ||
              entry.displayName.toLowerCase().contains(query) ||
              entry.id.toLowerCase().contains(query),
        )
        .toList(growable: false);
    List<core_proxy.ExtensionCatalogEntry> inScope(String scope) =>
        entries.where((entry) => entry.scope == scope).toList(growable: false);
    final current = _space.currentDevice;
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final padding = storeSliverPadding(
          width,
          maxWidth: 960,
          gutter: width < 600 ? 16 : 24,
        );
        return CustomScrollView(
          slivers: <Widget>[
            SliverPadding(
              padding: padding.copyWith(top: 12),
              sliver: SliverToBoxAdapter(
                child: _SpaceHero(space: _space, wide: width >= 600),
              ),
            ),
            if (snapshot == null)
              SliverPadding(
                padding: padding.copyWith(top: 24),
                sliver: const SliverToBoxAdapter(
                  child: StoreShimmer(
                    child: StoreSkeletonBox(height: 160, radius: 12),
                  ),
                ),
              )
            else ...<Widget>[
              _section(
                padding: padding,
                scope: 'space',
                title: '共享到设备空间',
                subtitle: _space.devices.length > 1
                    ? '同步到空间里的 ${_space.devices.length} 台设备，启用状态一起同步；每台设备上的对话都能用'
                    : '加入设备空间的设备都会同步这些扩展',
                trailing: _space.devices.isEmpty
                    ? null
                    : DeviceSphereStack(
                        devices: _space.devices,
                        currentDeviceId: _space.currentDeviceId,
                      ),
                entries: inScope('space'),
                empty: '还没有共享的扩展。把常用的扩展共享出去，其他设备上的对话也能用。',
                moveTo: 'device',
              ),
              _section(
                padding: padding,
                scope: 'device',
                title: '仅本机',
                subtitle: current == null
                    ? '只在这台设备上，只有在这里运行的对话能用'
                    : '只在「${current.deviceName}」上，只有在这里运行的对话能用',
                trailing: current == null
                    ? null
                    : DeviceSphereStack(
                        devices: <core_proxy.RuntimeDeviceSpaceDevice>[current],
                        currentDeviceId: current.deviceId,
                      ),
                entries: inScope('device'),
                empty: '没有只装在本机的扩展。',
                moveTo: 'space',
              ),
              if (inScope('localOnly').isNotEmpty)
                _section(
                  padding: padding,
                  scope: 'localOnly',
                  title: '本机运行',
                  subtitle: '通过本地命令启动的 MCP 只能在这台设备上运行，不能共享',
                  entries: inScope('localOnly'),
                  empty: '',
                  moveTo: null,
                ),
            ],
            const SliverToBoxAdapter(child: SizedBox(height: 56)),
          ],
        );
      },
    );
  }

  Widget _section({
    required EdgeInsets padding,
    required String scope,
    required String title,
    required String subtitle,
    required List<core_proxy.ExtensionCatalogEntry> entries,
    required String empty,
    required String? moveTo,
    Widget? trailing,
  }) {
    final dropped = _dropped;
    final pendingHere = dropped != null && _dropTarget == scope;
    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        StoreSectionHeader(
          icon: storeScopeIcon(scope),
          title: title,
          count: entries.length,
          subtitle: subtitle,
          trailing: trailing,
        ),
        AnimatedSize(
          duration: ExtensionMotion.of(context).standard,
          curve: ExtensionMotion.enter,
          alignment: Alignment.topCenter,
          child: pendingHere
              ? Padding(
                  padding: const EdgeInsets.only(bottom: 10),
                  child: _DropConfirm(
                    entry: dropped,
                    message: _moveMessage(dropped, scope),
                    confirmLabel: scope == 'space' ? '共享' : '移回本机',
                    onCancel: () => setState(() {
                      _dropped = null;
                      _dropTarget = null;
                    }),
                    onConfirm: () async {
                      setState(() {
                        _dropped = null;
                        _dropTarget = null;
                      });
                      await _move(dropped, scope);
                    },
                  ),
                )
              : const SizedBox(width: double.infinity),
        ),
        if (entries.isEmpty)
          _EmptySection(text: empty)
        else
          StoreGroupedList(
            children: <Widget>[
              for (final entry in entries) _row(entry, moveTo),
            ],
          ),
      ],
    );
    final Widget section;
    if (scope == 'localOnly') {
      // Matches the inset the drop targets reserve for their highlight.
      section = Padding(padding: const EdgeInsets.all(6), child: body);
    } else {
      section = DragTarget<core_proxy.ExtensionCatalogEntry>(
        onWillAcceptWithDetails: (details) =>
            details.data.scope != scope && details.data.scope != 'localOnly',
        onAcceptWithDetails: (details) => setState(() {
          _dropped = details.data;
          _dropTarget = scope;
        }),
        builder: (context, candidates, rejected) {
          final colorScheme = Theme.of(context).colorScheme;
          return AnimatedContainer(
            duration: ExtensionMotion.of(context).fast,
            padding: const EdgeInsets.all(6),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(16),
              color: candidates.isEmpty
                  ? colorScheme.primary.withValues(alpha: 0)
                  : colorScheme.primary.withValues(alpha: 0.06),
              border: Border.all(
                color: candidates.isEmpty
                    ? colorScheme.primary.withValues(alpha: 0)
                    : colorScheme.primary.withValues(alpha: 0.5),
                width: 1.4,
              ),
            ),
            child: body,
          );
        },
      );
    }
    return SliverPadding(
      padding: EdgeInsets.fromLTRB(
        padding.left - 6,
        18,
        padding.right - 6,
        0,
      ),
      sliver: SliverToBoxAdapter(child: section),
    );
  }

  Widget _row(core_proxy.ExtensionCatalogEntry entry, String? moveTo) {
    if (moveTo == null) {
      return ExtensionListRow(
        controller: _controller,
        entry: entry,
        trailing: Tooltip(
          message: '本地命令型 MCP 不能共享',
          child: Icon(
            Icons.lock_outline_rounded,
            size: 18,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    final busy = _controller.isBusy(ExtensionCatalogController.entryKey(entry));
    final row = StoreInlineConfirm(
      message: _moveMessage(entry, moveTo),
      confirmLabel: moveTo == 'space' ? '共享' : '移回本机',
      onConfirm: () => _move(entry, moveTo),
      trigger: (context, requestConfirm) => ExtensionListRow(
        controller: _controller,
        entry: entry,
        trailing: OutlinedButton.icon(
          onPressed: busy ? null : requestConfirm,
          style: OutlinedButton.styleFrom(
            visualDensity: VisualDensity.compact,
            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
          icon: Icon(
            moveTo == 'space'
                ? Icons.share_outlined
                : Icons.my_location_rounded,
            size: 16,
          ),
          label: Text(moveTo == 'space' ? '共享' : '移回本机'),
        ),
      ),
    );
    return _MouseDraggable<core_proxy.ExtensionCatalogEntry>(
      data: entry,
      feedback: _DragChip(controller: _controller, entry: entry),
      childWhenDragging: Opacity(opacity: 0.45, child: row),
      child: row,
    );
  }
}

/// Drags only with a mouse, so touch scrolling and taps stay untouched.
class _MouseDraggable<T extends Object> extends Draggable<T> {
  const _MouseDraggable({
    required super.data,
    required super.feedback,
    required super.childWhenDragging,
    required super.child,
  });

  @override
  MultiDragGestureRecognizer createRecognizer(
    GestureMultiDragStartCallback onStart,
  ) {
    return ImmediateMultiDragGestureRecognizer(
      supportedDevices: const <PointerDeviceKind>{PointerDeviceKind.mouse},
    )..onStart = onStart;
  }
}

class _DragChip extends StatelessWidget {
  const _DragChip({required this.controller, required this.entry});

  final ExtensionCatalogController controller;
  final core_proxy.ExtensionCatalogEntry entry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      elevation: 8,
      borderRadius: storeCardRadius,
      color: theme.colorScheme.surfaceContainerHigh,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(8, 8, 14, 8),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            CatalogEntryAvatar(
              clients: controller.clients,
              entry: entry,
              size: 32,
            ),
            const SizedBox(width: 10),
            Text(
              entry.displayName,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Confirmation shown in the section an extension was dropped on.
class _DropConfirm extends StatelessWidget {
  const _DropConfirm({
    required this.entry,
    required this.message,
    required this.confirmLabel,
    required this.onCancel,
    required this.onConfirm,
  });

  final core_proxy.ExtensionCatalogEntry entry;
  final String message;
  final String confirmLabel;
  final VoidCallback onCancel;
  final VoidCallback onConfirm;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.secondaryContainer.withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 10, 8, 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                ExtensionAvatar(
                  seed: '${entry.kind}:${entry.id}',
                  label: entry.displayName,
                  size: 28,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    message,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSecondaryContainer,
                      height: 1.45,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: <Widget>[
                TextButton(
                  onPressed: onCancel,
                  style: SettingsControlStyles.sectionTextButton(),
                  child: const Text('取消'),
                ),
                const SizedBox(width: 6),
                FilledButton(
                  onPressed: onConfirm,
                  style: SettingsControlStyles.sectionFilledButton(),
                  child: Text(confirmLabel),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _EmptySection extends StatelessWidget {
  const _EmptySection({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: storeCardRadius,
        border: Border.all(
          color: theme.colorScheme.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 16),
        child: Text(
          text,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }
}

/// The device space card: members on an arc around this device, the sync with
/// each, and the way into device space settings.
class _SpaceHero extends StatelessWidget {
  const _SpaceHero({required this.space, required this.wide});

  final DeviceSpaceState space;
  final bool wide;

  void _openSettings() {
    openExtensionScreen(
      const SettingsScreenRoute(category: SettingsCategory.accessLinks),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final current = space.currentDevice;
    final others = space.otherDevices;
    final sync = deviceSpaceSyncSummary(others, space.syncWith);
    final error = space.error;
    final Widget scene;
    if (current == null) {
      scene = error != null
          ? Text(
              '读取设备空间失败：$error',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.error,
              ),
            )
          : const StoreShimmer(child: StoreSkeletonBox(height: 120, radius: 12));
    } else if (others.isEmpty) {
      scene = Row(
        children: <Widget>[
          DeviceSphere.still(
            diameter: 64,
            icon: devicePlatformIcon(current.platform),
            current: true,
            online: true,
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  current.deviceName,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  '设备空间里只有这台设备。加入或邀请其他设备后，共享的扩展会自动同步过去，'
                  '那些设备上的对话也能用。',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    height: 1.45,
                  ),
                ),
                const SizedBox(height: 10),
                FilledButton.tonalIcon(
                  onPressed: _openSettings,
                  style: SettingsControlStyles.sectionFilledButton(),
                  icon: const Icon(Icons.add_link_rounded, size: 18),
                  label: const Text('添加设备'),
                ),
              ],
            ),
          ),
        ],
      );
    } else {
      scene = DeviceSpaceConstellation(
        current: current,
        others: others,
        syncOf: space.syncWith,
        height: wide ? 176 : 144,
        showNames: wide,
      );
    }
    final total = space.devices.length;
    return DecoratedBox(
      decoration: storeHeroDecoration(theme),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(18, 14, 12, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(Icons.hub_outlined, size: 20, color: colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        space.spaceName.trim().isEmpty
                            ? '设备空间'
                            : '设备空间 · ${space.spaceName}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: SettingsControlStyles.sectionTitleTextStyle(
                          context,
                        ),
                      ),
                      if (total > 0)
                        Text(
                          <String>[
                            '$total 台设备 · ${space.onlineCount} 台在线',
                            if (sync.isNotEmpty) sync,
                          ].join(' · '),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: sync.isNotEmpty
                                ? colorScheme.primary
                                : colorScheme.onSurfaceVariant,
                          ),
                        ),
                    ],
                  ),
                ),
                TextButton.icon(
                  onPressed: _openSettings,
                  style: SettingsControlStyles.sectionTextButton(),
                  iconAlignment: IconAlignment.end,
                  icon: const Icon(Icons.chevron_right_rounded, size: 18),
                  label: const Text('管理'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            scene,
          ],
        ),
      ),
    );
  }
}
