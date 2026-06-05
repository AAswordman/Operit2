// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../theme/OperitGlassSurface.dart';

class ArtifactProjectNodeTreeDialog extends StatelessWidget {
  const ArtifactProjectNodeTreeDialog({
    super.key,
    required this.project,
    required this.onSelectNode,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final ValueChanged<core_proxy.ArtifactProjectNodeResponse> onSelectNode;

  @override
  Widget build(BuildContext context) {
    final viewport = MediaQuery.sizeOf(context);
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Dialog(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: 760,
          maxHeight: viewport.height * 0.88,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 18, 8, 10),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          project.projectDisplayName.trim().isEmpty
                              ? project.projectId
                              : project.projectDisplayName,
                          style: textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 6),
                        Wrap(
                          spacing: 6,
                          runSpacing: 6,
                          children: <Widget>[
                            _SmallChip(text: _artifactTypeLabel(project.type)),
                            _SmallChip(text: '${project.nodes.length} 节点'),
                            _SmallChip(text: '${project.downloads} 下载'),
                            if (project.likes > 0)
                              _SmallChip(text: '${project.likes} 喜欢'),
                          ],
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close),
                    tooltip: '关闭',
                  ),
                ],
              ),
            ),
            if (project.projectDescription.trim().isNotEmpty)
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
                child: Text(
                  project.projectDescription,
                  maxLines: 4,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            const Divider(height: 1),
            Flexible(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 16, 20, 20),
                child: _ArtifactProjectTreeCanvas(
                  project: project,
                  onSelectNode: onSelectNode,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactProjectTreeCanvas extends StatefulWidget {
  const _ArtifactProjectTreeCanvas({
    required this.project,
    required this.onSelectNode,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final ValueChanged<core_proxy.ArtifactProjectNodeResponse> onSelectNode;

  @override
  State<_ArtifactProjectTreeCanvas> createState() =>
      _ArtifactProjectTreeCanvasState();
}

class _ArtifactProjectTreeCanvasState
    extends State<_ArtifactProjectTreeCanvas> {
  final TransformationController _controller = TransformationController();
  String? _fittedProjectId;
  Size? _fittedViewportSize;
  DateTime? _lastTransformGestureAt;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final metrics = const _ArtifactTreeMetrics();
    final layout = _buildArtifactProjectTreeLayout(widget.project, metrics);
    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportSize = Size(
          constraints.maxWidth,
          math.min(math.max(constraints.maxWidth * 0.58, 280), 560),
        );
        _fitTreeIntoViewport(layout, viewportSize);
        return OperitGlassSurface(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.18),
          layer: OperitGlassSurfaceLayer.card,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.14),
          ),
          child: SizedBox(
            height: viewportSize.height,
            child: InteractiveViewer(
              transformationController: _controller,
              constrained: false,
              minScale: 0.35,
              maxScale: 2.8,
              boundaryMargin: const EdgeInsets.all(360),
              onInteractionEnd: (_) {
                _lastTransformGestureAt = DateTime.now();
              },
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onDoubleTap: () {
                  _fitTreeIntoViewport(layout, viewportSize, force: true);
                },
                onTapUp: (details) {
                  final lastTransformGestureAt = _lastTransformGestureAt;
                  if (lastTransformGestureAt != null &&
                      DateTime.now()
                              .difference(lastTransformGestureAt)
                              .inMilliseconds <
                          140) {
                    return;
                  }
                  final position = details.localPosition;
                  for (final node in layout.nodes.reversed) {
                    if (node.rect.contains(position)) {
                      widget.onSelectNode(node.node);
                      return;
                    }
                  }
                },
                child: CustomPaint(
                  size: Size(layout.totalWidth, layout.totalHeight),
                  painter: _ArtifactProjectTreePainter(
                    project: widget.project,
                    layout: layout,
                    metrics: metrics,
                    colorScheme: colorScheme,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  void _fitTreeIntoViewport(
    _ArtifactProjectTreeLayout layout,
    Size viewportSize, {
    bool force = false,
  }) {
    final alreadyFitted =
        _fittedProjectId == widget.project.projectId &&
        _fittedViewportSize == viewportSize;
    if (alreadyFitted && !force) {
      return;
    }
    _fittedProjectId = widget.project.projectId;
    _fittedViewportSize = viewportSize;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      const padding = 20.0;
      final availableWidth = math.max(viewportSize.width - padding * 2, 1);
      final availableHeight = math.max(viewportSize.height - padding * 2, 1);
      final scale = math
          .min(
            math.min(
              availableWidth / layout.totalWidth,
              availableHeight / layout.totalHeight,
            ),
            1,
          )
          .clamp(0.35, 2.8)
          .toDouble();
      final dx = (viewportSize.width - layout.totalWidth * scale) / 2;
      final dy = (viewportSize.height - layout.totalHeight * scale) / 2;
      _controller.value = Matrix4.identity()
        ..translateByDouble(dx, dy, 0, 1)
        ..scaleByDouble(scale, scale, 1, 1);
    });
  }
}

class _ArtifactProjectTreePainter extends CustomPainter {
  const _ArtifactProjectTreePainter({
    required this.project,
    required this.layout,
    required this.metrics,
    required this.colorScheme,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final _ArtifactProjectTreeLayout layout;
  final _ArtifactTreeMetrics metrics;
  final ColorScheme colorScheme;

  @override
  void paint(Canvas canvas, Size size) {
    final nodeById = <String, _ArtifactProjectTreeLayoutNode>{
      for (final node in layout.nodes) node.node.nodeId: node,
    };
    final edgePaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = metrics.edgeWidth;

    for (final child in layout.nodes) {
      for (final parentNodeId in child.node.parentNodeIds) {
        final parent = nodeById[parentNodeId];
        if (parent == null) {
          continue;
        }
        edgePaint.color =
            parent.node.issue.state == 'open' &&
                child.node.issue.state == 'open'
            ? colorScheme.outlineVariant.withValues(alpha: 0.8)
            : colorScheme.outline.withValues(alpha: 0.45);
        _drawArtifactEdge(
          canvas: canvas,
          start: Offset(parent.rect.center.dx, parent.rect.bottom),
          end: Offset(child.rect.center.dx, child.rect.top),
          paint: edgePaint,
        );
      }
    }

    for (final node in layout.nodes) {
      final isDefault = node.node.nodeId == project.defaultNodeId;
      final palette = _resolveArtifactTreeNodePalette(
        colorScheme: colorScheme,
        isDefault: isDefault,
        isOpen: node.node.issue.state == 'open',
      );
      _drawArtifactNode(canvas, node, palette, isDefault);
    }
  }

  void _drawArtifactEdge({
    required Canvas canvas,
    required Offset start,
    required Offset end,
    required Paint paint,
  }) {
    final midY = (start.dy + end.dy) / 2;
    final path = Path()
      ..moveTo(start.dx, start.dy)
      ..cubicTo(start.dx, midY, end.dx, midY, end.dx, end.dy);
    canvas.drawPath(path, paint);
    final leftWing = Offset(
      end.dx - metrics.arrowSize * 0.55,
      end.dy - metrics.arrowSize,
    );
    final rightWing = Offset(
      end.dx + metrics.arrowSize * 0.55,
      end.dy - metrics.arrowSize,
    );
    canvas.drawLine(leftWing, end, paint);
    canvas.drawLine(rightWing, end, paint);
  }

  void _drawArtifactNode(
    Canvas canvas,
    _ArtifactProjectTreeLayoutNode layoutNode,
    _ArtifactTreeNodePalette palette,
    bool isDefault,
  ) {
    final rect = layoutNode.rect;
    final radius = Radius.circular(metrics.cornerRadius);
    final rrect = RRect.fromRectAndRadius(rect, radius);
    final fillPaint = Paint()..color = palette.fillColor;
    final borderPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = isDefault
          ? metrics.borderWidth * 1.18
          : metrics.borderWidth
      ..color = palette.borderColor;
    canvas.drawRRect(rrect, fillPaint);
    canvas.drawRRect(rrect, borderPaint);

    final node = layoutNode.node;
    final lines = <_ArtifactTreeTextLine>[
      _ArtifactTreeTextLine(
        text: _compactArtifactDate(node.publishedAt ?? node.issue.createdAt),
        style: TextStyle(fontSize: 10, color: palette.secondaryTextColor),
      ),
      _ArtifactTreeTextLine(
        text: node.publisherLogin.trim().isEmpty
            ? node.issue.user.login
            : node.publisherLogin,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w700,
          color: palette.primaryTextColor,
        ),
      ),
      _ArtifactTreeTextLine(
        text: 'v${node.version.trim().isEmpty ? '-' : node.version}',
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w700,
          color: palette.primaryTextColor,
        ),
      ),
    ];

    final painters = lines
        .map((line) {
          return TextPainter(
            text: TextSpan(text: line.text, style: line.style),
            textDirection: TextDirection.ltr,
            maxLines: 1,
            ellipsis: '...',
          )..layout(maxWidth: rect.width - metrics.contentPadding * 2);
        })
        .toList(growable: false);
    final totalTextHeight =
        painters.fold<double>(0, (sum, painter) => sum + painter.height) +
        metrics.lineGap * (painters.length - 1);
    var top = rect.top + (rect.height - totalTextHeight) / 2;
    for (final painter in painters) {
      painter.paint(canvas, Offset(rect.center.dx - painter.width / 2, top));
      top += painter.height + metrics.lineGap;
    }
  }

  _ArtifactTreeNodePalette _resolveArtifactTreeNodePalette({
    required ColorScheme colorScheme,
    required bool isDefault,
    required bool isOpen,
  }) {
    if (isDefault && isOpen) {
      return _ArtifactTreeNodePalette(
        fillColor: colorScheme.primaryContainer.withValues(alpha: 0.20),
        borderColor: colorScheme.primary,
        primaryTextColor: colorScheme.onSurface,
        secondaryTextColor: colorScheme.onSurfaceVariant,
      );
    }
    if (isOpen) {
      return _ArtifactTreeNodePalette(
        fillColor: colorScheme.secondaryContainer.withValues(alpha: 0.16),
        borderColor: colorScheme.primary.withValues(alpha: 0.54),
        primaryTextColor: colorScheme.onSurface,
        secondaryTextColor: colorScheme.onSurfaceVariant,
      );
    }
    return _ArtifactTreeNodePalette(
      fillColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.52),
      borderColor: colorScheme.outline.withValues(alpha: 0.54),
      primaryTextColor: colorScheme.onSurfaceVariant,
      secondaryTextColor: colorScheme.onSurfaceVariant.withValues(alpha: 0.92),
    );
  }

  @override
  bool shouldRepaint(covariant _ArtifactProjectTreePainter oldDelegate) {
    return oldDelegate.project != project ||
        oldDelegate.layout != layout ||
        oldDelegate.colorScheme != colorScheme;
  }
}

class _ArtifactProjectTreeLayoutNode {
  const _ArtifactProjectTreeLayoutNode({
    required this.node,
    required this.depth,
    required this.slot,
    required this.rect,
  });

  final core_proxy.ArtifactProjectNodeResponse node;
  final int depth;
  final double slot;
  final Rect rect;
}

class _ArtifactProjectTreeLayout {
  const _ArtifactProjectTreeLayout({
    required this.nodes,
    required this.totalWidth,
    required this.totalHeight,
  });

  final List<_ArtifactProjectTreeLayoutNode> nodes;
  final double totalWidth;
  final double totalHeight;
}

class _ArtifactTreeMetrics {
  const _ArtifactTreeMetrics();

  double get nodeWidth => 108;
  double get nodeHeight => 72;
  double get columnGap => 30;
  double get rowGap => 42;
  double get padding => 24;
  double get cornerRadius => 16;
  double get borderWidth => 3;
  double get edgeWidth => 5;
  double get arrowSize => 10;
  double get contentPadding => 8;
  double get lineGap => 2;
}

class _ArtifactTreeNodePalette {
  const _ArtifactTreeNodePalette({
    required this.fillColor,
    required this.borderColor,
    required this.primaryTextColor,
    required this.secondaryTextColor,
  });

  final Color fillColor;
  final Color borderColor;
  final Color primaryTextColor;
  final Color secondaryTextColor;
}

class _ArtifactTreeTextLine {
  const _ArtifactTreeTextLine({required this.text, required this.style});

  final String text;
  final TextStyle style;
}

class _ArtifactProjectNodePlacement {
  const _ArtifactProjectNodePlacement({
    required this.node,
    required this.preferredSlot,
    required this.order,
  });

  final core_proxy.ArtifactProjectNodeResponse node;
  final double? preferredSlot;
  final double order;
}

_ArtifactProjectTreeLayout _buildArtifactProjectTreeLayout(
  core_proxy.ArtifactProjectDetailResponse project,
  _ArtifactTreeMetrics metrics,
) {
  final nodeById = <String, core_proxy.ArtifactProjectNodeResponse>{
    for (final node in project.nodes) node.nodeId: node,
  };
  final depthCache = <String, int>{};

  int nodeDepth(core_proxy.ArtifactProjectNodeResponse node) {
    final cached = depthCache[node.nodeId];
    if (cached != null) {
      return cached;
    }
    final parents = <core_proxy.ArtifactProjectNodeResponse>[
      for (final parentId in node.parentNodeIds)
        if (nodeById[parentId] != null) nodeById[parentId]!,
    ];
    final depth = parents.isEmpty
        ? 0
        : parents.map(nodeDepth).reduce(math.max) + 1;
    depthCache[node.nodeId] = depth;
    return depth;
  }

  final nodesByDepth = <int, List<core_proxy.ArtifactProjectNodeResponse>>{};
  for (final node in project.nodes) {
    (nodesByDepth[nodeDepth(node)] ??=
            <core_proxy.ArtifactProjectNodeResponse>[])
        .add(node);
  }
  final maxDepth = nodesByDepth.keys.fold<int>(0, math.max);
  final slotByNodeId = <String, double>{};

  for (var depth = 0; depth <= maxDepth; depth += 1) {
    final nodesAtDepth =
        nodesByDepth[depth] ?? const <core_proxy.ArtifactProjectNodeResponse>[];
    if (nodesAtDepth.isEmpty) {
      continue;
    }
    final placements =
        <_ArtifactProjectNodePlacement>[
          for (var index = 0; index < nodesAtDepth.length; index += 1)
            _ArtifactProjectNodePlacement(
              node: nodesAtDepth[index],
              preferredSlot: _averageSlot(
                nodesAtDepth[index].parentNodeIds
                    .map((id) => slotByNodeId[id])
                    .whereType<double>()
                    .toList(growable: false),
              ),
              order: index.toDouble(),
            ),
        ]..sort((left, right) {
          final slotOrder = (left.preferredSlot ?? left.order).compareTo(
            right.preferredSlot ?? right.order,
          );
          if (slotOrder != 0) {
            return slotOrder;
          }
          final dateOrder = (left.node.publishedAt ?? left.node.issue.createdAt)
              .compareTo(right.node.publishedAt ?? right.node.issue.createdAt);
          return dateOrder == 0
              ? left.node.nodeId.compareTo(right.node.nodeId)
              : dateOrder;
        });

    final resolvedSlots = <double>[
      for (final placement in placements)
        placement.preferredSlot ?? placement.order,
    ];
    for (var index = 1; index < resolvedSlots.length; index += 1) {
      resolvedSlots[index] = math.max(
        resolvedSlots[index],
        resolvedSlots[index - 1] + 1,
      );
    }
    final preferredMean = _mean(<double>[
      for (final placement in placements)
        placement.preferredSlot ?? placement.order,
    ]);
    final resolvedMean = _mean(resolvedSlots);
    final shift = preferredMean - resolvedMean;
    for (var index = 0; index < resolvedSlots.length; index += 1) {
      slotByNodeId[placements[index].node.nodeId] =
          resolvedSlots[index] + shift;
    }
  }

  final slots = slotByNodeId.values.toList(growable: false);
  final minSlot = slots.isEmpty ? 0.0 : slots.reduce(math.min);
  final maxSlot = slots.isEmpty ? 0.0 : slots.reduce(math.max);
  final layoutNodes =
      <_ArtifactProjectTreeLayoutNode>[
        for (final node in project.nodes)
          _ArtifactProjectTreeLayoutNode(
            node: node,
            depth: nodeDepth(node),
            slot: slotByNodeId[node.nodeId] ?? 0,
            rect: Rect.fromLTWH(
              metrics.padding +
                  ((slotByNodeId[node.nodeId] ?? 0) - minSlot) *
                      (metrics.nodeWidth + metrics.columnGap),
              metrics.padding +
                  nodeDepth(node) * (metrics.nodeHeight + metrics.rowGap),
              metrics.nodeWidth,
              metrics.nodeHeight,
            ),
          ),
      ]..sort((left, right) {
        final depthOrder = left.depth.compareTo(right.depth);
        return depthOrder == 0 ? left.slot.compareTo(right.slot) : depthOrder;
      });

  return _ArtifactProjectTreeLayout(
    nodes: layoutNodes,
    totalWidth:
        metrics.padding * 2 +
        metrics.nodeWidth +
        (maxSlot - minSlot) * (metrics.nodeWidth + metrics.columnGap),
    totalHeight:
        metrics.padding * 2 +
        metrics.nodeHeight +
        maxDepth * (metrics.nodeHeight + metrics.rowGap),
  );
}

double? _averageSlot(List<double> values) {
  if (values.isEmpty) {
    return null;
  }
  return _mean(values);
}

double _mean(List<double> values) {
  return values.reduce((left, right) => left + right) / values.length;
}

String _compactArtifactDate(String value) {
  return value.split('T').first.trim().isEmpty ? value : value.split('T').first;
}

String _artifactTypeLabel(String type) {
  return switch (type.trim()) {
    'package' => 'Package',
    'script' => 'Script',
    final value when value.isNotEmpty => value,
    _ => 'Artifact',
  };
}

class _SmallChip extends StatelessWidget {
  const _SmallChip({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.62),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          text,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}
