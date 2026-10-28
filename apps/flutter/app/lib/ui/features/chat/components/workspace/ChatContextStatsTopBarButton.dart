// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../main/navigation/AppNavigationModels.dart';
import '../../../../main/screens/OperitScreens.dart';
import '../../../../main/screens/ScreenRouteRegistry.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../../settings/models/SettingsModels.dart';
import '../../viewmodel/ChatViewModel.dart';

/// Top bar execution node and context statistics pill button with click-to-expand popover.
class ChatContextStatsTopBarButton extends StatefulWidget {
  const ChatContextStatsTopBarButton({
    super.key,
    required this.viewModel,
    required this.currentChatId,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;

  @override
  State<ChatContextStatsTopBarButton> createState() =>
      _ChatContextStatsTopBarButtonState();
}

class _ChatContextStatsTopBarButtonState
    extends State<ChatContextStatsTopBarButton> {
  final GlobalKey _buttonKey = GlobalKey();
  Timer? _refreshTimer;
  OverlayEntry? _overlayEntry;
  StreamSubscription<core_proxy.BindingRouteStatus?>? _routeSubscription;
  bool _polling = false;

  core_proxy.BindingRouteStatus? _bindingStatus;
  core_proxy.CoreSpace? _space;
  core_proxy.RuntimeDeviceSpaceTopology? _topology;

  int _currentWindowSize = 0;
  int _inputTokenCount = 0;
  int _outputTokenCount = 0;
  double? _maxContextLength;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _bindRouteStatus();
    _loadAllData();
    _refreshTimer = Timer.periodic(const Duration(seconds: 4), (_) {
      _loadAllData();
    });
  }

  @override
  void didUpdateWidget(covariant ChatContextStatsTopBarButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _currentWindowSize = 0;
      _inputTokenCount = 0;
      _outputTokenCount = 0;
      _maxContextLength = null;
      _bindingStatus = null;
      _loading = true;
      _bindRouteStatus();
      _loadAllData();
    }
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _routeSubscription?.cancel();
    _overlayEntry?.remove();
    _overlayEntry = null;
    super.dispose();
  }

  void _bindRouteStatus() {
    _routeSubscription?.cancel();
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      _bindingStatus = null;
      return;
    }
    _routeSubscription = widget.viewModel
        .watchRouteStatus(chatId)
        .listen((status) {
          if (!mounted) return;
          setState(() {
            _bindingStatus = status;
          });
          _overlayEntry?.markNeedsBuild();
        }, onError: (_) {});
  }

  Future<void> _loadAllData() async {
    if (_polling) return;
    _polling = true;
    try {
      final chatId = widget.currentChatId;
      final summaryFuture = widget.viewModel.chatCore.chatInputMenuSummary(
        chatId: chatId,
      );
      final spaceFuture =
          widget.viewModel.clients.server.runtimeRemoteLinkService.deviceSpace();
      final topologyFuture =
          widget.viewModel.clients.server.runtimeRemoteLinkService
              .deviceSpaceTopology();

      final results = await Future.wait<dynamic>([
        summaryFuture.then<dynamic>((v) => v).catchError((_) => null),
        spaceFuture.then<dynamic>((v) => v).catchError((_) => null),
        topologyFuture.then<dynamic>((v) => v).catchError((_) => null),
      ]);

      if (!mounted || widget.currentChatId != chatId) return;

      final summary = results[0] as core_proxy.ChatInputMenuSummary?;
      final space = results[1] as core_proxy.CoreSpace?;
      final topology = results[2] as core_proxy.RuntimeDeviceSpaceTopology?;

      setState(() {
        if (summary != null) {
          _currentWindowSize = summary.currentWindowSize;
          _inputTokenCount = summary.inputTokenCount;
          _outputTokenCount = summary.outputTokenCount;
          _maxContextLength = summary.maxContextLength;
        }
        if (space != null) {
          _space = space;
        }
        if (topology != null) {
          _topology = topology;
        }
        _loading = false;
      });
      _overlayEntry?.markNeedsBuild();
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    } finally {
      _polling = false;
    }
  }

  int? get _maxContextTokens =>
      _maxContextLength == null ? null : (_maxContextLength! * 1024).round();

  double? get _contextUsagePercentage {
    final maxTokens = _maxContextTokens;
    if (maxTokens == null || maxTokens <= 0) return null;
    return (_currentWindowSize / maxTokens * 100).clamp(0.0, 999.0);
  }

  bool get _isLocal => _bindingStatus?.ownerIsLocal ?? true;

  core_proxy.RuntimeDeviceSpaceDevice? _findDevice(String? deviceId) {
    if (deviceId == null || _topology == null) return null;
    for (final device in _topology!.devices) {
      if (device.deviceId == deviceId) return device;
    }
    return null;
  }

  String get _currentDeviceShortLabel {
    if (_isLocal) {
      return '本机';
    }
    final remoteId = _bindingStatus?.ownerNodeId;
    final device = _findDevice(remoteId);
    if (device != null && device.deviceName.trim().isNotEmpty) {
      return device.deviceName.trim();
    }
    if (_bindingStatus?.ownerPlatform.isNotEmpty == true) {
      return _bindingStatus!.ownerPlatform;
    }
    return '远端';
  }

  IconData get _currentDeviceIcon {
    if (_isLocal) {
      final localDev = _findDevice(_topology?.currentDeviceId);
      return _deviceIcon(localDev?.platform ?? 'windows');
    }
    final dev = _findDevice(_bindingStatus?.ownerNodeId);
    return _deviceIcon(dev?.platform ?? _bindingStatus?.ownerPlatform);
  }

  void _dismissPopover() {
    final entry = _overlayEntry;
    if (entry == null) return;
    entry.remove();
    _overlayEntry = null;
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _claimHost() async {
    final chatId = widget.currentChatId;
    if (chatId == null) return;
    final l10n = AppLocalizations.of(context)!;
    try {
      final accepted = await showDialog<bool>(
        context: context,
        builder: (dialogContext) {
          return AlertDialog(
            content: Text(l10n.chatRouteClaimHostPrompt),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(false),
                child: Text(l10n.chatRouteClaimHostNo),
              ),
              FilledButton(
                onPressed: () => Navigator.of(dialogContext).pop(true),
                child: Text(l10n.chatRouteClaimHostYes),
              ),
            ],
          );
        },
      );
      if (!mounted || accepted != true) return;
      await widget.viewModel.claimMissingBindingLocal(chatId);
      if (!mounted) return;
      await _loadAllData();
      if (mounted) setState(() {});
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.maybeOf(context)?.showSnackBar(
        SnackBar(content: Text('$error')),
      );
    }
  }

  void _openDeviceSpaceSettings() {
    _dismissPopover();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.accessLinks),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  void _togglePopover() {
    if (_overlayEntry != null) {
      _dismissPopover();
      return;
    }

    final renderBox =
        _buttonKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null || !renderBox.hasSize) return;

    final buttonOffset = renderBox.localToGlobal(Offset.zero);
    final buttonSize = renderBox.size;
    final overlay = Overlay.maybeOf(context);
    if (overlay == null) return;

    unawaited(_loadAllData());

    _overlayEntry = OverlayEntry(
      builder: (popoverContext) {
        final mediaQuery = MediaQuery.of(popoverContext);
        final screenWidth = mediaQuery.size.width;
        final cardWidth = math.min(326.0, screenWidth - 24.0);
        final right = math.max(
          12.0,
          screenWidth - (buttonOffset.dx + buttonSize.width),
        );
        final top = buttonOffset.dy + buttonSize.height + 6;

        return Stack(
          children: <Widget>[
            // Tap outside to dismiss
            Positioned.fill(
              child: Listener(
                behavior: HitTestBehavior.opaque,
                onPointerDown: (_) => _dismissPopover(),
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              top: top,
              right: right,
              width: cardWidth,
              child: Material(
                color: Colors.transparent,
                child: _ContextStatsPopoverCard(
                  currentWindowSize: _currentWindowSize,
                  maxContextTokens: _maxContextTokens,
                  inputTokenCount: _inputTokenCount,
                  outputTokenCount: _outputTokenCount,
                  percentage: _contextUsagePercentage,
                  space: _space,
                  topology: _topology,
                  bindingStatus: _bindingStatus,
                  onClose: _dismissPopover,
                  onRefresh: _loadAllData,
                  onClaimHost: _claimHost,
                  onOpenDeviceSpace: _openDeviceSpaceSettings,
                ),
              ),
            ),
          ],
        );
      },
    );

    overlay.insert(_overlayEntry!);
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final percentage = _contextUsagePercentage;
    final isOpen = _overlayEntry != null;

    final Color badgeColor;
    if (percentage == null) {
      badgeColor = colorScheme.onSurfaceVariant;
    } else if (percentage >= 90) {
      badgeColor = colorScheme.error;
    } else if (percentage >= 75) {
      badgeColor = Colors.orangeAccent;
    } else {
      badgeColor = colorScheme.primary;
    }

    final displayText = _loading && percentage == null
        ? '...'
        : percentage == null
        ? '0%'
        : '${percentage.toStringAsFixed(0)}%';

    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 8),
      child: Center(
        child: InkWell(
          key: _buttonKey,
          onTap: _togglePopover,
          borderRadius: BorderRadius.circular(16),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
            decoration: BoxDecoration(
              color: isOpen
                  ? colorScheme.surfaceContainerHighest
                  : colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(16),
              border: Border.all(
                color: isOpen
                    ? colorScheme.primary.withValues(alpha: 0.6)
                    : colorScheme.outlineVariant.withValues(alpha: 0.35),
                width: 1,
              ),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(
                  _currentDeviceIcon,
                  size: 14,
                  color: _isLocal ? colorScheme.onSurfaceVariant : colorScheme.primary,
                ),
                const SizedBox(width: 5),
                Text(
                  _currentDeviceShortLabel,
                  style: theme.textTheme.labelMedium?.copyWith(
                    fontWeight: FontWeight.w500,
                    color: colorScheme.onSurface,
                  ),
                ),
                Text(
                  ' · ',
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: colorScheme.outline,
                  ),
                ),
                Text(
                  displayText,
                  style: theme.textTheme.labelMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                    color: badgeColor,
                  ),
                ),
                const SizedBox(width: 3),
                Icon(
                  isOpen
                      ? Icons.keyboard_arrow_up_rounded
                      : Icons.keyboard_arrow_down_rounded,
                  size: 16,
                  color: colorScheme.onSurfaceVariant,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ContextStatsPopoverCard extends StatelessWidget {
  const _ContextStatsPopoverCard({
    required this.currentWindowSize,
    required this.maxContextTokens,
    required this.inputTokenCount,
    required this.outputTokenCount,
    required this.percentage,
    required this.space,
    required this.topology,
    required this.bindingStatus,
    required this.onClose,
    required this.onRefresh,
    required this.onClaimHost,
    required this.onOpenDeviceSpace,
  });

  final int currentWindowSize;
  final int? maxContextTokens;
  final int inputTokenCount;
  final int outputTokenCount;
  final double? percentage;
  final core_proxy.CoreSpace? space;
  final core_proxy.RuntimeDeviceSpaceTopology? topology;
  final core_proxy.BindingRouteStatus? bindingStatus;
  final VoidCallback onClose;
  final VoidCallback onRefresh;
  final VoidCallback onClaimHost;
  final VoidCallback onOpenDeviceSpace;

  String _formatTokenCount(int count) {
    final str = count.toString();
    final reg = RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))');
    return str.replaceAllMapped(reg, (match) => '${match[1]},');
  }

  core_proxy.RuntimeDeviceSpaceDevice? _findDevice(String? deviceId) {
    if (deviceId == null || topology == null) return null;
    for (final device in topology!.devices) {
      if (device.deviceId == deviceId) return device;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final totalTokens = inputTokenCount + outputTokenCount;

    final ratio = (percentage ?? 0) / 100.0;
    final progress = ratio.clamp(0.0, 1.0);

    final Color statusColor;
    if (percentage != null && percentage! >= 90) {
      statusColor = colorScheme.error;
    } else if (percentage != null && percentage! >= 75) {
      statusColor = Colors.orangeAccent;
    } else {
      statusColor = colorScheme.primary;
    }

    final isLocal = bindingStatus?.ownerIsLocal ?? true;
    final isReachable = bindingStatus?.ownerReachable ?? true;
    final activeDeviceId = isLocal
        ? topology?.currentDeviceId
        : bindingStatus?.ownerNodeId;
    final activeDevice = _findDevice(activeDeviceId);
    final activeDeviceName = activeDevice?.deviceName.trim().isNotEmpty == true
        ? activeDevice!.deviceName.trim()
        : isLocal
        ? '本机'
        : (bindingStatus?.ownerPlatform.isNotEmpty == true
            ? '远程 ${bindingStatus!.ownerPlatform}'
            : '远程节点');
    final activePlatform =
        activeDevice?.platform ?? bindingStatus?.ownerPlatform ?? 'windows';

    final spaceName = space?.spaceName.trim().isNotEmpty == true
        ? space!.spaceName.trim()
        : '设备空间';

    final otherDevices = topology?.devices
            .where((d) => d.deviceId != topology?.currentDeviceId)
            .toList() ??
        const <core_proxy.RuntimeDeviceSpaceDevice>[];

    return OperitGlassSurface(
      layer: OperitGlassSurfaceLayer.card,
      color: colorScheme.surface.withValues(alpha: 0.94),
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.35),
        width: 1,
      ),
      shadows: <BoxShadow>[
        BoxShadow(
          color: Colors.black.withValues(alpha: 0.28),
          blurRadius: 20,
          spreadRadius: 1,
          offset: const Offset(0, 8),
        ),
      ],
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 520),
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(14),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              // Header: Space identity & refresh action
              Row(
                children: <Widget>[
                  Icon(
                    Icons.hub_outlined,
                    size: 16,
                    color: colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      spaceName,
                      style: theme.textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.refresh_rounded, size: 16),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                    tooltip: l10n.refresh,
                    onPressed: onRefresh,
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // Section 1: Execution Node Card
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.3),
                    width: 0.8,
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Container(
                          padding: const EdgeInsets.all(6),
                          decoration: BoxDecoration(
                            color: colorScheme.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Icon(
                            _deviceIcon(activePlatform),
                            size: 18,
                            color: isLocal
                                ? colorScheme.primary
                                : (isReachable
                                    ? colorScheme.tertiary
                                    : colorScheme.error),
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: <Widget>[
                              Row(
                                children: <Widget>[
                                  Flexible(
                                    child: Text(
                                      activeDeviceName,
                                      style: theme.textTheme.labelMedium?.copyWith(
                                        fontWeight: FontWeight.w600,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Container(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 5,
                                      vertical: 1.5,
                                    ),
                                    decoration: BoxDecoration(
                                      color: (isLocal
                                              ? colorScheme.primary
                                              : colorScheme.tertiary)
                                          .withValues(alpha: 0.12),
                                      borderRadius: BorderRadius.circular(6),
                                    ),
                                    child: Text(
                                      isLocal ? '当前设备' : '远程节点',
                                      style: theme.textTheme.labelSmall?.copyWith(
                                        color: isLocal
                                            ? colorScheme.primary
                                            : colorScheme.tertiary,
                                        fontSize: 10,
                                        fontWeight: FontWeight.w600,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 3),
                              Row(
                                children: <Widget>[
                                  Container(
                                    width: 6,
                                    height: 6,
                                    decoration: BoxDecoration(
                                      color: isReachable
                                          ? const Color(0xFF4CAF50)
                                          : colorScheme.error,
                                      shape: BoxShape.circle,
                                    ),
                                  ),
                                  const SizedBox(width: 5),
                                  Text(
                                    isLocal
                                        ? '本地 Core 驱动'
                                        : (isReachable
                                            ? '在线协同中'
                                            : '远端设备不可达'),
                                    style: theme.textTheme.bodySmall?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                      fontSize: 11,
                                    ),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                    if (!isLocal && !isReachable) ...<Widget>[
                      const SizedBox(height: 8),
                      SizedBox(
                        width: double.infinity,
                        child: FilledButton.tonal(
                          style: FilledButton.styleFrom(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 6,
                            ),
                            visualDensity: VisualDensity.compact,
                          ),
                          onPressed: onClaimHost,
                          child: const Text('接管至本机执行'),
                        ),
                      ),
                    ],
                  ],
                ),
              ),

              // Section 2: Space Collaboration Overview
              const SizedBox(height: 8),
              if (otherDevices.isNotEmpty) ...<Widget>[
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
                  child: Text(
                    '空间成员设备 (${topology!.devices.length})',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                ...otherDevices.map((device) {
                  final isCurrentActive =
                      device.deviceId == bindingStatus?.ownerNodeId;
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2.5),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          _deviceIcon(device.platform),
                          size: 14,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            device.deviceName,
                            style: theme.textTheme.bodySmall?.copyWith(
                              fontWeight: isCurrentActive
                                  ? FontWeight.w600
                                  : FontWeight.normal,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        Container(
                          width: 5,
                          height: 5,
                          decoration: BoxDecoration(
                            color: device.online
                                ? const Color(0xFF4CAF50)
                                : colorScheme.outlineVariant,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 4),
                        Text(
                          device.online
                              ? (device.relayHops == null || device.relayHops == 0
                                  ? '直连'
                                  : '${device.relayHops}跳')
                              : '离线',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 10,
                          ),
                        ),
                      ],
                    ),
                  );
                }),
              ] else ...<Widget>[
                InkWell(
                  onTap: onOpenDeviceSpace,
                  borderRadius: BorderRadius.circular(8),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.devices_other_rounded,
                          size: 14,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            '单设备空间 · 点击添加设备协同',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                              fontSize: 11,
                            ),
                          ),
                        ),
                        Icon(
                          Icons.arrow_forward_ios_rounded,
                          size: 10,
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ],
                    ),
                  ),
                ),
              ],

              Divider(
                height: 18,
                thickness: 0.5,
                color: colorScheme.outlineVariant.withValues(alpha: 0.35),
              ),

              // Section 3: Context & Token Statistics
              Row(
                children: <Widget>[
                  Icon(
                    Icons.data_usage_rounded,
                    size: 16,
                    color: statusColor,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    '上下文统计',
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const Spacer(),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
                    decoration: BoxDecoration(
                      color: statusColor.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      percentage == null
                          ? '加载中...'
                          : '${percentage!.toStringAsFixed(0)}%',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: statusColor,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              ClipRRect(
                borderRadius: BorderRadius.circular(4),
                child: LinearProgressIndicator(
                  value: progress,
                  minHeight: 5,
                  backgroundColor: colorScheme.surfaceContainerHighest,
                  valueColor: AlwaysStoppedAnimation<Color>(statusColor),
                ),
              ),
              const SizedBox(height: 10),
              _StatRow(
                label: '上下文窗口',
                value:
                    '${_formatTokenCount(currentWindowSize)} / ${maxContextTokens == null ? '--' : _formatTokenCount(maxContextTokens!)}',
              ),
              const SizedBox(height: 4),
              _StatRow(
                label: '输入 Token',
                value: _formatTokenCount(inputTokenCount),
              ),
              const SizedBox(height: 4),
              _StatRow(
                label: '输出 Token',
                value: _formatTokenCount(outputTokenCount),
              ),
              const SizedBox(height: 4),
              _StatRow(
                label: '总 Token',
                value: _formatTokenCount(totalTokens),
                highlighted: true,
              ),

              Divider(
                height: 18,
                thickness: 0.5,
                color: colorScheme.outlineVariant.withValues(alpha: 0.35),
              ),

              // Footer: Quick navigation to device space settings
              InkWell(
                onTap: onOpenDeviceSpace,
                borderRadius: BorderRadius.circular(8),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.settings_outlined,
                        size: 15,
                        color: colorScheme.primary,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        '设备空间与网络设置',
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: colorScheme.primary,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      const Spacer(),
                      Icon(
                        Icons.chevron_right_rounded,
                        size: 16,
                        color: colorScheme.primary,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StatRow extends StatelessWidget {
  const _StatRow({
    required this.label,
    required this.value,
    this.highlighted = false,
  });

  final String label;
  final String value;
  final bool highlighted;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;

    return Row(
      children: <Widget>[
        Text(
          label,
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        const Spacer(),
        Text(
          value,
          style: textTheme.bodySmall?.copyWith(
            color: highlighted ? colorScheme.primary : colorScheme.onSurface,
            fontWeight: highlighted ? FontWeight.w700 : FontWeight.w500,
          ),
        ),
      ],
    );
  }
}

IconData _deviceIcon(String? platform) {
  if (platform == null) return Icons.devices_other_rounded;
  final name = platform.trim().toLowerCase();
  if (name.contains('android')) return Icons.phone_android_rounded;
  if (name.contains('windows')) return Icons.laptop_windows_rounded;
  if (name.contains('mac') || name.contains('darwin')) {
    return Icons.laptop_mac_rounded;
  }
  if (name.contains('linux')) return Icons.dns_outlined;
  if (name.contains('ios')) return Icons.phone_iphone_rounded;
  return Icons.devices_other_rounded;
}
