// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';
import '../dialogs/MCPDetailsDialog.dart';

class MCPConfigScreen extends StatefulWidget {
  const MCPConfigScreen({
    super.key,
    required this.clients,
    required this.searchQuery,
    required this.reloadRevision,
  });

  final GeneratedCoreProxyClients clients;
  final String searchQuery;
  final int reloadRevision;

  @override
  State<MCPConfigScreen> createState() => _MCPConfigScreenState();
}

class _MCPConfigScreenState extends State<MCPConfigScreen> {
  bool _loading = true;
  String? _errorMessage;
  String _configDirectory = '';
  Map<String, core_proxy.ServerConfig> _servers =
      <String, core_proxy.ServerConfig>{};
  Map<String, core_proxy.PluginMetadata> _metadata =
      <String, core_proxy.PluginMetadata>{};
  Map<String, core_proxy.ServerStatus> _statuses =
      <String, core_proxy.ServerStatus>{};

  GeneratedMcpLocalServerCoreProxy get _localServer =>
      widget.clients.mcpLocalServer;

  @override
  void initState() {
    super.initState();
    _loadMcp();
  }

  @override
  void didUpdateWidget(covariant MCPConfigScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.reloadRevision != widget.reloadRevision) {
      _loadMcp();
    }
  }

  Future<void> _loadMcp() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final results = await Future.wait<Object>(<Future<Object>>[
        _localServer.getConfigDirectory(),
        _localServer.getAllMcpServers(),
        _localServer.getAllPluginMetadata(),
        _localServer.getAllServerStatus(),
      ]);
      if (!mounted) {
        return;
      }
      setState(() {
        _configDirectory = results[0] as String;
        _servers = results[1] as Map<String, core_proxy.ServerConfig>;
        _metadata = results[2] as Map<String, core_proxy.PluginMetadata>;
        _statuses = results[3] as Map<String, core_proxy.ServerStatus>;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load MCP config: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _setServerEnabled(String serverId, bool enabled) async {
    final current = _servers[serverId];
    final previous = current != null && !current.disabled;
    if (current != null) {
      setState(() {
        _servers = <String, core_proxy.ServerConfig>{
          ..._servers,
          serverId: core_proxy.ServerConfig(
            command: current.command,
            args: current.args,
            url: current.url,
            type: current.type,
            headers: current.headers,
            disabled: !enabled,
            autoApprove: current.autoApprove,
            env: current.env,
          ),
        };
      });
    }
    try {
      await _localServer.setServerEnabled(serverId: serverId, enabled: enabled);
    } catch (error, stackTrace) {
      debugPrint('Failed to update MCP enabled state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      if (current != null) {
        setState(() {
          _servers = <String, core_proxy.ServerConfig>{
            ..._servers,
            serverId: core_proxy.ServerConfig(
              command: current.command,
              args: current.args,
              url: current.url,
              type: current.type,
              headers: current.headers,
              disabled: !previous,
              autoApprove: current.autoApprove,
              env: current.env,
            ),
          };
        });
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _showDetails(String serverId) async {
    final metadata = _metadata[serverId];
    final server = _servers[serverId];
    final status = _statuses[serverId];
    if (!mounted) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return MCPDetailsDialog(
          serverId: serverId,
          metadata: metadata,
          server: server,
          status: status,
          clients: widget.clients,
          onConfigSaved: _loadMcp,
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _servers.isEmpty && _metadata.isEmpty) {
      return const M3LoadingPane();
    }
    if (error != null && _servers.isEmpty && _metadata.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadMcp,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }

    final ids = _filteredServerIds;
    return Stack(
      children: <Widget>[
        RefreshIndicator(
          onRefresh: _loadMcp,
          child: ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
            children: <Widget>[
              _MCPHeaderCard(directory: _configDirectory, onRefresh: _loadMcp),
              const SizedBox(height: 12),
              if (ids.isEmpty)
                EmptyState(
                  icon: Icons.extension_outlined,
                  title: '没有 MCP',
                  message: widget.searchQuery.trim().isEmpty
                      ? '当前没有可显示的 MCP 服务。'
                      : '没有匹配的 MCP 服务。',
                  scrollable: false,
                )
              else
                PackageInlineGrid(
                  itemCount: ids.length,
                  itemBuilder: (context, index) {
                    final serverId = ids[index];
                    final server = _servers[serverId];
                    final metadata = _metadata[serverId];
                    final status = _statuses[serverId];
                    final enabled = server != null && !server.disabled;
                    final toolCount = status?.cachedTools?.length;
                    final hasError = status?.errorMessage != null;
                    return PackageListItem(
                      icon: Icons.extension_outlined,
                      title: metadata?.name.trim().isNotEmpty == true
                          ? metadata!.name
                          : serverId,
                      subtitle: metadata?.description.trim().isNotEmpty == true
                          ? metadata!.description
                          : serverId,
                      metadata: <String>[
                        serverId,
                        if (metadata?.version.trim().isNotEmpty == true)
                          metadata!.version,
                        if (toolCount != null) '$toolCount 工具',
                        if (hasError) '错误',
                      ],
                      enabled: enabled,
                      onTap: () => _showDetails(serverId),
                      onEnabledChanged: (value) =>
                          _setServerEnabled(serverId, value),
                    );
                  },
                ),
            ],
          ),
        ),
        if (_loading && (_servers.isNotEmpty || _metadata.isNotEmpty))
          const Positioned.fill(child: M3LoadingOverlay()),
      ],
    );
  }

  List<String> get _filteredServerIds {
    final allIds = <String>{
      ..._servers.keys,
      ..._metadata.keys,
      ..._statuses.keys,
    };
    final query = widget.searchQuery.trim().toLowerCase();
    final ids = allIds.toList()
      ..sort(
        (left, right) => _displayName(left).compareTo(_displayName(right)),
      );
    if (query.isEmpty) {
      return ids;
    }
    return ids
        .where((id) {
          final metadata = _metadata[id];
          final status = _statuses[id];
          return id.toLowerCase().contains(query) ||
              _displayName(id).toLowerCase().contains(query) ||
              (metadata?.description.toLowerCase().contains(query) == true) ||
              (status?.cachedTools?.any(
                    (tool) => tool.name.toLowerCase().contains(query),
                  ) ==
                  true);
        })
        .toList(growable: false);
  }

  String _displayName(String serverId) {
    final metadata = _metadata[serverId];
    if (metadata != null && metadata.name.trim().isNotEmpty) {
      return metadata.name;
    }
    return serverId;
  }
}

class _MCPHeaderCard extends StatelessWidget {
  const _MCPHeaderCard({required this.directory, required this.onRefresh});

  final String directory;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.30),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: <Widget>[
            const Icon(Icons.extension_outlined),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    'MCP',
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (directory.trim().isNotEmpty)
                    Text(
                      directory,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                ],
              ),
            ),
            IconButton(
              tooltip: '刷新',
              onPressed: onRefresh,
              icon: const Icon(Icons.refresh),
            ),
          ],
        ),
      ),
    );
  }
}
