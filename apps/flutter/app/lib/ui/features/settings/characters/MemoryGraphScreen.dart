// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math' as math;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';

const XTypeGroup _memoryJsonFileTypeGroup = XTypeGroup(
  label: 'Operit memory JSON',
  extensions: <String>['json'],
);

class MemoryGraphScreen extends StatefulWidget {
  /// Creates the owner-scoped memory management graph page.
  const MemoryGraphScreen({
    super.key,
    required this.bridge,
    required this.ownerKey,
    required this.ownerName,
  });

  final OperitRuntimeBridge bridge;
  final String ownerKey;
  final String ownerName;

  /// Opens the owner-scoped memory graph page.
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

  /// Creates the state for the owner-scoped memory graph page.
  @override
  State<MemoryGraphScreen> createState() => _MemoryGraphScreenState();
}

class _MemoryGraphScreenState extends State<MemoryGraphScreen> {
  late Future<_MemoryGraphData> _future;
  final TextEditingController _searchController = TextEditingController();
  _MemoryGraphLayout? _layout;
  Size? _layoutSize;
  String _layoutSignature = '';
  String? _selectedNodeId;
  int? _selectedEdgeId;
  Future<core_proxy.Memory?>? _selectedMemoryFuture;
  String _folderPath = '';
  bool _folderPanelOpen = false;
  bool _busy = false;
  bool _linkMode = false;
  String? _linkSourceNodeId;
  double _scale = 1;
  Offset _offset = Offset.zero;
  double _startScale = 1;
  Offset _startOffset = Offset.zero;

  /// Returns the repository proxy scoped to the current owner key.
  GeneratedRepositoryMemoryRepositoryCoreProxy get _repository =>
      GeneratedRepositoryMemoryRepositoryCoreProxy(
        widget.bridge,
        CoreObjectPath(<String>[
          'repository',
          'memoryRepository',
          widget.ownerKey,
        ]),
      );

  /// Initializes the page by loading graph data.
  @override
  void initState() {
    super.initState();
    _future = _loadData();
  }

  /// Releases text editing resources.
  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Loads graph, folder, and current filter data.
  Future<_MemoryGraphData> _loadData() async {
    final repository = _repository;
    final graph = await repository.getMemoryGraph();
    final folders = await repository.getAllFolderPaths();
    final query = _searchController.text.trim();
    final activeFolder = _folderPath.trim();
    List<core_proxy.Memory> scopedMemories = <core_proxy.Memory>[];
    core_proxy.MemoryGraph displayGraph = graph;
    if (query.isNotEmpty) {
      scopedMemories = await repository.searchMemories(
        query: query,
        folderPath: null,
        relevanceThreshold: 0.0,
        createdAtStartMs: null,
        createdAtEndMs: null,
      );
      if (activeFolder.isNotEmpty) {
        scopedMemories = scopedMemories
            .where(
              (memory) =>
                  _matchesFolderTree(memory.folderPath ?? '', activeFolder),
            )
            .toList(growable: false);
      }
      displayGraph = _graphAroundMemoryIds(
        graph,
        scopedMemories.map((memory) => memory.uuid).toSet(),
      );
    } else if (activeFolder.isNotEmpty) {
      scopedMemories = await _loadFolderTreeMemories(folders, activeFolder);
      displayGraph = _graphForMemoryIds(
        graph,
        scopedMemories.map((memory) => memory.uuid).toSet(),
      );
    }
    return _MemoryGraphData(
      fullGraph: graph,
      displayGraph: displayGraph,
      folders: folders,
      scopedMemoryByUuid: <String, core_proxy.Memory>{
        for (final memory in scopedMemories) memory.uuid: memory,
      },
      query: query,
      folderPath: activeFolder,
    );
  }

  /// Loads memories from a folder and its visible child folders.
  Future<List<core_proxy.Memory>> _loadFolderTreeMemories(
    List<String> folders,
    String folderPath,
  ) async {
    final targetFolders = _folderAndChildren(folders, folderPath);
    final memoriesByUuid = <String, core_proxy.Memory>{};
    for (final targetFolder in targetFolders) {
      final memories = await _repository.getMemoriesByFolderPath(
        folderPath: targetFolder,
      );
      for (final memory in memories) {
        memoriesByUuid[memory.uuid] = memory;
      }
    }
    return memoriesByUuid.values.toList(growable: false);
  }

  /// Reloads data and clears transient graph selection.
  void _reload() {
    setState(() {
      _future = _loadData();
      _layout = null;
      _layoutSize = null;
      _layoutSignature = '';
      _selectedNodeId = null;
      _selectedEdgeId = null;
      _selectedMemoryFuture = null;
      _linkSourceNodeId = null;
    });
  }

  /// Refreshes the current data without changing filters.
  Future<void> _refresh() async {
    _reload();
  }

  /// Runs the current search text.
  void _runSearch() {
    _reload();
  }

  /// Clears the search box and reloads the graph.
  void _clearSearch() {
    _searchController.clear();
    _reload();
  }

  /// Selects a folder path and reloads the graph.
  void _selectFolder(String folderPath) {
    setState(() {
      _folderPath = folderPath;
      _future = _loadData();
      _layout = null;
      _layoutSize = null;
      _layoutSignature = '';
      _selectedNodeId = null;
      _selectedEdgeId = null;
      _selectedMemoryFuture = null;
      _linkSourceNodeId = null;
    });
  }

  /// Recomputes the graph layout when data or size changed.
  void _ensureLayout(core_proxy.MemoryGraph graph, Size size) {
    final signature = _graphSignature(graph);
    final oldSize = _layoutSize;
    final shouldCompute =
        _layout == null ||
        _layoutSignature != signature ||
        oldSize == null ||
        (oldSize.width - size.width).abs() > 32 ||
        (oldSize.height - size.height).abs() > 32;
    if (!shouldCompute) {
      return;
    }
    _layout = _MemoryGraphLayout.compute(graph, size);
    _layoutSize = size;
    _layoutSignature = signature;
    _scale = 1;
    _offset = Offset.zero;
  }

  /// Handles graph taps for normal selection and link creation.
  void _handleTap(
    TapUpDetails details,
    _MemoryGraphData data,
    _MemoryGraphLayout layout,
  ) {
    final graph = data.displayGraph;
    final world = (details.localPosition - _offset) / _scale;
    final hitNode = graph.nodes.reversed.where((node) {
      final center = layout.positions[node.id];
      return center != null &&
          _nodeWorldRect(node, center).inflate(16 / _scale).contains(world);
    }).firstOrNull;
    if (hitNode != null) {
      _selectNode(hitNode, data);
      return;
    }
    final hitEdge = graph.edges.where((edge) {
      final start = layout.positions[edge.sourceId];
      final end = layout.positions[edge.targetId];
      if (start == null || end == null) {
        return false;
      }
      return _distanceToSegment(world, start, end) < 18 / _scale;
    }).firstOrNull;
    setState(() {
      _selectedNodeId = null;
      _selectedEdgeId = hitEdge?.id;
      _selectedMemoryFuture = null;
    });
  }

  /// Selects a graph node or advances link mode.
  void _selectNode(core_proxy.MemoryGraphNode node, _MemoryGraphData data) {
    if (_linkMode) {
      _selectNodeForLink(node, data);
      return;
    }
    setState(() {
      _selectedNodeId = node.id;
      _selectedEdgeId = null;
      _selectedMemoryFuture = _memoryForNode(node, data);
    });
  }

  /// Selects source and target nodes for a new link.
  void _selectNodeForLink(
    core_proxy.MemoryGraphNode node,
    _MemoryGraphData data,
  ) {
    final sourceNodeId = _linkSourceNodeId;
    if (sourceNodeId == null) {
      setState(() {
        _linkSourceNodeId = node.id;
        _selectedNodeId = node.id;
        _selectedEdgeId = null;
        _selectedMemoryFuture = _memoryForNode(node, data);
      });
      return;
    }
    if (sourceNodeId == node.id) {
      return;
    }
    final sourceNode = data.displayGraph.nodes
        .where((candidate) => candidate.id == sourceNodeId)
        .firstOrNull;
    if (sourceNode == null) {
      setState(() {
        _linkSourceNodeId = null;
      });
      return;
    }
    _createLink(sourceNode, node, data);
  }

  /// Looks up full memory data for a graph node.
  Future<core_proxy.Memory?> _memoryForNode(
    core_proxy.MemoryGraphNode node,
    _MemoryGraphData data,
  ) async {
    final scopedMemory = data.scopedMemoryByUuid[node.id];
    if (scopedMemory != null) {
      return scopedMemory;
    }
    final memories = await _repository.findMemoriesByTitle(title: node.label);
    return memories.where((memory) => memory.uuid == node.id).firstOrNull;
  }

  /// Creates a memory link between two graph nodes.
  Future<void> _createLink(
    core_proxy.MemoryGraphNode sourceNode,
    core_proxy.MemoryGraphNode targetNode,
    _MemoryGraphData data,
  ) async {
    final sourceMemory = await _memoryForNode(sourceNode, data);
    final targetMemory = await _memoryForNode(targetNode, data);
    if (!mounted) {
      return;
    }
    if (sourceMemory == null || targetMemory == null) {
      _showSnack('无法定位记忆节点');
      return;
    }
    final edited = await _MemoryLinkEditorDialog.show(
      context: context,
      sourceTitle: sourceMemory.title,
      targetTitle: targetMemory.title,
    );
    if (edited == null) {
      setState(() {
        _linkSourceNodeId = null;
      });
      return;
    }
    setState(() => _busy = true);
    try {
      await _repository.linkMemories(
        sourceMemoryId: sourceMemory.id,
        targetMemoryId: targetMemory.id,
        type: edited.type,
        weight: edited.weight,
        description: edited.description,
      );
      if (!mounted) {
        return;
      }
      _showSnack('记忆关系已创建');
      setState(() {
        _linkSourceNodeId = null;
        _future = _loadData();
        _layout = null;
        _layoutSignature = '';
      });
    } catch (error) {
      if (mounted) {
        _showSnack('创建关系失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Opens the memory editor for creating a new memory.
  Future<void> _createMemory() async {
    final data = await _future;
    if (!mounted) {
      return;
    }
    final edited = await _MemoryEditorDialog.show(
      context: context,
      folders: data.folders,
      initialFolderPath: _folderPath,
    );
    if (edited == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _repository.createMemory(
        title: edited.title,
        content: edited.content,
        contentType: edited.contentType,
        source: edited.source,
        folderPath: edited.folderPath,
        tags: edited.tags,
      );
      if (!mounted) {
        return;
      }
      _showSnack('记忆已创建');
      _reload();
    } catch (error) {
      if (mounted) {
        _showSnack('创建记忆失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Opens the memory editor for an existing memory.
  Future<void> _editMemory(
    core_proxy.Memory memory,
    List<String> folders,
  ) async {
    final edited = await _MemoryEditorDialog.show(
      context: context,
      memory: memory,
      folders: folders,
      initialFolderPath: memory.folderPath ?? '',
    );
    if (edited == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _repository.updateMemory(
        memoryId: memory.id,
        newTitle: edited.title,
        newContent: edited.content,
        newContentType: edited.contentType,
        newSource: edited.source,
        newCredibility: edited.credibility,
        newImportance: edited.importance,
        newFolderPath: edited.folderPath,
        newTags: edited.tags,
      );
      if (!mounted) {
        return;
      }
      _showSnack('记忆已保存');
      _reload();
    } catch (error) {
      if (mounted) {
        _showSnack('保存记忆失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Deletes a memory after user confirmation.
  Future<void> _deleteMemory(core_proxy.Memory memory) async {
    final confirmed = await _confirm(
      title: '删除记忆',
      message: '确定删除「${memory.title}」吗？关联关系也会被移除。',
      confirmLabel: '删除',
    );
    if (!confirmed) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _repository.deleteMemory(memoryId: memory.id);
      if (!mounted) {
        return;
      }
      _showSnack('记忆已删除');
      _reload();
    } catch (error) {
      if (mounted) {
        _showSnack('删除记忆失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Deletes a memory link after user confirmation.
  Future<void> _deleteEdge(core_proxy.MemoryGraphEdge edge) async {
    final confirmed = await _confirm(
      title: '删除关系',
      message: '确定删除这条记忆关系吗？',
      confirmLabel: '删除',
    );
    if (!confirmed) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _repository.deleteLink(linkId: edge.id);
      if (!mounted) {
        return;
      }
      _showSnack('记忆关系已删除');
      _reload();
    } catch (error) {
      if (mounted) {
        _showSnack('删除关系失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Exports memory JSON to a user-selected file.
  Future<void> _exportJson() async {
    final suggestedName =
        'operit-memory-${DateTime.now().millisecondsSinceEpoch}.json';
    final location = await getSaveLocation(
      acceptedTypeGroups: const <XTypeGroup>[_memoryJsonFileTypeGroup],
      suggestedName: suggestedName,
    );
    if (location == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final jsonText = await _repository.exportMemoriesToJson();
      await XFile.fromData(
        Uint8List.fromList(utf8.encode(jsonText)),
        name: suggestedName,
        mimeType: 'application/json',
      ).saveTo(location.path);
      if (mounted) {
        _showSnack('已导出到 ${location.path}');
      }
    } catch (error) {
      if (mounted) {
        _showSnack('导出失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Copies exported memory JSON to the clipboard.
  Future<void> _copyJson() async {
    setState(() => _busy = true);
    try {
      final jsonText = await _repository.exportMemoriesToJson();
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (mounted) {
        _showSnack('记忆 JSON 已复制');
      }
    } catch (error) {
      if (mounted) {
        _showSnack('复制失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Imports memory JSON from a user-selected file.
  Future<void> _importJson() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[_memoryJsonFileTypeGroup],
    );
    if (file == null) {
      return;
    }
    if (!mounted) {
      return;
    }
    final strategy = await _ImportStrategyDialog.show(context: context);
    if (strategy == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final jsonText = await file.readAsString();
      final result = await _repository.importMemoriesFromJson(
        jsonString: jsonText,
        strategy: strategy,
      );
      if (!mounted) {
        return;
      }
      _showSnack(
        '导入完成：新增 ${result.newMemories}，更新 ${result.updatedMemories}，跳过 ${result.skippedMemories}，关系 ${result.newLinks}',
      );
      _reload();
    } catch (error) {
      if (mounted) {
        _showSnack('导入失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Toggles graph link creation mode.
  void _toggleLinkMode() {
    setState(() {
      _linkMode = !_linkMode;
      _linkSourceNodeId = null;
    });
  }

  /// Shows a confirmation dialog.
  Future<bool> _confirm({
    required String title,
    required String message,
    required String confirmLabel,
  }) async {
    final result = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(confirmLabel),
          ),
        ],
      ),
    );
    return result == true;
  }

  /// Shows a short status message.
  void _showSnack(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  /// Builds the page scaffold and graph canvas.
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
            tooltip: '导入 JSON',
            onPressed: _busy ? null : _importJson,
            icon: const Icon(Icons.upload_file_outlined),
          ),
          IconButton(
            tooltip: '导出 JSON',
            onPressed: _busy ? null : _exportJson,
            icon: const Icon(Icons.download_outlined),
          ),
          IconButton(
            tooltip: '复制 JSON',
            onPressed: _busy ? null : _copyJson,
            icon: const Icon(Icons.copy_all_outlined),
          ),
          IconButton(
            tooltip: l10n.refresh,
            onPressed: _busy ? null : _refresh,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      floatingActionButton: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          FloatingActionButton.small(
            heroTag: 'memory-link',
            tooltip: '创建关系',
            backgroundColor: _linkMode
                ? colorScheme.primary
                : colorScheme.secondaryContainer,
            foregroundColor: _linkMode
                ? colorScheme.onPrimary
                : colorScheme.onSecondaryContainer,
            onPressed: _busy ? null : _toggleLinkMode,
            child: const Icon(Icons.link),
          ),
          const SizedBox(height: 10),
          FloatingActionButton(
            heroTag: 'memory-create',
            tooltip: '新建记忆',
            onPressed: _busy ? null : _createMemory,
            child: const Icon(Icons.add),
          ),
        ],
      ),
      body: FutureBuilder<_MemoryGraphData>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
          }
          final data = snapshot.data;
          if (data == null) {
            return const M3LoadingPane();
          }
          final graph = data.displayGraph;
          return Column(
            children: <Widget>[
              _MemoryToolbar(
                controller: _searchController,
                folderPath: _folderPath,
                linkMode: _linkMode,
                busy: _busy,
                onSearch: _runSearch,
                onClearSearch: _clearSearch,
                onToggleFolders: () {
                  setState(() => _folderPanelOpen = !_folderPanelOpen);
                },
              ),
              if (_busy) const LinearProgressIndicator(minHeight: 2),
              Expanded(
                child: Row(
                  children: <Widget>[
                    AnimatedContainer(
                      duration: const Duration(milliseconds: 180),
                      curve: Curves.easeOutCubic,
                      width: _folderPanelOpen ? 260 : 0,
                      child: _folderPanelOpen
                          ? _MemoryFolderPanel(
                              folders: data.folders,
                              selectedFolderPath: _folderPath,
                              onSelected: _selectFolder,
                            )
                          : const SizedBox.shrink(),
                    ),
                    Expanded(
                      child: graph.nodes.isEmpty
                          ? _MemoryEmptyState(
                              query: data.query,
                              folderPath: data.folderPath,
                              colorScheme: colorScheme,
                              textTheme: textTheme,
                            )
                          : LayoutBuilder(
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
                                          .where(
                                            (node) =>
                                                node.id == _selectedNodeId,
                                          )
                                          .firstOrNull;
                                final selectedEdge = _selectedEdgeId == null
                                    ? null
                                    : graph.edges
                                          .where(
                                            (edge) =>
                                                edge.id == _selectedEdgeId,
                                          )
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
                                              (_startScale * details.scale)
                                                  .clamp(0.2, 5.0)
                                                  .toDouble();
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
                                          _handleTap(details, data, layout),
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
                                          linkSourceNodeId: _linkSourceNodeId,
                                        ),
                                        size: Size.infinite,
                                      ),
                                    ),
                                    Positioned(
                                      left: 16,
                                      top: 12,
                                      child: _MemoryGraphCounter(
                                        text:
                                            '${graph.nodes.length} 节点 · ${graph.edges.length} 关系',
                                      ),
                                    ),
                                    if (_linkMode)
                                      Positioned(
                                        right: 16,
                                        top: 12,
                                        child: _MemoryGraphCounter(
                                          text: _linkSourceNodeId == null
                                              ? '关系模式：选择起点'
                                              : '关系模式：选择终点',
                                        ),
                                      ),
                                    if (selectedNode != null ||
                                        selectedEdge != null)
                                      Positioned(
                                        left: 16,
                                        right: 16,
                                        bottom: 16,
                                        child: _MemoryGraphSelectionCard(
                                          node: selectedNode,
                                          edge: selectedEdge,
                                          graph: graph,
                                          memoryFuture: _selectedMemoryFuture,
                                          folders: data.folders,
                                          onClose: () {
                                            setState(() {
                                              _selectedNodeId = null;
                                              _selectedEdgeId = null;
                                              _selectedMemoryFuture = null;
                                            });
                                          },
                                          onEditMemory: _editMemory,
                                          onDeleteMemory: _deleteMemory,
                                          onDeleteEdge: _deleteEdge,
                                        ),
                                      ),
                                  ],
                                );
                              },
                            ),
                    ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _MemoryGraphData {
  /// Creates immutable graph data for the current filters.
  const _MemoryGraphData({
    required this.fullGraph,
    required this.displayGraph,
    required this.folders,
    required this.scopedMemoryByUuid,
    required this.query,
    required this.folderPath,
  });

  final core_proxy.MemoryGraph fullGraph;
  final core_proxy.MemoryGraph displayGraph;
  final List<String> folders;
  final Map<String, core_proxy.Memory> scopedMemoryByUuid;
  final String query;
  final String folderPath;
}

class _MemoryGraphLayout {
  /// Creates graph layout positions for each node id.
  const _MemoryGraphLayout(this.positions);

  final Map<String, Offset> positions;

  /// Computes a deterministic clustered layout in linear graph time.
  static _MemoryGraphLayout compute(core_proxy.MemoryGraph graph, Size size) {
    final positions = <String, Offset>{};
    final center = Offset(size.width / 2, size.height / 2);
    final clusterInfo = _GraphClusterInfo.fromGraph(graph);
    final clusters =
        clusterInfo.clusterIds
            .map(
              (clusterId) => _ClusterPlacement(
                id: clusterId,
                nodeIds: clusterInfo.nodeIdsByCluster[clusterId]!,
              ),
            )
            .toList(growable: false)
          ..sort(
            (left, right) =>
                right.nodeIds.length.compareTo(left.nodeIds.length),
          );
    final clusterCount = clusters.length;
    final columns = math.max(1, math.sqrt(clusterCount).ceil());
    final cellWidth = math.max(size.width * 0.92, 860.0);
    final cellHeight = math.max(size.height * 0.82, 680.0);
    for (
      var clusterIndex = 0;
      clusterIndex < clusters.length;
      clusterIndex += 1
    ) {
      final cluster = clusters[clusterIndex];
      final row = clusterIndex ~/ columns;
      final column = clusterIndex % columns;
      final x = center.dx + (column - (columns - 1) / 2) * cellWidth;
      final y =
          center.dy + (row - ((clusterCount - 1) ~/ columns) / 2) * cellHeight;
      _placeClusterNodes(
        graph: graph,
        cluster: cluster,
        center: Offset(x, y),
        positions: positions,
      );
    }
    return _MemoryGraphLayout(Map.unmodifiable(positions));
  }
}

class _ClusterPlacement {
  /// Creates a cluster placement payload.
  const _ClusterPlacement({required this.id, required this.nodeIds});

  final int id;
  final List<String> nodeIds;
}

class _GraphClusterInfo {
  /// Creates cluster information derived from graph connectivity.
  const _GraphClusterInfo({
    required this.clusterByNodeId,
    required this.nodeIdsByCluster,
    required this.clusterIds,
  });

  final Map<String, int> clusterByNodeId;
  final Map<int, List<String>> nodeIdsByCluster;
  final List<int> clusterIds;

  /// Groups nodes by non-cross-folder graph connectivity.
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
    final nodeIdsByCluster = <int, List<String>>{};
    var clusterId = 0;
    for (final node in graph.nodes) {
      if (!visited.add(node.id)) {
        continue;
      }
      clusterId += 1;
      final nodeIds = <String>[];
      final queue = <String>[node.id];
      var index = 0;
      while (index < queue.length) {
        final current = queue[index];
        index += 1;
        clusterByNodeId[current] = clusterId;
        nodeIds.add(current);
        for (final neighbor in adjacency[current]!) {
          if (visited.add(neighbor)) {
            queue.add(neighbor);
          }
        }
      }
      nodeIdsByCluster[clusterId] = List<String>.unmodifiable(nodeIds);
    }
    return _GraphClusterInfo(
      clusterByNodeId: Map.unmodifiable(clusterByNodeId),
      nodeIdsByCluster: Map.unmodifiable(nodeIdsByCluster),
      clusterIds: List<int>.unmodifiable(
        nodeIdsByCluster.keys.toList()..sort(),
      ),
    );
  }
}

class _MemoryGraphPainter extends CustomPainter {
  /// Creates a painter for the memory graph.
  const _MemoryGraphPainter({
    required this.graph,
    required this.layout,
    required this.scale,
    required this.offset,
    required this.colorScheme,
    required this.textTheme,
    required this.selectedNodeId,
    required this.selectedEdgeId,
    required this.linkSourceNodeId,
  });

  final core_proxy.MemoryGraph graph;
  final _MemoryGraphLayout layout;
  final double scale;
  final Offset offset;
  final ColorScheme colorScheme;
  final TextTheme textTheme;
  final String? selectedNodeId;
  final int? selectedEdgeId;
  final String? linkSourceNodeId;

  /// Paints the visible graph nodes and edges.
  @override
  void paint(Canvas canvas, Size size) {
    final nodeById = {for (final node in graph.nodes) node.id: node};
    final visibleRect = Offset.zero & size;
    final outlinePaint = Paint()
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
      if (!_edgeMayBeVisible(start, end, visibleRect)) {
        continue;
      }
      outlinePaint
        ..color = edge.id == selectedEdgeId
            ? colorScheme.error
            : colorScheme.outline.withValues(alpha: 0.58)
        ..strokeWidth = edge.isCrossFolderLink
            ? (edge.weight * 2.6 * scale).clamp(1.0, 8.0).toDouble()
            : (edge.weight * 5.8 * scale).clamp(1.2, 18.0).toDouble();
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
      final rect = _nodeScreenRect(node, screen, scale);
      if (!rect.overlaps(visibleRect)) {
        continue;
      }
      _drawNode(
        canvas,
        node,
        nodeById[node.id]!,
        screen,
        scale,
        node.id == selectedNodeId,
        node.id == linkSourceNodeId,
      );
    }
  }

  /// Draws a single memory node.
  void _drawNode(
    Canvas canvas,
    core_proxy.MemoryGraphNode node,
    core_proxy.MemoryGraphNode visualNode,
    Offset center,
    double scale,
    bool selected,
    bool linkSource,
  ) {
    final visualScale = scale.clamp(0.15, 2.2).toDouble();
    final textPainter = _textPainter(
      visualNode.label,
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
    final radius = Radius.circular(
      math.min(rect.height * 0.48, 20 * visualScale),
    );
    final underlayRect = rect.translate(0, 1.2 * visualScale);
    canvas.drawRRect(
      RRect.fromRectAndRadius(underlayRect, radius),
      Paint()..color = _nodeUnderlayColor(),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, radius),
      Paint()..color = _nodeFillColor(node),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, radius),
      Paint()
        ..color = linkSource
            ? colorScheme.tertiary
            : selected
            ? colorScheme.secondary
            : colorScheme.outline
        ..strokeWidth = linkSource || selected ? 2.4 : 1.7
        ..style = PaintingStyle.stroke,
    );
    canvas.save();
    canvas.translate(rect.left + 14 * visualScale, rect.top + 4 * visualScale);
    canvas.scale(visualScale);
    textPainter.paint(canvas, Offset.zero);
    canvas.restore();
  }

  /// Returns text color for node labels.
  Color _nodeTextColor() {
    return colorScheme.surface.computeLuminance() < 0.42
        ? const Color(0xFFE5E7EB)
        : const Color(0xFF1F2937);
  }

  /// Returns fill color for a graph node.
  Color _nodeFillColor(core_proxy.MemoryGraphNode node) {
    final nodeColor = Color(node.color & 0xFFFFFFFF);
    return Color.alphaBlend(
      nodeColor.withValues(alpha: 0.22),
      colorScheme.surface.computeLuminance() < 0.42
          ? const Color(0xFF2B313D)
          : const Color(0xFFE5E7EB),
    );
  }

  /// Returns underlay color for graph nodes.
  Color _nodeUnderlayColor() {
    return colorScheme.surface.computeLuminance() < 0.42
        ? const Color(0xFF1F2530)
        : const Color(0xFFD1D5DB);
  }

  /// Reports whether repainting is required.
  @override
  bool shouldRepaint(_MemoryGraphPainter oldDelegate) {
    return graph != oldDelegate.graph ||
        layout != oldDelegate.layout ||
        scale != oldDelegate.scale ||
        offset != oldDelegate.offset ||
        selectedNodeId != oldDelegate.selectedNodeId ||
        selectedEdgeId != oldDelegate.selectedEdgeId ||
        linkSourceNodeId != oldDelegate.linkSourceNodeId ||
        colorScheme != oldDelegate.colorScheme;
  }
}

class _MemoryToolbar extends StatelessWidget {
  /// Creates the top filter and action toolbar.
  const _MemoryToolbar({
    required this.controller,
    required this.folderPath,
    required this.linkMode,
    required this.busy,
    required this.onSearch,
    required this.onClearSearch,
    required this.onToggleFolders,
  });

  final TextEditingController controller;
  final String folderPath;
  final bool linkMode;
  final bool busy;
  final VoidCallback onSearch;
  final VoidCallback onClearSearch;
  final VoidCallback onToggleFolders;

  /// Builds the top memory toolbar.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surface,
      elevation: 1,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
        child: Row(
          children: <Widget>[
            IconButton(
              tooltip: '文件夹',
              onPressed: busy ? null : onToggleFolders,
              icon: const Icon(Icons.folder_outlined),
            ),
            const SizedBox(width: 6),
            Expanded(
              child: TextField(
                controller: controller,
                enabled: !busy,
                textInputAction: TextInputAction.search,
                onSubmitted: (_) => onSearch(),
                decoration: InputDecoration(
                  isDense: true,
                  prefixIcon: const Icon(Icons.search),
                  suffixIcon: controller.text.isEmpty
                      ? null
                      : IconButton(
                          tooltip: '清空',
                          onPressed: busy ? null : onClearSearch,
                          icon: const Icon(Icons.clear),
                        ),
                  hintText: '搜索标题、正文、来源或标签',
                  border: const OutlineInputBorder(),
                ),
              ),
            ),
            const SizedBox(width: 10),
            FilledButton.icon(
              onPressed: busy ? null : onSearch,
              icon: const Icon(Icons.search, size: 18),
              label: const Text('搜索'),
            ),
            const SizedBox(width: 10),
            _MemoryStatusChip(
              icon: linkMode ? Icons.link : Icons.account_tree_outlined,
              text: linkMode ? '关系模式' : _folderLabel(folderPath),
            ),
          ],
        ),
      ),
    );
  }
}

class _MemoryFolderPanel extends StatelessWidget {
  /// Creates the folder filtering side panel.
  const _MemoryFolderPanel({
    required this.folders,
    required this.selectedFolderPath,
    required this.onSelected,
  });

  final List<String> folders;
  final String selectedFolderPath;
  final ValueChanged<String> onSelected;

  /// Builds the folder list panel.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final sortedFolders = folders.toList(growable: false)..sort();
    return Material(
      color: colorScheme.surfaceContainerLow,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 14, 12, 8),
            child: Row(
              children: <Widget>[
                Icon(Icons.folder_outlined, color: colorScheme.primary),
                const SizedBox(width: 8),
                Text('记忆文件夹', style: Theme.of(context).textTheme.titleSmall),
              ],
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(vertical: 6),
              children: <Widget>[
                _FolderTile(
                  title: '全部',
                  selected: selectedFolderPath.isEmpty,
                  depth: 0,
                  onTap: () => onSelected(''),
                ),
                for (final folder in sortedFolders)
                  _FolderTile(
                    title: folder,
                    selected: folder == selectedFolderPath,
                    depth: _folderDepth(folder),
                    onTap: () => onSelected(folder),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _FolderTile extends StatelessWidget {
  /// Creates a single folder navigation row.
  const _FolderTile({
    required this.title,
    required this.selected,
    required this.depth,
    required this.onTap,
  });

  final String title;
  final bool selected;
  final int depth;
  final VoidCallback onTap;

  /// Builds a folder row.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final displayTitle = title.isEmpty ? '未分类' : title.split('/').last;
    return Padding(
      padding: EdgeInsets.only(left: 8.0 + depth * 14.0, right: 8, top: 2),
      child: ListTile(
        dense: true,
        selected: selected,
        selectedTileColor: colorScheme.primaryContainer.withValues(alpha: 0.7),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        leading: Icon(selected ? Icons.folder_open : Icons.folder, size: 20),
        title: Text(displayTitle, maxLines: 1, overflow: TextOverflow.ellipsis),
        subtitle: depth == 0 && title == displayTitle ? null : Text(title),
        onTap: onTap,
      ),
    );
  }
}

class _MemoryGraphCounter extends StatelessWidget {
  /// Creates a floating graph status chip.
  const _MemoryGraphCounter({required this.text});

  final String text;

  /// Builds the floating graph status chip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.88),
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

class _MemoryStatusChip extends StatelessWidget {
  /// Creates a compact status chip.
  const _MemoryStatusChip({required this.icon, required this.text});

  final IconData icon;
  final String text;

  /// Builds the compact status chip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: colorScheme.secondaryContainer.withValues(alpha: 0.62),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 17, color: colorScheme.onSecondaryContainer),
          const SizedBox(width: 6),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 180),
            child: Text(
              text,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: colorScheme.onSecondaryContainer,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _MemoryEmptyState extends StatelessWidget {
  /// Creates the empty graph state.
  const _MemoryEmptyState({
    required this.query,
    required this.folderPath,
    required this.colorScheme,
    required this.textTheme,
  });

  final String query;
  final String folderPath;
  final ColorScheme colorScheme;
  final TextTheme textTheme;

  /// Builds the empty graph state.
  @override
  Widget build(BuildContext context) {
    final hasFilter = query.isNotEmpty || folderPath.isNotEmpty;
    return Center(
      child: Text(
        hasFilter ? '当前筛选没有记忆节点' : '当前记忆库还没有节点',
        style: textTheme.bodyMedium?.copyWith(
          color: colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _MemoryGraphSelectionCard extends StatelessWidget {
  /// Creates a selected node or edge details card.
  const _MemoryGraphSelectionCard({
    required this.node,
    required this.edge,
    required this.graph,
    required this.memoryFuture,
    required this.folders,
    required this.onClose,
    required this.onEditMemory,
    required this.onDeleteMemory,
    required this.onDeleteEdge,
  });

  final core_proxy.MemoryGraphNode? node;
  final core_proxy.MemoryGraphEdge? edge;
  final core_proxy.MemoryGraph graph;
  final Future<core_proxy.Memory?>? memoryFuture;
  final List<String> folders;
  final VoidCallback onClose;
  final Future<void> Function(core_proxy.Memory memory, List<String> folders)
  onEditMemory;
  final Future<void> Function(core_proxy.Memory memory) onDeleteMemory;
  final Future<void> Function(core_proxy.MemoryGraphEdge edge) onDeleteEdge;

  /// Builds the selected node or edge details card.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final title =
        node?.label ?? edge?.label ?? l10n.settingsCharactersMemoryGraphLink;
    final subtitle = edge == null
        ? node?.id
        : '${_nodeLabel(edge!.sourceId)}  →  ${_nodeLabel(edge!.targetId)}';
    return Material(
      color: colorScheme.surfaceContainerHighest,
      elevation: 3,
      borderRadius: BorderRadius.circular(20),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
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
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: colorScheme.onSurfaceVariant),
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
            if (memoryFuture != null)
              FutureBuilder<core_proxy.Memory?>(
                future: memoryFuture,
                builder: (context, snapshot) {
                  final memory = snapshot.data;
                  if (snapshot.connectionState != ConnectionState.done) {
                    return const Padding(
                      padding: EdgeInsets.only(top: 10),
                      child: LinearProgressIndicator(minHeight: 2),
                    );
                  }
                  if (memory == null) {
                    return const Padding(
                      padding: EdgeInsets.only(top: 8),
                      child: Text('未读取到完整记忆内容'),
                    );
                  }
                  return _MemoryDetailsBlock(
                    memory: memory,
                    folders: folders,
                    onEditMemory: onEditMemory,
                    onDeleteMemory: onDeleteMemory,
                  );
                },
              ),
            if (edge != null)
              Align(
                alignment: Alignment.centerRight,
                child: TextButton.icon(
                  onPressed: () => onDeleteEdge(edge!),
                  icon: const Icon(Icons.delete_outline),
                  label: const Text('删除关系'),
                ),
              ),
          ],
        ),
      ),
    );
  }

  /// Returns a display label for a node id.
  String _nodeLabel(String nodeId) {
    return graph.nodes
        .where((candidate) => candidate.id == nodeId)
        .map((candidate) => candidate.label)
        .firstOrNull!;
  }
}

class _MemoryDetailsBlock extends StatelessWidget {
  /// Creates the selected memory detail block.
  const _MemoryDetailsBlock({
    required this.memory,
    required this.folders,
    required this.onEditMemory,
    required this.onDeleteMemory,
  });

  final core_proxy.Memory memory;
  final List<String> folders;
  final Future<void> Function(core_proxy.Memory memory, List<String> folders)
  onEditMemory;
  final Future<void> Function(core_proxy.Memory memory) onDeleteMemory;

  /// Builds memory details and management actions.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: <Widget>[
              _InfoChip(
                label: '文件夹',
                value: _folderLabel(memory.folderPath ?? ''),
              ),
              _InfoChip(label: '来源', value: memory.source),
              _InfoChip(
                label: '可信度',
                value: memory.credibility.toStringAsFixed(2),
              ),
              _InfoChip(
                label: '重要性',
                value: memory.importance.toStringAsFixed(2),
              ),
              _InfoChip(label: '更新', value: _formatMillis(memory.updatedAt)),
            ],
          ),
          if (memory.tags.isNotEmpty) ...<Widget>[
            const SizedBox(height: 8),
            Wrap(
              spacing: 6,
              runSpacing: 4,
              children: <Widget>[
                for (final tag in memory.tags)
                  Chip(
                    label: Text(tag.name),
                    visualDensity: VisualDensity.compact,
                  ),
              ],
            ),
          ],
          const SizedBox(height: 8),
          Container(
            constraints: const BoxConstraints(maxHeight: 120),
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: colorScheme.surface.withValues(alpha: 0.58),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: colorScheme.outlineVariant),
            ),
            child: SingleChildScrollView(
              child: SelectableText(
                memory.content,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: <Widget>[
              TextButton.icon(
                onPressed: () => onEditMemory(memory, folders),
                icon: const Icon(Icons.edit_outlined),
                label: const Text('编辑'),
              ),
              const SizedBox(width: 6),
              TextButton.icon(
                onPressed: () => onDeleteMemory(memory),
                icon: const Icon(Icons.delete_outline),
                label: const Text('删除'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  /// Creates a small label-value chip.
  const _InfoChip({required this.label, required this.value});

  final String label;
  final String value;

  /// Builds the label-value chip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: colorScheme.outlineVariant),
      ),
      child: Text(
        '$label：$value',
        style: Theme.of(context).textTheme.labelSmall,
      ),
    );
  }
}

class _MemoryEditorResult {
  /// Creates a memory editor result payload.
  const _MemoryEditorResult({
    required this.title,
    required this.content,
    required this.contentType,
    required this.source,
    required this.folderPath,
    required this.tags,
    required this.credibility,
    required this.importance,
  });

  final String title;
  final String content;
  final String contentType;
  final String source;
  final String folderPath;
  final List<String> tags;
  final double credibility;
  final double importance;
}

class _MemoryEditorDialog extends StatefulWidget {
  /// Creates a memory editor dialog.
  const _MemoryEditorDialog({
    this.memory,
    required this.folders,
    required this.initialFolderPath,
  });

  final core_proxy.Memory? memory;
  final List<String> folders;
  final String initialFolderPath;

  /// Shows a memory editor dialog.
  static Future<_MemoryEditorResult?> show({
    required BuildContext context,
    core_proxy.Memory? memory,
    required List<String> folders,
    required String initialFolderPath,
  }) {
    return showDialog<_MemoryEditorResult>(
      context: context,
      builder: (context) => _MemoryEditorDialog(
        memory: memory,
        folders: folders,
        initialFolderPath: initialFolderPath,
      ),
    );
  }

  /// Creates state for the memory editor dialog.
  @override
  State<_MemoryEditorDialog> createState() => _MemoryEditorDialogState();
}

class _MemoryEditorDialogState extends State<_MemoryEditorDialog> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  late final TextEditingController _titleController;
  late final TextEditingController _contentController;
  late final TextEditingController _contentTypeController;
  late final TextEditingController _sourceController;
  late final TextEditingController _folderController;
  late final TextEditingController _tagsController;
  late double _credibility;
  late double _importance;

  /// Initializes text controllers from the edited memory.
  @override
  void initState() {
    super.initState();
    final memory = widget.memory;
    _titleController = TextEditingController(text: memory?.title ?? '');
    _contentController = TextEditingController(text: memory?.content ?? '');
    _contentTypeController = TextEditingController(
      text: memory?.contentType ?? 'text/plain',
    );
    _sourceController = TextEditingController(text: memory?.source ?? 'manual');
    _folderController = TextEditingController(
      text: memory?.folderPath ?? widget.initialFolderPath,
    );
    _tagsController = TextEditingController(
      text: memory?.tags.map((tag) => tag.name).join(', ') ?? '',
    );
    _credibility = memory?.credibility ?? 0.5;
    _importance = memory?.importance ?? 0.5;
  }

  /// Releases editor text controllers.
  @override
  void dispose() {
    _titleController.dispose();
    _contentController.dispose();
    _contentTypeController.dispose();
    _sourceController.dispose();
    _folderController.dispose();
    _tagsController.dispose();
    super.dispose();
  }

  /// Saves dialog input into the result payload.
  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final tags = _tagsController.text
        .split(',')
        .map((tag) => tag.trim())
        .where((tag) => tag.isNotEmpty)
        .toList(growable: false);
    Navigator.of(context).pop(
      _MemoryEditorResult(
        title: _titleController.text.trim(),
        content: _contentController.text,
        contentType: _contentTypeController.text.trim(),
        source: _sourceController.text.trim(),
        folderPath: _folderController.text.trim(),
        tags: tags,
        credibility: _credibility,
        importance: _importance,
      ),
    );
  }

  /// Builds the memory editor dialog.
  @override
  Widget build(BuildContext context) {
    final folderOptions = <String>{
      '',
      ...widget.folders,
      _folderController.text,
    }.map((folder) => folder.trim()).toSet().toList(growable: false)..sort();
    return AlertDialog(
      title: Text(widget.memory == null ? '新建记忆' : '编辑记忆'),
      content: SizedBox(
        width: 720,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                TextFormField(
                  controller: _titleController,
                  decoration: const InputDecoration(labelText: '标题'),
                  validator: (value) =>
                      value == null || value.trim().isEmpty ? '请输入标题' : null,
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _contentController,
                  minLines: 8,
                  maxLines: 14,
                  decoration: const InputDecoration(
                    labelText: '内容',
                    alignLabelWithHint: true,
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 10),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: TextFormField(
                        controller: _contentTypeController,
                        decoration: const InputDecoration(labelText: '内容类型'),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: TextFormField(
                        controller: _sourceController,
                        decoration: const InputDecoration(labelText: '来源'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                DropdownButtonFormField<String>(
                  initialValue:
                      folderOptions.contains(_folderController.text.trim())
                      ? _folderController.text.trim()
                      : '',
                  items: <DropdownMenuItem<String>>[
                    for (final folder in folderOptions)
                      DropdownMenuItem<String>(
                        value: folder,
                        child: Text(_folderLabel(folder)),
                      ),
                  ],
                  decoration: const InputDecoration(labelText: '文件夹'),
                  onChanged: (value) {
                    _folderController.text = value ?? '';
                  },
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _folderController,
                  decoration: const InputDecoration(labelText: '文件夹路径'),
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _tagsController,
                  decoration: const InputDecoration(labelText: '标签（逗号分隔）'),
                ),
                const SizedBox(height: 12),
                _SliderEditor(
                  label: '可信度',
                  value: _credibility,
                  onChanged: (value) => setState(() => _credibility = value),
                ),
                _SliderEditor(
                  label: '重要性',
                  value: _importance,
                  onChanged: (value) => setState(() => _importance = value),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _save, child: const Text('保存')),
      ],
    );
  }
}

class _SliderEditor extends StatelessWidget {
  /// Creates a numeric slider editor.
  const _SliderEditor({
    required this.label,
    required this.value,
    required this.onChanged,
  });

  final String label;
  final double value;
  final ValueChanged<double> onChanged;

  /// Builds the slider editor.
  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        SizedBox(width: 64, child: Text(label)),
        Expanded(
          child: Slider(
            value: value.clamp(0.0, 1.0).toDouble(),
            onChanged: onChanged,
          ),
        ),
        SizedBox(
          width: 42,
          child: Text(value.toStringAsFixed(2), textAlign: TextAlign.end),
        ),
      ],
    );
  }
}

class _MemoryLinkEditorResult {
  /// Creates a memory link editor result payload.
  const _MemoryLinkEditorResult({
    required this.type,
    required this.weight,
    required this.description,
  });

  final String type;
  final double weight;
  final String description;
}

class _MemoryLinkEditorDialog extends StatefulWidget {
  /// Creates a memory link creation dialog.
  const _MemoryLinkEditorDialog({
    required this.sourceTitle,
    required this.targetTitle,
  });

  final String sourceTitle;
  final String targetTitle;

  /// Shows a memory link creation dialog.
  static Future<_MemoryLinkEditorResult?> show({
    required BuildContext context,
    required String sourceTitle,
    required String targetTitle,
  }) {
    return showDialog<_MemoryLinkEditorResult>(
      context: context,
      builder: (context) => _MemoryLinkEditorDialog(
        sourceTitle: sourceTitle,
        targetTitle: targetTitle,
      ),
    );
  }

  /// Creates state for the memory link dialog.
  @override
  State<_MemoryLinkEditorDialog> createState() =>
      _MemoryLinkEditorDialogState();
}

class _MemoryLinkEditorDialogState extends State<_MemoryLinkEditorDialog> {
  final TextEditingController _typeController = TextEditingController(
    text: 'related',
  );
  final TextEditingController _descriptionController = TextEditingController();
  double _weight = 1;

  /// Releases link editor controllers.
  @override
  void dispose() {
    _typeController.dispose();
    _descriptionController.dispose();
    super.dispose();
  }

  /// Saves dialog input into the result payload.
  void _save() {
    final type = _typeController.text.trim();
    if (type.isEmpty) {
      return;
    }
    Navigator.of(context).pop(
      _MemoryLinkEditorResult(
        type: type,
        weight: _weight,
        description: _descriptionController.text.trim(),
      ),
    );
  }

  /// Builds the memory link dialog.
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('创建记忆关系'),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('${widget.sourceTitle}  →  ${widget.targetTitle}'),
            const SizedBox(height: 12),
            TextField(
              controller: _typeController,
              decoration: const InputDecoration(labelText: '关系类型'),
            ),
            const SizedBox(height: 10),
            _SliderEditor(
              label: '权重',
              value: _weight,
              onChanged: (value) => setState(() => _weight = value),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _descriptionController,
              minLines: 2,
              maxLines: 4,
              decoration: const InputDecoration(
                labelText: '描述',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _save, child: const Text('创建')),
      ],
    );
  }
}

class _ImportStrategyDialog extends StatelessWidget {
  /// Creates the import strategy dialog.
  const _ImportStrategyDialog();

  /// Shows the import strategy dialog.
  static Future<core_proxy.ImportStrategy?> show({
    required BuildContext context,
  }) {
    return showDialog<core_proxy.ImportStrategy>(
      context: context,
      builder: (context) => const _ImportStrategyDialog(),
    );
  }

  /// Builds the import strategy dialog.
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('导入记忆 JSON'),
      content: const Text('请选择同名记忆的处理方式。'),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        TextButton(
          onPressed: () =>
              Navigator.of(context).pop(core_proxy.ImportStrategy.skip),
          child: const Text('跳过'),
        ),
        TextButton(
          onPressed: () =>
              Navigator.of(context).pop(core_proxy.ImportStrategy.update),
          child: const Text('更新'),
        ),
        FilledButton(
          onPressed: () =>
              Navigator.of(context).pop(core_proxy.ImportStrategy.createNew),
          child: const Text('创建新记忆'),
        ),
      ],
    );
  }
}

/// Places all nodes in one cluster around a center point.
void _placeClusterNodes({
  required core_proxy.MemoryGraph graph,
  required _ClusterPlacement cluster,
  required Offset center,
  required Map<String, Offset> positions,
}) {
  final degree = <String, int>{for (final nodeId in cluster.nodeIds) nodeId: 0};
  for (final edge in graph.edges) {
    if (degree.containsKey(edge.sourceId)) {
      degree[edge.sourceId] = degree[edge.sourceId]! + 1;
    }
    if (degree.containsKey(edge.targetId)) {
      degree[edge.targetId] = degree[edge.targetId]! + 1;
    }
  }
  final nodeIds = cluster.nodeIds.toList(growable: false)
    ..sort((left, right) {
      final degreeOrder = degree[right]!.compareTo(degree[left]!);
      if (degreeOrder != 0) {
        return degreeOrder;
      }
      return left.compareTo(right);
    });
  if (nodeIds.length == 1) {
    positions[nodeIds.single] = center;
    return;
  }
  positions[nodeIds.first] = center;
  var placed = 1;
  var ring = 1;
  const nodeSpacing = 240.0;
  while (placed < nodeIds.length) {
    final radius = 220.0 + (ring - 1) * 190.0;
    final capacity = math.max(6, (math.pi * 2 * radius / nodeSpacing).floor());
    final count = math.min(capacity, nodeIds.length - placed);
    for (var index = 0; index < count; index += 1) {
      final angle = -math.pi / 2 + math.pi * 2 * index / count;
      positions[nodeIds[placed + index]] =
          center + Offset(math.cos(angle), math.sin(angle)) * radius;
    }
    placed += count;
    ring += 1;
  }
}

/// Returns a stable signature for graph layout invalidation.
String _graphSignature(core_proxy.MemoryGraph graph) {
  return '${graph.nodes.length}:${graph.edges.length}:${graph.nodes.map((node) => node.id).join('|')}:${graph.edges.map((edge) => '${edge.id}:${edge.sourceId}:${edge.targetId}').join('|')}';
}

/// Returns a graph containing exactly the selected memory ids.
core_proxy.MemoryGraph _graphForMemoryIds(
  core_proxy.MemoryGraph graph,
  Set<String> memoryIds,
) {
  final nodes = graph.nodes
      .where((node) => memoryIds.contains(node.id))
      .toList(growable: false);
  final nodeIds = nodes.map((node) => node.id).toSet();
  final edges = graph.edges
      .where(
        (edge) =>
            nodeIds.contains(edge.sourceId) && nodeIds.contains(edge.targetId),
      )
      .toList(growable: false);
  return core_proxy.MemoryGraph(nodes: nodes, edges: edges);
}

/// Returns a graph containing selected memory ids and their direct neighbors.
core_proxy.MemoryGraph _graphAroundMemoryIds(
  core_proxy.MemoryGraph graph,
  Set<String> memoryIds,
) {
  final expandedIds = <String>{...memoryIds};
  for (final edge in graph.edges) {
    if (memoryIds.contains(edge.sourceId)) {
      expandedIds.add(edge.targetId);
    }
    if (memoryIds.contains(edge.targetId)) {
      expandedIds.add(edge.sourceId);
    }
  }
  return _graphForMemoryIds(graph, expandedIds);
}

/// Returns a folder path and its visible child paths.
List<String> _folderAndChildren(List<String> folders, String folderPath) {
  final prefix = '$folderPath/';
  return folders
      .where((folder) => folder == folderPath || folder.startsWith(prefix))
      .toList(growable: false);
}

/// Returns true when a memory belongs to a folder subtree.
bool _matchesFolderTree(String memoryFolderPath, String folderPath) {
  final normalizedFolder = folderPath.trim();
  final normalizedMemoryFolder = memoryFolderPath.trim();
  final prefix = '$normalizedFolder/';
  return normalizedMemoryFolder == normalizedFolder ||
      normalizedMemoryFolder.startsWith(prefix);
}

/// Returns the display label for a folder path.
String _folderLabel(String folderPath) {
  final trimmed = folderPath.trim();
  return trimmed.isEmpty ? '全部' : trimmed;
}

/// Returns the visible tree depth of a folder path.
int _folderDepth(String folderPath) {
  final trimmed = folderPath.trim();
  if (trimmed.isEmpty) {
    return 0;
  }
  return trimmed.split('/').length - 1;
}

/// Formats epoch milliseconds for compact display.
String _formatMillis(int millis) {
  final date = DateTime.fromMillisecondsSinceEpoch(millis);
  return '${date.year}-${_twoDigits(date.month)}-${_twoDigits(date.day)} ${_twoDigits(date.hour)}:${_twoDigits(date.minute)}';
}

/// Formats one integer as two digits.
String _twoDigits(int value) {
  return value.toString().padLeft(2, '0');
}

/// Creates a text painter for graph labels.
TextPainter _textPainter(
  String text,
  TextStyle? style, {
  required double maxWidth,
}) {
  return TextPainter(
    text: TextSpan(text: text, style: style),
    maxLines: 3,
    ellipsis: '...',
    textDirection: TextDirection.ltr,
    textScaler: TextScaler.noScaling,
  )..layout(maxWidth: maxWidth);
}

/// Estimates a node world rectangle without measuring text every frame.
Rect _nodeWorldRect(core_proxy.MemoryGraphNode node, Offset center) {
  final width = _estimatedNodeWidth(node.label);
  return Rect.fromCenter(center: center, width: width, height: 54);
}

/// Estimates a node screen rectangle without measuring text every frame.
Rect _nodeScreenRect(
  core_proxy.MemoryGraphNode node,
  Offset center,
  double scale,
) {
  final visualScale = scale.clamp(0.15, 2.2).toDouble();
  final worldRect = _nodeWorldRect(node, center);
  return Rect.fromCenter(
    center: center,
    width: worldRect.width * visualScale,
    height: worldRect.height * visualScale,
  );
}

/// Estimates rendered node width from label length.
double _estimatedNodeWidth(String label) {
  final textWidth = (label.length.clamp(4, 34) * 7.8).toDouble();
  return (textWidth + 34).clamp(82.0, 314.0).toDouble();
}

/// Returns true when an edge intersects the visible canvas area.
bool _edgeMayBeVisible(Offset start, Offset end, Rect visibleRect) {
  if (visibleRect.contains(start) || visibleRect.contains(end)) {
    return true;
  }
  return Rect.fromPoints(start, end).inflate(24).overlaps(visibleRect);
}

/// Computes the shortest distance from a point to a line segment.
double _distanceToSegment(Offset point, Offset start, Offset end) {
  final lengthSq = (start - end).distanceSquared;
  if (lengthSq == 0) {
    return (point - start).distance;
  }
  final t =
      (((point.dx - start.dx) * (end.dx - start.dx) +
                  (point.dy - start.dy) * (end.dy - start.dy)) /
              lengthSq)
          .clamp(0.0, 1.0)
          .toDouble();
  final projection = start + (end - start) * t;
  return (point - projection).distance;
}

/// Draws a dashed line between two points.
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
