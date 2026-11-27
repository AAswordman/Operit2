// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../common/contributions/ChatInputMenuHost.dart';
import '../../../../../../common/contributions/ContributionPresentationResult.dart';
import '../../../../../../common/contributions/EmbeddedContributionDialog.dart';
import '../../../../../../common/icons/MaterialIconNameResolver.dart';
import '../../../../../../main/navigation/ToolPkgCatalogChangeBus.dart';
import '../../../../viewmodel/ChatSelectionTransition.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import '../../../workspace/context_stats/TopBarCapsuleModels.dart';

const String _kSvgSliders =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/></svg>';
const String _kSvgBlocks =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><rect width="7" height="7" x="14" y="3" rx="1"/><path d="M10 21V8a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-5a1 1 0 0 0-1-1H3"/></svg>';
const String _kSvgShieldCheck =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></svg>';
const String _kSvgEye =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/></svg>';
const String _kSvgFilePen =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M12.5 22H18a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v9.5"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M13.378 15.626a1 1 0 1 0-3.004-3.004l-5.01 5.012a2 2 0 0 0-.506.854l-.837 2.87a.5.5 0 0 0 .62.62l2.87-.837a2 2 0 0 0 .854-.506z"/></svg>';
const String _kSvgZap =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/></svg>';
const String _kSvgStream =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/></svg>';
const String _kSvgGauge =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/></svg>';
const String _kSvgChevronDown =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>';
const String _kSvgClipboardList =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><path d="M12 11h4"/><path d="M12 16h4"/><path d="M8 11h.01"/><path d="M8 16h.01"/></svg>';
const String _kSvgSmile =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" x2="9.01" y1="9" y2="9"/><line x1="15" x2="15.01" y1="9" y2="9"/></svg>';
const String _kSvgSparkles =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/><path d="M20 3v4"/><path d="M22 5h-4"/></svg>';
const String _kSvgTarget =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/></svg>';

const Map<String, String> _kKnownPluginIconSvgs = <String, String>{
  'assignment': _kSvgClipboardList,
  'assignment_outlined': _kSvgClipboardList,
  'checklist': _kSvgClipboardList,
  'fact_check': _kSvgClipboardList,
  'face': _kSvgSmile,
  'sentiment_satisfied': _kSvgSmile,
  'auto_awesome': _kSvgSparkles,
  'psychology': _kSvgSparkles,
  'track_changes': _kSvgTarget,
  'gps_fixed': _kSvgTarget,
  'tune': _kSvgSliders,
  'extension': _kSvgBlocks,
  'extension_outlined': _kSvgBlocks,
  'hub': _kSvgBlocks,
};

/// Shared SVG icon widget used by the composer tune button and input menu.
class InputMenuTuneSvgIcon extends StatelessWidget {
  const InputMenuTuneSvgIcon({super.key, this.size = 18, required this.color});

  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return _MenuSvgIcon(_kSvgSliders, size: size, color: color);
  }
}

/// Provides a coordinated entrance and exit animation for the input menu overlay.
class InputMenuPopupTransition extends StatefulWidget {
  const InputMenuPopupTransition({
    super.key,
    required this.alignment,
    required this.child,
  });

  final Alignment alignment;
  final Widget child;

  @override
  State<InputMenuPopupTransition> createState() =>
      InputMenuPopupTransitionState();
}

class InputMenuPopupTransitionState extends State<InputMenuPopupTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 240),
    reverseDuration: const Duration(milliseconds: 150),
  );
  late final CurvedAnimation _opacityCurve = CurvedAnimation(
    parent: _controller,
    curve: const Interval(0.0, 0.68, curve: Curves.easeOut),
    reverseCurve: Curves.easeIn,
  );
  late final CurvedAnimation _motionCurve = CurvedAnimation(
    parent: _controller,
    curve: const Cubic(0.18, 0.9, 0.24, 1.015),
    reverseCurve: Curves.easeInCubic,
  );
  bool _started = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.value = 1.0;
    } else {
      unawaited(_controller.forward());
    }
  }

  /// Plays the reverse exit animation before the overlay entry is removed.
  Future<void> dismiss() async {
    if (!mounted || MediaQuery.disableAnimationsOf(context)) {
      return;
    }
    try {
      await _controller.reverse().orCancel;
    } on TickerCanceled {
      // Ignored when a new popup interaction cancels dismissal.
    }
  }

  /// Restores the open state if the user toggles the popup mid-close.
  void cancelDismiss() {
    if (!mounted) return;
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.value = 1.0;
    } else {
      unawaited(_controller.forward());
    }
  }

  @override
  void dispose() {
    _opacityCurve.dispose();
    _motionCurve.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final boundaryChild = RepaintBoundary(child: widget.child);
    return FadeTransition(
      opacity: _opacityCurve,
      child: AnimatedBuilder(
        animation: _controller,
        child: boundaryChild,
        builder: (context, child) {
          final t = _motionCurve.value;
          final scale = 0.93 + 0.07 * t;
          final offsetY = 10.0 * (1.0 - t);
          return Transform.translate(
            offset: Offset(0, offsetY),
            child: Transform.scale(
              scale: scale,
              alignment: widget.alignment,
              child: child,
            ),
          );
        },
      ),
    );
  }
}

class _MenuSvgIcon extends StatelessWidget {
  const _MenuSvgIcon(this.svg, {this.size = 15, required this.color});

  final String svg;
  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return SvgPicture.string(
      svg,
      width: size,
      height: size,
      colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
    );
  }
}

class AgentInputMenuPopup extends StatefulWidget {
  /// Creates the shared menu with a lifecycle guard owned by its real composer.
  const AgentInputMenuPopup({
    super.key,
    required this.viewModel,
    required this.currentChatId,
    required this.onDismiss,
    required this.isChatContextCurrent,
    this.showNativeStatistics = true,
    this.leadingChildren = const <Widget>[],
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onDismiss;
  final bool Function() isChatContextCurrent;
  final bool showNativeStatistics;
  final List<Widget> leadingChildren;

  @override
  State<AgentInputMenuPopup> createState() => _AgentInputMenuPopupState();
}

class _AgentInputMenuPopupState extends State<AgentInputMenuPopup> {
  Future<_AgentInputMenuData>? _settingsFuture;
  Timer? _pluginChangeTimer;
  String? _settingsSignature;
  bool _checkingPluginChangeVersion = false;
  int _tabIndex = 0;
  int _mutationVersion = 0;
  int _permissionMutationVersion = 0;
  int _streamMutationVersion = 0;
  final Map<String, int> _pluginMutationVersions = <String, int>{};
  int _inFlightMutations = 0;
  _ToolPermissionMode? _optimisticPermissionMode;
  bool? _optimisticDisableStreamOutput;
  final Map<String, bool> _optimisticPluginStates = <String, bool>{};
  bool _summaryHasContent = false;
  bool _sectionsHasContent = false;

  /// Starts observers for the current chat menu settings.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
    _startPluginChangeObserver();
  }

  /// Keeps menu data scoped to the current chat and runtime client.
  @override
  void didUpdateWidget(covariant AgentInputMenuPopup oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _settingsSignature = null;
      _mutationVersion++;
      _permissionMutationVersion++;
      _streamMutationVersion++;
      _pluginMutationVersions.clear();
      _inFlightMutations = 0;
      _optimisticPermissionMode = null;
      _optimisticDisableStreamOutput = null;
      _optimisticPluginStates.clear();
      _settingsFuture = _loadSettings();
    }
  }

  /// Stops menu polling when the popup closes.
  @override
  void dispose() {
    _pluginChangeTimer?.cancel();
    super.dispose();
  }

  void _startPluginChangeObserver() {
    _pluginChangeTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      _checkPluginChangeVersion();
    });
  }

  /// Refreshes native settings and registered toggle metadata for this chat.
  Future<void> _checkPluginChangeVersion() async {
    if (_checkingPluginChangeVersion || _inFlightMutations > 0) {
      return;
    }
    _checkingPluginChangeVersion = true;
    final observedVersion = _mutationVersion;
    try {
      final chatId = widget.currentChatId;
      final viewModel = widget.viewModel;
      final settings = await viewModel.chatCore.chatInputMenuSettings(
        chatId: chatId,
      );
      if (!mounted ||
          widget.currentChatId != chatId ||
          widget.viewModel != viewModel ||
          _inFlightMutations > 0 ||
          observedVersion != _mutationVersion) {
        return;
      }
      final nextData = _menuData(settings);
      final signature = jsonEncode(nextData.toJson());
      if (_settingsSignature != signature) {
        _settingsSignature = signature;
        setState(() {
          _clearSyncedOptimisticState(nextData);
          _settingsFuture = Future.value(nextData);
        });
      }
    } catch (error) {
      debugPrint('Chat input menu refresh: $error');
    } finally {
      _checkingPluginChangeVersion = false;
    }
  }

  /// Reads native composer settings without consuming retired plugin-domain flags.
  Future<_AgentInputMenuData> _loadSettings() async {
    final settings = await widget.viewModel.chatCore.chatInputMenuSettings(
      chatId: widget.currentChatId,
    );
    final data = _menuData(settings);
    _settingsSignature = jsonEncode(data.toJson());
    return data;
  }

  void _clearSyncedOptimisticState(_AgentInputMenuData data) {
    if (_optimisticPermissionMode == data.toolPermissionMode) {
      _optimisticPermissionMode = null;
    }
    if (_optimisticDisableStreamOutput == data.disableStreamOutput) {
      _optimisticDisableStreamOutput = null;
    }
    for (final toggle in data.pluginToggles) {
      if (_optimisticPluginStates[toggle.id] == toggle.isChecked) {
        _optimisticPluginStates.remove(toggle.id);
      }
    }
  }

  /// Selects only host-owned behavior and actual registered plugin contributions.
  _AgentInputMenuData _menuData(core_proxy.ChatInputMenuSettings settings) {
    return _AgentInputMenuData(
      permissionMode: settings.permissionMode,
      disableStreamOutput: settings.disableStreamOutput,
      pluginToggles: settings.pluginToggles,
    );
  }

  /// Updates only native fields; null fields remain unchanged under the Core contract.
  Future<void> _saveSettings({
    core_proxy.AiPermissionMode? permissionMode,
    bool? disableStreamOutput,
  }) => widget.viewModel.chatCore.saveChatInputMenuSettings(
    chatId: widget.currentChatId,
    permissionMode: permissionMode,
    disableStreamOutput: disableStreamOutput,
    disableUserPreferenceDescription: null,
  );

  /// Skips refreshes when a pending menu action completes after dismissal.
  Future<void> _reloadSettings({int? expectedVersion}) async {
    if (!mounted) return;
    if (expectedVersion != null && expectedVersion != _mutationVersion) {
      return;
    }
    final chatId = widget.currentChatId;
    final viewModel = widget.viewModel;
    final reloadVersion = _mutationVersion;
    try {
      final data = await _loadSettings();
      if (!mounted ||
          widget.currentChatId != chatId ||
          widget.viewModel != viewModel ||
          reloadVersion != _mutationVersion) {
        return;
      }
      setState(() {
        _clearSyncedOptimisticState(data);
        _settingsFuture = Future.value(data);
      });
    } catch (error) {
      debugPrint('Chat input menu reload: $error');
    }
  }

  Future<void> _setPermissionMode(_ToolPermissionMode mode) async {
    final version = ++_mutationVersion;
    final permVersion = ++_permissionMutationVersion;
    _inFlightMutations++;
    setState(() {
      _optimisticPermissionMode = mode;
    });
    try {
      await _saveSettings(permissionMode: mode.permissionMode);
      unawaited(_reloadSettings(expectedVersion: version));
    } catch (error) {
      if (mounted && permVersion == _permissionMutationVersion) {
        setState(() {
          _optimisticPermissionMode = null;
        });
      }
      rethrow;
    } finally {
      _inFlightMutations--;
    }
  }

  Future<void> _toggleDisableStreamOutput(_AgentInputMenuData data) async {
    final current = _optimisticDisableStreamOutput ?? data.disableStreamOutput;
    final next = !current;
    final version = ++_mutationVersion;
    final streamVersion = ++_streamMutationVersion;
    _inFlightMutations++;
    setState(() {
      _optimisticDisableStreamOutput = next;
    });
    try {
      await _saveSettings(disableStreamOutput: next);
      unawaited(_reloadSettings(expectedVersion: version));
    } catch (error) {
      if (mounted && streamVersion == _streamMutationVersion) {
        setState(() {
          _optimisticDisableStreamOutput = null;
        });
      }
      rethrow;
    } finally {
      _inFlightMutations--;
    }
  }

  Future<void> _togglePlugin(
    core_proxy.InputMenuToggleDefinitionSnapshot toggle,
  ) async {
    final current = _optimisticPluginStates[toggle.id] ?? toggle.isChecked;
    final version = ++_mutationVersion;
    final pluginVersion = (_pluginMutationVersions[toggle.id] ?? 0) + 1;
    _pluginMutationVersions[toggle.id] = pluginVersion;
    _inFlightMutations++;
    setState(() {
      _optimisticPluginStates[toggle.id] = !current;
    });
    try {
      await widget.viewModel.chatCore.triggerChatInputMenuToggle(
        toggleId: toggle.id,
        chatId: widget.currentChatId,
      );
      unawaited(_reloadSettings(expectedVersion: version));
    } catch (error) {
      if (mounted && _pluginMutationVersions[toggle.id] == pluginVersion) {
        setState(() {
          _optimisticPluginStates.remove(toggle.id);
        });
      }
      rethrow;
    } finally {
      _inFlightMutations--;
    }
  }

  /// Presents a registered section action through the root overlay's lifetime.
  Future<void> _presentSectionAction(
    core_proxy.ToolPkgUiRoute route,
    Object? input,
  ) async {
    if (!widget.isChatContextCurrent()) return;
    final clients = widget.viewModel.clients;
    final chatId = widget.currentChatId;
    final isCurrent = widget.isChatContextCurrent;
    final presentationContext = Navigator.of(
      context,
      rootNavigator: true,
    ).overlay!.context;
    widget.onDismiss();
    if (!isCurrent() || !presentationContext.mounted) return;
    await showEmbeddedContributionDialog(
      route: route,
      input: input,
      context: presentationContext,
      clients: clients,
      hostState: <String, Object?>{'chatId': chatId},
    );
  }

  List<Widget> _effectiveLeadingChildren() {
    final children = List<Widget>.of(widget.leadingChildren);
    while (children.isNotEmpty && children.last is Divider) {
      children.removeLast();
    }
    return children;
  }

  /// Builds the Scheme 2 tabbed input menu popup with embedded secondary plugin sections.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final surfaceColor = isDark
        ? colorScheme.surfaceContainerHigh
        : colorScheme.surface;
    final borderColor = colorScheme.outlineVariant.withValues(
      alpha: isDark ? 0.72 : 0.85,
    );
    final leadingChildren = _effectiveLeadingChildren();
    final hasTopContext =
        _summaryHasContent ||
        leadingChildren.isNotEmpty ||
        widget.showNativeStatistics;

    return Material(
      color: Colors.transparent,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: surfaceColor,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: borderColor, width: 1),
          boxShadow: <BoxShadow>[
            BoxShadow(
              color: Colors.black.withValues(alpha: isDark ? 0.42 : 0.11),
              blurRadius: 18,
              offset: const Offset(0, 6),
            ),
            BoxShadow(
              color: Colors.black.withValues(alpha: isDark ? 0.20 : 0.04),
              blurRadius: 5,
              offset: const Offset(0, 2),
            ),
          ],
        ),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(11),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 280, maxHeight: 380),
            child: FutureBuilder<_AgentInputMenuData>(
              future: _settingsFuture,
              builder: (context, snapshot) {
                if (snapshot.hasError) {
                  return Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Text('菜单加载失败：${snapshot.error}'),
                        const SizedBox(height: 4),
                        TextButton(
                          onPressed: _reloadSettings,
                          child: const Text('重试'),
                        ),
                      ],
                    ),
                  );
                }
                final data = snapshot.data;
                if (data == null) {
                  return const SizedBox(
                    width: 280,
                    height: 92,
                    child: Center(
                      child: SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    ),
                  );
                }

                final effectiveMode =
                    _optimisticPermissionMode ?? data.toolPermissionMode;
                final effectiveDisableStream =
                    _optimisticDisableStreamOutput ?? data.disableStreamOutput;
                final enabledPluginCount = data.pluginToggles
                    .where(
                      (toggle) =>
                          _optimisticPluginStates[toggle.id] ??
                          toggle.isChecked,
                    )
                    .length;

                return SingleChildScrollView(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      _MeasureSize(
                        onChange: (size) {
                          final visible = size.height > 0.5;
                          if (visible != _summaryHasContent && mounted) {
                            setState(() {
                              _summaryHasContent = visible;
                            });
                          }
                        },
                        child: _ChatSessionSummarySection(
                          viewModel: widget.viewModel,
                          currentChatId: widget.currentChatId,
                          onDismiss: widget.onDismiss,
                          isChatContextCurrent: widget.isChatContextCurrent,
                          showNativeStatistics: widget.showNativeStatistics,
                        ),
                      ),
                      if (leadingChildren.isNotEmpty) ...<Widget>[
                        if (_summaryHasContent || widget.showNativeStatistics)
                          Divider(
                            height: 1,
                            color: colorScheme.outlineVariant.withValues(
                              alpha: 0.45,
                            ),
                          ),
                        ...leadingChildren,
                      ],
                      if (hasTopContext)
                        Divider(
                          height: 1,
                          color: colorScheme.outlineVariant.withValues(
                            alpha: 0.45,
                          ),
                        ),
                      Padding(
                        padding: const EdgeInsets.fromLTRB(8, 6, 8, 4),
                        child: _SegmentedTabBar(
                          activeIndex: _tabIndex,
                          pluginCount: data.pluginToggles.length,
                          enabledPluginCount: enabledPluginCount,
                          hasPluginSections: _sectionsHasContent,
                          onSelect: (index) {
                            if (_tabIndex == index) return;
                            setState(() {
                              _tabIndex = index;
                            });
                          },
                        ),
                      ),
                      _SegmentedTabContent(
                        activeIndex: _tabIndex,
                        firstChild: _ControlsTabPane(
                          permissionMode: effectiveMode,
                          disableStreamOutput: effectiveDisableStream,
                          onSelectPermission: _setPermissionMode,
                          onToggleStream: () =>
                              _toggleDisableStreamOutput(data),
                        ),
                        secondChild: _PluginsTabPane(
                          clients: widget.viewModel.clients,
                          chatId: widget.currentChatId,
                          toggles: data.pluginToggles,
                          optimisticStates: _optimisticPluginStates,
                          sectionsHasContent: _sectionsHasContent,
                          onTogglePlugin: _togglePlugin,
                          onSectionsSizeChanged: (size) {
                            final visible = size.height > 0.5;
                            if (visible != _sectionsHasContent && mounted) {
                              setState(() {
                                _sectionsHasContent = visible;
                              });
                            }
                          },
                          onPresentSectionAction: _presentSectionAction,
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}

class _SegmentedTabContent extends StatefulWidget {
  const _SegmentedTabContent({
    required this.activeIndex,
    required this.firstChild,
    required this.secondChild,
  });

  final int activeIndex;
  final Widget firstChild;
  final Widget secondChild;

  @override
  State<_SegmentedTabContent> createState() => _SegmentedTabContentState();
}

class _SegmentedTabContentState extends State<_SegmentedTabContent>
    with SingleTickerProviderStateMixin {
  final GlobalKey _firstKey = GlobalKey();
  final GlobalKey _secondKey = GlobalKey();
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 220),
    value: widget.activeIndex.toDouble(),
  );

  @override
  void didUpdateWidget(covariant _SegmentedTabContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.activeIndex != widget.activeIndex) {
      final target = widget.activeIndex.toDouble();
      if (MediaQuery.disableAnimationsOf(context)) {
        _controller.value = target;
      } else {
        unawaited(
          _controller.animateTo(target, curve: Curves.easeOutCubic),
        );
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final first = KeyedSubtree(key: _firstKey, child: widget.firstChild);
    final second = KeyedSubtree(key: _secondKey, child: widget.secondChild);
    return AnimatedSize(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      alignment: Alignment.topCenter,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, _) {
          final t = _controller.value.clamp(0.0, 1.0);
          if (t == 0.0 && widget.activeIndex == 0) {
            return Stack(
              children: <Widget>[
                first,
                Offstage(
                  offstage: true,
                  child: TickerMode(enabled: false, child: second),
                ),
              ],
            );
          }
          if (t == 1.0 && widget.activeIndex == 1) {
            return Stack(
              children: <Widget>[
                Offstage(
                  offstage: true,
                  child: TickerMode(enabled: false, child: first),
                ),
                second,
              ],
            );
          }

          final firstOpacity = (1.0 - t).clamp(0.0, 1.0);
          final secondOpacity = t.clamp(0.0, 1.0);
          final firstAnimated = Opacity(
            opacity: firstOpacity,
            child: Transform.translate(
              offset: Offset(-12.0 * t, 0),
              child: first,
            ),
          );
          final secondAnimated = Opacity(
            opacity: secondOpacity,
            child: Transform.translate(
              offset: Offset(12.0 * (1.0 - t), 0),
              child: second,
            ),
          );
          final showingFirstAsTarget = widget.activeIndex == 0;
          return Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              showingFirstAsTarget
                  ? firstAnimated
                  : _outgoingOverlayPane(firstAnimated),
              showingFirstAsTarget
                  ? _outgoingOverlayPane(secondAnimated)
                  : secondAnimated,
            ],
          );
        },
      ),
    );
  }

  Widget _outgoingOverlayPane(Widget child) {
    return Positioned.fill(
      child: IgnorePointer(
        child: OverflowBox(
          minHeight: 0,
          maxHeight: double.infinity,
          alignment: Alignment.topCenter,
          child: child,
        ),
      ),
    );
  }
}

class _SegmentedTabBar extends StatelessWidget {
  const _SegmentedTabBar({
    required this.activeIndex,
    required this.pluginCount,
    required this.enabledPluginCount,
    required this.hasPluginSections,
    required this.onSelect,
  });

  final int activeIndex;
  final int pluginCount;
  final int enabledPluginCount;
  final bool hasPluginSections;
  final ValueChanged<int> onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final isDark = theme.brightness == Brightness.dark;
    final trackColor = isDark
        ? colorScheme.surfaceContainerLowest.withValues(alpha: 0.92)
        : colorScheme.surfaceContainerLow;
    final activePillColor = isDark
        ? colorScheme.surfaceContainerHighest
        : colorScheme.surface;

    return _SlidingSegmentedTrack(
      height: 29,
      itemCount: 2,
      selectedIndex: activeIndex,
      trackColor: trackColor,
      trackBorderColor: colorScheme.outlineVariant.withValues(alpha: 0.48),
      pillColor: activePillColor,
      pillBorderColor: colorScheme.outlineVariant.withValues(alpha: 0.6),
      pillShadowColor: Colors.black.withValues(alpha: isDark ? 0.22 : 0.05),
      pillBlurRadius: 3,
      pillShadowOffset: const Offset(0, 1),
      onSelected: onSelect,
      itemBuilder: (context, index, activeProgress) {
        final color = Color.lerp(
          colorScheme.onSurfaceVariant,
          colorScheme.onSurface,
          activeProgress,
        )!;
        if (index == 0) {
          return _SegmentedTabLabel(
            svg: _kSvgSliders,
            label: '控制',
            color: color,
            textTheme: textTheme,
          );
        }
        return _SegmentedTabLabel(
          svg: _kSvgBlocks,
          label: '插件',
          color: color,
          textTheme: textTheme,
          trailing: _pluginBadge(
            colorScheme,
            textTheme,
            activeProgress: activeProgress,
          ),
        );
      },
    );
  }

  Widget _pluginBadge(
    ColorScheme colorScheme,
    TextTheme textTheme, {
    required double activeProgress,
  }) {
    if (pluginCount == 0 && hasPluginSections) {
      return Container(
        width: 5,
        height: 5,
        decoration: BoxDecoration(
          color: colorScheme.primary,
          shape: BoxShape.circle,
        ),
      );
    }
    final hasActive = enabledPluginCount > 0;
    final badgeText = pluginCount == 0
        ? '0'
        : '$enabledPluginCount/$pluginCount';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 4.5, vertical: 0.5),
      decoration: BoxDecoration(
        color: hasActive
            ? colorScheme.primary.withValues(alpha: 0.16)
            : colorScheme.onSurface.withValues(
                alpha: 0.05 + 0.03 * activeProgress,
              ),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        badgeText,
        style: textTheme.labelSmall?.copyWith(
          fontSize: 9.5,
          height: 1.15,
          fontWeight: FontWeight.w600,
          color: hasActive ? colorScheme.primary : colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _SegmentedTabLabel extends StatelessWidget {
  const _SegmentedTabLabel({
    required this.svg,
    required this.label,
    required this.color,
    required this.textTheme,
    this.trailing,
  });

  final String svg;
  final String label;
  final Color color;
  final TextTheme textTheme;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        _MenuSvgIcon(svg, size: 12, color: color),
        const SizedBox(width: 4),
        Text(
          label,
          style: textTheme.bodySmall!.copyWith(
            fontSize: 11.5,
            color: color,
            fontWeight: FontWeight.w600,
          ),
        ),
        if (trailing != null) ...<Widget>[const SizedBox(width: 4), trailing!],
      ],
    );
  }
}

class _ControlsTabPane extends StatelessWidget {
  const _ControlsTabPane({
    required this.permissionMode,
    required this.disableStreamOutput,
    required this.onSelectPermission,
    required this.onToggleStream,
  });

  final _ToolPermissionMode permissionMode;
  final bool disableStreamOutput;
  final ValueChanged<_ToolPermissionMode> onSelectPermission;
  final VoidCallback onToggleStream;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isStreaming = !disableStreamOutput;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 2, 10, 7),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Row(
                children: <Widget>[
                  _MenuSvgIcon(
                    _kSvgShieldCheck,
                    size: 12,
                    color: colorScheme.primary.withValues(alpha: 0.88),
                  ),
                  const SizedBox(width: 5),
                  Text(
                    '工具',
                    style: textTheme.labelSmall?.copyWith(
                      fontSize: 11,
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 180),
                      switchInCurve: Curves.easeOutCubic,
                      switchOutCurve: Curves.easeInCubic,
                      transitionBuilder: (child, animation) => FadeTransition(
                        opacity: animation,
                        child: SlideTransition(
                          position: Tween<Offset>(
                            begin: const Offset(0, 0.16),
                            end: Offset.zero,
                          ).animate(animation),
                          child: child,
                        ),
                      ),
                      child: Align(
                        key: ValueKey<_ToolPermissionMode>(permissionMode),
                        alignment: Alignment.centerRight,
                        child: Text(
                          permissionMode.hint,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.labelSmall?.copyWith(
                            fontSize: 10,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.78,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 5),
              _PermissionModeSelector(
                selectedMode: permissionMode,
                onSelected: onSelectPermission,
              ),
            ],
          ),
        ),
        Divider(
          height: 1,
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 6, 10, 1),
          child: Text(
            '行为',
            style: textTheme.labelSmall?.copyWith(
              fontSize: 11,
              color: colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(6, 1, 6, 1),
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            hoverColor: colorScheme.onSurface.withValues(alpha: 0.05),
            onTap: onToggleStream,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 5),
              child: Row(
                children: <Widget>[
                  AnimatedContainer(
                    duration: const Duration(milliseconds: 200),
                    curve: Curves.easeOutCubic,
                    width: 24,
                    height: 24,
                    decoration: BoxDecoration(
                      color: isStreaming
                          ? colorScheme.primary.withValues(alpha: 0.14)
                          : colorScheme.surfaceContainerLowest.withValues(
                              alpha: 0.75,
                            ),
                      borderRadius: BorderRadius.circular(7),
                    ),
                    child: Center(
                      child: _MenuSvgIcon(
                        _kSvgStream,
                        size: 13,
                        color: isStreaming
                            ? colorScheme.primary
                            : colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Row(
                          children: <Widget>[
                            Text(
                              '流式输出',
                              style: textTheme.bodySmall?.copyWith(
                                fontSize: 11.5,
                                color: colorScheme.onSurface,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const SizedBox(width: 5),
                            AnimatedSwitcher(
                              duration: const Duration(milliseconds: 160),
                              child: Container(
                                key: ValueKey<bool>(isStreaming),
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 4.5,
                                  vertical: 0.5,
                                ),
                                decoration: BoxDecoration(
                                  color: isStreaming
                                      ? colorScheme.primary.withValues(
                                          alpha: 0.14,
                                        )
                                      : colorScheme.onSurface.withValues(
                                          alpha: 0.06,
                                        ),
                                  borderRadius: BorderRadius.circular(999),
                                ),
                                child: Text(
                                  isStreaming ? '流式' : '非流式',
                                  style: textTheme.labelSmall?.copyWith(
                                    fontSize: 9.5,
                                    height: 1.1,
                                    color: isStreaming
                                        ? colorScheme.primary
                                        : colorScheme.onSurfaceVariant,
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 1),
                        AnimatedSwitcher(
                          duration: const Duration(milliseconds: 160),
                          child: Align(
                            key: ValueKey<bool>(isStreaming),
                            alignment: Alignment.centerLeft,
                            child: Text(
                              isStreaming ? '逐字实时输出响应' : '生成完成后整段显示',
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: textTheme.labelSmall?.copyWith(
                                fontSize: 10,
                                color: colorScheme.onSurfaceVariant.withValues(
                                  alpha: 0.8,
                                ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 6),
                  _CompactPillSwitch(
                    checked: isStreaming,
                    onTap: onToggleStream,
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _PluginsTabPane extends StatelessWidget {
  const _PluginsTabPane({
    required this.clients,
    required this.chatId,
    required this.toggles,
    required this.optimisticStates,
    required this.sectionsHasContent,
    required this.onTogglePlugin,
    required this.onSectionsSizeChanged,
    required this.onPresentSectionAction,
  });

  final GeneratedCoreProxyClients clients;
  final String? chatId;
  final List<core_proxy.InputMenuToggleDefinitionSnapshot> toggles;
  final Map<String, bool> optimisticStates;
  final bool sectionsHasContent;
  final ValueChanged<core_proxy.InputMenuToggleDefinitionSnapshot>
  onTogglePlugin;
  final ValueChanged<Size> onSectionsSizeChanged;
  final Future<void> Function(core_proxy.ToolPkgUiRoute route, Object? input)
  onPresentSectionAction;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final isDark = theme.brightness == Brightness.dark;
    final isCompletelyEmpty = toggles.isEmpty && !sectionsHasContent;

    return Padding(
      padding: const EdgeInsets.fromLTRB(6, 1, 6, 1),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (final toggle in toggles)
            _PluginToggleRow(
              toggle: toggle,
              checked: optimisticStates[toggle.id] ?? toggle.isChecked,
              onTap: () => onTogglePlugin(toggle),
            ),
          if (toggles.isNotEmpty && sectionsHasContent)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 3),
              child: Divider(
                height: 1,
                color: colorScheme.outlineVariant.withValues(alpha: 0.45),
              ),
            ),
          // Built-in character-card memory & plugin menu sections stay tucked
          // inside this secondary Plugins tab instead of the top level.
          AnimatedContainer(
            duration: const Duration(milliseconds: 180),
            curve: Curves.easeOutCubic,
            decoration: sectionsHasContent
                ? BoxDecoration(
                    color: isDark
                        ? colorScheme.surfaceContainerLowest.withValues(
                            alpha: 0.72,
                          )
                        : colorScheme.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: colorScheme.outlineVariant.withValues(alpha: 0.45),
                      width: 1,
                    ),
                  )
                : null,
            clipBehavior: sectionsHasContent ? Clip.antiAlias : Clip.none,
            child: Theme(
              data: theme.copyWith(
                switchTheme: SwitchThemeData(
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
              ),
              child: _MeasureSize(
                onChange: onSectionsSizeChanged,
                child: ChatInputMenuHost(
                  clients: clients,
                  chatId: chatId,
                  surface: 'chat_input_menu_sections',
                  onPresent: onPresentSectionAction,
                ),
              ),
            ),
          ),
          if (isCompletelyEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Container(
                    width: 30,
                    height: 30,
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceContainerLowest.withValues(
                        alpha: 0.85,
                      ),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Center(
                      child: _MenuSvgIcon(
                        _kSvgBlocks,
                        size: 15,
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.72,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    '暂无可用插件开关',
                    style: textTheme.bodySmall?.copyWith(
                      fontSize: 11.5,
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '启用快捷插件或绑定角色卡后将显示在此处',
                    textAlign: TextAlign.center,
                    style: textTheme.labelSmall?.copyWith(
                      fontSize: 10,
                      color: colorScheme.onSurfaceVariant.withValues(
                        alpha: 0.7,
                      ),
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class _PluginToggleRow extends StatelessWidget {
  const _PluginToggleRow({
    required this.toggle,
    required this.checked,
    required this.onTap,
  });

  final core_proxy.InputMenuToggleDefinitionSnapshot toggle;
  final bool checked;
  final VoidCallback onTap;

  Widget _buildLeadingIcon(Color iconColor) {
    final iconName = toggle.icon?.trim();
    if (iconName == null || iconName.isEmpty) {
      return _MenuSvgIcon(_kSvgBlocks, size: 13, color: iconColor);
    }
    final resolvedIcon = MaterialIconNameResolver.resolveOrNull(iconName);
    if (resolvedIcon == null) {
      throw ArgumentError.value(
        toggle.icon,
        'materialIconName',
        'Unknown Material icon name',
      );
    }
    final normalized = iconName
        .replaceAllMapped(
          RegExp(r'([A-Z])([A-Z][a-z])'),
          (m) => '${m[1]}_${m[2]}',
        )
        .replaceAllMapped(
          RegExp(r'([a-z0-9])([A-Z])'),
          (m) => '${m[1]}_${m[2]}',
        )
        .toLowerCase();
    final mappedSvg = _kKnownPluginIconSvgs[normalized];
    if (mappedSvg != null) {
      return _MenuSvgIcon(mappedSvg, size: 13, color: iconColor);
    }
    return Icon(resolvedIcon, size: 13.5, color: iconColor);
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final enabled = toggle.isEnabled;
    final description = toggle.description?.trim() ?? '';
    final iconColor = !enabled
        ? colorScheme.onSurfaceVariant.withValues(alpha: 0.45)
        : checked
        ? colorScheme.primary
        : colorScheme.onSurfaceVariant;

    return InkWell(
      borderRadius: BorderRadius.circular(8),
      hoverColor: colorScheme.onSurface.withValues(alpha: 0.05),
      onTap: enabled ? onTap : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 5),
        child: Row(
          children: <Widget>[
            AnimatedContainer(
              duration: const Duration(milliseconds: 200),
              curve: Curves.easeOutCubic,
              width: 24,
              height: 24,
              decoration: BoxDecoration(
                color: checked && enabled
                    ? colorScheme.primary.withValues(alpha: 0.14)
                    : colorScheme.surfaceContainerLowest.withValues(
                        alpha: enabled ? 0.75 : 0.4,
                      ),
                borderRadius: BorderRadius.circular(7),
              ),
              child: Center(child: _buildLeadingIcon(iconColor)),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    toggle.title ?? toggle.id,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall?.copyWith(
                      fontSize: 11.5,
                      color: enabled
                          ? colorScheme.onSurface
                          : colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.65,
                            ),
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  if (description.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 1),
                    Text(
                      description,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall?.copyWith(
                        fontSize: 10,
                        height: 1.2,
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: enabled ? 0.8 : 0.5,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(width: 6),
            _CompactPillSwitch(
              checked: checked,
              enabled: enabled,
              onTap: onTap,
            ),
          ],
        ),
      ),
    );
  }
}

class _CompactPillSwitch extends StatelessWidget {
  const _CompactPillSwitch({
    required this.checked,
    required this.onTap,
    this.enabled = true,
  });

  final bool checked;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final trackColor = !enabled
        ? colorScheme.onSurface.withValues(alpha: 0.12)
        : checked
        ? colorScheme.primary
        : colorScheme.surfaceContainerHighest;
    final thumbColor = !enabled
        ? colorScheme.onSurfaceVariant.withValues(alpha: 0.45)
        : checked
        ? colorScheme.onPrimary
        : colorScheme.onSurfaceVariant;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: enabled ? onTap : null,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOutCubic,
        width: 32,
        height: 18,
        padding: const EdgeInsets.all(1.5),
        decoration: BoxDecoration(
          color: trackColor,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: checked || !enabled
                ? Colors.transparent
                : colorScheme.outlineVariant.withValues(alpha: 0.85),
            width: 0.75,
          ),
        ),
        child: AnimatedAlign(
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOutCubic,
          alignment: checked ? Alignment.centerRight : Alignment.centerLeft,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            curve: Curves.easeOutCubic,
            width: checked ? 13 : 11.5,
            height: checked ? 13 : 11.5,
            decoration: BoxDecoration(
              color: thumbColor,
              shape: BoxShape.circle,
              boxShadow: <BoxShadow>[
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.2),
                  blurRadius: 2,
                  offset: const Offset(0, 1),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ChatSessionSummarySection extends StatefulWidget {
  /// Retains the real chat owner while the surrounding overlay is dismissed.
  const _ChatSessionSummarySection({
    required this.viewModel,
    required this.currentChatId,
    required this.onDismiss,
    required this.isChatContextCurrent,
    this.showNativeStatistics = true,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onDismiss;
  final bool Function() isChatContextCurrent;
  final bool showNativeStatistics;

  /// Creates the state for the current chat summary section.
  @override
  State<_ChatSessionSummarySection> createState() =>
      _ChatSessionSummarySectionState();
}

class _ChatSessionSummarySectionState
    extends State<_ChatSessionSummarySection> {
  Timer? _summaryTimer;
  bool _pollingSummary = false;
  Future<core_proxy.ChatInputMenuSummary>? _summaryFuture;
  int _currentWindowSize = 0;
  int _inputTokenCount = 0;
  int _outputTokenCount = 0;
  bool _statsExpanded = false;
  bool _presentingContextAction = false;

  /// Loads plugin actions and native statistics for this exact menu context.
  @override
  void initState() {
    super.initState();
    if (widget.showNativeStatistics) {
      _summaryFuture = _loadSummary();
      _summaryTimer = Timer.periodic(const Duration(seconds: 1), (_) async {
        if (_pollingSummary) return;
        _pollingSummary = true;
        try {
          final chatId = widget.currentChatId;
          final viewModel = widget.viewModel;
          final value = await _loadSummary();
          if (mounted &&
              widget.currentChatId == chatId &&
              widget.viewModel == viewModel) {
            setState(() {
              _summaryFuture = Future.value(value);
            });
          }
        } catch (error) {
          debugPrint('Chat menu summary refresh: $error');
        } finally {
          _pollingSummary = false;
        }
      });
    }
  }

  /// Replaces context futures when the actual chat or runtime owner changes.
  @override
  void didUpdateWidget(covariant _ChatSessionSummarySection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _currentWindowSize = 0;
      _inputTokenCount = 0;
      _outputTokenCount = 0;
      if (widget.showNativeStatistics) {
        _summaryFuture = _loadSummary();
      }
    }
  }

  /// Refreshes the captured runtime descriptor only after its own valid completion.
  Future<void> _presentChatUiAction(
    core_proxy.ToolPkgUiRoute route,
    Object? input,
  ) async {
    if (_presentingContextAction || !widget.isChatContextCurrent()) return;
    final viewModel = widget.viewModel;
    final clients = viewModel.clients;
    final chatCore = viewModel.chatCore;
    final chatId = widget.currentChatId;
    final ownerIsCurrent = widget.isChatContextCurrent;
    final dismissMenu = widget.onDismiss;
    final presentationContext = Navigator.of(
      context,
      rootNavigator: true,
    ).overlay!.context;
    final messenger = ScaffoldMessenger.of(context);
    final initialChatId = Completer<String?>();
    StreamSubscription<String?>? selectionSubscription;
    Object? selectionError;
    StackTrace? selectionStackTrace;
    var selectionChanged = ChatSelectionTransition.requests.value != null;
    setState(() => _presentingContextAction = true);

    /// Expires this invocation when any pending chat selection starts.
    void onSelectionTransition() {
      if (ChatSelectionTransition.requests.value != null) {
        selectionChanged = true;
      }
    }

    /// Validates the real composer, root host and every observed selection event.
    bool ownsCurrentContext() =>
        !selectionChanged && ownerIsCurrent() && presentationContext.mounted;

    /// Surfaces broken selection observation instead of publishing stale metadata.
    void requireSelectionObservation() {
      final error = selectionError;
      if (error != null) {
        Error.throwWithStackTrace(error, selectionStackTrace!);
      }
    }

    ChatSelectionTransition.requests.addListener(onSelectionTransition);
    try {
      selectionSubscription = viewModel.watchCurrentChatId().listen(
        (currentChatId) {
          if (currentChatId != chatId) selectionChanged = true;
          if (!initialChatId.isCompleted) {
            initialChatId.complete(currentChatId);
          }
        },
        onError: (Object error, StackTrace stackTrace) {
          selectionError = error;
          selectionStackTrace = stackTrace;
          if (!initialChatId.isCompleted) {
            initialChatId.completeError(error, stackTrace);
          }
        },
        onDone: () {
          final error = StateError('The chat selection observer closed.');
          selectionError = error;
          selectionStackTrace = StackTrace.current;
          if (!initialChatId.isCompleted) {
            initialChatId.completeError(error, selectionStackTrace);
          }
        },
      );
      dismissMenu();
      await initialChatId.future;
      requireSelectionObservation();
      if (!ownsCurrentContext() || !presentationContext.mounted) return;
      final result = await showEmbeddedContributionDialog(
        route: route,
        input: input,
        context: presentationContext,
        clients: clients,
        hostState: <String, Object?>{'chatId': chatId},
      );
      if (result?.status != ContributionPresentationStatus.completed ||
          !ownsCurrentContext()) {
        return;
      }
      requireSelectionObservation();
      final currentChatId = await chatCore.currentChatIdFlow().first;
      if (currentChatId != chatId || !ownsCurrentContext()) return;
      requireSelectionObservation();
      final value = result!.value;
      if (value is Map && value['type'] == 'toolpkg.chat.activate') {
        final targetChatId = value['chatId'];
        if (value.length != 2 ||
            targetChatId is! String ||
            targetChatId.trim().isEmpty) {
          throw const FormatException(
            'Chat activation requires exactly type and a nonblank chatId.',
          );
        }
        if (targetChatId != chatId) {
          await chatCore.switchChat(chatId: targetChatId);
        }
        return;
      }
      // A new-session active choice has no existing runtime descriptor to publish.
      if (chatId != null) {
        await chatCore.chatConfiguration(chatId: chatId);
      }
      final selectedAfterRefresh = await chatCore.currentChatIdFlow().first;
      if (selectedAfterRefresh != chatId || !ownsCurrentContext()) return;
      requireSelectionObservation();
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
    } catch (error) {
      if (ownsCurrentContext() && messenger.mounted) {
        messenger.showSnackBar(SnackBar(content: Text('插件操作失败：$error')));
      }
    } finally {
      ChatSelectionTransition.requests.removeListener(onSelectionTransition);
      await selectionSubscription?.cancel();
      if (mounted) setState(() => _presentingContextAction = false);
    }
  }

  /// Embeds registered plugin DSL content without interpreting its menu layout.
  Widget _buildContextActions() => ChatInputMenuHost(
    clients: widget.viewModel.clients,
    chatId: widget.currentChatId,
    onPresent: _presentChatUiAction,
  );

  /// Reads native token statistics without consulting any plugin domain records.
  Future<core_proxy.ChatInputMenuSummary> _loadSummary() async {
    final chatId = widget.currentChatId;
    final viewModel = widget.viewModel;
    final summary = await viewModel.chatCore.chatInputMenuSummary(
      chatId: chatId,
    );
    if (mounted &&
        widget.currentChatId == chatId &&
        widget.viewModel == viewModel) {
      setState(() {
        _currentWindowSize = summary.currentWindowSize;
        _inputTokenCount = summary.inputTokenCount;
        _outputTokenCount = summary.outputTokenCount;
      });
    }
    return summary;
  }

  /// Stops overlay observers without cancelling the already detached presentation.
  @override
  void dispose() {
    _summaryTimer?.cancel();
    super.dispose();
  }

  /// Builds registered chat context actions and native token statistics.
  @override
  Widget build(BuildContext context) {
    if (!widget.showNativeStatistics) {
      return _buildContextActions();
    }
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        _buildContextActions(),
        _buildNativeStatistics(colorScheme, textTheme),
      ],
    );
  }

  /// Reports native statistics failures separately from plugin-owned selectors.
  Widget _buildNativeStatistics(ColorScheme colorScheme, TextTheme textTheme) =>
      FutureBuilder<core_proxy.ChatInputMenuSummary>(
        future: _summaryFuture,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Padding(
              padding: const EdgeInsets.all(12),
              child: Text('统计加载失败：${snapshot.error}'),
            );
          }
          final summary = snapshot.data;
          final budget = summary == null ? null : resolveContextBudget(summary);
          final contextUsagePercentage = budget == null
              ? null
              : _contextUsagePercentage(budget);
          final cacheHitRate = summary?.cacheHitRate;
          final cachedInputTokenCount = summary?.cachedInputTokenCount;
          final totalTokenCount = _inputTokenCount + _outputTokenCount;
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              InkWell(
                onTap: () {
                  setState(() {
                    _statsExpanded = !_statsExpanded;
                  });
                },
                child: ConstrainedBox(
                  constraints: const BoxConstraints(minHeight: 34),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: Row(
                      children: <Widget>[
                        _MenuSvgIcon(
                          _kSvgGauge,
                          size: 14,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 8),
                        Text('统计', style: textTheme.bodySmall),
                        const Spacer(),
                        Text(
                          contextUsagePercentage == null
                              ? '加载中...'
                              : '${contextUsagePercentage.toStringAsFixed(0)}%',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(width: 5),
                        AnimatedRotation(
                          turns: _statsExpanded ? 0.5 : 0.0,
                          duration: const Duration(milliseconds: 180),
                          curve: Curves.easeOutCubic,
                          child: _MenuSvgIcon(
                            _kSvgChevronDown,
                            size: 14,
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              if (_statsExpanded)
                ColoredBox(
                  color: colorScheme.surface.withValues(alpha: 0.42),
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(38, 6, 12, 8),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        _ChatStatValueRow(
                          label: '上下文窗口',
                          value: _contextWindowLabel(
                            currentWindowSize: _currentWindowSize,
                            budget: budget,
                          ),
                        ),
                        _ChatStatValueRow(
                          label: '输入 Token',
                          value: _formatTokenCount(_inputTokenCount),
                        ),
                        _ChatStatValueRow(
                          label: '输出 Token',
                          value: _formatTokenCount(_outputTokenCount),
                        ),
                        if (cacheHitRate != null)
                          _ChatStatValueRow(
                            label: '缓存命中率',
                            value: cachedInputTokenCount != null
                                ? '${formatCacheHitPercentage(cacheHitRate)} (${_formatTokenCount(cachedInputTokenCount)})'
                                : formatCacheHitPercentage(cacheHitRate),
                          ),
                        _ChatStatValueRow(
                          label: '总 Token',
                          value: _formatTokenCount(totalTokenCount),
                          highlighted: true,
                        ),
                      ],
                    ),
                  ),
                ),
            ],
          );
        },
      );

  /// Calculates the current context usage percentage for the summary row.
  double _contextUsagePercentage(ContextBudgetSnapshot budget) {
    if (!budget.hasLimit) {
      return 0;
    }
    return (_currentWindowSize / budget.effectiveLimit * 100).clamp(0, 999);
  }

  /// Formats the context window values used in the expanded statistics.
  String _contextWindowLabel({
    required int currentWindowSize,
    required ContextBudgetSnapshot? budget,
  }) {
    if (budget == null) {
      return '加载中...';
    }
    final primary =
        '${_formatTokenCount(currentWindowSize)} / ${_formatTokenCount(budget.effectiveLimit)}';
    if (!budget.hasCompressionSupplement) {
      return primary;
    }
    return '$primary (最大 ${_formatTokenCount(budget.rawMaxTokens)})';
  }

  /// Formats token counts with compact thousands separators.
  String _formatTokenCount(int value) {
    return value.toString().replaceAllMapped(
      RegExp(r'(?<=\d)(?=(\d{3})+$)'),
      (match) => ',',
    );
  }
}

class _ChatStatValueRow extends StatelessWidget {
  const _ChatStatValueRow({
    required this.label,
    required this.value,
    this.highlighted = false,
  });

  final String label;
  final String value;
  final bool highlighted;

  /// Builds one value row in the expanded chat statistics section.
  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label, style: textTheme.labelSmall)),
          Text(
            value,
            style: textTheme.labelSmall?.copyWith(
              color: highlighted
                  ? colorScheme.primary
                  : colorScheme.onSurfaceVariant,
              fontWeight: highlighted ? FontWeight.w700 : FontWeight.normal,
            ),
          ),
        ],
      ),
    );
  }
}

class _AgentInputMenuData {
  /// Creates menu state without carrying plugin-owned preference flags.
  const _AgentInputMenuData({
    required this.permissionMode,
    required this.disableStreamOutput,
    required this.pluginToggles,
  });

  final core_proxy.AiPermissionMode permissionMode;
  final bool disableStreamOutput;
  final List<core_proxy.InputMenuToggleDefinitionSnapshot> pluginToggles;

  /// Encodes only native composer settings and registered plugin toggle metadata.
  Map<String, Object?> toJson() => <String, Object?>{
    'permissionMode': permissionMode.toJson(),
    'disableStreamOutput': disableStreamOutput,
    'pluginToggles': pluginToggles
        .map((toggle) => toggle.toJson())
        .toList(growable: false),
  };

  _ToolPermissionMode get toolPermissionMode {
    return switch (permissionMode) {
      core_proxy.AiPermissionMode.readOnly => _ToolPermissionMode.readOnly,
      core_proxy.AiPermissionMode.workspaceWrite =>
        _ToolPermissionMode.workspaceWrite,
      core_proxy.AiPermissionMode.full => _ToolPermissionMode.full,
    };
  }
}

enum _ToolPermissionMode {
  readOnly('只读', '仅查看文件与检索', _kSvgEye, core_proxy.AiPermissionMode.readOnly),
  workspaceWrite(
    '读写',
    '仅限当前工作区读写',
    _kSvgFilePen,
    core_proxy.AiPermissionMode.workspaceWrite,
  ),
  full('完整', '开放全部工具与终端', _kSvgZap, core_proxy.AiPermissionMode.full);

  const _ToolPermissionMode(
    this.label,
    this.hint,
    this.svgIcon,
    this.permissionMode,
  );

  final String label;
  final String hint;
  final String svgIcon;
  final core_proxy.AiPermissionMode permissionMode;
}

class _PermissionModeSelector extends StatefulWidget {
  const _PermissionModeSelector({
    required this.selectedMode,
    required this.onSelected,
  });

  final _ToolPermissionMode selectedMode;
  final ValueChanged<_ToolPermissionMode> onSelected;

  @override
  State<_PermissionModeSelector> createState() =>
      _PermissionModeSelectorState();
}

class _PermissionModeSelectorState extends State<_PermissionModeSelector>
    with SingleTickerProviderStateMixin {
  static final Animatable<double> _iconPulseSequence = TweenSequence<double>(
    <TweenSequenceItem<double>>[
      TweenSequenceItem<double>(
        tween: Tween<double>(
          begin: 1.0,
          end: 1.08,
        ).chain(CurveTween(curve: Curves.easeOutCubic)),
        weight: 42,
      ),
      TweenSequenceItem<double>(
        tween: Tween<double>(
          begin: 1.08,
          end: 1.0,
        ).chain(CurveTween(curve: Curves.easeInOut)),
        weight: 58,
      ),
    ],
  );

  late final AnimationController _pulseController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 220),
  );
  bool _suppressNextPulse = false;

  @override
  void didUpdateWidget(covariant _PermissionModeSelector oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selectedMode != widget.selectedMode) {
      if (_suppressNextPulse) {
        _suppressNextPulse = false;
      } else if (!MediaQuery.disableAnimationsOf(context)) {
        unawaited(_pulseController.forward(from: 0.0));
      }
    }
  }

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textTheme = theme.textTheme;
    final isDark = theme.brightness == Brightness.dark;
    final modes = _ToolPermissionMode.values;
    final trackColor = isDark
        ? colorScheme.surfaceContainerLowest.withValues(alpha: 0.92)
        : colorScheme.surfaceContainerLow;

    return _SlidingSegmentedTrack(
      height: 30,
      itemCount: modes.length,
      selectedIndex: widget.selectedMode.index,
      trackColor: trackColor,
      trackBorderColor: colorScheme.outlineVariant.withValues(alpha: 0.45),
      pillColor: colorScheme.primary.withValues(alpha: isDark ? 0.17 : 0.12),
      pillBorderColor: colorScheme.primary.withValues(alpha: 0.46),
      pillShadowColor: colorScheme.primary.withValues(alpha: 0.14),
      pillBlurRadius: 6,
      pillShadowOffset: const Offset(0, 1.5),
      onDragWillSelect: () {
        _suppressNextPulse = true;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          _suppressNextPulse = false;
        });
      },
      onSelected: (index) => widget.onSelected(modes[index]),
      itemBuilder: (context, index, activeProgress) {
        final mode = modes[index];
        final isSelected = index == widget.selectedMode.index;
        final color = Color.lerp(
          colorScheme.onSurfaceVariant,
          colorScheme.primary,
          activeProgress,
        )!;
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            AnimatedBuilder(
              animation: _pulseController,
              builder: (context, child) {
                final scale = isSelected
                    ? _iconPulseSequence.evaluate(_pulseController)
                    : 1.0;
                return Transform.scale(scale: scale, child: child);
              },
              child: _MenuSvgIcon(mode.svgIcon, size: 12, color: color),
            ),
            const SizedBox(width: 3.5),
            Text(
              mode.label,
              style: textTheme.bodySmall!.copyWith(
                fontSize: 11.5,
                color: color,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        );
      },
    );
  }
}

/// Physics-driven GPU-composited sliding pill shared by segmented selectors.
class _SlidingSegmentedTrack extends StatefulWidget {
  const _SlidingSegmentedTrack({
    required this.height,
    required this.itemCount,
    required this.selectedIndex,
    required this.trackColor,
    required this.trackBorderColor,
    required this.pillColor,
    required this.pillBorderColor,
    required this.pillShadowColor,
    required this.pillBlurRadius,
    required this.pillShadowOffset,
    required this.onSelected,
    required this.itemBuilder,
    this.onDragWillSelect,
  });

  final double height;
  final int itemCount;
  final int selectedIndex;
  final Color trackColor;
  final Color trackBorderColor;
  final Color pillColor;
  final Color pillBorderColor;
  final Color pillShadowColor;
  final double pillBlurRadius;
  final Offset pillShadowOffset;
  final ValueChanged<int> onSelected;
  final VoidCallback? onDragWillSelect;
  final Widget Function(
    BuildContext context,
    int index,
    double activeProgress,
  )
  itemBuilder;

  @override
  State<_SlidingSegmentedTrack> createState() => _SlidingSegmentedTrackState();
}

class _SlidingSegmentedTrackState extends State<_SlidingSegmentedTrack>
    with SingleTickerProviderStateMixin {
  static const SpringDescription _spring = SpringDescription(
    mass: 1.0,
    stiffness: 580.0,
    damping: 39.5,
  );
  static const Tolerance _tolerance = Tolerance(
    distance: 0.001,
    velocity: 0.005,
  );

  late final AnimationController _controller = AnimationController.unbounded(
    vsync: this,
    value: widget.selectedIndex.toDouble(),
  );
  late int _targetIndex = widget.selectedIndex;
  bool _dragging = false;

  @override
  void didUpdateWidget(covariant _SlidingSegmentedTrack oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.itemCount != widget.itemCount) {
      final maxIdx = widget.itemCount - 1;
      _targetIndex = _targetIndex.clamp(0, maxIdx);
      _controller.value = _controller.value.clamp(0.0, maxIdx.toDouble());
    }
    if (oldWidget.selectedIndex != widget.selectedIndex &&
        !_dragging &&
        _targetIndex != widget.selectedIndex) {
      _animateToIndex(widget.selectedIndex);
    }
  }

  void _animateToIndex(int index, {double? initialVelocity}) {
    _targetIndex = index;
    if (!mounted) return;
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.value = index.toDouble();
      return;
    }
    final velocity = (initialVelocity ?? _controller.velocity).clamp(
      -18.0,
      18.0,
    );
    final simulation = SpringSimulation(
      _spring,
      _controller.value,
      index.toDouble(),
      velocity,
      tolerance: _tolerance,
    );
    unawaited(_controller.animateWith(simulation));
  }

  void _handleDragStart() {
    _dragging = true;
    _controller.stop();
  }

  void _handleDragUpdate(DragUpdateDetails details, double slotWidth) {
    if (!_dragging || slotWidth <= 0) return;
    final delta = (details.primaryDelta ?? 0) / slotWidth;
    final maxIdx = (widget.itemCount - 1).toDouble();
    final current = _controller.value;
    final pushingPastLeft = current <= 0 && delta < 0;
    final pushingPastRight = current >= maxIdx && delta > 0;
    if (pushingPastLeft || pushingPastRight) {
      _controller.value = (current + delta * 0.25).clamp(
        -0.06,
        maxIdx + 0.06,
      );
    } else {
      _controller.value = (current + delta).clamp(-0.06, maxIdx + 0.06);
    }
  }

  void _handleDragEnd(DragEndDetails details, double slotWidth) {
    if (!_dragging) return;
    _dragging = false;
    final velocitySlots = slotWidth > 0
        ? (details.primaryVelocity ?? 0) / slotWidth
        : 0.0;
    final projected = (_controller.value + velocitySlots * 0.07)
        .round()
        .clamp(0, widget.itemCount - 1);
    var effectiveVelocity = velocitySlots;
    if (projected == 0 && effectiveVelocity < 0) {
      effectiveVelocity = effectiveVelocity.clamp(-6.5, 0.0);
    } else if (projected == widget.itemCount - 1 && effectiveVelocity > 0) {
      effectiveVelocity = effectiveVelocity.clamp(0.0, 6.5);
    }
    _animateToIndex(projected, initialVelocity: effectiveVelocity);
    if (projected != widget.selectedIndex) {
      widget.onDragWillSelect?.call();
      widget.onSelected(projected);
    }
  }

  void _handleDragCancel() {
    if (!_dragging) return;
    _dragging = false;
    _animateToIndex(widget.selectedIndex);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const innerPad = 2.5;
    const borderWidth = 1.0;
    final innerHeight = widget.height - innerPad * 2 - borderWidth * 2;
    final pillDecoration = BoxDecoration(
      color: widget.pillColor,
      borderRadius: BorderRadius.circular(6),
      border: Border.all(color: widget.pillBorderColor, width: 1),
      boxShadow: <BoxShadow>[
        BoxShadow(
          color: widget.pillShadowColor,
          blurRadius: widget.pillBlurRadius,
          offset: widget.pillShadowOffset,
        ),
      ],
    );

    return Container(
      height: widget.height,
      padding: const EdgeInsets.all(innerPad),
      decoration: BoxDecoration(
        color: widget.trackColor,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: widget.trackBorderColor, width: borderWidth),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final slotWidth = constraints.maxWidth / widget.itemCount;
          final staticPill = SizedBox(
            width: slotWidth,
            height: innerHeight,
            child: DecoratedBox(decoration: pillDecoration),
          );
          final maxIndex = (widget.itemCount - 1).toDouble();
          final maxX = maxIndex * slotWidth;
          const maxVisualOvershoot = 0.75;

          return RepaintBoundary(
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onHorizontalDragStart: (_) => _handleDragStart(),
              onHorizontalDragUpdate: (d) => _handleDragUpdate(d, slotWidth),
              onHorizontalDragEnd: (d) => _handleDragEnd(d, slotWidth),
              onHorizontalDragCancel: _handleDragCancel,
              child: AnimatedBuilder(
                animation: _controller,
                child: staticPill,
                builder: (context, pillChild) {
                  final pos = _controller.value;
                  final rawX = pos * slotWidth;
                  final clampedX = rawX.clamp(
                    -maxVisualOvershoot,
                    maxX + maxVisualOvershoot,
                  );
                  final squeeze = rawX < -maxVisualOvershoot
                      ? (-rawX - maxVisualOvershoot).clamp(0.0, 2.0)
                      : rawX > maxX + maxVisualOvershoot
                      ? (rawX - maxX - maxVisualOvershoot).clamp(0.0, 2.0)
                      : 0.0;
                  final scaleX = slotWidth > 0 && squeeze > 0
                      ? ((slotWidth - squeeze) / slotWidth).clamp(0.92, 1.0)
                      : 1.0;
                  final scaleAlignment = rawX > maxX + maxVisualOvershoot
                      ? Alignment.centerRight
                      : Alignment.centerLeft;

                  return Stack(
                    clipBehavior: Clip.none,
                    children: <Widget>[
                      Positioned(
                        left: 0,
                        top: 0,
                        bottom: 0,
                        width: slotWidth,
                        child: Transform.translate(
                          offset: Offset(clampedX, 0),
                          child: Transform(
                            alignment: scaleAlignment,
                            transform: Matrix4.diagonal3Values(
                              scaleX,
                              1.0,
                              1.0,
                            ),
                            child: pillChild!,
                          ),
                        ),
                      ),
                      Row(
                        children: <Widget>[
                          for (var index = 0; index < widget.itemCount; index++)
                            Expanded(
                              child: _SegmentTapScaler(
                                onTap: () {
                                  if (index != _targetIndex) {
                                    _animateToIndex(index);
                                  }
                                  if (index != widget.selectedIndex) {
                                    widget.onSelected(index);
                                  }
                                },
                                child: widget.itemBuilder(
                                  context,
                                  index,
                                  (1.0 - (pos - index).abs()).clamp(0.0, 1.0),
                                ),
                              ),
                            ),
                        ],
                      ),
                    ],
                  );
                },
              ),
            ),
          );
        },
      ),
    );
  }
}

class _SegmentTapScaler extends StatefulWidget {
  const _SegmentTapScaler({required this.onTap, required this.child});

  final VoidCallback onTap;
  final Widget child;

  @override
  State<_SegmentTapScaler> createState() => _SegmentTapScalerState();
}

class _SegmentTapScalerState extends State<_SegmentTapScaler> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _pressed = true),
        onTapUp: (_) {
          if (_pressed) setState(() => _pressed = false);
        },
        onTapCancel: () {
          if (_pressed) setState(() => _pressed = false);
        },
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _pressed ? 0.95 : 1.0,
          duration: const Duration(milliseconds: 85),
          curve: Curves.easeOut,
          child: Center(child: widget.child),
        ),
      ),
    );
  }
}

class _MeasureSize extends StatefulWidget {
  const _MeasureSize({required this.onChange, required this.child});

  final ValueChanged<Size> onChange;
  final Widget child;

  @override
  State<_MeasureSize> createState() => _MeasureSizeState();
}

class _MeasureSizeState extends State<_MeasureSize> {
  Size? _lastNotified;

  void _reportSize(Size size) {
    if (_lastNotified == size) return;
    _lastNotified = size;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        widget.onChange(size);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return _MeasureSizeProxy(onLayout: _reportSize, child: widget.child);
  }
}

class _MeasureSizeProxy extends SingleChildRenderObjectWidget {
  const _MeasureSizeProxy({required this.onLayout, required super.child});

  final ValueChanged<Size> onLayout;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _MeasureSizeRenderObject(onLayout);
  }

  @override
  void updateRenderObject(
    BuildContext context,
    covariant _MeasureSizeRenderObject renderObject,
  ) {
    renderObject.onLayout = onLayout;
  }
}

class _MeasureSizeRenderObject extends RenderProxyBox {
  _MeasureSizeRenderObject(this.onLayout);

  ValueChanged<Size> onLayout;
  Size? _previousSize;

  @override
  void performLayout() {
    super.performLayout();
    if (child == null) return;
    final newSize = size;
    if (_previousSize != newSize) {
      _previousSize = newSize;
      onLayout(newSize);
    }
  }
}
