// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../main/navigation/AppNavigationModels.dart';
import '../../../../main/screens/OperitScreens.dart';
import '../../../../main/screens/ScreenRouteRegistry.dart';
import '../../../settings/models/SettingsModels.dart';
import '../../viewmodel/ChatViewModel.dart';
import 'context_stats/ContextUsagePanel.dart';
import 'context_stats/DecentralizedFiberTopologyPanel.dart';
import 'context_stats/TopBarCapsuleModels.dart';
import 'context_stats/TopBarDynamicCapsule.dart';

/// Shows live context and routing activity in an elongated top-bar capsule
/// paired with a modular context-usage and decentralized fiber-topology menu.
class ChatContextStatsTopBarButton extends StatefulWidget {
  /// Creates the context and node-routing menu for the current conversation.
  const ChatContextStatsTopBarButton({
    super.key,
    required this.viewModel,
    required this.currentChatId,
    this.processingStateListenable,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final ValueListenable<core_proxy.InputProcessingState>?
  processingStateListenable;

  /// Creates the statistics polling, live status, and menu state.
  @override
  State<ChatContextStatsTopBarButton> createState() =>
      _ChatContextStatsTopBarButtonState();
}

class _ChatContextStatsTopBarButtonState
    extends State<ChatContextStatsTopBarButton> {
  final MenuController _menuController = MenuController();
  Timer? _refreshTimer;
  Timer? _compactBannerTimer;
  Timer? _routeBannerTimer;
  StreamSubscription<core_proxy.BindingRouteStatus?>? _routeSubscription;
  core_proxy.ChatInputMenuSummary? _summary;
  core_proxy.RuntimeDeviceSpaceTopology? _topology;
  core_proxy.BindingRouteStatus? _bindingStatus;
  bool _pollingSummary = false;
  bool _pollingTopology = false;
  bool _topologyFailed = false;
  bool _loading = true;
  bool _loadFailed = false;
  bool _routeFailed = false;
  bool _isOpen = false;
  bool _compacting = false;
  bool _recentCompactCompleted = false;
  int? _recentCompactedSavedTokens;
  String? _compactError;
  String? _switchingDeviceId;
  String? _recentSwitchedDeviceName;
  String? _switchError;
  int _requestEpoch = 0;

  ContextVisualMode _contextVisualMode = ContextVisualMode.segmented;
  NodeLabelDisplayMode _nodeDisplayMode = NodeLabelDisplayMode.auto;

  core_proxy.InputProcessingState get _processingState =>
      widget.processingStateListenable?.value ??
      core_proxy.InputProcessingState.idle();

  @override
  void initState() {
    super.initState();
    widget.processingStateListenable?.addListener(_onProcessingStateChanged);
    _bindRouteStatus();
    unawaited(_loadAllData());
    _refreshTimer = Timer.periodic(const Duration(seconds: 4), (_) {
      unawaited(_loadAllData());
    });
  }

  @override
  void didUpdateWidget(covariant ChatContextStatsTopBarButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.processingStateListenable != widget.processingStateListenable) {
      oldWidget.processingStateListenable?.removeListener(
        _onProcessingStateChanged,
      );
      widget.processingStateListenable?.addListener(_onProcessingStateChanged);
    }
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _requestEpoch++;
      _compactBannerTimer?.cancel();
      _routeBannerTimer?.cancel();
      _pollingSummary = false;
      _pollingTopology = false;
      _topologyFailed = false;
      _summary = null;
      _topology = null;
      _bindingStatus = null;
      _loading = true;
      _loadFailed = false;
      _routeFailed = false;
      _compacting = false;
      _recentCompactCompleted = false;
      _recentCompactedSavedTokens = null;
      _compactError = null;
      _switchingDeviceId = null;
      _recentSwitchedDeviceName = null;
      _switchError = null;
      _bindRouteStatus();
      unawaited(_loadAllData());
    }
  }

  @override
  void dispose() {
    _requestEpoch++;
    widget.processingStateListenable?.removeListener(_onProcessingStateChanged);
    _refreshTimer?.cancel();
    _compactBannerTimer?.cancel();
    _routeBannerTimer?.cancel();
    _routeSubscription?.cancel();
    super.dispose();
  }

  void _onProcessingStateChanged() {
    if (!mounted) return;
    setState(() {});
    final tag = _processingState.tag;
    if (tag == 'Completed' || tag == 'Idle') {
      unawaited(_loadSummary());
    }
  }

  void _bindRouteStatus() {
    _routeSubscription?.cancel();
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) return;
    final epoch = _requestEpoch;
    _routeSubscription = widget.viewModel
        .watchRouteStatus(chatId)
        .listen(
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

  Future<void> _loadAllData() async {
    await Future.wait<void>(<Future<void>>[_loadSummary(), _loadTopology()]);
  }

  Future<void> _loadSummary() async {
    if (_pollingSummary) return;
    _pollingSummary = true;
    final epoch = _requestEpoch;
    try {
      final summary = await widget.viewModel.chatCore.chatInputMenuSummary(
        chatId: widget.currentChatId,
      );
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _summary = summary;
        _loading = false;
        _loadFailed = false;
      });
    } catch (_) {
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _summary = null;
        _loading = false;
        _loadFailed = true;
      });
    } finally {
      if (epoch == _requestEpoch) _pollingSummary = false;
    }
  }

  Future<void> _loadTopology() async {
    if (_pollingTopology) return;
    _pollingTopology = true;
    final epoch = _requestEpoch;
    try {
      final topology = await widget
          .viewModel
          .clients
          .server
          .runtimeRemoteLinkService
          .deviceSpaceTopology();
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _topology = topology;
        _topologyFailed = false;
      });
    } catch (_) {
      if (!mounted || epoch != _requestEpoch) return;
      setState(() {
        _topology = null;
        _topologyFailed = true;
      });
    } finally {
      if (epoch == _requestEpoch) _pollingTopology = false;
    }
  }

  Future<void> _triggerCompaction() async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty || _compacting) return;
    final epoch = _requestEpoch;
    final beforeTokens = _summary?.currentWindowSize;
    _compactBannerTimer?.cancel();
    setState(() {
      _compacting = true;
      _recentCompactCompleted = false;
      _recentCompactedSavedTokens = null;
      _compactError = null;
    });
    try {
      final compacted = await widget.viewModel.compactChat(chatId);
      if (!mounted || epoch != _requestEpoch) return;
      if (!compacted) {
        setState(() {
          _compactError = '当前暂无可压缩的上下文历史';
        });
        return;
      }
      await _loadSummary();
      if (!mounted || epoch != _requestEpoch) return;
      final afterTokens = _summary?.currentWindowSize;
      final saved =
          beforeTokens != null &&
              afterTokens != null &&
              beforeTokens > afterTokens
          ? beforeTokens - afterTokens
          : null;
      setState(() {
        _recentCompactCompleted = true;
        _recentCompactedSavedTokens = saved;
      });
      _compactBannerTimer = Timer(const Duration(milliseconds: 3200), () {
        if (!mounted || epoch != _requestEpoch) return;
        setState(() {
          _recentCompactCompleted = false;
          _recentCompactedSavedTokens = null;
        });
      });
    } catch (error) {
      if (!mounted || epoch != _requestEpoch) return;
      setState(() => _compactError = '压缩失败：$error');
    } finally {
      if (mounted && epoch == _requestEpoch) {
        setState(() => _compacting = false);
      }
    }
  }

  Future<void> _switchExecutionDevice(
    core_proxy.RuntimeDeviceSpaceDevice device,
  ) async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty || _switchingDeviceId != null) {
      return;
    }
    final epoch = _requestEpoch;
    _routeBannerTimer?.cancel();
    setState(() {
      _switchingDeviceId = device.deviceId;
      _recentSwitchedDeviceName = null;
      _switchError = null;
    });
    try {
      final service = widget.viewModel.clients.server.runtimeRemoteLinkService;
      final status = await service.changeChatExecutionDevice(
        chatId: chatId,
        targetNodeId: device.deviceId,
      );
      if (!mounted || epoch != _requestEpoch) return;
      final targetLabel = shortDeviceLabel(
        device,
        isLocal: device.deviceId == _topology?.currentDeviceId,
      );
      setState(() {
        _bindingStatus = status;
        _routeFailed = false;
        _recentSwitchedDeviceName = targetLabel;
      });
      _routeBannerTimer = Timer(const Duration(milliseconds: 2800), () {
        if (!mounted || epoch != _requestEpoch) return;
        setState(() => _recentSwitchedDeviceName = null);
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

  String? get _ownerDeviceId => _bindingStatus?.ownerIsLocal == true
      ? _topology?.currentDeviceId
      : _bindingStatus?.ownerNodeId;

  bool _canSelectDevice(core_proxy.RuntimeDeviceSpaceDevice device) {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) return false;
    if (_switchingDeviceId != null) return false;
    if (device.deviceId == _ownerDeviceId) return false;
    if (!device.online) return false;
    return device.deviceId == _topology?.currentDeviceId ||
        device.coreVersion != null;
  }

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

  void _cycleNodeDisplayMode() {
    setState(() {
      _nodeDisplayMode = switch (_nodeDisplayMode) {
        NodeLabelDisplayMode.auto => NodeLabelDisplayMode.taskOnly,
        NodeLabelDisplayMode.taskOnly => NodeLabelDisplayMode.nameOnly,
        NodeLabelDisplayMode.nameOnly => NodeLabelDisplayMode.auto,
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final mediaQuery = MediaQuery.of(context);
    final processingState = _processingState;
    final capsulePresentation = TopBarCapsuleResolver.resolve(
      TopBarActivitySnapshot(
        summary: _summary,
        loadingSummary: _loading,
        summaryFailed: _loadFailed,
        processingState: processingState,
        routeStatus: _bindingStatus,
        routeFailed: _routeFailed,
        topology: _topology,
        compacting: _compacting,
        recentCompactedSavedTokens: _recentCompactedSavedTokens,
        recentCompactCompleted: _recentCompactCompleted,
        switchingDeviceId: _switchingDeviceId,
        recentSwitchedDeviceName: _recentSwitchedDeviceName,
      ),
      l10n,
    );

    final canCompact =
        widget.currentChatId?.trim().isNotEmpty == true &&
        (_summary?.currentWindowSize ?? 0) > 0 &&
        processingState.tag != 'Summarizing';

    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 2),
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
            shape: WidgetStatePropertyAll(
              RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(14),
                side: BorderSide(
                  color: colorScheme.outlineVariant.withValues(alpha: 0.65),
                ),
              ),
            ),
          ),
          builder: (context, controller, _) => TopBarDynamicCapsule(
            presentation: capsulePresentation,
            isOpen: _isOpen,
            onPressed: () {
              if (controller.isOpen) {
                controller.close();
              } else {
                controller.open();
              }
            },
          ),
          menuChildren: <Widget>[
            SizedBox(
              width: math.min(276.0, mediaQuery.size.width - 24),
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  maxHeight: math.min(400.0, mediaQuery.size.height - 24),
                ),
                child: SingleChildScrollView(
                  primary: false,
                  padding: const EdgeInsets.all(10),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      ContextUsagePanel(
                        summary: _summary,
                        loading: _loading,
                        loadFailed: _loadFailed,
                        visualMode: _contextVisualMode,
                        compacting: _compacting,
                        canCompact: canCompact,
                        compactError: _compactError,
                        onVisualModeChanged: (mode) {
                          setState(() => _contextVisualMode = mode);
                        },
                        onCompactPressed: _triggerCompaction,
                        onRefreshPressed: _loadAllData,
                        pollingSummary: _pollingSummary,
                      ),
                      const Divider(height: 12),
                      DecentralizedFiberTopologyPanel(
                        topology: _topology,
                        topologyFailed: _topologyFailed,
                        bindingStatus: _bindingStatus,
                        routeFailed: _routeFailed,
                        processingState: processingState,
                        switchingDeviceId: _switchingDeviceId,
                        switchError: _switchError,
                        ownerDeviceId: _ownerDeviceId,
                        displayMode: _nodeDisplayMode,
                        onDisplayModeCycle: _cycleNodeDisplayMode,
                        canSelectDevice: _canSelectDevice,
                        onSelectDevice: _switchExecutionDevice,
                        onAddDevice: _openDeviceSpaceSettings,
                      ),
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
