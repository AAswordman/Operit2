// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../main/navigation/AppNavigationModels.dart';
import '../../../../main/screens/OperitScreens.dart';
import '../../../../main/screens/ScreenRouteRegistry.dart';
import '../../../settings/models/SettingsModels.dart';
import '../../viewmodel/ChatViewModel.dart';

/// Shows context usage with one compact statistics and device menu.
class ChatContextStatsTopBarButton extends StatefulWidget {
  /// Creates the context menu for the current conversation.
  const ChatContextStatsTopBarButton({
    super.key,
    required this.viewModel,
    required this.currentChatId,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;

  /// Creates the statistics polling and menu state.
  @override
  State<ChatContextStatsTopBarButton> createState() =>
      _ChatContextStatsTopBarButtonState();
}

class _ChatContextStatsTopBarButtonState
    extends State<ChatContextStatsTopBarButton> {
  final MenuController _menuController = MenuController();
  Timer? _refreshTimer;
  StreamSubscription<core_proxy.BindingRouteStatus?>? _routeSubscription;
  core_proxy.ChatInputMenuSummary? _summary;
  core_proxy.RuntimeDeviceSpaceTopology? _topology;
  core_proxy.BindingRouteStatus? _bindingStatus;
  bool _polling = false;
  bool _loading = true;
  bool _loadFailed = false;
  bool _routeFailed = false;
  bool _isOpen = false;
  int _requestEpoch = 0;

  /// Starts polling without creating a second device settings surface.
  @override
  void initState() {
    super.initState();
    _bindRouteStatus();
    unawaited(_loadAllData());
    _refreshTimer = Timer.periodic(const Duration(seconds: 4), (_) {
      unawaited(_loadAllData());
    });
  }

  /// Invalidates in-flight data when the conversation or runtime changes.
  @override
  void didUpdateWidget(covariant ChatContextStatsTopBarButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _requestEpoch++;
      _polling = false;
      _summary = null;
      _topology = null;
      _bindingStatus = null;
      _loading = true;
      _loadFailed = false;
      _routeFailed = false;
      _bindRouteStatus();
      unawaited(_loadAllData());
    }
  }

  /// Stops periodic work and detaches the route stream.
  @override
  void dispose() {
    _requestEpoch++;
    _refreshTimer?.cancel();
    _routeSubscription?.cancel();
    super.dispose();
  }

  /// Marks the executing device in the single device list.
  void _bindRouteStatus() {
    _routeSubscription?.cancel();
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) return;
    final epoch = _requestEpoch;
    _routeSubscription = widget.viewModel.watchRouteStatus(chatId).listen(
      (status) {
        if (!mounted || epoch != _requestEpoch) return;
        setState(() {
          _bindingStatus = status;
          _routeFailed = false;
        });
      },
      onError: (Object error) {
        if (!mounted || epoch != _requestEpoch) return;
        setState(() {
          _bindingStatus = null;
          _routeFailed = true;
        });
      },
    );
  }

  /// Loads only the context summary and the device list shown in this menu.
  Future<void> _loadAllData() async {
    if (_polling) return;
    _polling = true;
    final epoch = _requestEpoch;
    final viewModel = widget.viewModel;
    try {
      final results = await Future.wait<Object>(<Future<Object>>[
        viewModel.chatCore.chatInputMenuSummary(chatId: widget.currentChatId),
        viewModel.clients.server.runtimeRemoteLinkService.deviceSpaceTopology(),
      ]);
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _summary = results[0] as core_proxy.ChatInputMenuSummary;
        _topology = results[1] as core_proxy.RuntimeDeviceSpaceTopology;
        _loading = false;
        _loadFailed = false;
      });
    } catch (_) {
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _summary = null;
        _topology = null;
        _loading = false;
        _loadFailed = true;
      });
    } finally {
      if (epoch == _requestEpoch) _polling = false;
    }
  }

  /// Converts the model context limit from Ki tokens to tokens.
  int? get _maxContextTokens =>
      _summary == null ? null : (_summary!.maxContextLength * 1024).round();

  /// Leaves unavailable limits unknown rather than presenting a false zero.
  double? get _contextUsagePercentage {
    final limit = _maxContextTokens;
    if (limit == null || limit <= 0 || _summary == null) return null;
    return _summary!.currentWindowSize / limit * 100;
  }

  /// Opens the existing device settings through the menu's only add-device action.
  void _openDeviceSpaceSettings() {
    _menuController.close();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.accessLinks),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  /// Builds a native Material menu without nested status cards or duplicate actions.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final percentage = _contextUsagePercentage;
    final displayText = _loading
        ? '...'
        : percentage == null
        ? '--'
        : '${percentage.toStringAsFixed(0)}%';
    final mediaQuery = MediaQuery.of(context);
    final ownerDeviceId = _bindingStatus?.ownerIsLocal == true
        ? _topology?.currentDeviceId
        : _bindingStatus?.ownerNodeId;
    final ownerInDeviceList = _topology?.devices.any(
      (device) => device.deviceId == ownerDeviceId,
    );

    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 8),
      child: Center(
        child: MenuAnchor(
          controller: _menuController,
          alignmentOffset: const Offset(0, 4),
          onOpen: () {
            setState(() => _isOpen = true);
            unawaited(_loadAllData());
          },
          onClose: () => setState(() => _isOpen = false),
          style: MenuStyle(
            backgroundColor: WidgetStatePropertyAll(
              colorScheme.surfaceContainer,
            ),
            padding: const WidgetStatePropertyAll(EdgeInsets.zero),
          ),
          builder: (context, controller, _) => TextButton.icon(
            onPressed: () {
              if (controller.isOpen) {
                controller.close();
              } else {
                controller.open();
              }
            },
            icon: const Icon(Icons.data_usage_rounded, size: 14),
            label: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Text(displayText),
                const SizedBox(width: 3),
                Icon(
                  _isOpen ? Icons.expand_less_rounded : Icons.expand_more_rounded,
                  size: 16,
                ),
              ],
            ),
            style: TextButton.styleFrom(
              foregroundColor: percentage != null && percentage >= 90
                  ? colorScheme.error
                  : colorScheme.onSurfaceVariant,
              minimumSize: const Size(0, 28),
              padding: const EdgeInsets.symmetric(horizontal: 8),
              visualDensity: VisualDensity.compact,
            ),
          ),
          menuChildren: <Widget>[
            SizedBox(
              width: math.min(300.0, mediaQuery.size.width - 24),
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  maxHeight: math.min(440.0, mediaQuery.size.height - 24),
                ),
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: Text(
                              '上下文占用',
                              style: Theme.of(context).textTheme.titleSmall,
                            ),
                          ),
                          IconButton(
                            tooltip: AppLocalizations.of(context)!.refresh,
                            onPressed: _polling ? null : _loadAllData,
                            icon: const Icon(Icons.refresh_rounded, size: 16),
                            visualDensity: VisualDensity.compact,
                          ),
                        ],
                      ),
                      if (_loading)
                        const Padding(
                          padding: EdgeInsets.symmetric(vertical: 8),
                          child: Text('正在加载…'),
                        )
                      else if (_loadFailed)
                        Text(
                          '无法加载上下文和设备信息',
                          style: TextStyle(color: colorScheme.error),
                        )
                      else ...<Widget>[
                        _StatRow(
                          label: '上下文窗口',
                          value: '${_summary!.currentWindowSize} / '
                              '${_maxContextTokens! > 0 ? _maxContextTokens : "未设置上限"}',
                        ),
                        _StatRow(
                          label: '输入 Token',
                          value: '${_summary!.inputTokenCount}',
                        ),
                        _StatRow(
                          label: '输出 Token',
                          value: '${_summary!.outputTokenCount}',
                        ),
                      ],
                      const Divider(height: 16),
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: Text(
                              '设备',
                              style: Theme.of(context).textTheme.titleSmall,
                            ),
                          ),
                          IconButton(
                            tooltip: '添加设备',
                            onPressed: _openDeviceSpaceSettings,
                            icon: const Icon(Icons.add_rounded, size: 18),
                            color: colorScheme.onSurfaceVariant,
                            visualDensity: VisualDensity.compact,
                          ),
                        ],
                      ),
                      if (!_loading && !_loadFailed) ...<Widget>[
                        for (final device in _topology!.devices)
                          _DeviceRow(
                            device: device,
                            isLocal:
                                device.deviceId == _topology!.currentDeviceId,
                            isExecuting: device.deviceId == ownerDeviceId,
                            online: device.deviceId == ownerDeviceId
                                ? _bindingStatus!.ownerReachable
                                : device.online,
                          ),
                        if (ownerDeviceId != null && ownerInDeviceList == false)
                          Text(
                            '执行设备不在当前空间中',
                            style: TextStyle(color: colorScheme.error),
                          ),
                        if (_routeFailed)
                          Text(
                            '无法读取执行设备状态',
                            style: TextStyle(color: colorScheme.error),
                          ),
                      ],
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StatRow extends StatelessWidget {
  /// Creates one context metric without repeating the top bar percentage.
  const _StatRow({required this.label, required this.value});

  final String label;
  final String value;

  /// Allows values to wrap on narrow screens and with enlarged text.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.end,
              style: theme.textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}

class _DeviceRow extends StatelessWidget {
  /// Creates the sole row for a device, including its execution and connection state.
  const _DeviceRow({
    required this.device,
    required this.isLocal,
    required this.isExecuting,
    required this.online,
  });

  final core_proxy.RuntimeDeviceSpaceDevice device;
  final bool isLocal;
  final bool isExecuting;
  final bool online;

  /// Keeps device identity and status together instead of splitting them across cards.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final connection = !online
        ? '离线'
        : device.relayHops != null && device.relayHops! > 0
        ? '${device.relayHops} 跳'
        : '在线';
    final status = <String>[
      if (isLocal) '本机',
      if (isExecuting) '当前执行',
      connection,
    ].join(' · ');
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: <Widget>[
          Icon(
            Icons.devices_other_rounded,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              device.deviceName,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall,
            ),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              status,
              textAlign: TextAlign.end,
              style: theme.textTheme.labelSmall?.copyWith(
                color: online
                    ? theme.colorScheme.onSurfaceVariant
                    : theme.colorScheme.error,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
