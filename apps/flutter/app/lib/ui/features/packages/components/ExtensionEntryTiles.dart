// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../model/ExtensionCatalogController.dart';
import 'ExtensionActions.dart';
import 'ExtensionAvatar.dart';
import 'ExtensionMotion.dart';
import 'ExtensionNavigation.dart';
import 'StoreVisuals.dart';

/// Runtime wording shared by rows and cards. [restatesSwitch] marks wording
/// that only repeats the enabled state, which tiles beside a switch leave out.
({String label, bool restatesSwitch}) extensionRuntimeStatus(
  core_proxy.ExtensionCatalogEntry entry,
) {
  final enabled = ExtensionCatalogController.isEnabled(entry);
  if (entry.kind == 'skill') {
    return (label: enabled ? 'AI 可见' : 'AI 不可见', restatesSwitch: true);
  }
  if (entry.runtimeState.startsWith('error:')) {
    return (label: '启动失败', restatesSwitch: false);
  }
  if (entry.runtimeState == 'error') {
    return (
      label: entry.kind == 'a2a' ? '配置异常' : '依赖异常',
      restatesSwitch: false,
    );
  }
  if (entry.kind == 'a2a' && entry.runtimeState == 'unconfigured') {
    return (label: '未读取 Agent Card', restatesSwitch: false);
  }
  if (entry.kind == 'mcp') {
    return switch (entry.runtimeState) {
      'running' => (label: '运行中', restatesSwitch: false),
      _ =>
        enabled
            ? (label: '未运行', restatesSwitch: false)
            : (label: '已停用', restatesSwitch: true),
    };
  }
  return (label: enabled ? '已启用' : '已停用', restatesSwitch: true);
}

/// One line of facts: kind, location and version, minus what the section
/// heading above the tile already says.
String extensionEntryFacts(
  core_proxy.ExtensionCatalogEntry entry, {
  bool showKind = true,
  bool showScope = true,
}) {
  return <String>[
    if (showKind) StoreKind.of(entry.kind).label,
    if (showScope) storeScopeLabel(entry.scope),
    if (entry.version.trim().isNotEmpty) 'v${entry.version}',
  ].join(' · ');
}

/// Avatar, name and facts of an installed extension.
class ExtensionEntrySummary extends StatelessWidget {
  const ExtensionEntrySummary({
    super.key,
    required this.controller,
    required this.entry,
    this.avatarSize = 40,
    this.showKind = true,
    this.showScope = true,
    this.showEnabledState = true,
  });

  final ExtensionCatalogController controller;
  final core_proxy.ExtensionCatalogEntry entry;
  final double avatarSize;
  final bool showKind;

  /// False under a location heading, which already names the scope.
  final bool showScope;

  /// False beside an enable switch; problems and MCP run states still show.
  final bool showEnabledState;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final key = ExtensionCatalogController.entryKey(entry);
    final enabled =
        controller.pendingEnabled(key) ??
        ExtensionCatalogController.isEnabled(entry);
    final problem = ExtensionCatalogController.hasProblem(entry);
    final update = controller.hasUpdate(entry);
    final status = extensionRuntimeStatus(entry);
    final running = entry.kind == 'mcp' && entry.runtimeState == 'running';
    final facts = extensionEntryFacts(
      entry,
      showKind: showKind,
      showScope: showScope,
    );
    final parts = <InlineSpan>[
      if (showEnabledState || problem || !status.restatesSwitch)
        TextSpan(
          children: <InlineSpan>[
            if (running)
              WidgetSpan(
                alignment: PlaceholderAlignment.middle,
                child: Padding(
                  padding: const EdgeInsets.only(right: 5),
                  child: _StatusDot(color: colorScheme.primary),
                ),
              ),
            TextSpan(text: status.label),
          ],
          style: TextStyle(
            color: problem
                ? colorScheme.error
                : running
                ? colorScheme.primary
                : null,
            fontWeight: problem || running ? FontWeight.w600 : null,
          ),
        ),
      if (facts.isNotEmpty) TextSpan(text: facts),
      if (update)
        TextSpan(
          text: '有新版本',
          style: TextStyle(
            color: colorScheme.primary,
            fontWeight: FontWeight.w600,
          ),
        ),
    ];
    return Row(
      children: <Widget>[
        CatalogEntryAvatar(
          clients: controller.clients,
          entry: entry,
          size: avatarSize,
          enabled: enabled,
          busy: controller.isBusy(key),
          showKind: showKind,
          badge: problem
              ? ExtensionAvatarBadge.error
              : update
              ? ExtensionAvatarBadge.update
              : ExtensionAvatarBadge.none,
        ),
        const SizedBox(width: 12),
        Expanded(
          child: _DisabledFade(
            enabled: enabled,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Text(
                  entry.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 2),
                Text.rich(
                  TextSpan(
                    children: <InlineSpan>[
                      for (
                        var index = 0;
                        index < parts.length;
                        index++
                      ) ...<InlineSpan>[
                        if (index > 0) const TextSpan(text: ' · '),
                        parts[index],
                      ],
                    ],
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

/// Fades the text of a disabled extension so it reads as off at a glance. The
/// avatar fades on its own and the switch stays fully visible.
class _DisabledFade extends StatelessWidget {
  const _DisabledFade({required this.enabled, required this.child});

  final bool enabled;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final motion = ExtensionMotion.of(context);
    return AnimatedOpacity(
      opacity: enabled ? 1 : 0.55,
      duration: motion.standard,
      curve: ExtensionMotion.enter,
      child: child,
    );
  }
}

class _StatusDot extends StatelessWidget {
  const _StatusDot({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 7,
      height: 7,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
        boxShadow: <BoxShadow>[
          BoxShadow(color: color.withValues(alpha: 0.5), blurRadius: 4),
        ],
      ),
    );
  }
}

/// Switch bound to the controller's optimistic enabled state, drawn at the
/// settings rows' compact scale so it does not outweigh the name beside it.
class ExtensionEnableSwitch extends StatelessWidget {
  const ExtensionEnableSwitch({
    super.key,
    required this.controller,
    required this.entry,
  });

  final ExtensionCatalogController controller;
  final core_proxy.ExtensionCatalogEntry entry;

  @override
  Widget build(BuildContext context) {
    final key = ExtensionCatalogController.entryKey(entry);
    final enabled =
        controller.pendingEnabled(key) ??
        ExtensionCatalogController.isEnabled(entry);
    final toggle = Transform.scale(
      scale: 0.82,
      child: Switch(
        value: enabled,
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        onChanged: controller.isBusy(key)
            ? null
            : (value) {
                unawaited(HapticFeedback.selectionClick());
                unawaited(_toggle(context, value));
              },
      ),
    );
    if (entry.scope != 'space') {
      return toggle;
    }
    return Tooltip(message: '启用状态会同步到设备空间的其他设备', child: toggle);
  }

  Future<void> _toggle(BuildContext context, bool value) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      await controller.setEnabled(entry, value);
    } catch (error, stackTrace) {
      debugPrint('Failed to toggle extension: $error\n$stackTrace');
      messenger.showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    }
  }
}

/// Row of a grouped list; it flashes when a catalog read changed the entry.
class ExtensionListRow extends StatelessWidget {
  const ExtensionListRow({
    super.key,
    required this.controller,
    required this.entry,
    this.trailing,
    this.showKind = true,
    this.showScope = true,
    this.showEnabledState = true,
  });

  final ExtensionCatalogController controller;
  final core_proxy.ExtensionCatalogEntry entry;
  final Widget? trailing;
  final bool showKind;
  final bool showScope;

  /// Pass false when [trailing] is the enable switch.
  final bool showEnabledState;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final motion = ExtensionMotion.of(context);
    final recent = controller.isRecentlyChanged(
      ExtensionCatalogController.entryKey(entry),
    );
    final trailing = this.trailing;
    return TweenAnimationBuilder<double>(
      tween: Tween<double>(end: recent ? 1 : 0),
      duration: recent ? motion.standard : motion.emphasized * 3,
      curve: ExtensionMotion.enter,
      builder: (context, glow, child) => ColoredBox(
        color: colorScheme.primary.withValues(alpha: 0.09 * glow),
        child: child,
      ),
      child: InkWell(
        onTap: () => openExtensionDetail(kind: entry.kind, id: entry.id),
        onLongPress: () => showExtensionQuickActions(
          context,
          controller: controller,
          entry: entry,
        ),
        onSecondaryTapUp: (details) => showExtensionQuickActions(
          context,
          controller: controller,
          entry: entry,
          pointer: details.globalPosition,
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 10, 10, 10),
          child: Row(
            children: <Widget>[
              Expanded(
                child: ExtensionEntrySummary(
                  controller: controller,
                  entry: entry,
                  showKind: showKind,
                  showScope: showScope,
                  showEnabledState: showEnabledState,
                ),
              ),
              if (trailing != null) ...<Widget>[
                const SizedBox(width: 10),
                trailing,
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Grid card with the description and the enable switch.
class ExtensionCard extends StatelessWidget {
  const ExtensionCard({
    super.key,
    required this.controller,
    required this.entry,
    this.showKind = true,
    this.showScope = true,
  });

  final ExtensionCatalogController controller;
  final core_proxy.ExtensionCatalogEntry entry;
  final bool showKind;
  final bool showScope;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final description = entry.description.trim();
    final key = ExtensionCatalogController.entryKey(entry);
    final enabled =
        controller.pendingEnabled(key) ??
        ExtensionCatalogController.isEnabled(entry);
    return StorePressable(
      highlight: controller.isRecentlyChanged(key),
      onTap: () => openExtensionDetail(kind: entry.kind, id: entry.id),
      onLongPress: () => showExtensionQuickActions(
        context,
        controller: controller,
        entry: entry,
      ),
      onSecondaryTapUp: (details) => showExtensionQuickActions(
        context,
        controller: controller,
        entry: entry,
        pointer: details.globalPosition,
      ),
      padding: const EdgeInsets.fromLTRB(14, 12, 8, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: ExtensionEntrySummary(
                  controller: controller,
                  entry: entry,
                  avatarSize: 44,
                  showKind: showKind,
                  showScope: showScope,
                  showEnabledState: false,
                ),
              ),
              ExtensionEnableSwitch(controller: controller, entry: entry),
            ],
          ),
          const SizedBox(height: 10),
          Expanded(
            child: _DisabledFade(
              enabled: enabled,
              child: Padding(
                padding: const EdgeInsets.only(right: 6),
                child: Text(
                  description.isEmpty ? '没有简介' : description,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant.withValues(
                      alpha: description.isEmpty ? 0.6 : 1,
                    ),
                    height: 1.45,
                  ),
                ),
              ),
            ),
          ),
          _DisabledFade(
            enabled: enabled,
            child: _CapabilityTags(entry: entry),
          ),
        ],
      ),
    );
  }
}

/// What a ToolPkg contributes to the app, or the tool count of the others.
class _CapabilityTags extends StatelessWidget {
  const _CapabilityTags({required this.entry});

  final core_proxy.ExtensionCatalogEntry entry;

  @override
  Widget build(BuildContext context) {
    final tags = <(IconData, String)>[
      if (entry.kind == 'toolpkg') ...<(IconData, String)>[
        if (entry.subpackageCount > 0)
          (Icons.inventory_2_outlined, '子包 ${entry.subpackageCount}'),
        if (entry.hasUi) (Icons.dashboard_customize_outlined, '界面'),
        if (entry.chatSidebarEntryIds.isNotEmpty)
          (Icons.view_sidebar_outlined, '侧边栏'),
      ] else if (entry.kind != 'skill' && entry.toolCount > 0)
        (Icons.build_outlined, '工具 ${entry.toolCount}'),
    ];
    if (tags.isEmpty) {
      return const SizedBox(height: 22);
    }
    return SizedBox(
      height: 22,
      child: Row(
        children: <Widget>[
          for (final (icon, label) in tags) ...<Widget>[
            StoreBadge(label: label, icon: icon),
            const SizedBox(width: 6),
          ],
        ],
      ),
    );
  }
}
