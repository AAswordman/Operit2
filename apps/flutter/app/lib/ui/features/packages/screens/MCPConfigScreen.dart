// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';

class MCPConfigScreen extends StatefulWidget {
  const MCPConfigScreen({
    super.key,
    required this.clients,
    required this.searchQuery,
  });

  final GeneratedCoreProxyClients clients;
  final String searchQuery;

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
    if (current != null) {
      setState(() {
        _servers = <String, core_proxy.ServerConfig>{
          ..._servers,
          serverId: core_proxy.ServerConfig(
            command: current.command,
            args: current.args,
            disabled: !enabled,
            autoApprove: current.autoApprove,
            env: current.env,
          ),
        };
      });
    }
    try {
      await _localServer.setServerEnabled(serverId: serverId, enabled: enabled);
      await _loadMcp();
    } catch (error, stackTrace) {
      debugPrint('Failed to update MCP enabled state: $error\n$stackTrace');
      await _loadMcp();
      if (!mounted) {
        return;
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
    final configJson = await _localServer.getPluginConfig(pluginId: serverId);
    if (!mounted) {
      return;
    }
    showDialog<void>(
      context: context,
      builder: (context) {
        return _MCPDetailsDialog(
          serverId: serverId,
          metadata: metadata,
          server: server,
          status: status,
          configJson: configJson,
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _servers.isEmpty && _metadata.isEmpty) {
      return const Center(child: CircularProgressIndicator());
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
                        metadata?.type ?? 'local',
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
          const Center(child: CircularProgressIndicator()),
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
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.32),
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

class _MCPDetailsDialog extends StatelessWidget {
  const _MCPDetailsDialog({
    required this.serverId,
    required this.metadata,
    required this.server,
    required this.status,
    required this.configJson,
  });

  final String serverId;
  final core_proxy.PluginMetadata? metadata;
  final core_proxy.ServerConfig? server;
  final core_proxy.ServerStatus? status;
  final String configJson;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      icon: const Icon(Icons.extension_outlined),
      title: Text(
        metadata?.name.trim().isNotEmpty == true ? metadata!.name : serverId,
      ),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Text('ID: $serverId'),
              if (metadata != null) ...<Widget>[
                Text('作者: ${metadata!.author}'),
                Text('版本: ${metadata!.version}'),
                Text('类型: ${metadata!.type}'),
                if (metadata!.repoUrl.trim().isNotEmpty)
                  Text('Repo: ${metadata!.repoUrl}'),
                if (metadata!.endpoint != null)
                  Text('Endpoint: ${metadata!.endpoint}'),
              ],
              if (server != null) ...<Widget>[
                const SizedBox(height: 12),
                Text('命令: ${server!.command} ${server!.args.join(" ")}'),
                Text('自动批准: ${server!.autoApprove.join(", ")}'),
              ],
              if (status?.errorMessage != null) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  status!.errorMessage!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              if (status?.cachedTools?.isNotEmpty == true) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  '工具',
                  style: Theme.of(
                    context,
                  ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 8),
                for (final tool in status!.cachedTools!)
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.build_outlined),
                    title: Text(tool.name),
                    subtitle: Text(tool.description),
                  ),
              ],
              if (configJson.trim().isNotEmpty) ...<Widget>[
                const SizedBox(height: 12),
                SelectableText(configJson),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        FilledButton.tonal(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }
}
