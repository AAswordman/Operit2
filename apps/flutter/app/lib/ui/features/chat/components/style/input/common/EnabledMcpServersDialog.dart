// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../packages/utils/MCPCommandRunner.dart';

/// Opens a quick dialog showing only currently enabled MCP servers.
Future<void> showEnabledMcpServersDialog({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
}) {
  return showDialog<void>(
    context: context,
    builder: (context) => _EnabledMcpServersDialog(clients: clients),
  );
}

class _EnabledMcpServerItem {
  const _EnabledMcpServerItem({
    required this.serverId,
    required this.title,
    required this.description,
    required this.running,
    required this.errorMessage,
  });

  final String serverId;
  final String title;
  final String description;
  final bool running;
  final String? errorMessage;
}

class _EnabledMcpServersDialog extends StatefulWidget {
  const _EnabledMcpServersDialog({required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<_EnabledMcpServersDialog> createState() =>
      _EnabledMcpServersDialogState();
}

class _EnabledMcpServersDialogState extends State<_EnabledMcpServersDialog> {
  late Future<List<_EnabledMcpServerItem>> _future;
  final Map<String, bool> _enabledStates = <String, bool>{};
  final Set<String> _busyIds = <String>{};
  String? _actionError;

  @override
  void initState() {
    super.initState();
    _future = _loadEnabledServers();
  }

  Future<List<_EnabledMcpServerItem>> _loadEnabledServers() async {
    final mcpLocalServer = widget.clients.permissionsMcpRuntimeMcpLocalServer;
    final results = await Future.wait<Object>(<Future<Object>>[
      mcpLocalServer.getAllMcpServers(),
      mcpLocalServer.getAllPluginMetadata(),
      mcpLocalServer.getAllServerStatus(),
    ]);
    final servers = results[0] as Map<String, core_proxy.ServerConfig>;
    final metadataMap = results[1] as Map<String, core_proxy.PluginMetadata>;
    final statusMap = results[2] as Map<String, core_proxy.ServerStatus>;

    final items = <_EnabledMcpServerItem>[];
    final entries = servers.entries.where((entry) => !entry.value.disabled).toList()
      ..sort((left, right) => left.key.compareTo(right.key));
    for (final entry in entries) {
      final serverId = entry.key;
      final metadata = metadataMap[serverId];
      final status = statusMap[serverId];
      final title = metadata?.name.trim() ?? '';
      final error = status?.errorMessage?.trim();
      final running =
          status != null &&
          status.lastStartTime > 0 &&
          status.lastStartTime >= status.lastStopTime;
      _enabledStates.putIfAbsent(serverId, () => true);
      items.add(
        _EnabledMcpServerItem(
          serverId: serverId,
          title: title.isEmpty ? serverId : title,
          description: metadata?.description.trim() ?? '',
          running: running,
          errorMessage: (error == null || error.isEmpty) ? null : error,
        ),
      );
    }
    return items;
  }

  Future<void> _toggleServer(_EnabledMcpServerItem item, bool enabled) async {
    if (_busyIds.contains(item.serverId)) {
      return;
    }
    setState(() {
      _busyIds.add(item.serverId);
      _enabledStates[item.serverId] = enabled;
      _actionError = null;
    });
    try {
      await widget.clients.permissionsMcpRuntimeMcpLocalServer.setServerEnabled(
        serverId: item.serverId,
        enabled: enabled,
      );
      await applyMcpServerLifecycle(
        clients: widget.clients,
        serverId: item.serverId,
        enabled: enabled,
      );
      ToolPkgCatalogChangeBus.notifyCatalogChanged(reloadPluginCode: false);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _enabledStates[item.serverId] = !enabled;
        _actionError = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _busyIds.remove(item.serverId);
        });
      }
    }
  }

  void _openPackageManager() {
    Navigator.of(context).pop();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const PackageManagerScreenRoute(),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  @override
  Widget build(BuildContext context) {
    final chinese = Localizations.localeOf(
      context,
    ).languageCode.toLowerCase().startsWith('zh');
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return AlertDialog(
      title: Text(
        chinese ? '已开启的 MCP 服务' : 'Enabled MCP Servers',
        style: theme.textTheme.titleMedium,
      ),
      content: SizedBox(
        width: 440,
        child: FutureBuilder<List<_EnabledMcpServerItem>>(
          future: _future,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              return SelectableText(
                snapshot.error.toString(),
                style: TextStyle(color: colorScheme.error),
              );
            }
            if (snapshot.connectionState != ConnectionState.done) {
              return const SizedBox(
                height: 120,
                child: Center(child: CircularProgressIndicator()),
              );
            }
            final items = snapshot.requireData;
            if (items.isEmpty) {
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 20),
                child: Text(
                  chinese
                      ? '当前没有已开启的 MCP 服务器，可在包管理中启用。'
                      : 'No MCP servers are currently enabled. Enable one in Package Manager.',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              );
            }
            return ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 380),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  if (_actionError != null) ...<Widget>[
                    Text(
                      _actionError!,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.error,
                      ),
                    ),
                    const SizedBox(height: 8),
                  ],
                  Flexible(
                    child: ListView.separated(
                      shrinkWrap: true,
                      itemCount: items.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final item = items[index];
                        final enabled = _enabledStates[item.serverId] ?? true;
                        final busy = _busyIds.contains(item.serverId);
                        final statusLabel = item.errorMessage != null
                            ? item.errorMessage!
                            : !enabled
                            ? (chinese ? '已停用' : 'Disabled')
                            : item.running
                            ? (chinese ? '运行中' : 'Running')
                            : (chinese ? '已启用' : 'Enabled');
                        final subtitleParts = <String>[
                          if (item.title != item.serverId) item.serverId,
                          statusLabel,
                          if (item.description.isNotEmpty) item.description,
                        ];
                        return ListTile(
                          dense: true,
                          contentPadding: const EdgeInsets.symmetric(
                            horizontal: 4,
                            vertical: 2,
                          ),
                          leading: Icon(
                            Icons.hub_outlined,
                            size: 20,
                            color: item.errorMessage != null
                                ? colorScheme.error
                                : enabled
                                ? colorScheme.tertiary
                                : colorScheme.onSurfaceVariant,
                          ),
                          title: Text(
                            item.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Text(
                            subtitleParts.join(' · '),
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: item.errorMessage != null
                                  ? colorScheme.error
                                  : colorScheme.onSurfaceVariant,
                            ),
                          ),
                          trailing: busy
                              ? const SizedBox(
                                  width: 20,
                                  height: 20,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                  ),
                                )
                              : Switch(
                                  value: enabled,
                                  onChanged: (value) =>
                                      unawaited(_toggleServer(item, value)),
                                ),
                        );
                      },
                    ),
                  ),
                ],
              ),
            );
          },
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _openPackageManager,
          child: Text(chinese ? '打开包管理' : 'Package Manager'),
        ),
        FilledButton.tonal(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(chinese ? '关闭' : 'Close'),
        ),
      ],
    );
  }
}
