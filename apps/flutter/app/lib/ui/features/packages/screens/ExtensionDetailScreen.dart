// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/PageActivityMixin.dart';
import '../../settings/components/SettingsControlStyles.dart';
import '../components/DeviceSpaceVisuals.dart';
import '../components/ExtensionAvatar.dart';
import '../components/ExtensionMotion.dart';
import '../components/ExtensionTopBar.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../dialogs/MCPConfigDialogs.dart';
import '../dialogs/A2AImportDialog.dart';
import '../dialogs/A2ATaskDialog.dart';
import '../dialogs/MCPToolRunDialog.dart';
import '../dialogs/PackageToolRunDialog.dart';
import '../model/DeviceSpaceState.dart';
import '../model/ExtensionCatalogController.dart';
import '../utils/MCPCommandRunner.dart';
import '../utils/PackageDisplayUtils.dart';
import 'ToolPkgDesktopWidgetsScreen.dart';
import 'ToolPkgUiLauncherScreen.dart';

/// Detail route shared by installed extensions and offline official ones.
class ExtensionDetailScreen extends StatefulWidget {
  const ExtensionDetailScreen({
    super.key,
    required this.kind,
    required this.id,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final String kind;
  final String id;
  final GeneratedCoreProxyClients clients;

  @override
  State<ExtensionDetailScreen> createState() => _ExtensionDetailScreenState();
}

sealed class _KindDetails {
  const _KindDetails();
}

class _ToolPkgDetails extends _KindDetails {
  const _ToolPkgDetails({required this.runtime, required this.details});

  final core_proxy.ToolPkgContainerRuntime runtime;
  final core_proxy.ToolPkgContainerDetails? details;
}

class _ScriptDetails extends _KindDetails {
  const _ScriptDetails({required this.package});

  final core_proxy.ToolPackage package;
}

class _SkillDetails extends _KindDetails {
  const _SkillDetails({required this.content});

  final String? content;
}

class _A2ADetails extends _KindDetails {
  const _A2ADetails(this.config);
  final Map<String, dynamic> config;
}

class _McpDetails extends _KindDetails {
  const _McpDetails({
    required this.server,
    required this.metadata,
    required this.status,
  });

  final core_proxy.ServerConfig? server;
  final core_proxy.PluginMetadata? metadata;
  final core_proxy.ServerStatus? status;
}

class _ExtensionDetailScreenState extends State<ExtensionDetailScreen>
    with PageActivityMixin<ExtensionDetailScreen> {
  late final ExtensionCatalogController _controller =
      ExtensionCatalogController.of(widget.clients);
  late final DeviceSpaceState _space = DeviceSpaceState.of(widget.clients);
  String _installScope = defaultExtensionInstallScope;
  _KindDetails? _details;
  Object? _detailsError;
  String? _detailsMarker;
  bool _useEnglish = false;
  bool _active = false;
  bool _mcpBusy = false;
  bool _describing = false;
  final Set<String> _togglingSubpackages = <String>{};

  GeneratedCoreProxyClients get _clients => widget.clients;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onCatalogChanged);
    _space.addListener(_onSpaceChanged);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _useEnglish =
        Localizations.localeOf(context).languageCode.toLowerCase() != 'zh';
  }

  @override
  void onPageActivityChanged(bool active) {
    if (active == _active) {
      return;
    }
    _active = active;
    if (active) {
      _controller.attach();
      _space.attach();
      // Activity can change mid-build; read details after this frame.
      scheduleMicrotask(_syncDetails);
    } else {
      _controller.detach();
      _space.detach();
    }
  }

  @override
  void dispose() {
    if (_active) {
      _controller.detach();
      _space.detach();
    }
    _controller.removeListener(_onCatalogChanged);
    _space.removeListener(_onSpaceChanged);
    super.dispose();
  }

  void _onCatalogChanged() {
    if (!mounted) {
      return;
    }
    setState(() {});
    _syncDetails();
  }

  void _onSpaceChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  /// Loads kind details once installed and again when its state changes.
  void _syncDetails() {
    if (!mounted) {
      return;
    }
    final entry = _controller.findEntry(widget.kind, widget.id);
    if (entry == null) {
      return;
    }
    final marker =
        '${entry.activationState}|${entry.runtimeState}|${entry.version}|${entry.scope}';
    if (marker == _detailsMarker) {
      return;
    }
    _detailsMarker = marker;
    unawaited(_loadDetails());
  }

  Future<void> _loadDetails() async {
    setState(() => _detailsError = null);
    try {
      final details = await _fetchDetails();
      if (!mounted) {
        return;
      }
      setState(() => _details = details);
    } catch (error, stackTrace) {
      debugPrint('Failed to load extension details: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() => _detailsError = error);
    }
  }

  Future<_KindDetails> _fetchDetails() async {
    switch (widget.kind) {
      case 'toolpkg':
        final packageManager = _clients.application.packageManager();
        final runtimes = await packageManager.getToolPkgContainerRuntimes();
        final runtime = runtimes.firstWhere(
          (item) => item.packageName == widget.id,
        );
        final details = await packageManager.getToolPkgContainerDetails(
          packageName: widget.id,
          useEnglish: _useEnglish,
        );
        return _ToolPkgDetails(runtime: runtime, details: details);
      case 'script':
        final packages = await _clients.application
            .packageManager()
            .getExecutableAvailablePackages();
        final package = packages[widget.id];
        if (package == null) {
          throw StateError('包详情不可用：${widget.id}');
        }
        return _ScriptDetails(package: package);
      case 'skill':
        final content = await _clients.application
            .skillRepository()
            .readSkillContent(skillName: widget.id);
        return _SkillDetails(content: content);
      case 'a2a':
        final text = await _clients.application.getA2AConfig(id: widget.id);
        return _A2ADetails(jsonDecode(text) as Map<String, dynamic>);
      case 'mcp':
        final server = _clients.permissionsMcpRuntimeMcpLocalServer;
        final servers = await server.getAllMcpServers();
        final metadata = await server.getAllPluginMetadata();
        final statuses = await server.getAllServerStatus();
        return _McpDetails(
          server: servers[widget.id],
          metadata: metadata[widget.id],
          status: statuses[widget.id],
        );
      default:
        throw StateError('Unsupported extension kind: ${widget.kind}');
    }
  }

  @override
  Widget build(BuildContext context) {
    final entry = _controller.findEntry(widget.kind, widget.id);
    final candidate = entry == null
        ? _controller.findCandidate(widget.kind, widget.id)
        : null;
    final title = entry?.displayName ?? candidate?.displayName;
    final Widget body;
    if (entry != null) {
      body = _buildInstalled(context, entry);
    } else if (candidate != null) {
      body = _buildCandidate(context, candidate);
    } else if (_controller.snapshot == null) {
      body = const Align(
        alignment: Alignment.topCenter,
        child: Padding(
          padding: EdgeInsets.fromLTRB(24, 0, 24, 0),
          child: _DetailsSkeleton(),
        ),
      );
    } else {
      body = Center(
        child: Text(
          '这个扩展已不在目录中',
          style: Theme.of(context).textTheme.titleMedium,
        ),
      );
    }
    return ExtensionTopBar(
      title: title == null
          ? null
          : (context) => Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600),
            ),
      actions: entry == null
          ? null
          : (context) => <Widget>[_overflowMenu(entry)],
      child: body,
    );
  }

  // ---------------------------------------------------------------- installed

  Widget _buildInstalled(
    BuildContext context,
    core_proxy.ExtensionCatalogEntry entry,
  ) {
    final key = ExtensionCatalogController.entryKey(entry);
    final busy = _controller.isBusy(key);
    final enabled = ExtensionCatalogController.isEnabled(entry);
    final toggleValue = _controller.pendingEnabled(key) ?? enabled;
    final hasUpdate = _controller.hasUpdate(entry);
    final canOpenUi = entry.kind == 'toolpkg' && entry.hasUi;
    final subtitle = <String>[
      if (entry.authors.isNotEmpty) entry.authors.join('、'),
      storeSourceLabel(entry.source),
    ].join(' · ');
    final mcpError = entry.runtimeState.startsWith('error:')
        ? entry.runtimeState.substring('error:'.length)
        : null;
    final details = _details;
    final wide = MediaQuery.sizeOf(context).width >= 600;
    return StoreDetailScaffold(
      header: StoreDetailHeader(
        icon: CatalogEntryAvatar(
          clients: _clients,
          entry: entry,
          size: wide ? 88 : 72,
          enabled: toggleValue,
          busy: busy,
          badge: ExtensionCatalogController.hasProblem(entry)
              ? ExtensionAvatarBadge.error
              : hasUpdate
              ? ExtensionAvatarBadge.update
              : ExtensionAvatarBadge.none,
        ),
        title: entry.displayName,
        subtitle: subtitle,
        badges: _entryBadges(entry, hasUpdate),
        actions: <Widget>[
          if (hasUpdate)
            StoreGetButton(
              label: '更新',
              icon: Icons.arrow_circle_up_rounded,
              prominent: true,
              large: true,
              busy: busy,
              onPressed: () => _run(() => _controller.update(entry)),
            )
          else if (canOpenUi)
            StoreGetButton(
              label: '打开',
              icon: Icons.open_in_new_rounded,
              prominent: true,
              large: true,
              onPressed: enabled ? () => _openUi(null) : null,
            ),
          _EnableToggle(
            label: _toggleLabel(entry, toggleValue),
            value: toggleValue,
            busy: busy,
            onChanged: (value) {
              unawaited(HapticFeedback.selectionClick());
              unawaited(_run(() => _controller.setEnabled(entry, value)));
            },
          ),
        ],
        footer: entry.scope == 'space'
            ? const Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.hub_outlined, size: 14),
                  SizedBox(width: 6),
                  Flexible(child: Text('启用状态在设备空间的所有设备间同步')),
                ],
              )
            : null,
      ),
      children: <Widget>[
        StoreStatsStrip(stats: _installedStats(entry, enabled)),
        if (mcpError != null)
          Padding(
            padding: const EdgeInsets.only(top: 20),
            child: StoreNoticeCard(
              icon: Icons.error_outline_rounded,
              tone: StoreNoticeTone.error,
              title: 'MCP 服务启动失败',
              message: mcpError,
            ),
          ),
        if (entry.description.trim().isNotEmpty || entry.kind == 'mcp')
          StoreDetailSection(
            title: '简介',
            trailing: details is _McpDetails
                ? _mcpDescriptionActions(entry, details)
                : null,
            child: entry.description.trim().isEmpty
                ? const _EmptyLine(text: '还没有简介，可以手动填写或让 AI 生成。')
                : StoreExpandableText(text: entry.description),
          ),
        _availability(entry, busy),
        ..._kindSections(entry, enabled),
        StoreDetailSection(
          title: '信息',
          child: StoreInfoTable(items: _infoItems(entry)),
        ),
        StoreDetailSection(title: '管理', child: _manageCard(entry, busy)),
      ],
    );
  }

  String _toggleLabel(core_proxy.ExtensionCatalogEntry entry, bool enabled) {
    if (entry.kind == 'skill') {
      return enabled ? 'AI 可见' : 'AI 不可见';
    }
    return enabled ? '已启用' : '已停用';
  }

  List<Widget> _entryBadges(
    core_proxy.ExtensionCatalogEntry entry,
    bool hasUpdate,
  ) {
    final colorScheme = Theme.of(context).colorScheme;
    final kind = StoreKind.of(entry.kind);
    return <Widget>[
      StoreBadge(label: kind.label, icon: kind.icon),
      if (entry.version.trim().isNotEmpty)
        StoreBadge(label: 'v${entry.version}'),
      StoreBadge(
        label: storeScopeLabel(entry.scope),
        icon: storeScopeIcon(entry.scope),
      ),
      if (entry.isCustomized)
        const StoreBadge(label: '已自定义', icon: Icons.edit_outlined),
      if (hasUpdate)
        StoreBadge(
          label: '有新版本',
          icon: Icons.arrow_upward_rounded,
          color: colorScheme.primaryContainer,
          foreground: colorScheme.onPrimaryContainer,
        ),
    ];
  }

  List<StoreStat> _installedStats(
    core_proxy.ExtensionCatalogEntry entry,
    bool enabled,
  ) {
    final kind = StoreKind.of(entry.kind);
    return <StoreStat>[
      StoreStat(label: '类型', value: kind.label, icon: kind.icon),
      StoreStat(label: '状态', value: _statusLabel(entry, enabled)),
      if (entry.version.trim().isNotEmpty)
        StoreStat(label: '版本', value: entry.version),
      if (entry.kind == 'toolpkg')
        StoreStat(label: '子包', value: '${entry.subpackageCount}'),
      if (entry.kind == 'toolpkg' && entry.uiModuleCount > 0)
        StoreStat(label: '界面', value: '${entry.uiModuleCount}'),
      if (entry.kind == 'script' || entry.kind == 'mcp')
        StoreStat(label: '工具', value: '${entry.toolCount}'),
      StoreStat(
        label: '位置',
        value: storeScopeLabel(entry.scope),
        icon: storeScopeIcon(entry.scope),
      ),
    ];
  }

  String _statusLabel(core_proxy.ExtensionCatalogEntry entry, bool enabled) {
    if (entry.kind == 'skill') {
      return enabled ? 'AI 可见' : 'AI 不可见';
    }
    if (entry.kind == 'mcp') {
      if (entry.runtimeState.startsWith('error:')) {
        return '异常';
      }
      return switch (entry.runtimeState) {
        'running' => '运行中',
        'stopped' => enabled ? '未运行' : '已停用',
        _ => entry.runtimeState,
      };
    }
    if (entry.runtimeState == 'error') {
      return entry.kind == 'a2a' ? '配置异常' : '依赖异常';
    }
    return enabled ? '已启用' : '已停用';
  }

  // ------------------------------------------------------------ availability

  /// Which devices can use the extension, and the switch that changes it.
  Widget _availability(core_proxy.ExtensionCatalogEntry entry, bool busy) {
    final theme = Theme.of(context);
    final current = _space.currentDevice;
    final devices = _space.devices;
    final reach = entry.scope == 'space'
        ? devices
        : <core_proxy.RuntimeDeviceSpaceDevice>[?current];
    final here = current == null ? '这台设备' : '「${current.deviceName}」';
    final description = switch (entry.scope) {
      'space' =>
        devices.length > 1
            ? '同步到设备空间的 ${devices.length} 台设备，启用状态和配置一起同步；每台设备上的对话都能用它。'
            : '已共享到设备空间，之后加入的设备都会同步它和它的配置。',
      'device' => '只在$here上，只有在这里运行的对话能用它。',
      'localOnly' => '通过本地命令启动的 MCP 只能在$here上运行，不能共享。',
      _ => throw StateError('Unknown extension scope: ${entry.scope}'),
    };
    final movable = entry.scope == 'device' || entry.scope == 'space';
    final target = entry.scope == 'space' ? 'device' : 'space';
    final name = '「${entry.displayName}」';
    final message = target == 'space'
        ? (devices.length > 1
              ? '共享后$name会同步到设备空间的 ${devices.length} 台设备，启用状态和配置里的密钥也会一起同步。'
              : '共享后，之后加入设备空间的设备都会同步$name，配置里的密钥也会一起同步。')
        : '移回本机后，设备空间里的其他设备会移除$name和它的配置。';
    Widget summary(VoidCallback? requestConfirm) => Padding(
      padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              if (reach.isNotEmpty) ...<Widget>[
                DeviceSphereStack(
                  devices: reach,
                  currentDeviceId: _space.currentDeviceId,
                  diameter: 28,
                ),
                const SizedBox(width: 12),
              ],
              Expanded(
                child: Text(
                  description,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                    height: 1.45,
                  ),
                ),
              ),
            ],
          ),
          if (requestConfirm != null) ...<Widget>[
            const SizedBox(height: 12),
            SettingsSegmentedSelector<String>(
              value: entry.scope,
              onChanged: (scope) {
                if (scope != entry.scope && !busy) {
                  requestConfirm();
                }
              },
              segments: <ButtonSegment<String>>[
                ButtonSegment<String>(
                  value: 'device',
                  icon: Icon(storeScopeIcon('device'), size: 16),
                  label: Text(storeScopeLabel('device')),
                ),
                ButtonSegment<String>(
                  value: 'space',
                  icon: Icon(storeScopeIcon('space'), size: 16),
                  label: Text(storeScopeLabel('space')),
                ),
              ],
            ),
          ],
        ],
      ),
    );
    return StoreDetailSection(
      title: '可用位置',
      child: StoreGroupedList(
        children: <Widget>[
          if (movable)
            StoreInlineConfirm(
              message: message,
              confirmLabel: target == 'space' ? '共享到设备空间' : '移回本机',
              onConfirm: () => _run(() => _controller.moveScope(entry, target)),
              trigger: (context, requestConfirm) => summary(requestConfirm),
            )
          else
            summary(null),
        ],
      ),
    );
  }

  // ------------------------------------------------------------ kind details

  List<Widget> _kindSections(
    core_proxy.ExtensionCatalogEntry entry,
    bool enabled,
  ) {
    final details = _details;
    if (details == null) {
      final error = _detailsError;
      if (error != null) {
        return <Widget>[
          Padding(
            padding: const EdgeInsets.only(top: 24),
            child: StoreNoticeCard(
              icon: Icons.error_outline_rounded,
              tone: StoreNoticeTone.error,
              title: '详情加载失败',
              message: '$error',
              action: TextButton(
                onPressed: _loadDetails,
                child: const Text('重试'),
              ),
            ),
          ),
        ];
      }
      return const <Widget>[_DetailsSkeleton()];
    }
    return switch (details) {
      _ToolPkgDetails toolPkg => _toolPkgSections(enabled, toolPkg),
      _ScriptDetails script => _scriptSections(script),
      _SkillDetails skill => _skillSections(skill),
      _McpDetails mcp => _mcpSections(entry, enabled, mcp),
      _A2ADetails a2a => _a2aSections(enabled, a2a),
    };
  }

  List<Widget> _a2aSections(bool enabled, _A2ADetails data) {
    final card = data.config['card'] as Map<String, dynamic>?;
    final skills = card?['skills'] as List<dynamic>? ?? const [];
    return <Widget>[
      StoreDetailSection(
        title: '连接',
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            StoreInfoTable(
              items: <StoreInfoItem>[
                StoreInfoItem('服务地址', '${data.config['url'] ?? ''}'),
                StoreInfoItem('包名', 'a2a.${widget.id}'),
                StoreInfoItem(
                  'Agent Card',
                  card == null ? '尚未读取，首次调用时获取' : '已读取',
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                OutlinedButton.icon(
                  onPressed: () => _editA2A(data),
                  icon: const Icon(Icons.edit_outlined),
                  label: const Text('编辑配置'),
                ),
                OutlinedButton.icon(
                  onPressed: () => _refreshA2A(data),
                  icon: const Icon(Icons.refresh_rounded),
                  label: const Text('刷新 Agent Card'),
                ),
                FilledButton.icon(
                  onPressed: enabled
                      ? () => showDialog<void>(
                          context: context,
                          builder: (context) =>
                              A2ATaskDialog(clients: _clients, id: widget.id),
                        )
                      : null,
                  icon: const Icon(Icons.play_arrow_rounded),
                  label: const Text('测试任务'),
                ),
              ],
            ),
          ],
        ),
      ),
      StoreDetailSection(
        title: '能力',
        count: skills.length,
        child: skills.isEmpty
            ? const _EmptyLine(text: 'Agent Card 尚未提供能力说明。')
            : StoreGroupedList(
                children: <Widget>[
                  for (final skill in skills)
                    StoreListRow(
                      leading: const StoreRowGlyph(icon: Icons.hub_outlined),
                      title: '${skill['name'] ?? skill['id'] ?? ''}',
                      subtitle: '${skill['description'] ?? ''}',
                    ),
                ],
              ),
      ),
      const StoreDetailSection(
        title: '包工具',
        child: Text(
          'send_message · 发送任务与补充消息\nget_task · 查询任务状态和结果\ncancel_task · 请求取消任务',
        ),
      ),
    ];
  }

  Future<void> _editA2A(_A2ADetails data) async {
    final saved = await showDialog<bool>(
      context: context,
      builder: (context) => A2AImportDialog(
        clients: _clients,
        id: widget.id,
        configJson: jsonEncode(data.config),
      ),
    );
    if (saved == true) {
      await _controller.refresh();
      await _loadDetails();
    }
  }

  Future<void> _refreshA2A(_A2ADetails data) => _run(() async {
    final text = await _clients.application.discoverA2AAgent(
      configJson: jsonEncode(data.config),
    );
    await _clients.application.importA2AConfig(
      configJson: jsonEncode({
        'a2aAgents': {widget.id: jsonDecode(text)},
      }),
    );
    await _controller.refresh();
    await _loadDetails();
  });

  // ------------------------------------------------------------------ toolpkg

  List<Widget> _toolPkgSections(bool enabled, _ToolPkgDetails data) {
    final runtime = data.runtime;
    final details = data.details;
    return <Widget>[
      if (runtime.dependencyIssues.isNotEmpty)
        StoreDetailSection(
          title: '前置插件问题',
          count: runtime.dependencyIssues.length,
          child: Column(
            children: <Widget>[
              for (final issue in runtime.dependencyIssues)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: StoreNoticeCard(
                    icon: Icons.link_off_rounded,
                    tone: StoreNoticeTone.error,
                    title: '${issue.id} · ${_dependencyIssueStatus(issue)}',
                    message: _dependencyIssueDescription(issue),
                  ),
                ),
            ],
          ),
        ),
      if (details != null && details.toolboxUiModules.isNotEmpty)
        StoreDetailSection(
          title: '界面',
          count: details.toolboxUiModules.length,
          child: StoreGroupedList(
            children: <Widget>[
              for (final module in details.toolboxUiModules)
                StoreListRow(
                  leading: const StoreRowGlyph(
                    icon: Icons.dashboard_customize_outlined,
                  ),
                  title: module.title,
                  subtitle: module.description.trim().isNotEmpty
                      ? module.description
                      : '${module.uiModuleId} · ${module.runtime}',
                  trailing: StoreGetButton(
                    label: '打开',
                    onPressed: enabled ? () => _openUi(module.routeId) : null,
                  ),
                ),
            ],
          ),
        ),
      if (details != null)
        StoreDetailSection(
          title: '子包',
          count: details.subpackages.length,
          child: details.subpackages.isEmpty
              ? const _EmptyLine(text: '这个插件没有子包。')
              : StoreGroupedList(
                  children: <Widget>[
                    for (final subpackage in details.subpackages)
                      StoreListRow(
                        leading: const StoreRowGlyph(
                          icon: Icons.inventory_2_outlined,
                        ),
                        title: subpackage.displayName,
                        subtitle: subpackage.description.trim().isNotEmpty
                            ? subpackage.description
                            : '${subpackage.packageName} · ${subpackage.toolCount} 个工具',
                        trailing: _subpackageSwitch(subpackage, enabled),
                      ),
                  ],
                ),
        ),
      if (details != null && details.requires.isNotEmpty)
        StoreDetailSection(
          title: '依赖',
          count: details.requires.length,
          child: StoreGroupedList(
            children: <Widget>[
              for (final requirement in details.requires)
                StoreListRow(
                  leading: const StoreRowGlyph(icon: Icons.link_rounded),
                  title: requirement.id,
                  subtitle: <String>[
                    requirement.description,
                    if (requirement.minVersion != null)
                      '≥ ${requirement.minVersion}',
                    if (requirement.maxVersion != null)
                      '≤ ${requirement.maxVersion}',
                  ].where((part) => part.trim().isNotEmpty).join(' · '),
                ),
            ],
          ),
        ),
      if (details != null && details.workspaceTemplates.isNotEmpty)
        StoreDetailSection(
          title: '工作区模板',
          count: details.workspaceTemplates.length,
          child: StoreGroupedList(
            children: <Widget>[
              for (final template in details.workspaceTemplates)
                StoreListRow(
                  leading: const StoreRowGlyph(
                    icon: Icons.folder_copy_outlined,
                  ),
                  title: template.displayName,
                  subtitle: template.description,
                  trailing: StoreBadge(label: template.projectType),
                ),
            ],
          ),
        ),
      if (runtime.desktopWidgets.isNotEmpty)
        StoreDetailSection(
          title: '桌面小组件',
          count: runtime.desktopWidgets.length,
          child: StoreGroupedList(
            children: <Widget>[
              StoreListRow(
                leading: const StoreRowGlyph(icon: Icons.widgets_outlined),
                title: '管理桌面小组件',
                subtitle: enabled ? '查看并添加这个插件提供的小组件' : '启用插件后可用',
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: enabled ? () => _openDesktopWidgets(runtime) : null,
              ),
            ],
          ),
        ),
      StoreDetailSection(
        title: '包含内容',
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          children: <Widget>[
            _ContentCount(label: '资源', count: runtime.resources.length),
            _ContentCount(label: '界面模块', count: runtime.uiModules.length),
            _ContentCount(
              label: '导航入口',
              count: runtime.navigationEntries.length,
            ),
            _ContentCount(label: '桌面小组件', count: runtime.desktopWidgets.length),
            _ContentCount(
              label: '工作区模板',
              count: runtime.workspaceTemplates.length,
            ),
            _ContentCount(label: 'AI 供应商', count: runtime.aiProviders.length),
          ],
        ),
      ),
    ];
  }

  Widget _subpackageSwitch(
    core_proxy.ToolPkgSubpackageInfo subpackage,
    bool containerEnabled,
  ) {
    if (_togglingSubpackages.contains(subpackage.packageName)) {
      return const SizedBox.square(
        dimension: 22,
        child: CircularProgressIndicator(strokeWidth: 2),
      );
    }
    return Switch(
      value: subpackage.enabled,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      onChanged: containerEnabled
          ? (value) => _setSubpackageEnabled(subpackage, value)
          : null,
    );
  }

  Future<void> _setSubpackageEnabled(
    core_proxy.ToolPkgSubpackageInfo subpackage,
    bool enabled,
  ) async {
    setState(() => _togglingSubpackages.add(subpackage.packageName));
    try {
      final success = await _clients.application
          .packageManager()
          .setToolPkgSubpackageEnabled(
            subpackagePackageName: subpackage.packageName,
            enabled: enabled,
          );
      if (!success) {
        throw StateError('子包状态切换失败：${subpackage.packageName}');
      }
      await _loadDetails();
    } catch (error, stackTrace) {
      debugPrint('Failed to toggle ToolPkg subpackage: $error\n$stackTrace');
      _showError(error);
    } finally {
      if (mounted) {
        setState(() => _togglingSubpackages.remove(subpackage.packageName));
      }
    }
  }

  Future<void> _openUi(String? routeId) async {
    final runtime = await _toolPkgRuntime();
    if (!mounted) {
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => ToolPkgUiLauncherScreen(
          clients: _clients,
          plugin: runtime,
          initialRouteId: routeId,
        ),
      ),
    );
  }

  Future<core_proxy.ToolPkgContainerRuntime> _toolPkgRuntime() async {
    final details = _details;
    if (details is _ToolPkgDetails) {
      return details.runtime;
    }
    final runtimes = await _clients.application
        .packageManager()
        .getToolPkgContainerRuntimes();
    return runtimes.firstWhere((item) => item.packageName == widget.id);
  }

  void _openDesktopWidgets(core_proxy.ToolPkgContainerRuntime runtime) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) =>
            ToolPkgDesktopWidgetsScreen(clients: _clients, plugin: runtime),
      ),
    );
  }

  // ------------------------------------------------------------------- script

  List<Widget> _scriptSections(_ScriptDetails data) {
    final package = data.package;
    final colorScheme = Theme.of(context).colorScheme;
    return <Widget>[
      StoreDetailSection(
        title: '工具',
        count: package.tools.length,
        child: package.tools.isEmpty
            ? const _EmptyLine(text: '这个包没有工具。')
            : StoreGroupedList(
                children: <Widget>[
                  for (final tool in package.tools)
                    StoreListRow(
                      leading: const StoreRowGlyph(icon: Icons.build_outlined),
                      title: _toolTitle(tool),
                      subtitle: localizedText(tool.description),
                      footer: Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: <Widget>[
                          StoreBadge(label: tool.name, icon: Icons.tag_rounded),
                          for (final parameter in tool.parameters)
                            StoreBadge(
                              label:
                                  '${parameter.name}${parameter.requiredValue ? ' *' : ''}',
                            ),
                        ],
                      ),
                      trailing: StoreGetButton(
                        label: '运行',
                        icon: Icons.play_arrow_rounded,
                        onPressed: () => _runPackageTool(package, tool),
                      ),
                    ),
                ],
              ),
      ),
      if (package.env.isNotEmpty)
        StoreDetailSection(
          title: '环境变量',
          count: package.env.length,
          child: StoreGroupedList(
            children: <Widget>[
              for (final env in package.env)
                StoreListRow(
                  leading: const StoreRowGlyph(icon: Icons.key_outlined),
                  title: env.name,
                  monospaceTitle: true,
                  subtitle: localizedText(env.description),
                  trailing: env.requiredValue
                      ? StoreBadge(
                          label: '必填',
                          color: colorScheme.errorContainer,
                          foreground: colorScheme.onErrorContainer,
                        )
                      : null,
                ),
            ],
          ),
        ),
      if (package.states.isNotEmpty)
        StoreDetailSection(
          title: '状态',
          count: package.states.length,
          child: StoreGroupedList(
            children: <Widget>[
              for (final state in package.states)
                StoreListRow(
                  leading: const StoreRowGlyph(icon: Icons.rule_outlined),
                  title: state.id,
                  subtitle: <String>[
                    '条件：${state.condition}',
                    '${state.tools.length} 个工具',
                    if (state.excludeTools.isNotEmpty)
                      '排除 ${state.excludeTools.length} 个',
                    if (state.inheritTools) '继承工具',
                  ].join(' · '),
                ),
            ],
          ),
        ),
    ];
  }

  String _toolTitle(core_proxy.PackageTool tool) {
    final display = tool.display;
    if (display != null) {
      final label = localizedText(display.label).trim();
      if (label.isNotEmpty) {
        return label;
      }
    }
    return tool.name;
  }

  void _runPackageTool(
    core_proxy.ToolPackage package,
    core_proxy.PackageTool tool,
  ) {
    showDialog<void>(
      context: context,
      builder: (context) => PackageToolRunDialog(
        packageName: package.name,
        tool: tool,
        clients: _clients,
      ),
    );
  }

  // -------------------------------------------------------------------- skill

  List<Widget> _skillSections(_SkillDetails data) {
    final content = data.content;
    return <Widget>[
      StoreDetailSection(
        title: 'SKILL.md',
        child: content == null || content.trim().isEmpty
            ? const _EmptyLine(text: '读取不到技能内容。')
            : _CollapsibleCode(text: content),
      ),
    ];
  }

  // ---------------------------------------------------------------------- mcp

  List<Widget> _mcpSections(
    core_proxy.ExtensionCatalogEntry entry,
    bool enabled,
    _McpDetails data,
  ) {
    final theme = Theme.of(context);
    final server = data.server;
    final tools =
        data.status?.cachedTools ?? const <core_proxy.CachedToolInfo>[];
    final running = entry.runtimeState == 'running';
    final success = theme.brightness == Brightness.dark
        ? const Color(0xFF7BD88F)
        : const Color(0xFF2E7D4F);
    final configItems = server == null
        ? const <StoreInfoItem>[]
        : _mcpConfigItems(server);
    return <Widget>[
      StoreDetailSection(
        title: '运行',
        child: StoreGroupedList(
          children: <Widget>[
            StoreListRow(
              leading: StoreRowGlyph(
                icon: running
                    ? Icons.play_circle_outline_rounded
                    : Icons.pause_circle_outline_rounded,
                color: running ? success : null,
              ),
              title: running ? '运行中' : (enabled ? '未运行' : '已停用'),
              subtitle: entry.scope == 'localOnly'
                  ? '通过本地命令启动，只在这台设备上运行。'
                  : '连接远程服务。',
              trailing: _mcpBusy
                  ? const SizedBox.square(
                      dimension: 22,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Wrap(
                      spacing: 4,
                      children: <Widget>[
                        TextButton.icon(
                          style: SettingsControlStyles.sectionTextButton(),
                          onPressed: !enabled || server == null
                              ? null
                              : () => _runMcpLifecycle(
                                  () => running
                                      ? restartMcpServer(
                                          clients: _clients,
                                          serverId: entry.id,
                                        )
                                      : startMcpServer(
                                          clients: _clients,
                                          serverId: entry.id,
                                        ),
                                ),
                          icon: Icon(
                            running
                                ? Icons.restart_alt_rounded
                                : Icons.play_arrow_rounded,
                            size: 18,
                          ),
                          label: Text(running ? '重启' : '启动'),
                        ),
                        if (running)
                          TextButton.icon(
                            style: SettingsControlStyles.sectionTextButton(),
                            onPressed: () => _runMcpLifecycle(
                              () => killMcpServer(
                                clients: _clients,
                                serverId: entry.id,
                              ),
                            ),
                            icon: const Icon(Icons.stop_rounded, size: 18),
                            label: const Text('停止'),
                          ),
                      ],
                    ),
            ),
          ],
        ),
      ),
      StoreDetailSection(
        title: '配置',
        trailing: TextButton.icon(
          style: SettingsControlStyles.sectionTextButton(),
          onPressed: server == null || _mcpBusy
              ? null
              : () => _editMcpConfig(server),
          icon: const Icon(Icons.edit_outlined, size: 18),
          label: const Text('编辑'),
        ),
        child: configItems.every((item) => item.value.trim().isEmpty)
            ? const _EmptyLine(text: '没有可显示的配置。')
            : StoreInfoTable(items: configItems),
      ),
      StoreDetailSection(
        title: '工具',
        count: tools.length,
        child: tools.isEmpty
            ? const _EmptyLine(text: '还没有缓存的工具，服务启动后会自动获取。')
            : StoreGroupedList(
                children: <Widget>[
                  for (final tool in tools)
                    StoreListRow(
                      leading: const StoreRowGlyph(icon: Icons.build_outlined),
                      title: tool.name,
                      subtitle: tool.description,
                      footer: _mcpParameterBadges(tool),
                      trailing: StoreGetButton(
                        label: '运行',
                        icon: Icons.play_arrow_rounded,
                        onPressed: () => _runMcpTool(entry.id, tool),
                      ),
                    ),
                ],
              ),
      ),
    ];
  }

  Widget? _mcpParameterBadges(core_proxy.CachedToolInfo tool) {
    final names = mcpToolParameterNames(tool.inputSchema);
    if (names.isEmpty) {
      return null;
    }
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: <Widget>[for (final name in names) StoreBadge(label: name)],
    );
  }

  List<StoreInfoItem> _mcpConfigItems(core_proxy.ServerConfig server) {
    return <StoreInfoItem>[
      StoreInfoItem('URL', server.url ?? ''),
      StoreInfoItem('传输', server.type ?? ''),
      StoreInfoItem(
        '命令',
        server.command.trim().isEmpty
            ? ''
            : <String>[server.command, ...server.args].join(' '),
      ),
      StoreInfoItem('环境变量', server.env.keys.join(', ')),
      StoreInfoItem('Headers', server.headers.keys.join(', ')),
      StoreInfoItem('自动批准', server.autoApprove.join(', ')),
    ];
  }

  Widget _mcpDescriptionActions(
    core_proxy.ExtensionCatalogEntry entry,
    _McpDetails details,
  ) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        IconButton(
          tooltip: '编辑简介',
          onPressed: _mcpBusy ? null : () => _editMcpMetadata(entry, details),
          icon: const Icon(Icons.edit_note_outlined),
        ),
        IconButton(
          tooltip: '让 AI 重新生成简介',
          onPressed: _mcpBusy
              ? null
              : () => _regenerateMcpDescription(entry, details),
          icon: _describing
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.auto_fix_high_outlined),
        ),
      ],
    );
  }

  /// Metadata to edit; servers without saved metadata start from the catalog entry.
  core_proxy.PluginMetadata _mcpMetadata(
    core_proxy.ExtensionCatalogEntry entry,
    _McpDetails details,
  ) {
    final metadata = details.metadata;
    if (metadata != null) {
      return metadata;
    }
    return core_proxy.PluginMetadata(
      name: entry.displayName,
      description: entry.description,
      author: entry.authors.join(', '),
      version: entry.version,
    );
  }

  Future<void> _afterMcpChange() async {
    await _controller.refresh();
    await _loadDetails();
  }

  Future<void> _runMcpLifecycle(Future<String> Function() action) async {
    setState(() => _mcpBusy = true);
    try {
      await action();
      await _afterMcpChange();
    } catch (error, stackTrace) {
      debugPrint('Failed to update MCP lifecycle: $error\n$stackTrace');
      _showError(error);
    } finally {
      if (mounted) {
        setState(() => _mcpBusy = false);
      }
    }
  }

  Future<void> _regenerateMcpDescription(
    core_proxy.ExtensionCatalogEntry entry,
    _McpDetails details,
  ) async {
    setState(() {
      _mcpBusy = true;
      _describing = true;
    });
    try {
      final metadata = _mcpMetadata(entry, details);
      final generated = await _clients.application
          .mcpRepository()
          .generatePluginDescription(
            pluginId: entry.id,
            pluginName: metadata.name,
          );
      await _clients.permissionsMcpRuntimeMcpLocalServer
          .addOrUpdatePluginMetadata(
            pluginId: entry.id,
            metadata: core_proxy.PluginMetadata(
              name: metadata.name,
              description: generated,
              author: metadata.author,
              version: metadata.version,
            ),
          );
      await _afterMcpChange();
    } catch (error, stackTrace) {
      debugPrint('Failed to regenerate MCP description: $error\n$stackTrace');
      _showError(error);
    } finally {
      if (mounted) {
        setState(() {
          _mcpBusy = false;
          _describing = false;
        });
      }
    }
  }

  void _editMcpMetadata(
    core_proxy.ExtensionCatalogEntry entry,
    _McpDetails details,
  ) {
    showDialog<void>(
      context: context,
      builder: (context) => MCPMetadataEditDialog(
        serverId: entry.id,
        metadata: _mcpMetadata(entry, details),
        clients: _clients,
        onSaved: _afterMcpChange,
      ),
    );
  }

  void _editMcpConfig(core_proxy.ServerConfig server) {
    showDialog<void>(
      context: context,
      builder: (context) => MCPConfigEditDialog(
        serverId: widget.id,
        server: server,
        clients: _clients,
        onSaved: _afterMcpChange,
      ),
    );
  }

  void _runMcpTool(String serverId, core_proxy.CachedToolInfo tool) {
    showDialog<void>(
      context: context,
      builder: (context) =>
          MCPToolRunDialog(serverId: serverId, tool: tool, clients: _clients),
    );
  }

  // ----------------------------------------------------------- info & manage

  List<StoreInfoItem> _infoItems(core_proxy.ExtensionCatalogEntry entry) {
    final details = _details;
    final marketEntryId = entry.marketEntryId;
    return <StoreInfoItem>[
      StoreInfoItem('ID', entry.id),
      StoreInfoItem('类型', StoreKind.of(entry.kind).label),
      StoreInfoItem('版本', entry.version),
      StoreInfoItem('作者', entry.authors.join('、')),
      StoreInfoItem(
        '来源',
        entry.isCustomized
            ? '${storeSourceLabel(entry.source)}（已自定义）'
            : storeSourceLabel(entry.source),
      ),
      StoreInfoItem('分类', entry.category),
      if (details is _ToolPkgDetails) ...<StoreInfoItem>[
        StoreInfoItem('ToolPkg API', details.runtime.apiVersion),
        StoreInfoItem('入口', details.runtime.mainEntry),
        StoreInfoItem('路径', details.runtime.sourcePath),
        StoreInfoItem('许可证', details.runtime.license ?? ''),
      ],
      if (details is _ScriptDetails)
        StoreInfoItem(
          '默认状态',
          details.package.enabledByDefault ? '默认启用' : '默认停用',
        ),
      if (marketEntryId != null) StoreInfoItem('市场条目', marketEntryId),
    ];
  }

  Widget _manageCard(core_proxy.ExtensionCatalogEntry entry, bool busy) {
    final colorScheme = Theme.of(context).colorScheme;
    final customized = entry.isCustomized;
    final removable = entry.source != 'builtin' || customized;
    if (!removable) {
      return const _EmptyLine(text: '内置扩展不能移除，可以停用它。');
    }
    final name = '「${entry.displayName}」';
    final message = customized
        ? '卸载$name的自定义版本，恢复使用内置版本。数据和配置会保留。'
        : entry.scope == 'space'
        ? '$name会从设备空间的所有设备上移除，连同它的文件和配置。此操作不可撤销。'
        : '删除$name和它的文件，移除后需要重新安装才能使用。此操作不可撤销。';
    return StoreGroupedList(
      children: <Widget>[
        StoreInlineConfirm(
          message: message,
          confirmLabel: customized ? '卸载自定义版本' : '移除',
          destructive: true,
          onConfirm: () => _remove(entry),
          trigger: (context, requestConfirm) => StoreListRow(
            leading: StoreRowGlyph(
              icon: customized
                  ? Icons.restore_rounded
                  : Icons.delete_outline_rounded,
              color: colorScheme.error,
            ),
            title: customized ? '卸载自定义版本' : '移除扩展',
            subtitle: customized ? '恢复使用内置版本' : '删除这个扩展和它的文件',
            trailing: TextButton(
              onPressed: busy ? null : requestConfirm,
              style: SettingsControlStyles.sectionTextButton().copyWith(
                foregroundColor: WidgetStatePropertyAll<Color>(
                  colorScheme.error,
                ),
              ),
              child: Text(customized ? '卸载' : '移除'),
            ),
          ),
        ),
      ],
    );
  }

  Future<void> _remove(core_proxy.ExtensionCatalogEntry entry) async {
    final removed = await _run(() => _controller.remove(entry));
    if (removed && !entry.isCustomized && mounted) {
      // The extension is gone; return to where the user came from.
      unawaited(Navigator.of(context).maybePop());
    }
  }

  Widget _overflowMenu(core_proxy.ExtensionCatalogEntry entry) {
    return PopupMenuButton<String>(
      tooltip: '更多',
      icon: const Icon(Icons.more_vert_rounded),
      onSelected: (value) {
        switch (value) {
          case 'copy-id':
            unawaited(Clipboard.setData(ClipboardData(text: entry.id)));
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('已复制 ID'),
                behavior: SnackBarBehavior.floating,
              ),
            );
          case 'reload':
            unawaited(_loadDetails());
        }
      },
      itemBuilder: (context) => const <PopupMenuEntry<String>>[
        PopupMenuItem<String>(
          value: 'copy-id',
          child: ListTile(
            leading: Icon(Icons.copy_rounded),
            title: Text('复制 ID'),
          ),
        ),
        PopupMenuItem<String>(
          value: 'reload',
          child: ListTile(
            leading: Icon(Icons.refresh_rounded),
            title: Text('重新加载详情'),
          ),
        ),
      ],
    );
  }

  // ---------------------------------------------------------------- candidate

  Widget _buildCandidate(
    BuildContext context,
    core_proxy.ExtensionCatalogCandidate candidate,
  ) {
    final kind = StoreKind.of(candidate.kind);
    final busy = _controller.isBusy(
      ExtensionCatalogController.candidateKey(candidate),
    );
    final requiredScope = candidate.requiredScope;
    final scope = requiredScope ?? _installScope;
    final wide = MediaQuery.sizeOf(context).width >= 600;
    return StoreDetailScaffold(
      header: StoreDetailHeader(
        icon: ExtensionAvatar(
          seed: '${candidate.kind}:${candidate.id}',
          label: candidate.displayName,
          size: wide ? 88 : 72,
          busy: busy,
        ),
        title: candidate.displayName,
        subtitle: <String>[
          if (candidate.authors.isNotEmpty) candidate.authors.join('、'),
          'Operit 官方',
        ].join(' · '),
        badges: <Widget>[
          StoreBadge(label: kind.label, icon: kind.icon),
          if (candidate.version.trim().isNotEmpty)
            StoreBadge(label: 'v${candidate.version}'),
          const StoreBadge(label: '无需联网', icon: Icons.offline_bolt_outlined),
        ],
        actions: <Widget>[
          StoreGetButton(
            label: '安装',
            icon: Icons.download_rounded,
            prominent: true,
            large: true,
            busy: busy,
            onPressed: () =>
                _run(() => _controller.installCandidate(candidate, scope)),
          ),
          if (requiredScope == null)
            SettingsSegmentedSelector<String>(
              value: _installScope,
              onChanged: (value) {
                if (!busy) {
                  setState(() => _installScope = value);
                }
              },
              segments: <ButtonSegment<String>>[
                for (final option in const <String>['device', 'space'])
                  ButtonSegment<String>(
                    value: option,
                    icon: Icon(storeScopeIcon(option), size: 16),
                    label: Text(storeScopeLabel(option)),
                  ),
              ],
            )
          else
            StoreBadge(
              label: '只能装在${storeScopeLabel(requiredScope)}',
              icon: storeScopeIcon(requiredScope),
            ),
        ],
      ),
      children: <Widget>[
        StoreStatsStrip(
          stats: <StoreStat>[
            StoreStat(label: '类型', value: kind.label, icon: kind.icon),
            if (candidate.version.trim().isNotEmpty)
              StoreStat(label: '版本', value: candidate.version),
            if (candidate.subpackageCount > 0)
              StoreStat(label: '子包', value: '${candidate.subpackageCount}'),
            if (candidate.kind == 'script' && candidate.toolCount > 0)
              StoreStat(label: '工具', value: '${candidate.toolCount}'),
            StoreStat(
              label: '安装到',
              value: storeScopeLabel(scope),
              icon: storeScopeIcon(scope),
            ),
          ],
        ),
        if (candidate.description.trim().isNotEmpty)
          StoreDetailSection(
            title: '简介',
            child: StoreExpandableText(text: candidate.description),
          ),
        StoreDetailSection(
          title: '安装位置',
          child: StoreGroupedList(
            children: <Widget>[
              StoreListRow(
                leading: StoreRowGlyph(icon: storeScopeIcon(scope)),
                title: storeScopeLabel(scope),
                subtitle: scope == 'space'
                    ? '同步到设备空间的所有设备，启用状态和配置一起同步，配置里的密钥也会共享；每台设备上的对话都能用它。'
                    : '只装在这台设备上，只有在这里运行的对话能用它；安装后可以在扩展详情里共享出去。',
              ),
            ],
          ),
        ),
        StoreDetailSection(
          title: '信息',
          child: StoreInfoTable(
            items: <StoreInfoItem>[
              StoreInfoItem('ID', candidate.id),
              StoreInfoItem('类型', kind.label),
              StoreInfoItem('版本', candidate.version),
              StoreInfoItem('作者', candidate.authors.join('、')),
              StoreInfoItem('分类', candidate.category),
              const StoreInfoItem('来源', 'Operit 官方离线包'),
            ],
          ),
        ),
      ],
    );
  }

  // ------------------------------------------------------------------ helpers

  /// Runs an action and reports a failure in a SnackBar; returns success.
  Future<bool> _run(Future<void> Function() action) async {
    try {
      await action();
      return true;
    } catch (error, stackTrace) {
      debugPrint('Extension action failed: $error\n$stackTrace');
      _showError(error);
      return false;
    }
  }

  void _showError(Object error) {
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
    );
  }
}

/// Describes why a prerequisite prevents automatic loading.
String _dependencyIssueStatus(core_proxy.ToolPkgDependencyIssue issue) {
  return switch (issue.code) {
    'missing' => '未安装',
    'disabled' => '未启用',
    'version_incompatible' => '版本不兼容',
    'unresolved' => '依赖无法解析',
    _ => throw StateError(
      'Unsupported ToolPkg dependency issue code: ${issue.code}',
    ),
  };
}

/// Explains the prerequisite failure and its declared version range.
String _dependencyIssueDescription(core_proxy.ToolPkgDependencyIssue issue) {
  final base = switch (issue.code) {
    'missing' => '未安装这个前置插件。',
    'disabled' => '前置插件已安装，但当前未启用。',
    'version_incompatible' => '前置插件的版本不满足 manifest 声明的范围。',
    'unresolved' => '前置插件存在循环依赖或其依赖无法满足，当前插件已停止加载。',
    _ => throw StateError(
      'Unsupported ToolPkg dependency issue code: ${issue.code}',
    ),
  };
  final ranges = <String>[
    if (issue.requiredMinVersion != null) '需要 ≥ ${issue.requiredMinVersion}',
    if (issue.requiredMaxVersion != null) '需要 ≤ ${issue.requiredMaxVersion}',
    if (issue.installedVersion != null) '当前 v${issue.installedVersion}',
  ];
  return ranges.isEmpty ? base : '$base ${ranges.join('，')}';
}

/// Labeled switch for enabling an installed extension.
class _EnableToggle extends StatelessWidget {
  const _EnableToggle({
    required this.label,
    required this.value,
    required this.busy,
    required this.onChanged,
  });

  final String label;
  final bool value;
  final bool busy;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final motion = ExtensionMotion.of(context);
    return AnimatedContainer(
      duration: motion.standard,
      curve: ExtensionMotion.enter,
      height: 38,
      padding: const EdgeInsets.only(left: 14, right: 4),
      decoration: ShapeDecoration(
        color: value
            ? colorScheme.primaryContainer.withValues(alpha: 0.45)
            : colorScheme.surface.withValues(alpha: 0),
        shape: StadiumBorder(
          side: BorderSide(
            color: value
                ? colorScheme.primary.withValues(alpha: 0.35)
                : colorScheme.outlineVariant,
          ),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          AnimatedSize(
            duration: motion.fast,
            child: busy
                ? const Padding(
                    padding: EdgeInsets.only(right: 8),
                    child: SizedBox.square(
                      dimension: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  )
                : const SizedBox.shrink(),
          ),
          AnimatedSwitcher(
            duration: motion.fast,
            child: Text(
              label,
              key: ValueKey<String>(label),
              style: theme.textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          const SizedBox(width: 6),
          SizedBox(
            width: 52,
            child: Center(
              child: Switch(
                value: value,
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                onChanged: busy ? null : onChanged,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ContentCount extends StatelessWidget {
  const _ContentCount({required this.label, required this.count});

  final String label;
  final int count;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final active = count > 0;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: storeCardColor(theme.colorScheme),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: storeCardBorderColor(theme.colorScheme)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Text(
            '$count',
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w700,
              color: active
                  ? theme.colorScheme.onSurface
                  : theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

class _EmptyLine extends StatelessWidget {
  const _EmptyLine({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return StoreGroupedList(
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
          child: Text(
            text,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }
}

/// Monospace document preview that folds long content.
class _CollapsibleCode extends StatefulWidget {
  const _CollapsibleCode({required this.text});

  final String text;

  @override
  State<_CollapsibleCode> createState() => _CollapsibleCodeState();
}

class _CollapsibleCodeState extends State<_CollapsibleCode> {
  static const double _foldedHeight = 320;
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final motion = ExtensionMotion.of(context);
    final long = '\n'.allMatches(widget.text).length > 16;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        AnimatedSize(
          duration: motion.emphasized,
          curve: ExtensionMotion.enter,
          alignment: Alignment.topCenter,
          child: ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: !long || _expanded ? double.infinity : _foldedHeight,
            ),
            child: ClipRRect(
              borderRadius: storeCardRadius,
              child: Container(
                width: double.infinity,
                padding: const EdgeInsets.all(14),
                color: theme.colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.6,
                ),
                child: SingleChildScrollView(
                  physics: const NeverScrollableScrollPhysics(),
                  child: SelectableText(
                    widget.text,
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                      height: 1.6,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
        if (long)
          TextButton(
            onPressed: () => setState(() => _expanded = !_expanded),
            style: SettingsControlStyles.sectionTextButton(),
            child: Text(_expanded ? '收起' : '展开全文'),
          ),
      ],
    );
  }
}

class _DetailsSkeleton extends StatelessWidget {
  const _DetailsSkeleton();

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 24),
      child: StoreShimmer(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            const StoreSkeletonBox(width: 120, height: 16),
            const SizedBox(height: 12),
            for (var index = 0; index < 3; index += 1) ...<Widget>[
              const StoreSkeletonBox(height: 60, radius: 12),
              const SizedBox(height: 8),
            ],
          ],
        ),
      ),
    );
  }
}
