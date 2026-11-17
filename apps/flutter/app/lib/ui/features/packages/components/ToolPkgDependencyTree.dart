// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../model/ToolPkgDependencyGraph.dart';
import 'ExtensionAvatar.dart';
import 'ExtensionNavigation.dart';
import 'StoreDetailKit.dart';

/// A compact, bounded-width tree. Shared prerequisites become links after their
/// first appearance; totals count unique plugins, rather than tree rows.
class ToolPkgDependencyTree extends StatelessWidget {
  const ToolPkgDependencyTree({
    super.key,
    required this.clients,
    required this.rootId,
    required this.nodes,
  });

  final GeneratedCoreProxyClients clients;
  final String rootId;
  final List<ToolPkgDependencyNode> nodes;

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.centerLeft,
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 640),
      child: LayoutBuilder(
        builder: (context, constraints) =>
            _buildTree(context, constraints.maxWidth),
      ),
    ),
  );

  Widget _buildTree(BuildContext context, double width) {
    final indent = width < 360 ? 24.0 : 32.0;
    final maxDepth = width < 360 ? 2 : 3;
    final colorScheme = Theme.of(context).colorScheme;
    final branchColor = colorScheme.primary.withValues(alpha: 0.75);
    final byId = {for (final node in nodes) node.id: node};
    final seen = <String>{};
    final rows = <Widget>[];
    String name(String id) => byId[id]?.entry?.displayName ?? id;

    void append(
      List<ToolPkgDependencyNode> siblings,
      String parentId,
      List<bool> ancestors,
    ) {
      for (var i = 0; i < siblings.length; i++) {
        final node = siblings[i];
        final last = i == siblings.length - 1;
        final repeated = !seen.add(node.id);
        final children = nodes
            .where(
              (child) =>
                  child.sources.any((source) => source.parentId == node.id),
            )
            .toList();
        final source = node.sources.firstWhere(
          (source) => source.parentId == parentId,
        );
        rows.add(
          _DependencyTreeRow(
            clients: clients,
            node: node,
            ancestors: ancestors,
            last: last,
            optional: source.optional,
            repeated: repeated,
            indent: indent,
            maxDepth: maxDepth,
            branchColor: branchColor,
            hasChildren: !repeated && children.isNotEmpty,
            sourceNames: {
              for (final source in node.sources)
                source.parentId: source.parentId == rootId
                    ? '当前插件'
                    : name(source.parentId),
            },
          ),
        );
        if (!repeated) {
          append(children, node.id, [...ancestors, last]);
        }
      }
    }

    final groups = <Widget>[];
    final roots = nodes.where(
      (node) => node.sources.any((source) => source.parentId == rootId),
    );
    for (final root in roots) {
      rows.clear();
      append([root], rootId, []);
      groups.add(
        Container(
          key: ValueKey('dependency-branch:${root.id}'),
          margin: EdgeInsets.only(top: groups.isEmpty ? 0 : 8),
          decoration: BoxDecoration(
            color: colorScheme.primary.withValues(alpha: 0.06),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Column(children: List<Widget>.of(rows)),
        ),
      );
    }
    final satisfied = nodes
        .where((node) => node.state == ToolPkgDependencyState.satisfied)
        .length;
    Widget summary(bool required) {
      final group = nodes.where((node) => node.required == required);
      final ready = group
          .where((node) => node.state == ToolPkgDependencyState.satisfied)
          .length;
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          CustomPaint(
            size: const Size(20, 12),
            painter: _BranchLegend(branchColor, !required),
          ),
          const SizedBox(width: 6),
          Text(
            '${required ? '必需' : '可选'} $ready/${group.length}',
            style: Theme.of(context).textTheme.labelMedium,
          ),
        ],
      );
    }

    return StoreDetailSection(
      title: '插件依赖',
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            satisfied == nodes.length
                ? Icons.check_circle_outline_rounded
                : Icons.pending_outlined,
            size: 16,
            color: satisfied == nodes.length
                ? colorScheme.primary
                : colorScheme.error,
          ),
          const SizedBox(width: 5),
          Text(
            '$satisfied/${nodes.length}',
            semanticsLabel: '已满足 $satisfied，共 ${nodes.length} 个依赖',
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Wrap(
              spacing: 20,
              runSpacing: 6,
              children: [
                if (nodes.any((node) => node.required)) summary(true),
                if (nodes.any((node) => !node.required)) summary(false),
              ],
            ),
          ),
          StoreGroupedList(
            showDividers: false,
            children: [
              Padding(
                padding: const EdgeInsets.all(6),
                child: Column(children: groups),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _DependencyTreeRow extends StatelessWidget {
  const _DependencyTreeRow({
    required this.clients,
    required this.node,
    required this.ancestors,
    required this.last,
    required this.optional,
    required this.repeated,
    required this.sourceNames,
    required this.indent,
    required this.maxDepth,
    required this.branchColor,
    required this.hasChildren,
  });

  final GeneratedCoreProxyClients clients;
  final ToolPkgDependencyNode node;
  final List<bool> ancestors;
  final bool last;
  final bool optional;
  final bool repeated;
  final Map<String, String> sourceNames;
  final double indent;
  final int maxDepth;
  final Color branchColor;
  final bool hasChildren;

  String _range(ToolPkgDependencySource source) => [
    if (source.requirement.minVersion != null)
      '≥ ${source.requirement.minVersion}',
    if (source.requirement.maxVersion != null)
      '≤ ${source.requirement.maxVersion}',
  ].join(' · ');

  void _showInfo(BuildContext context) {
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (context) => SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                node.entry?.displayName ?? node.id,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              Text(node.state.label),
              if (node.entry != null) Text('已安装 v${node.entry!.version}'),
              for (final source in node.sources)
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: Icon(
                    source.optional
                        ? Icons.link_outlined
                        : Icons.account_tree_outlined,
                  ),
                  title: Text(
                    '${sourceNames[source.parentId]}${source.optional ? '（可选）' : '需要'}',
                  ),
                  subtitle: Text(
                    [
                      source.requirement.description,
                      _range(source),
                    ].where((s) => s.isNotEmpty).join(' · '),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final entry = node.entry;
    final theme = Theme.of(context);
    final satisfied = node.state == ToolPkgDependencyState.satisfied;
    final statusIcon = switch (node.state) {
      ToolPkgDependencyState.satisfied => Icons.check_circle_rounded,
      ToolPkgDependencyState.disabled => Icons.pause_circle_outline_rounded,
      ToolPkgDependencyState.missing => Icons.download_for_offline_outlined,
      ToolPkgDependencyState.cycle => Icons.loop_rounded,
      _ => Icons.error_outline_rounded,
    };
    final title = entry?.displayName ?? node.id;
    final depth = math.min(ancestors.length, maxDepth);
    return Semantics(
      label: '$title，${node.state.label}${repeated ? '，共享依赖' : ''}',
      child: InkWell(
        onTap: entry == null
            ? () => _showInfo(context)
            : () => openExtensionDetail(kind: entry.kind, id: entry.id),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: CustomPaint(
            foregroundPainter: hasChildren
                ? _ChildStemPainter(
                    x:
                        (math.min(ancestors.length + 1, maxDepth) - 1) *
                            indent +
                        14,
                    color: branchColor,
                  )
                : null,
            child: Row(
              children: [
                CustomPaint(
                  size: Size(depth * indent, 50),
                  painter: _TreeBranchPainter(
                    ancestors: ancestors,
                    last: last,
                    optional: optional,
                    color: branchColor,
                    indent: indent,
                    maxDepth: maxDepth,
                  ),
                ),
                if (entry == null)
                  const StoreRowGlyph(icon: Icons.link_rounded)
                else
                  CatalogEntryAvatar(clients: clients, entry: entry, size: 28),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        entry == null
                            ? _range(node.sources.first)
                            : 'v${entry.version}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                if (optional)
                  Padding(
                    padding: const EdgeInsets.only(left: 4),
                    child: Text(
                      '可选',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                if (repeated)
                  Tooltip(
                    message: '共享依赖，已在树中展开',
                    child: Icon(
                      Icons.link_rounded,
                      size: 15,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                IconButton(
                  tooltip: '$title · ${node.state.label} · 查看版本要求',
                  onPressed: () => _showInfo(context),
                  visualDensity: VisualDensity.compact,
                  constraints: const BoxConstraints(
                    minWidth: 32,
                    minHeight: 40,
                  ),
                  padding: const EdgeInsets.all(4),
                  icon: Icon(
                    statusIcon,
                    size: 18,
                    color: satisfied
                        ? theme.colorScheme.primary
                        : theme.colorScheme.error,
                  ),
                ),
                if (entry != null)
                  Icon(
                    Icons.chevron_right_rounded,
                    size: 16,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ChildStemPainter extends CustomPainter {
  const _ChildStemPainter({required this.x, required this.color});
  final double x;
  final Color color;
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round;
    canvas.drawLine(
      Offset(x, size.height / 2 + 14),
      Offset(x, size.height),
      paint,
    );
  }

  @override
  bool shouldRepaint(_ChildStemPainter oldDelegate) =>
      oldDelegate.x != x || oldDelegate.color != color;
}

class _TreeBranchPainter extends CustomPainter {
  const _TreeBranchPainter({
    required this.ancestors,
    required this.last,
    required this.optional,
    required this.color,
    required this.indent,
    required this.maxDepth,
  });
  final double indent;
  final int maxDepth;
  final List<bool> ancestors;
  final bool last;
  final bool optional;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (ancestors.isEmpty) return;
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round;
    // The stem starts under the parent's avatar and stays continuous across
    // sibling rows. Each root gets its own branch group.
    for (var i = 1; i < math.min(ancestors.length, maxDepth); i++) {
      if (!ancestors[i]) {
        final x = 14 + (i - 1) * indent;
        canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
      }
    }
    final x = size.width - indent + 14;
    final middle = size.height / 2;
    canvas.drawLine(
      Offset(x, 0),
      Offset(x, last ? middle : size.height),
      paint,
    );
    if (optional) {
      for (double start = x; start < size.width - 2; start += 7) {
        canvas.drawLine(
          Offset(start, middle),
          Offset(math.min(start + 4, size.width - 2), middle),
          paint,
        );
      }
    } else {
      canvas.drawLine(Offset(x, middle), Offset(size.width - 2, middle), paint);
    }
    // A small arrow makes the dependency direction visible without extra text.
    canvas.drawLine(
      Offset(size.width - 5, middle - 3),
      Offset(size.width - 2, middle),
      paint,
    );
    canvas.drawLine(
      Offset(size.width - 5, middle + 3),
      Offset(size.width - 2, middle),
      paint,
    );
  }

  @override
  bool shouldRepaint(_TreeBranchPainter oldDelegate) =>
      oldDelegate.color != color ||
      oldDelegate.indent != indent ||
      oldDelegate.maxDepth != maxDepth ||
      oldDelegate.last != last ||
      oldDelegate.optional != optional ||
      oldDelegate.ancestors.length != ancestors.length ||
      Iterable.generate(
        ancestors.length,
      ).any((i) => oldDelegate.ancestors[i] != ancestors[i]);
}

class _BranchLegend extends CustomPainter {
  const _BranchLegend(this.color, this.dashed);
  final Color color;
  final bool dashed;
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2;
    for (double x = 0; x < size.width; x += dashed ? 6 : size.width) {
      canvas.drawLine(
        Offset(x, size.height / 2),
        Offset(
          dashed ? math.min(x + 4, size.width) : size.width,
          size.height / 2,
        ),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_BranchLegend oldDelegate) =>
      oldDelegate.color != color || oldDelegate.dashed != dashed;
}
