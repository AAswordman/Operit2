// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../l10n/generated/app_localizations.dart';
import '../../style/MessageHeaderMetadata.dart';
import '../../style/input/common/InputProcessingStatusLane.dart';

/// Semantic visual tone of the elongated top-bar status capsule.
enum TopBarCapsuleTone { idle, active, success, warning, error }

/// Kinetic icon style rendered in the leading slot of the top-bar capsule.
enum TopBarCapsuleIconKind {
  usageRing,
  compacting,
  completedBurst,
  routingFiber,
  executingCompute,
  linkAlert,
}

/// Visual sub-mode inside the context usage block.
enum ContextVisualMode { segmented, telemetry }

/// Display strategy for the single-line dynamic peer node label.
enum NodeLabelDisplayMode { auto, nameOnly, taskOnly }

/// Visual activity state of an optical-fiber curve connecting two nodes.
enum FiberLinkState { idle, active, preview, handoff, bidirectional, broken }

/// One segment displayed in the Scheme A context usage bar.
class ContextSegmentEntry {
  const ContextSegmentEntry({
    required this.id,
    required this.label,
    required this.tokens,
    required this.color,
  });

  final String id;
  final String label;
  final int tokens;
  final Color color;
}

/// Immutable presentation snapshot consumed by [TopBarDynamicCapsule].
class TopBarCapsulePresentation {
  const TopBarCapsulePresentation({
    required this.key,
    required this.iconKey,
    required this.iconKind,
    required this.tone,
    required this.label,
    required this.tooltip,
    this.progressFraction,
    this.contextPercentage,
    this.showShimmer = false,
  });

  final String key;
  final String iconKey;
  final TopBarCapsuleIconKind iconKind;
  final TopBarCapsuleTone tone;
  final String label;
  final String tooltip;
  final double? progressFraction;
  final double? contextPercentage;
  final bool showShimmer;
}

/// Aggregated state inputs used to derive [TopBarCapsulePresentation].
class TopBarActivitySnapshot {
  const TopBarActivitySnapshot({
    required this.summary,
    required this.loadingSummary,
    required this.summaryFailed,
    required this.processingState,
    required this.routeStatus,
    required this.routeFailed,
    required this.topology,
    required this.compacting,
    required this.recentCompactedSavedTokens,
    required this.recentCompactCompleted,
    required this.switchingDeviceId,
    required this.recentSwitchedDeviceName,
  });

  final core_proxy.ChatInputMenuSummary? summary;
  final bool loadingSummary;
  final bool summaryFailed;
  final core_proxy.InputProcessingState processingState;
  final core_proxy.BindingRouteStatus? routeStatus;
  final bool routeFailed;
  final core_proxy.RuntimeDeviceSpaceTopology? topology;
  final bool compacting;
  final int? recentCompactedSavedTokens;
  final bool recentCompactCompleted;
  final String? switchingDeviceId;
  final String? recentSwitchedDeviceName;
}

/// Resolves live activity inputs into a decoupled capsule presentation.
///
/// Additional runtime states can be inserted into the priority chain below
/// without modifying the capsule widget itself.
class TopBarCapsuleResolver {
  const TopBarCapsuleResolver._();

  static TopBarCapsulePresentation resolve(
    TopBarActivitySnapshot snapshot,
    AppLocalizations l10n,
  ) {
    final pct = _contextUsagePercentage(snapshot.summary);
    final ownerDevice = _resolveOwnerDevice(snapshot);
    final ownerShort = _resolveOwnerShortLabel(snapshot, ownerDevice);

    // 1. Node handoff / route switching
    final switchingId = snapshot.switchingDeviceId;
    final routeTransitioning =
        snapshot.routeStatus != null &&
        snapshot.routeStatus!.activeNodeId.isNotEmpty &&
        snapshot.routeStatus!.activeNodeId != snapshot.routeStatus!.ownerNodeId;
    if (switchingId != null || routeTransitioning) {
      final targetId = switchingId ?? snapshot.routeStatus!.ownerNodeId;
      final targetDevice = _findDevice(snapshot.topology, targetId);
      final targetShort = targetDevice != null
          ? shortDeviceLabel(
              targetDevice,
              isLocal: targetDevice.deviceId == snapshot.topology?.currentDeviceId,
            )
          : '节点';
      return TopBarCapsulePresentation(
        key: 'routing-$targetId',
        iconKey: 'routing',
        iconKind: TopBarCapsuleIconKind.routingFiber,
        tone: TopBarCapsuleTone.active,
        label: '交接 → $targetShort',
        tooltip: '正在将任务路由交接至 $targetShort',
        progressFraction: 0.65,
        contextPercentage: pct,
        showShimmer: true,
      );
    }

    // 2. Recent route handoff completion pulse
    if (snapshot.recentSwitchedDeviceName case final switchedName?) {
      return TopBarCapsulePresentation(
        key: 'route-done-$switchedName',
        iconKey: 'completed',
        iconKind: TopBarCapsuleIconKind.completedBurst,
        tone: TopBarCapsuleTone.success,
        label: '已切换 · $switchedName',
        tooltip: '任务已交接至 $switchedName',
        contextPercentage: pct,
      );
    }

    // 3. Context summarization / compaction
    final isSummarizing =
        snapshot.compacting || snapshot.processingState.tag == 'Summarizing';
    if (isSummarizing) {
      final stageProgress = snapshot.processingState.tag == 'Summarizing'
          ? summaryStageProgress(snapshot.processingState.message)
          : 0.35;
      final stageLabel = _compactStageLabel(snapshot.processingState, l10n);
      return TopBarCapsulePresentation(
        key: 'compacting-${snapshot.processingState.message}',
        iconKey: 'compacting',
        iconKind: TopBarCapsuleIconKind.compacting,
        tone: TopBarCapsuleTone.active,
        label: stageLabel,
        tooltip: stageLabel,
        progressFraction: stageProgress.clamp(0.15, 0.95),
        contextPercentage: pct,
        showShimmer: true,
      );
    }

    // 4. Recent compaction completion pulse
    if (snapshot.recentCompactCompleted) {
      final saved = snapshot.recentCompactedSavedTokens;
      final label = saved != null && saved > 0
          ? '压缩完毕 · 省 ${formatCompactTokenCount(saved)}'
          : '压缩完毕';
      return TopBarCapsulePresentation(
        key: 'compact-done-${saved ?? 0}',
        iconKey: 'completed',
        iconKind: TopBarCapsuleIconKind.completedBurst,
        tone: TopBarCapsuleTone.success,
        label: label,
        tooltip: label,
        contextPercentage: pct,
      );
    }

    // 5. Route error or unreachable owner node
    if (snapshot.routeFailed || snapshot.routeStatus?.ownerReachable == false) {
      final nodeLabel = ownerShort ?? '执行节点';
      return TopBarCapsulePresentation(
        key: 'link-alert-$nodeLabel',
        iconKey: 'link-alert',
        iconKind: TopBarCapsuleIconKind.linkAlert,
        tone: TopBarCapsuleTone.error,
        label: '$nodeLabel · 离线',
        tooltip: '无法连接到执行节点 $nodeLabel',
        contextPercentage: pct,
      );
    }

    // 6. Active model / tool execution
    final processing = snapshot.processingState;
    if (_isActiveExecutionState(processing)) {
      final activity = _executionActivityLabel(processing);
      final prefix = ownerShort != null ? '$ownerShort · ' : '';
      return TopBarCapsulePresentation(
        key: 'exec-${processing.tag}-${processing.toolName}-$ownerShort',
        iconKey: 'executing',
        iconKind: TopBarCapsuleIconKind.executingCompute,
        tone: TopBarCapsuleTone.active,
        label: '$prefix$activity',
        tooltip: '$prefix$activity',
        contextPercentage: pct,
        showShimmer: true,
      );
    }

    // 7. Error state from chat processing
    if (processing.tag == 'Error') {
      final nodeLabel = ownerShort ?? '会话';
      return TopBarCapsulePresentation(
        key: 'exec-error',
        iconKey: 'link-alert',
        iconKind: TopBarCapsuleIconKind.linkAlert,
        tone: TopBarCapsuleTone.error,
        label: '$nodeLabel · 异常',
        tooltip: processing.message.trim().isNotEmpty
            ? processing.message.trim()
            : '执行发生异常',
        contextPercentage: pct,
      );
    }

    // 8. Idle telemetry ring + percentage + active node
    final String idleLabel;
    if (snapshot.loadingSummary) {
      idleLabel = ownerShort != null ? '... · $ownerShort' : '...';
    } else if (pct == null) {
      idleLabel = ownerShort ?? '';
    } else {
      final pctText = '${pct.toStringAsFixed(0)}%';
      idleLabel = ownerShort != null ? '$pctText · $ownerShort' : pctText;
    }
    final pctTooltip = snapshot.loadingSummary
        ? l10n.loading
        : pct == null
        ? '--'
        : '${pct.toStringAsFixed(0)}%';
    final tone = pct != null && pct >= 90
        ? TopBarCapsuleTone.error
        : pct != null && pct >= 80
        ? TopBarCapsuleTone.warning
        : TopBarCapsuleTone.idle;
    return TopBarCapsulePresentation(
      key: 'idle-$idleLabel',
      iconKey: 'ring-$pctTooltip-${tone.name}',
      iconKind: TopBarCapsuleIconKind.usageRing,
      tone: tone,
      label: idleLabel,
      tooltip: snapshot.summaryFailed
          ? l10n.contextStatsSummaryLoadFailed
          : '上下文占用 $pctTooltip${ownerShort != null ? ' · $ownerShort' : ''}',
      contextPercentage: pct,
    );
  }

  static double? _contextUsagePercentage(core_proxy.ChatInputMenuSummary? s) {
    if (s == null || s.maxContextTokens <= 0) return null;
    return s.currentWindowSize / s.maxContextTokens * 100;
  }

  static core_proxy.RuntimeDeviceSpaceDevice? _resolveOwnerDevice(
    TopBarActivitySnapshot snapshot,
  ) {
    final topology = snapshot.topology;
    if (topology == null) return null;
    final ownerId = snapshot.routeStatus?.ownerIsLocal == true
        ? topology.currentDeviceId
        : snapshot.routeStatus?.ownerNodeId;
    if (ownerId == null || ownerId.isEmpty) return null;
    return _findDevice(topology, ownerId);
  }

  static String? _resolveOwnerShortLabel(
    TopBarActivitySnapshot snapshot,
    core_proxy.RuntimeDeviceSpaceDevice? ownerDevice,
  ) {
    final topology = snapshot.topology;
    if (ownerDevice != null) {
      return shortDeviceLabel(
        ownerDevice,
        isLocal: ownerDevice.deviceId == topology?.currentDeviceId,
      );
    }
    if (snapshot.routeStatus?.ownerIsLocal == true) {
      return '本机';
    }
    return null;
  }

  static core_proxy.RuntimeDeviceSpaceDevice? _findDevice(
    core_proxy.RuntimeDeviceSpaceTopology? topology,
    String id,
  ) {
    if (topology == null) return null;
    for (final device in topology.devices) {
      if (device.deviceId == id) return device;
    }
    return null;
  }

  static String _compactStageLabel(
    core_proxy.InputProcessingState state,
    AppLocalizations l10n,
  ) {
    if (state.tag != 'Summarizing') return '正在压缩中…';
    final msg = inputProcessingStatusText(l10n, state);
    if (msg.isEmpty) return '正在压缩中…';
    return msg;
  }

  static bool _isActiveExecutionState(core_proxy.InputProcessingState state) {
    return switch (state.tag) {
      'Processing' ||
      'Connecting' ||
      'Receiving' ||
      'ExecutingTool' ||
      'ToolProgress' ||
      'ProcessingToolResult' ||
      'ExecutingPlan' => true,
      _ => false,
    };
  }

  static String _executionActivityLabel(core_proxy.InputProcessingState state) {
    return switch (state.tag) {
      'ExecutingTool' || 'ToolProgress' =>
        state.toolName.trim().isNotEmpty ? state.toolName.trim() : '执行工具',
      'ProcessingToolResult' =>
        state.toolName.trim().isNotEmpty ? '整理 ${state.toolName.trim()}' : '处理结果',
      'Connecting' => '连接模型',
      'Receiving' => '生成中',
      'ExecutingPlan' => '执行计划',
      'Processing' => '处理中',
      _ => '运行中',
    };
  }
}

/// Formats a concise node name for capsule and topology chips.
String shortDeviceLabel(
  core_proxy.RuntimeDeviceSpaceDevice device, {
  required bool isLocal,
}) {
  if (isLocal) return '本机';
  final trimmed = device.deviceName.trim();
  if (trimmed.isEmpty) return device.platform;
  final firstWord = trimmed.split(RegExp(r'\s+')).first;
  if (trimmed.length <= 12) return trimmed;
  return firstWord.length <= 12 ? firstWord : '${firstWord.substring(0, 11)}…';
}
