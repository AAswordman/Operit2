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
  String? _switchingDeviceId;
  String? _switchError;
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
      _switchingDeviceId = null;
      _switchError = null;
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

  /// Changes the owning Core and keeps selection tied to the confirmed route.
  Future<void> _switchExecutionDevice(
    core_proxy.RuntimeDeviceSpaceDevice device,
  ) async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty || _switchingDeviceId != null) {
      return;
    }
    final epoch = _requestEpoch;
    setState(() {
      _switchingDeviceId = device.deviceId;
      _switchError = null;
    });
    try {
      final service = widget.viewModel.clients.server.runtimeRemoteLinkService;
      final status = await service.changeChatExecutionDevice(
        chatId: chatId,
        targetNodeId: device.deviceId,
      );
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _bindingStatus = status;
        _routeFailed = false;
      });
      unawaited(_loadAllData());
    } catch (error) {
      if (!mounted || epoch != _requestEpoch) return;
      setState(() => _switchError = '切换失败：$error');
    } finally {
      if (mounted && epoch == _requestEpoch) {
        setState(() => _switchingDeviceId = null);
      }
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
                  primary: false,
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
                            isSwitching: device.deviceId == _switchingDeviceId,
                            onSelect:
                                widget.currentChatId?.trim().isNotEmpty ==
                                        true &&
                                    _switchingDeviceId == null &&
                                    device.deviceId != ownerDeviceId &&
                                    device.online &&
                                    (device.deviceId ==
                                            _topology!.currentDeviceId ||
                                        device.coreVersion != null)
                                ? () => _switchExecutionDevice(device)
                                : null,
                            online: device.deviceId == ownerDeviceId
                                ? _bindingStatus!.ownerReachable
                                : device.online,
                          ),
                        if (ownerDeviceId != null && ownerInDeviceList == false)
                          Text(
                            '执行设备不在当前空间中',
                            style: TextStyle(color: colorScheme.error),
                          ),
                        if (_switchError case final error?)
                          Padding(
                            padding: const EdgeInsets.only(top: 6),
                            child: Tooltip(
                              message: error,
                              child: Text(
                                error,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.bodySmall
                                    ?.copyWith(color: colorScheme.error),
                              ),
                            ),
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

  /// Aligns values to the trailing edge while allowing narrow-screen wrapping.
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
          Expanded(
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
    required this.isSwitching,
    required this.onSelect,
  });

  final core_proxy.RuntimeDeviceSpaceDevice device;
  final bool isLocal;
  final bool isExecuting;
  final bool online;
  final bool isSwitching;
  final VoidCallback? onSelect;

  /// Keeps roles and connectivity compact while names use the remaining width.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final connection = !online
        ? '离线'
        : device.relayHops != null && device.relayHops! > 0
        ? '在线 · ${device.relayHops} 跳中继'
        : '在线';
    final status = <String>[
      if (isLocal) '本机',
      if (isExecuting) '当前执行设备',
      if (isSwitching) '正在切换',
      connection,
    ].join('，');
    final onlineColor = theme.brightness == Brightness.dark
        ? Colors.green.shade400
        : Colors.green.shade700;

    return Semantics(
      container: true,
      label: '${device.deviceName}，$status',
      excludeSemantics: true,
      button: true,
      selected: isExecuting,
      enabled: onSelect != null,
      onTap: onSelect,
      child: InkWell(
        onTap: onSelect,
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 4),
          child: Row(
            children: <Widget>[
              Tooltip(
                message: isLocal ? '本机' : '远程设备',
                child: Icon(
                  isLocal
                      ? Icons.computer_rounded
                      : Icons.devices_other_rounded,
                  size: 16,
                  color: colors.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Tooltip(
                  message: device.deviceName,
                  child: Text(
                    device.deviceName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall,
                  ),
                ),
              ),
              if (isSwitching) ...<Widget>[
                const SizedBox(width: 8),
                const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ] else if (isExecuting) ...<Widget>[
                const SizedBox(width: 8),
                Tooltip(
                  message: '当前执行设备',
                  child: Icon(
                    Icons.check_rounded,
                    size: 16,
                    color: colors.primary,
                  ),
                ),
              ],
              const SizedBox(width: 8),
              Tooltip(
                message: connection,
                child: SizedBox(
                  width: 16,
                  height: 22,
                  child: Center(
                    child: Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: online ? onlineColor : null,
                        border: online
                            ? null
                            : Border.all(
                                color: colors.onSurfaceVariant,
                                width: 1.5,
                              ),
                      ),
                    ),
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
