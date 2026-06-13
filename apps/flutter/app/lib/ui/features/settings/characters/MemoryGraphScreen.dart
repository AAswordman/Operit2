// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';

class MemoryGraphScreen extends StatefulWidget {
  const MemoryGraphScreen({
    super.key,
    required this.bridge,
    required this.ownerKey,
    required this.ownerName,
  });

  final OperitRuntimeBridge bridge;
  final String ownerKey;
  final String ownerName;

  static Future<void> open({
    required BuildContext context,
    required OperitRuntimeBridge bridge,
    required String ownerKey,
    required String ownerName,
  }) {
    return Navigator.of(context).push<void>(
      MaterialPageRoute<void>(
        builder: (context) => MemoryGraphScreen(
          bridge: bridge,
          ownerKey: ownerKey,
          ownerName: ownerName,
        ),
      ),
    );
  }

  @override
  State<MemoryGraphScreen> createState() => _MemoryGraphScreenState();
}

class _MemoryGraphScreenState extends State<MemoryGraphScreen> {
  late Future<core_proxy.MemoryGraph> _future;
  _MemoryGraphLayout? _layout;
  Size? _layoutSize;
  String? _selectedNodeId;
  int? _selectedEdgeId;
  double _scale = 1;
  Offset _offset = Offset.zero;
  double _startScale = 1;
  Offset _startOffset = Offset.zero;

  GeneratedRepositoryMemoryRepositoryCoreProxy get _repository =>
      GeneratedRepositoryMemoryRepositoryCoreProxy(
        widget.bridge,
        CoreObjectPath(<String>[
          'repository',
          'memoryRepository',
          widget.ownerKey,
        ]),
      );

  @override
  void initState() {
    super.initState();
    _future = _repository.getMemoryGraph();
  }

  void _resetLayout(core_proxy.MemoryGraph graph, Size size) {
    _layout = _MemoryGraphLayout.compute(graph, size);
    _layoutSize = size;
    _scale = 1;
    _offset = Offset.zero;
    _selectedNodeId = null;
    _selectedEdgeId = null;
  }

  void _ensureLayout(core_proxy.MemoryGraph graph, Size size) {
    final oldSize = _layoutSize;
    final shouldCompute =
        _layout == null ||
        oldSize == null ||
        (oldSize.width - size.width).abs() > 24 ||
        (oldSize.height - size.height).abs() > 24;
    if (!shouldCompute) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      setState(() {
        _resetLayout(graph, size);
      });
    });
  }

  void _handleTap(
    TapUpDetails details,
    core_proxy.MemoryGraph graph,
    _MemoryGraphLayout layout,
    TextTheme textTheme,
  ) {
    final world = (details.localPosition - _offset) / _scale;
    final hitNode = graph.nodes.reversed
        .where(
          (node) => _nodeWorldRect(node, layout.positions[node.id]!, textTheme)
              .inflate(12 / _scale)
              .contains(world),
        )
        .firstOrNull;
    if (hitNode != null) {
      setState(() {
        _selectedNodeId = hitNode.id;
        _selectedEdgeId = null;
      });
      return;
    }
    final hitEdge = graph.edges
        .where((edge) {
          final start = layout.positions[edge.sourceId];
          final end = layout.positions[edge.targetId];
          if (start == null || end == null) {
            return false;
          }
          return _distanceToSegment(world, start, end) < 18 / _scale;
        })
        .firstOrNull;
    setState(() {
      _selectedNodeId = null;
      _selectedEdgeId = hitEdge?.id;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(
          title: Text(l10n.settingsCharactersMemoryGraphTitle(widget.ownerName)),
          leading: IconButton(
            tooltip: l10n.close,
            onPressed: () => Navigator.of(context).pop(),
            icon: const Icon(Icons.close),
          ),
          actions: <Widget>[
            IconButton(
              tooltip: l10n.refresh,
              onPressed: () {
                setState(() {
                  _future = _repository.getMemoryGraph();
                  _layout = null;
                  _layoutSize = null;
                });
              },
              icon: const Icon(Icons.refresh),
            ),
          ],
      ),
      body: FutureBuilder<core_proxy.MemoryGraph>(
          future: _future,
          builder: (context, snapshot) {
            final graph = snapshot.data;
            if (graph == null) {
              return const M3LoadingPane();
            }
            if (graph.nodes.isEmpty) {
              return Center(
                child: Text(
                  l10n.settingsCharactersMemoryGraphEmpty,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              );
            }
            return LayoutBuilder(
              builder: (context, constraints) {
                final size = Size(
                  constraints.maxWidth,
                  constraints.maxHeight,
                );
                _ensureLayout(graph, size);
                final layout = _layout;
                if (layout == null) {
                  return const M3LoadingPane();
                }
                final selectedNode = _selectedNodeId == null
                    ? null
                    : graph.nodes
                        .where((node) => node.id == _selectedNodeId)
                        .firstOrNull;
                final selectedEdge = _selectedEdgeId == null
                    ? null
                    : graph.edges
                        .where((edge) => edge.id == _selectedEdgeId)
                        .firstOrNull;
                return Stack(
                  children: <Widget>[
                    GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onScaleStart: (details) {
                        _startScale = _scale;
                        _startOffset = _offset;
                      },
                      onScaleUpdate: (details) {
                        setState(() {
                          final nextScale =
                              (_startScale * details.scale).clamp(0.2, 5.0);
                          final focal = details.localFocalPoint;
                          _offset =
                              (_startOffset - focal) *
                                  (nextScale / _startScale) +
                              focal +
                              details.focalPointDelta;
                          _scale = nextScale;
                        });
                      },
                      onTapUp: (details) =>
                          _handleTap(details, graph, layout, textTheme),
                      child: CustomPaint(
                        painter: _MemoryGraphPainter(
                          graph: graph,
                          layout: layout,
                          scale: _scale,
                          offset: _offset,
                          colorScheme: colorScheme,
                          textTheme: textTheme,
                          selectedNodeId: _selectedNodeId,
                          selectedEdgeId: _selectedEdgeId,
                        ),
                        size: Size.infinite,
                      ),
                    ),
                    Positioned(
                      left: 16,
                      top: 12,
                      child: _MemoryGraphCounter(
                        text: l10n.settingsCharactersMemoryGraphStats(
                          graph.nodes.length,
                          graph.edges.length,
                        ),
                      ),
                    ),
                    if (selectedNode != null || selectedEdge != null)
                      Positioned(
                        left: 16,
                        right: 16,
                        bottom: 16,
                        child: _MemoryGraphSelectionCard(
                          node: selectedNode,
                          edge: selectedEdge,
                          graph: graph,
                          onClose: () {
                            setState(() {
                              _selectedNodeId = null;
                              _selectedEdgeId = null;
                            });
                          },
                        ),
                      ),
                  ],
                );
              },
            );
          },
      ),
    );
  }
}

class _MemoryGraphLayout {
  const _MemoryGraphLayout(this.positions);

  final Map<String, Offset> positions;

  static _MemoryGraphLayout compute(core_proxy.MemoryGraph graph, Size size) {
    final positions = <String, Offset>{};
    final center = Offset(size.width / 2, size.height / 2);
    final clusterInfo = _GraphClusterInfo.fromGraph(graph);
    final random = math.Random(17);
    final clusterIds = clusterInfo.clusterIds;
    final clusterCount = clusterIds.length;
    final ringRadius = math.min(size.width, size.height) * 0.34;
    final clusterCenters = <int, Offset>{};
    for (var index = 0; index < clusterIds.length; index += 1) {
      final clusterId = clusterIds[index];
      final angle = math.pi * 2 * index / clusterCount;
      clusterCenters[clusterId] = clusterCount == 1
          ? center
          : center + Offset(math.cos(angle), math.sin(angle)) * ringRadius;
    }

    final neighborsByNode = <String, List<String>>{};
    for (final node in graph.nodes) {
      neighborsByNode[node.id] = <String>[];
    }
    for (final edge in graph.edges) {
      neighborsByNode[edge.sourceId]?.add(edge.targetId);
      neighborsByNode[edge.targetId]?.add(edge.sourceId);
    }
    for (final node in graph.nodes) {
      final clusterId = clusterInfo.clusterByNodeId[node.id]!;
      final clusterCenter = clusterCenters[clusterId]!;
      final clusterSize = clusterInfo.clusterSizes[clusterId]!;
      final scatterRadius = math.min(70 + clusterSize * 4, 200).toDouble();
      final angle = random.nextDouble() * math.pi * 2;
      final radius = random.nextDouble() * scatterRadius;
      positions[node.id] =
          clusterCenter + Offset(math.cos(angle), math.sin(angle)) * radius;
    }

    final nodeCount = graph.nodes.length;
    final iterations = nodeCount > 200
        ? 150
        : nodeCount > 100
        ? 200
        : nodeCount > 50
        ? 250
        : 300;
    const repulsionStrength = 380000.0;
    const attractionStrength = 0.07;
    const idealEdgeLength = 560.0;
    const gravityStrength = 0.005;
    const minNodeSeparation = 380.0;
    final nodeIdsByCluster = <int, List<String>>{};
    for (final entry in clusterInfo.clusterByNodeId.entries) {
      nodeIdsByCluster.putIfAbsent(entry.value, () => <String>[]).add(entry.key);
    }

    for (var step = 0; step < iterations; step += 1) {
      final forces = <String, Offset>{
        for (final node in graph.nodes) node.id: Offset.zero,
      };
      for (var i = 0; i < graph.nodes.length; i += 1) {
        final left = graph.nodes[i];
        final leftPos = positions[left.id]!;
        for (var j = i + 1; j < graph.nodes.length; j += 1) {
          final right = graph.nodes[j];
          final rightPos = positions[right.id]!;
          final delta = leftPos - rightPos;
          final distanceSq = math.max(
            delta.dx * delta.dx + delta.dy * delta.dy,
            1.0,
          );
          final distance = math.sqrt(distanceSq);
          final direction = delta / distance;
          final force = repulsionStrength / distanceSq;
          forces[left.id] = forces[left.id]! + direction * force;
          forces[right.id] = forces[right.id]! - direction * force;
          if (distance < minNodeSeparation) {
            final overlapRatio =
                (minNodeSeparation - distance) / minNodeSeparation;
            final separationForce = math.pow(overlapRatio, 2.25) * 1200;
            forces[left.id] =
                forces[left.id]! + direction * separationForce.toDouble();
            forces[right.id] =
                forces[right.id]! - direction * separationForce.toDouble();
          }
        }
      }
      for (final edge in graph.edges) {
        final sourcePos = positions[edge.sourceId];
        final targetPos = positions[edge.targetId];
        if (sourcePos == null || targetPos == null) {
          continue;
        }
        final delta = targetPos - sourcePos;
        final distanceSq = math.max(delta.dx * delta.dx + delta.dy * delta.dy, 1);
        final distance = math.sqrt(distanceSq);
        final direction = delta / distance;
        final edgeTypeAttraction = edge.isCrossFolderLink ? 0.45 : 1.95;
        final edgeCompression = edge.isCrossFolderLink ? 0.8 : 2.15;
        final edgeIdealLength =
            edge.isCrossFolderLink ? idealEdgeLength * 1.35 : idealEdgeLength * 0.72;
        final adjustedAttraction =
            attractionStrength * (1 + edge.weight * 0.35) * edgeTypeAttraction;
        final normalizedDelta = (distance - edgeIdealLength) / edgeIdealLength;
        final absNormalizedDelta = normalizedDelta.abs();
        var springForce = 0.0;
        if (absNormalizedDelta < 0.06) {
          springForce =
              adjustedAttraction * edgeIdealLength * normalizedDelta * 0.18;
        } else if (normalizedDelta > 0) {
          springForce =
              adjustedAttraction *
              edgeIdealLength *
              math.pow(normalizedDelta, 1.35).toDouble();
        } else {
          springForce =
              -adjustedAttraction *
              edgeIdealLength *
              math.pow(absNormalizedDelta, 2.35).toDouble() *
              0.78 *
              edgeCompression;
        }
        final force = direction * springForce;
        forces[edge.sourceId] = forces[edge.sourceId]! + force;
        forces[edge.targetId] = forces[edge.targetId]! - force;
      }

      final clusterSums = <int, Offset>{};
      final clusterCounts = <int, int>{};
      for (final node in graph.nodes) {
        final clusterId = clusterInfo.clusterByNodeId[node.id]!;
        clusterSums[clusterId] =
            (clusterSums[clusterId] ?? Offset.zero) + positions[node.id]!;
        clusterCounts[clusterId] = (clusterCounts[clusterId] ?? 0) + 1;
      }
      final centroids = <int, Offset>{};
      for (final entry in clusterSums.entries) {
        centroids[entry.key] = entry.value / clusterCounts[entry.key]!.toDouble();
      }
      for (final node in graph.nodes) {
        final clusterId = clusterInfo.clusterByNodeId[node.id]!;
        final clusterSize = clusterInfo.clusterSizes[clusterId]!;
        if (clusterSize <= 1) {
          continue;
        }
        final centroid = centroids[clusterId]!;
        final nodePos = positions[node.id]!;
        final toCenter = centroid - nodePos;
        final distance = math.max(toCenter.distance, 1.0);
        final desiredRadius =
            minNodeSeparation * 0.42 + math.min(clusterSize - 1, 24) * 3.2;
        if (distance <= desiredRadius) {
          continue;
        }
        final normalized = (distance - desiredRadius) / desiredRadius;
        final cohesion = math.pow(normalized, 1.2).toDouble() * 46;
        forces[node.id] = forces[node.id]! + toCenter / distance * cohesion;
      }
      final centroidIds = centroids.keys.toList(growable: false);
      for (var i = 0; i < centroidIds.length; i += 1) {
        for (var j = i + 1; j < centroidIds.length; j += 1) {
          final leftId = centroidIds[i];
          final rightId = centroidIds[j];
          final delta = centroids[leftId]! - centroids[rightId]!;
          final distance = delta.distance;
          if (distance <= 1) {
            continue;
          }
          const preferredDistance = idealEdgeLength * 1.15;
          if (distance >= preferredDistance) {
            continue;
          }
          final direction = delta / distance;
          final overlap = ((preferredDistance - distance) / preferredDistance)
              .clamp(0.0, 1.0);
          final repulse = math.pow(overlap, 1.35).toDouble() * 38;
          for (final nodeId in nodeIdsByCluster[leftId]!) {
            forces[nodeId] = forces[nodeId]! + direction * repulse;
          }
          for (final nodeId in nodeIdsByCluster[rightId]!) {
            forces[nodeId] = forces[nodeId]! - direction * repulse;
          }
        }
      }
      for (final node in graph.nodes) {
        forces[node.id] = forces[node.id]! + (center - positions[node.id]!) * gravityStrength;
      }
      final coolingProgress = step / math.max(iterations - 1, 1);
      final temperature = math.max(
        0.9,
        idealEdgeLength * 0.42 * math.pow(1 - coolingProgress, 2.15),
      );
      for (final node in graph.nodes) {
        final force = forces[node.id]!;
        final length = force.distance;
        if (length < 0.0001) {
          continue;
        }
        final limitedMove = math.min(length, temperature) * 0.78;
        positions[node.id] = positions[node.id]! + force / length * limitedMove;
      }
    }
    return _MemoryGraphLayout(Map.unmodifiable(positions));
  }
}

class _GraphClusterInfo {
  const _GraphClusterInfo({
    required this.clusterByNodeId,
    required this.clusterSizes,
    required this.clusterIds,
  });

  final Map<String, int> clusterByNodeId;
  final Map<int, int> clusterSizes;
  final List<int> clusterIds;

  static _GraphClusterInfo fromGraph(core_proxy.MemoryGraph graph) {
    final adjacency = <String, List<String>>{};
    for (final node in graph.nodes) {
      adjacency[node.id] = <String>[];
    }
    for (final edge in graph.edges) {
      if (edge.isCrossFolderLink) {
        continue;
      }
      adjacency[edge.sourceId]?.add(edge.targetId);
      adjacency[edge.targetId]?.add(edge.sourceId);
    }
    final visited = <String>{};
    final clusterByNodeId = <String, int>{};
    final clusterSizes = <int, int>{};
    var clusterId = 0;
    for (final node in graph.nodes) {
      if (!visited.add(node.id)) {
        continue;
      }
      clusterId += 1;
      var size = 0;
      final queue = <String>[node.id];
      var index = 0;
      while (index < queue.length) {
        final current = queue[index];
        index += 1;
        clusterByNodeId[current] = clusterId;
        size += 1;
        for (final neighbor in adjacency[current]!) {
          if (visited.add(neighbor)) {
            queue.add(neighbor);
          }
        }
      }
      clusterSizes[clusterId] = size;
    }
    return _GraphClusterInfo(
      clusterByNodeId: Map.unmodifiable(clusterByNodeId),
      clusterSizes: Map.unmodifiable(clusterSizes),
      clusterIds: List<int>.unmodifiable(clusterSizes.keys.toList()..sort()),
    );
  }
}

class _MemoryGraphPainter extends CustomPainter {
  const _MemoryGraphPainter({
    required this.graph,
    required this.layout,
    required this.scale,
    required this.offset,
    required this.colorScheme,
    required this.textTheme,
    required this.selectedNodeId,
    required this.selectedEdgeId,
  });

  final core_proxy.MemoryGraph graph;
  final _MemoryGraphLayout layout;
  final double scale;
  final Offset offset;
  final ColorScheme colorScheme;
  final TextTheme textTheme;
  final String? selectedNodeId;
  final int? selectedEdgeId;

  @override
  void paint(Canvas canvas, Size size) {
    final nodeById = {for (final node in graph.nodes) node.id: node};
    final visibleRect = Offset.zero & size;
    final outlinePaint = Paint()
      ..color = colorScheme.outline.withValues(alpha: 0.58)
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    for (final edge in graph.edges) {
      final startWorld = layout.positions[edge.sourceId];
      final endWorld = layout.positions[edge.targetId];
      if (startWorld == null || endWorld == null) {
        continue;
      }
      final start = startWorld * scale + offset;
      final end = endWorld * scale + offset;
      final sourceNode = nodeById[edge.sourceId];
      final targetNode = nodeById[edge.targetId];
      final sourceVisible = sourceNode != null &&
          _nodeScreenRect(sourceNode, start, textTheme, scale).overlaps(visibleRect);
      final targetVisible = targetNode != null &&
          _nodeScreenRect(targetNode, end, textTheme, scale).overlaps(visibleRect);
      if (!sourceVisible && !targetVisible) {
        continue;
      }
      outlinePaint
        ..color = edge.id == selectedEdgeId
            ? colorScheme.error
            : colorScheme.outline.withValues(alpha: 0.58)
        ..strokeWidth = edge.isCrossFolderLink
            ? (edge.weight * 2.6 * scale).clamp(1.0, 8.0)
            : (edge.weight * 5.8 * scale).clamp(1.2, 18.0);
      if (edge.isCrossFolderLink) {
        _drawDashedLine(canvas, start, end, outlinePaint);
      } else {
        canvas.drawLine(start, end, outlinePaint);
      }
      final label = edge.label;
      if (label != null && label.isNotEmpty) {
        final center = (start + end) / 2;
        if (visibleRect.contains(center)) {
          final painter = _textPainter(
            label,
            textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
            maxWidth: 220,
          );
          painter.paint(
            canvas,
            center - Offset(painter.width / 2, painter.height / 2),
          );
        }
      }
    }
    for (final node in graph.nodes) {
      final world = layout.positions[node.id];
      if (world == null) {
        continue;
      }
      final screen = world * scale + offset;
      final rect = _nodeScreenRect(node, screen, textTheme, scale);
      if (!rect.overlaps(visibleRect)) {
        continue;
      }
      _drawNode(canvas, node, screen, scale, node.id == selectedNodeId);
    }
  }

  void _drawNode(
    Canvas canvas,
    core_proxy.MemoryGraphNode node,
    Offset center,
    double scale,
    bool selected,
  ) {
    final visualScale = scale.clamp(0.15, 2.2);
    final textPainter = _textPainter(
      node.label,
      textTheme.labelMedium?.copyWith(color: _nodeTextColor()),
      maxWidth: 280,
    );
    final width = textPainter.width + 28;
    final height = textPainter.height + 8;
    final rect = Rect.fromCenter(
      center: center,
      width: width * visualScale,
      height: height * visualScale,
    );
    final radius = Radius.circular(math.min(rect.height * 0.48, 20 * visualScale));
    final underlayRect = rect.translate(0, 1.2 * visualScale);
    canvas.drawRRect(
      RRect.fromRectAndRadius(underlayRect, radius),
      Paint()..color = _nodeUnderlayColor(),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, radius),
      Paint()..color = _nodeFillColor(),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, radius),
      Paint()
        ..color = selected ? colorScheme.secondary : colorScheme.outline
        ..strokeWidth = selected ? 2.2 : 1.8
        ..style = PaintingStyle.stroke,
    );
    canvas.save();
    canvas.translate(rect.left + 14 * visualScale, rect.top + 4 * visualScale);
    canvas.scale(visualScale);
    textPainter.paint(canvas, Offset.zero);
    canvas.restore();
  }

  Color _nodeTextColor() {
    return colorScheme.surface.computeLuminance() < 0.42
        ? const Color(0xFFE5E7EB)
        : const Color(0xFF1F2937);
  }

  Color _nodeFillColor() {
    return colorScheme.surface.computeLuminance() < 0.42
        ? const Color(0xFF2B313D)
        : const Color(0xFFE5E7EB);
  }

  Color _nodeUnderlayColor() {
    return colorScheme.surface.computeLuminance() < 0.42
        ? const Color(0xFF1F2530)
        : const Color(0xFFD1D5DB);
  }

  @override
  bool shouldRepaint(_MemoryGraphPainter oldDelegate) {
    return graph != oldDelegate.graph ||
        layout != oldDelegate.layout ||
        scale != oldDelegate.scale ||
        offset != oldDelegate.offset ||
        selectedNodeId != oldDelegate.selectedNodeId ||
        selectedEdgeId != oldDelegate.selectedEdgeId ||
        colorScheme != oldDelegate.colorScheme;
  }
}

class _MemoryGraphCounter extends StatelessWidget {
  const _MemoryGraphCounter({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.86),
      borderRadius: BorderRadius.circular(16),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
        child: Text(
          text,
          style: Theme.of(context).textTheme.labelMedium?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }
}

class _MemoryGraphSelectionCard extends StatelessWidget {
  const _MemoryGraphSelectionCard({
    required this.node,
    required this.edge,
    required this.graph,
    required this.onClose,
  });

  final core_proxy.MemoryGraphNode? node;
  final core_proxy.MemoryGraphEdge? edge;
  final core_proxy.MemoryGraph graph;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final title = node?.label ?? edge?.label ?? l10n.settingsCharactersMemoryGraphLink;
    final subtitle = edge == null
        ? node?.id
        : '${_nodeLabel(edge!.sourceId)}  ->  ${_nodeLabel(edge!.targetId)}';
    return Material(
      color: colorScheme.surfaceContainerHighest,
      elevation: 3,
      borderRadius: BorderRadius.circular(20),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
        child: Row(
          children: <Widget>[
            Icon(
              node == null ? Icons.link : Icons.circle_outlined,
              size: 20,
              color: colorScheme.primary,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  if (subtitle != null) ...<Widget>[
                    const SizedBox(height: 3),
                    Text(
                      subtitle,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            IconButton(
              tooltip: l10n.close,
              onPressed: onClose,
              icon: const Icon(Icons.close),
            ),
          ],
        ),
      ),
    );
  }

  String _nodeLabel(String nodeId) {
    return graph.nodes
        .where((candidate) => candidate.id == nodeId)
        .map((candidate) => candidate.label)
        .firstOrNull!;
  }
}

TextPainter _textPainter(String text, TextStyle? style, {required double maxWidth}) {
  return TextPainter(
    text: TextSpan(text: text, style: style),
    maxLines: 3,
    ellipsis: '...',
    textDirection: TextDirection.ltr,
    textScaler: TextScaler.noScaling,
  )..layout(maxWidth: maxWidth);
}

Rect _nodeWorldRect(
  core_proxy.MemoryGraphNode node,
  Offset center,
  TextTheme textTheme,
) {
  final painter = _textPainter(node.label, textTheme.labelMedium, maxWidth: 280);
  return Rect.fromCenter(
    center: center,
    width: painter.width + 28,
    height: painter.height + 8,
  );
}

Rect _nodeScreenRect(
  core_proxy.MemoryGraphNode node,
  Offset center,
  TextTheme textTheme,
  double scale,
) {
  final visualScale = scale.clamp(0.15, 2.2);
  final worldRect = _nodeWorldRect(node, center, textTheme);
  return Rect.fromCenter(
    center: center,
    width: worldRect.width * visualScale,
    height: worldRect.height * visualScale,
  );
}

double _distanceToSegment(Offset point, Offset start, Offset end) {
  final lengthSq = (start - end).distanceSquared;
  if (lengthSq == 0) {
    return (point - start).distance;
  }
  final t = (((point.dx - start.dx) * (end.dx - start.dx) +
              (point.dy - start.dy) * (end.dy - start.dy)) /
          lengthSq)
      .clamp(0.0, 1.0);
  final projection = start + (end - start) * t;
  return (point - projection).distance;
}

void _drawDashedLine(Canvas canvas, Offset start, Offset end, Paint paint) {
  const dash = 12.0;
  const gap = 12.0;
  final delta = end - start;
  final distance = delta.distance;
  if (distance <= 0) {
    return;
  }
  final direction = delta / distance;
  var drawn = 0.0;
  while (drawn < distance) {
    final segmentStart = start + direction * drawn;
    final segmentEnd = start + direction * math.min(drawn + dash, distance);
    canvas.drawLine(segmentStart, segmentEnd, paint);
    drawn += dash + gap;
  }
}
