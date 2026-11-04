// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';

import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../common/ThinkingLevelText.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import '../../../../../settings/model/ProviderLogo.dart';

class AgentModelSelectorPopup extends StatefulWidget {
  const AgentModelSelectorPopup({
    super.key,
    required this.viewModel,
    required this.onDismiss,
    required this.onModelChanged,
    this.onThinkingChanged,
  });

  final ChatViewModel viewModel;
  final VoidCallback onDismiss;
  final void Function(
    String modelId, {
    String? providerTypeId,
    String? providerName,
  })
  onModelChanged;

  /// Called with the chat model's thinking after the user sets its level.
  final ValueChanged<core_proxy.ModelThinkingSettings>? onThinkingChanged;

  /// Creates the mutable state for the standalone model selector popup.
  @override
  State<AgentModelSelectorPopup> createState() =>
      _AgentModelSelectorPopupState();
}

class _AgentModelSelectorPopupState extends State<AgentModelSelectorPopup> {
  Future<_AgentModelSelectorData>? _settingsFuture;
  String? _expandedProviderId;
  String? _expandedFamilyKey;

  /// The chat model's thinking as last set here, shown over the loaded data.
  core_proxy.ModelThinkingSettings? _thinking;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  /// Initializes model selector settings loading.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  /// Loads model, context, and thinking settings for the popup.
  Future<_AgentModelSelectorData> _loadSettings() {
    return _loadAgentModelSelectorData(_clients);
  }

  /// Applies a provider model as the chat model.
  Future<void> _selectModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    if (_isDisallowedChatModel(model.id)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('禁止使用autoglm作为对话主模型。对话模型和ui控制模型是分离的，请选择任意一个别的聪明的大模型。'),
        ),
      );
      return;
    }
    await _clients.preferencesFunctionalConfigManager.setModelForFunction(
      functionType: core_proxy.FunctionType.chat,
      providerId: provider.id,
      modelId: model.id,
    );
    widget.onModelChanged(
      model.id,
      providerTypeId: provider.providerTypeId,
      providerName: provider.name,
    );
    widget.onDismiss();
  }

  /// Sets the chat model's thinking level and shows the level it now uses.
  Future<void> _setThinkingLevel(
    _AgentModelSelectorData data,
    core_proxy.ThinkingLevel level,
  ) async {
    final settings = await _setChatThinkingLevel(
      context,
      _clients,
      data,
      level,
    );
    if (!mounted) {
      return;
    }
    setState(() {
      _thinking = settings;
      if (settings == null) {
        _settingsFuture = _loadSettings();
      }
    });
    if (settings != null) {
      widget.onThinkingChanged?.call(settings);
    }
  }

  /// Navigates to the model settings screen.
  void _openModelSettings() {
    widget.onDismiss();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.model),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  /// Builds the model selector popup card.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final popupContainerColor = colorScheme.surfaceContainer;
    return Material(
      color: Colors.transparent,
      child: Card(
        margin: EdgeInsets.zero,
        color: popupContainerColor,
        elevation: 4,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 300, maxHeight: 420),
          child: FutureBuilder<_AgentModelSelectorData>(
            future: _settingsFuture,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                Error.throwWithStackTrace(
                  snapshot.error!,
                  snapshot.stackTrace!,
                );
              }
              final data = snapshot.data;
              if (data == null) {
                return const SizedBox(
                  width: 300,
                  height: 96,
                  child: Center(child: CircularProgressIndicator()),
                );
              }
              return SingleChildScrollView(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    AgentThinkingLevelRow(
                      settings: _thinking ?? data.thinkingSettings,
                      onChanged: (level) => _setThinkingLevel(data, level),
                    ),
                    _ModelSelectorItem(
                      popupContainerColor: popupContainerColor,
                      providers: data.providers,
                      summaries: data.summaries,
                      currentBinding: data.currentBinding,
                      expanded: true,
                      expandedProviderId: _expandedProviderId,
                      expandedFamilyKey: _expandedFamilyKey,
                      onExpandedChanged: (_) {},
                      onExpandedProviderChanged: (providerId) {
                        setState(() {
                          _expandedProviderId = providerId;
                          if (providerId == null) {
                            _expandedFamilyKey = null;
                          }
                        });
                      },
                      onFamilyExpandedChanged: (familyKey) {
                        setState(() {
                          _expandedFamilyKey = familyKey;
                        });
                      },
                      onSelectModel: _selectModel,
                      onManageClick: _openModelSettings,
                      onInfoClick: () {},
                      showChevron: false,
                      showInfoButton: false,
                      manageInHeader: true,
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class AgentModelMenuSection extends StatefulWidget {
  const AgentModelMenuSection({
    super.key,
    required this.viewModel,
    required this.onDismiss,
  });

  final ChatViewModel viewModel;
  final VoidCallback onDismiss;

  /// Creates the mutable state for the embedded model menu section.
  @override
  State<AgentModelMenuSection> createState() => _AgentModelMenuSectionState();
}

class _AgentModelMenuSectionState extends State<AgentModelMenuSection> {
  Future<_AgentModelSelectorData>? _settingsFuture;
  String? _expandedProviderId;
  String? _expandedFamilyKey;
  bool _modelSectionExpanded = false;
  bool _modelDropdownExpanded = false;

  /// The chat model's thinking as last set here, shown over the loaded data.
  core_proxy.ModelThinkingSettings? _thinking;

  GeneratedCoreProxyClients get _clients => widget.viewModel.clients;

  /// Loads the current model selector data when the menu section mounts.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
  }

  /// Loads model selector state for the embedded input menu section.
  Future<_AgentModelSelectorData> _loadSettings() {
    return _loadAgentModelSelectorData(_clients);
  }

  /// Selects one configured model for chat from the embedded menu.
  Future<void> _selectModel(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  ) async {
    if (_isDisallowedChatModel(model.id)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('禁止使用autoglm作为对话主模型。对话模型和ui控制模型是分离的，请选择任意一个别的聪明的大模型。'),
        ),
      );
      return;
    }
    await _clients.preferencesFunctionalConfigManager.setModelForFunction(
      functionType: core_proxy.FunctionType.chat,
      providerId: provider.id,
      modelId: model.id,
    );
    widget.onDismiss();
  }

  /// Sets the chat model's thinking level from the embedded menu.
  Future<void> _setThinkingLevel(
    _AgentModelSelectorData data,
    core_proxy.ThinkingLevel level,
  ) async {
    final settings = await _setChatThinkingLevel(
      context,
      _clients,
      data,
      level,
    );
    if (!mounted) {
      return;
    }
    setState(() {
      _thinking = settings;
      if (settings == null) {
        _settingsFuture = _loadSettings();
      }
    });
  }

  /// Navigates to the model settings screen from the embedded menu.
  void _openModelSettings() {
    widget.onDismiss();
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.model),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  /// Builds the embedded model selector row and its inline list.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return FutureBuilder<_AgentModelSelectorData>(
      future: _settingsFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return _SettingsHeaderRow(
            icon: Icons.data_object_outlined,
            title: '模型',
            value: '加载中...',
            expanded: _modelSectionExpanded,
            onTap: () {
              setState(() {
                _modelSectionExpanded = !_modelSectionExpanded;
              });
            },
            onInfoClick: () {},
            showInfoButton: false,
          );
        }
        final embeddedPanelColor = colorScheme.surface.withValues(alpha: 0.42);
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _SettingsHeaderRow(
              icon: Icons.data_object_outlined,
              title: '模型',
              value: data.currentBinding.modelId,
              expanded: _modelSectionExpanded,
              onTap: () {
                setState(() {
                  _modelSectionExpanded = !_modelSectionExpanded;
                  if (_modelSectionExpanded == false) {
                    _modelDropdownExpanded = false;
                    _expandedProviderId = null;
                  }
                });
              },
              onInfoClick: () {},
              showInfoButton: false,
            ),
            if (_modelSectionExpanded)
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 4, 8, 6),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: ColoredBox(
                    color: embeddedPanelColor,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        _ModelSelectorItem(
                          popupContainerColor: embeddedPanelColor,
                          headerIcon: Icons.tune_outlined,
                          headerTitle: '选择模型',
                          providers: data.providers,
                          summaries: data.summaries,
                          currentBinding: data.currentBinding,
                          expanded: _modelDropdownExpanded,
                          expandedProviderId: _expandedProviderId,
                          expandedFamilyKey: _expandedFamilyKey,
                          onExpandedChanged: (expanded) {
                            setState(() {
                              _modelDropdownExpanded = expanded;
                            });
                          },
                          onExpandedProviderChanged: (providerId) {
                            setState(() {
                              _expandedProviderId = providerId;
                              if (providerId == null) {
                                _expandedFamilyKey = null;
                              }
                            });
                          },
                          onFamilyExpandedChanged: (familyKey) {
                            setState(() {
                              _expandedFamilyKey = familyKey;
                            });
                          },
                          onSelectModel: _selectModel,
                          onManageClick: _openModelSettings,
                          onInfoClick: () {},
                          showInfoButton: false,
                        ),
                        const SizedBox(height: 2),
                        AgentThinkingLevelRow(
                          settings: _thinking ?? data.thinkingSettings,
                          onChanged: (level) => _setThinkingLevel(data, level),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }
}

/// The thinking levels of one model on a slider, weakest first.
/// Exposed for widget tests; not meant to be reused elsewhere.
@visibleForTesting
class AgentThinkingSliderRow extends StatefulWidget {
  const AgentThinkingSliderRow({
    super.key,
    required this.stops,
    required this.stop,
    required this.offStop,
    required this.onChanged,
  });

  /// The level names, at least two; a single level is shown as a fixed note.
  final List<String> stops;
  final int stop;

  /// Whether the first stop turns thinking off.
  final bool offStop;
  final ValueChanged<int> onChanged;

  /// Test hooks for locating the track and thumb in widget tests.
  @visibleForTesting
  static const Key trackKey = ValueKey<String>('agent-thinking-track');
  @visibleForTesting
  static const Key thumbKey = ValueKey<String>('agent-thinking-thumb');

  /// Creates the mutable state for the merged thinking slider.
  @override
  State<AgentThinkingSliderRow> createState() => _AgentThinkingSliderRowState();
}

class _AgentThinkingSliderRowState extends State<AgentThinkingSliderRow> {
  static const Duration _morphDuration = Duration(milliseconds: 380);
  static const Curve _morphCurve = Curves.easeOutCubic;

  int _stop = 0;
  bool _dragging = false;

  /// Initializes the selected stop position.
  @override
  void initState() {
    super.initState();
    _stop = widget.stop;
  }

  /// Syncs the stop when the resolved thinking selection changes.
  @override
  void didUpdateWidget(covariant AgentThinkingSliderRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_dragging && oldWidget.stop != widget.stop) {
      _stop = widget.stop;
    }
  }

  /// Maps one track tap or drag position onto the nearest stop.
  void _applyPosition(double dx, double trackWidth) {
    final last = widget.stops.length - 1;
    // The 1.5px track border insets the positioned children, so the thumb
    // center travels between 12.5px (border + inset + half thumb) and the
    // mirrored right edge of the outer track width.
    final fraction = ((dx - 12.5) / (trackWidth - 25)).clamp(0.0, 1.0);
    final next = (fraction * last).round().clamp(0, last);
    if (next == _stop) {
      return;
    }
    setState(() {
      _stop = next;
    });
    widget.onChanged(next);
  }

  /// Builds the merged thinking toggle and quality slider row.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final l10n = AppLocalizations.of(context)!;
    final on = !(widget.offStop && _stop == 0);
    final last = widget.stops.length - 1;
    final fraction = last > 0 ? _stop / last : 1.0;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: SizedBox(
        height: 40,
        child: Row(
          children: <Widget>[
            Icon(
              Icons.psychology,
              size: 16,
              color: on
                  ? colorScheme.primary
                  : colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
            ),
            const SizedBox(width: 4),
            Text('${l10n.thinkingLevelLabel}:', style: textTheme.bodySmall),
            const SizedBox(width: 6),
            AnimatedDefaultTextStyle(
              duration: _morphDuration,
              style: textTheme.bodySmall!.copyWith(
                color: on ? colorScheme.primary : colorScheme.onSurfaceVariant,
                fontWeight: on ? FontWeight.bold : FontWeight.normal,
              ),
              child: SizedBox(
                width: 64,
                child: Text(
                  widget.stops[_stop],
                  textAlign: TextAlign.end,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
            const SizedBox(width: 6),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final trackWidth = constraints.maxWidth;
                  final travel = trackWidth - 25;
                  final thumbLeft = 3 + fraction * travel;
                  // The fill reaches past the thumb's trailing edge so the track
                  // reads fully covered, meeting the pill's inner end at the
                  // last stop (19 = 3 inset + 16 thumb; +3 closes the tail gap).
                  final fillWidth = 19 + fraction * (travel + 3);
                  return GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTapUp: (details) =>
                        _applyPosition(details.localPosition.dx, trackWidth),
                    onHorizontalDragDown: (details) {
                      _dragging = true;
                      _applyPosition(details.localPosition.dx, trackWidth);
                    },
                    onHorizontalDragUpdate: (details) =>
                        _applyPosition(details.localPosition.dx, trackWidth),
                    onHorizontalDragEnd: (_) => _dragging = false,
                    onHorizontalDragCancel: () => _dragging = false,
                    child: Center(
                      child: AnimatedContainer(
                        key: AgentThinkingSliderRow.trackKey,
                        duration: _morphDuration,
                        curve: _morphCurve,
                        height: 21,
                        width: double.infinity,
                        decoration: BoxDecoration(
                          color: colorScheme.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(999),
                          border: Border.all(
                            width: 1.5,
                            color: on
                                ? Colors.transparent
                                : colorScheme.outline,
                          ),
                        ),
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(999),
                          child: Stack(
                            clipBehavior: Clip.none,
                            children: <Widget>[
                              Positioned(
                                left: 0,
                                top: 0,
                                bottom: 0,
                                child: AnimatedContainer(
                                  duration: _dragging
                                      ? const Duration(milliseconds: 60)
                                      : const Duration(milliseconds: 300),
                                  width: on ? fillWidth : 0,
                                  decoration: BoxDecoration(
                                    borderRadius: BorderRadius.circular(999),
                                    gradient: LinearGradient(
                                      begin: Alignment.centerLeft,
                                      end: Alignment.centerRight,
                                      colors: <Color>[
                                        colorScheme.primary.withValues(
                                          alpha: 0.30,
                                        ),
                                        colorScheme.primary.withValues(
                                          alpha: 0.92,
                                        ),
                                      ],
                                    ),
                                  ),
                                ),
                              ),
                              // Closed-state layer: the highlight block sits
                              // on the left half like a switch. On open it
                              // first slides to the right, then dissolves
                              // while the gradient track morphs in.
                              Positioned.fill(
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 2,
                                    vertical: 3,
                                  ),
                                  child: AnimatedAlign(
                                    alignment: on
                                        ? Alignment.centerRight
                                        : Alignment.centerLeft,
                                    duration: const Duration(milliseconds: 260),
                                    curve: Curves.easeInOutCubic,
                                    child: AnimatedOpacity(
                                      duration: const Duration(
                                        milliseconds: 460,
                                      ),
                                      curve: Curves.easeInCubic,
                                      opacity: on ? 0.0 : 1.0,
                                      child: FractionallySizedBox(
                                        widthFactor: 0.5,
                                        child: Container(
                                          decoration: BoxDecoration(
                                            color: colorScheme.primary
                                                .withValues(alpha: 0.55),
                                            borderRadius: BorderRadius.circular(
                                              5,
                                            ),
                                          ),
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                              for (var j = 0; j < widget.stops.length; j++)
                                Positioned(
                                  left: 11 + (j / last) * travel - 2,
                                  // Centered within the 18px inner height
                                  // left after the 1.5px border inset.
                                  top: 7,
                                  child: AnimatedOpacity(
                                    duration: const Duration(milliseconds: 240),
                                    opacity: on ? 1 : 0,
                                    child: AnimatedContainer(
                                      duration: const Duration(
                                        milliseconds: 200,
                                      ),
                                      width: 4,
                                      height: 4,
                                      decoration: BoxDecoration(
                                        color: on && j <= _stop
                                            ? colorScheme.onSurfaceVariant
                                            : colorScheme.outlineVariant,
                                        shape: BoxShape.circle,
                                      ),
                                    ),
                                  ),
                                ),
                              // The knob stays hidden while closed; once the
                              // switch turns on it pops out in place with a
                              // springy scale-and-fade emergence.
                              Positioned(
                                key: AgentThinkingSliderRow.thumbKey,
                                left: thumbLeft,
                                top: 1,
                                child: AnimatedScale(
                                  scale: on ? 1.0 : 0.0,
                                  duration: _morphDuration,
                                  curve: _morphCurve,
                                  child: AnimatedOpacity(
                                    duration: const Duration(milliseconds: 240),
                                    opacity: on ? 1.0 : 0.0,
                                    child: Container(
                                      width: 16,
                                      height: 16,
                                      decoration: BoxDecoration(
                                        color: colorScheme.onSurfaceVariant,
                                        shape: BoxShape.circle,
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AgentModelSelectorData {
  const _AgentModelSelectorData({
    required this.providers,
    required this.summaries,
    required this.currentBinding,
    required this.thinkingSettings,
  });

  final List<core_proxy.ProviderProfile> providers;

  /// Settings each configured model runs with, catalog values included.
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding currentBinding;

  /// The chat model's thinking levels and the level it uses.
  final core_proxy.ModelThinkingSettings thinkingSettings;
}

/// The chat model's thinking control: its levels on a slider, a note when there is
/// nothing to move between, or nothing when requests cannot change how it thinks.
class AgentThinkingLevelRow extends StatelessWidget {
  const AgentThinkingLevelRow({
    super.key,
    required this.settings,
    required this.onChanged,
  });

  final core_proxy.ModelThinkingSettings settings;
  final ValueChanged<core_proxy.ThinkingLevel> onChanged;

  /// Builds the control that fits the levels the model offers.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final levels = settings.levels;
    if (levels.isEmpty) {
      return thinkingAlwaysOn(settings.spec)
          ? _ThinkingNoteRow(text: l10n.thinkingAlwaysOn)
          : const SizedBox.shrink();
    }
    if (levels.length == 1) {
      return _ThinkingNoteRow(text: _levelStopLabel(levels.single));
    }
    final index = levels.indexWhere(
      (option) => option.level == settings.current,
    );
    if (index < 0) {
      throw StateError(
        'The thinking level in use is absent from the offered levels',
      );
    }
    return AgentThinkingSliderRow(
      stops: <String>[for (final option in levels) _levelStopLabel(option)],
      stop: index,
      offStop: levels.first.level == core_proxy.ThinkingLevel.off,
      onChanged: (stop) => onChanged(levels[stop].level),
    );
  }
}

/// Names one level on the slider, with the tokens a budget level asks for.
String _levelStopLabel(core_proxy.ThinkingLevelOption option) {
  final name = thinkingLevelName(option.level);
  final budget = option.budgetTokens;
  return budget == null ? name : '$name ${thinkingBudgetText(budget)}';
}

/// A thinking row with a fixed value instead of a slider.
class _ThinkingNoteRow extends StatelessWidget {
  const _ThinkingNoteRow({required this.text});

  final String text;

  /// Builds the row in the slider row's place and spacing.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: SizedBox(
        height: 40,
        child: Row(
          children: <Widget>[
            Icon(Icons.psychology, size: 16, color: colorScheme.primary),
            const SizedBox(width: 4),
            Text('${l10n.thinkingLevelLabel}:', style: textTheme.bodySmall),
            const SizedBox(width: 6),
            Text(
              text,
              style: textTheme.bodySmall!.copyWith(
                color: colorScheme.primary,
                fontWeight: FontWeight.bold,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Sets the chat model's thinking level, `null` when it could not be set; the user
/// is told why.
Future<core_proxy.ModelThinkingSettings?> _setChatThinkingLevel(
  BuildContext context,
  GeneratedCoreProxyClients clients,
  _AgentModelSelectorData data,
  core_proxy.ThinkingLevel level,
) async {
  final messenger = ScaffoldMessenger.of(context);
  final failed = AppLocalizations.of(context)!.failed;
  try {
    return await clients.preferencesModelConfigManager.setThinkingLevelForModel(
      providerId: data.currentBinding.providerId,
      modelId: data.currentBinding.modelId,
      level: level,
    );
  } catch (error) {
    messenger.showSnackBar(SnackBar(content: Text('$failed: $error')));
    return null;
  }
}

class _ModelSelectorItem extends StatelessWidget {
  const _ModelSelectorItem({
    required this.popupContainerColor,
    this.headerIcon = Icons.data_object_outlined,
    this.headerTitle = '模型:',
    required this.providers,
    required this.summaries,
    required this.currentBinding,
    required this.expanded,
    required this.expandedProviderId,
    required this.expandedFamilyKey,
    required this.onExpandedChanged,
    required this.onExpandedProviderChanged,
    required this.onFamilyExpandedChanged,
    required this.onSelectModel,
    required this.onManageClick,
    required this.onInfoClick,
    this.showChevron = true,
    this.showInfoButton = true,
    this.manageInHeader = false,
  });

  final Color popupContainerColor;
  final IconData headerIcon;
  final String headerTitle;
  final List<core_proxy.ProviderProfile> providers;
  final List<core_proxy.ProviderModelSummary> summaries;
  final core_proxy.FunctionModelBinding currentBinding;
  final bool expanded;
  final String? expandedProviderId;
  final String? expandedFamilyKey;
  final ValueChanged<bool> onExpandedChanged;
  final ValueChanged<String?> onExpandedProviderChanged;
  final ValueChanged<String?> onFamilyExpandedChanged;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onSelectModel;
  final VoidCallback onManageClick;
  final VoidCallback onInfoClick;
  final bool showChevron;
  final bool showInfoButton;
  final bool manageInHeader;

  /// Builds the model selector header and inline provider list.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final currentProvider = providers
        .where((provider) => provider.id == currentBinding.providerId)
        .firstOrNull;
    final currentModel = currentProvider?.models
        .where((model) => model.id == currentBinding.modelId)
        .firstOrNull;
    return Column(
      children: <Widget>[
        if (manageInHeader && currentProvider != null && currentModel != null)
          _CurrentModelLine(
            provider: currentProvider,
            model: currentModel,
            onManageClick: onManageClick,
          )
        else
          _SettingsHeaderRow(
            icon: headerIcon,
            title: headerTitle,
            value: currentBinding.modelId,
            expanded: expanded,
            onTap: () => onExpandedChanged(!expanded),
            onInfoClick: onInfoClick,
            showChevron: showChevron,
            showInfoButton: showInfoButton,
            trailing: manageInHeader
                ? _ManageIconButton(onPressed: onManageClick)
                : null,
          ),
        if (expanded)
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 2, 8, 6),
            child: Column(
              children: <Widget>[
                if (providers.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 4),
                    child: Text(
                      '没有可用的模型',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                for (final provider in providers)
                  AgentModelProviderRow(
                    provider: provider,
                    summaries: summaries,
                    selected:
                        provider.id == currentBinding.providerId &&
                        provider.models.any(
                          (model) => model.id == currentBinding.modelId,
                        ),
                    selectedProviderId: currentBinding.providerId,
                    selectedModelId: currentBinding.modelId,
                    expanded: expandedProviderId == provider.id,
                    expandedFamilyKey: expandedFamilyKey,
                    onExpandedChanged: onExpandedProviderChanged,
                    onFamilyExpandedChanged: onFamilyExpandedChanged,
                    onSelectModel: onSelectModel,
                  ),
                if (!manageInHeader) ...[
                  const SizedBox(height: 4),
                  Align(
                    alignment: Alignment.center,
                    child: InkWell(
                      mouseCursor: SystemMouseCursors.click,
                      borderRadius: BorderRadius.circular(4),
                      hoverColor: colorScheme.primary.withValues(alpha: 0.08),
                      splashColor: colorScheme.primary.withValues(alpha: 0.12),
                      highlightColor: colorScheme.primary.withValues(
                        alpha: 0.06,
                      ),
                      onTap: onManageClick,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        child: Text(
                          '管理配置',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
      ],
    );
  }
}

/// One flat provider row in the directory tree and its family/model children.
class AgentModelProviderRow extends StatelessWidget {
  const AgentModelProviderRow({
    super.key,
    required this.provider,
    required this.summaries,
    required this.selected,
    required this.selectedProviderId,
    required this.selectedModelId,
    required this.expanded,
    required this.expandedFamilyKey,
    required this.onExpandedChanged,
    required this.onFamilyExpandedChanged,
    required this.onSelectModel,
  });

  final core_proxy.ProviderProfile provider;

  /// Settings each configured model runs with, catalog values included.
  final List<core_proxy.ProviderModelSummary> summaries;
  final bool selected;
  final String selectedProviderId;
  final String selectedModelId;
  final bool expanded;
  final String? expandedFamilyKey;
  final ValueChanged<String?> onExpandedChanged;
  final ValueChanged<String?> onFamilyExpandedChanged;
  final void Function(
    core_proxy.ProviderProfile provider,
    core_proxy.ModelProfile model,
  )
  onSelectModel;

  /// Builds one flat provider row plus its expandable family sections.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final models = provider.models;
    final hasMultipleModels = models.length > 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        InkWell(
          borderRadius: BorderRadius.circular(6),
          hoverColor: colorScheme.onSurface.withValues(alpha: 0.08),
          splashColor: colorScheme.onSurface.withValues(alpha: 0.05),
          highlightColor: colorScheme.onSurface.withValues(alpha: 0.06),
          onTap: () {
            if (hasMultipleModels) {
              onExpandedChanged(expanded ? null : provider.id);
            } else if (models.isNotEmpty) {
              onSelectModel(provider, models.first);
            } else {
              onExpandedChanged(expanded ? null : provider.id);
            }
          },
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
            decoration: selected
                ? BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.08),
                    borderRadius: BorderRadius.circular(6),
                  )
                : null,
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 22),
              child: Row(
                children: <Widget>[
                  ProviderLogo(
                    providerTypeId: provider.providerTypeId,
                    fallbackName: provider.name,
                    size: 16,
                  ),
                  const SizedBox(width: 7),
                  Expanded(
                    child: Text(
                      provider.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodySmall!.copyWith(
                        color: selected
                            ? colorScheme.primary
                            : colorScheme.onSurface,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 4),
                  if (hasMultipleModels) ...<Widget>[
                    Text(
                      '${models.length}',
                      style: textTheme.labelSmall!.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(width: 2),
                    Icon(
                      expanded
                          ? Icons.keyboard_arrow_up
                          : Icons.keyboard_arrow_down,
                      size: 14,
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ] else ...<Widget>[
                    Flexible(
                      child: Text(
                        models.isEmpty
                            ? provider.providerTypeId
                            : models.first.id,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.labelSmall!.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    if (selected) ...<Widget>[
                      const SizedBox(width: 4),
                      Icon(Icons.check, size: 12, color: colorScheme.primary),
                    ],
                  ],
                ],
              ),
            ),
          ),
        ),
        if (hasMultipleModels && expanded) ..._familySections(context),
      ],
    );
  }

  /// Returns the settings one of this provider's models runs with.
  core_proxy.ProviderModelSummary? _summaryFor(core_proxy.ModelProfile model) {
    return summaries
        .where(
          (summary) =>
              summary.providerId == provider.id && summary.modelId == model.id,
        )
        .firstOrNull;
  }

  /// Builds the family (or direct model) sections for the expanded provider.
  List<Widget> _familySections(BuildContext context) {
    final families = _modelFamilies(provider.models);
    if (families.length <= 1) {
      return <Widget>[
        Padding(
          padding: const EdgeInsets.only(left: 26),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              for (final model in provider.models)
                _ModelNameRow(
                  model: model,
                  summary: _summaryFor(model),
                  selected:
                      provider.id == selectedProviderId &&
                      model.id == selectedModelId,
                  onTap: () => onSelectModel(provider, model),
                ),
            ],
          ),
        ),
      ];
    }
    return <Widget>[
      for (final (label, familyModels) in families) ...<Widget>[
        Padding(
          padding: const EdgeInsets.only(left: 14),
          child: _ModelFamilyRow(
            label: label,
            count: familyModels.length,
            open: expandedFamilyKey == '${provider.id}||$label',
            onToggle: () => onFamilyExpandedChanged(
              expandedFamilyKey == '${provider.id}||$label'
                  ? null
                  : '${provider.id}||$label',
            ),
          ),
        ),
        if (expandedFamilyKey == '${provider.id}||$label')
          Padding(
            padding: const EdgeInsets.only(left: 26),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                for (final model in familyModels)
                  _ModelNameRow(
                    model: model,
                    summary: _summaryFor(model),
                    selected:
                        provider.id == selectedProviderId &&
                        model.id == selectedModelId,
                    onTap: () => onSelectModel(provider, model),
                  ),
              ],
            ),
          ),
      ],
    ];
  }
}

/// One collapsible model-family row inside an expanded provider.
class _ModelFamilyRow extends StatelessWidget {
  const _ModelFamilyRow({
    required this.label,
    required this.count,
    required this.open,
    required this.onToggle,
  });

  final String label;
  final int count;
  final bool open;
  final VoidCallback onToggle;

  /// Builds one indented family row with its model count and chevron.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return InkWell(
      borderRadius: BorderRadius.circular(5),
      hoverColor: colorScheme.onSurface.withValues(alpha: 0.08),
      splashColor: colorScheme.onSurface.withValues(alpha: 0.05),
      highlightColor: colorScheme.onSurface.withValues(alpha: 0.06),
      onTap: onToggle,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 20),
          child: Row(
            children: <Widget>[
              Text(
                label,
                style: textTheme.labelSmall!.copyWith(
                  color: colorScheme.onSurface,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const Spacer(),
              Text(
                '$count',
                style: textTheme.labelSmall!.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 2),
              Icon(
                open ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
                size: 12,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Derives a model family label from the model id prefix.
String? _modelFamilyOf(String modelId) {
  const rules = <(String, String)>[
    (r'^(gpt|o[134])', 'GPT'),
    (r'^claude', 'Claude'),
    (r'^gemini', 'Gemini'),
    (r'^deepseek', 'DeepSeek'),
    (r'^llama', 'Llama'),
    (r'^qwen', 'Qwen'),
    (r'^glm', 'GLM'),
    (r'^kimi', 'Kimi'),
    (r'^doubao', 'Doubao'),
    (r'^minimax', 'MiniMax'),
  ];
  final lower = modelId.toLowerCase();
  for (final (pattern, label) in rules) {
    if (RegExp(pattern).hasMatch(lower)) {
      return label;
    }
  }
  return null;
}

/// Groups models into insertion-ordered families derived from id prefixes.
List<(String, List<core_proxy.ModelProfile>)> _modelFamilies(
  List<core_proxy.ModelProfile> models,
) {
  final families = <String, List<core_proxy.ModelProfile>>{};
  for (final model in models) {
    final family = _modelFamilyOf(model.id) ?? '其他';
    families.putIfAbsent(family, () => <core_proxy.ModelProfile>[]).add(model);
  }
  return families.entries.map((entry) => (entry.key, entry.value)).toList();
}

/// The flat single-line header showing the active chat model identity.
class _CurrentModelLine extends StatelessWidget {
  const _CurrentModelLine({
    required this.provider,
    required this.model,
    required this.onManageClick,
  });

  final core_proxy.ProviderProfile provider;
  final core_proxy.ModelProfile model;
  final VoidCallback onManageClick;

  /// Builds the flat current-model header line with the manage button.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 30),
        child: Row(
          children: <Widget>[
            Text(
              '模型',
              style: textTheme.labelSmall!.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(width: 6),
            Expanded(
              child: Row(
                children: <Widget>[
                  Flexible(
                    flex: 3,
                    child: Text(
                      model.id,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodySmall!.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    '·',
                    style: textTheme.labelSmall!.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Flexible(
                    flex: 1,
                    child: Text(
                      provider.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall!.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 6),
            InkWell(
              borderRadius: BorderRadius.circular(999),
              hoverColor: colorScheme.primary.withValues(alpha: 0.18),
              splashColor: colorScheme.primary.withValues(alpha: 0.14),
              highlightColor: colorScheme.primary.withValues(alpha: 0.10),
              onTap: onManageClick,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
                decoration: BoxDecoration(
                  color: colorScheme.primary.withValues(alpha: 0.10),
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  '配置管理',
                  style: textTheme.labelSmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
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

class _ModelNameRow extends StatelessWidget {
  const _ModelNameRow({
    required this.model,
    required this.summary,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ModelProfile model;
  final core_proxy.ProviderModelSummary? summary;
  final bool selected;
  final VoidCallback onTap;

  /// Builds one flat selectable model row with metadata and hover only.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return InkWell(
      borderRadius: BorderRadius.circular(5),
      hoverColor: colorScheme.onSurface.withValues(alpha: 0.08),
      splashColor: colorScheme.onSurface.withValues(alpha: 0.05),
      highlightColor: colorScheme.onSurface.withValues(alpha: 0.06),
      onTap: onTap,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
        constraints: const BoxConstraints(minHeight: 20),
        decoration: BoxDecoration(
          color: selected
              ? colorScheme.primary.withValues(alpha: 0.12)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(5),
        ),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Text(
                model.id,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: textTheme.labelSmall!.copyWith(
                  color: selected ? colorScheme.primary : colorScheme.onSurface,
                  fontWeight: selected ? FontWeight.bold : FontWeight.normal,
                ),
              ),
            ),
            ..._modelMetaChips(context, summary),
            if (selected) ...<Widget>[
              const SizedBox(width: 4),
              Icon(Icons.check, size: 11, color: colorScheme.primary),
            ],
          ],
        ),
      ),
    );
  }
}

class _ModelMetaChip extends StatelessWidget {
  const _ModelMetaChip({required this.label, this.outlined = false});

  final String label;
  final bool outlined;

  /// Builds one compact model metadata chip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final decoration = outlined
        ? BoxDecoration(
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: colorScheme.outlineVariant),
          )
        : BoxDecoration(
            color: Colors.white.withValues(alpha: 0.045),
            borderRadius: BorderRadius.circular(4),
          );
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
      decoration: decoration,
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall!.copyWith(
          fontSize: 10,
          letterSpacing: 0.3,
          color: colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

/// Builds the metadata chips (capability tag and context length) for one model.
List<Widget> _modelMetaChips(
  BuildContext context,
  core_proxy.ProviderModelSummary? summary,
) {
  final contextLabel = _formatContextChip(summary?.context.maxContextLength);
  final chips = <Widget>[
    if (summary?.capabilities.directImage == true)
      const _ModelMetaChip(label: '视觉', outlined: true),
    if (summary?.thinking.supported == true)
      _ModelMetaChip(
        label: AppLocalizations.of(context)!.thinkingChip,
        outlined: true,
      ),
    if (contextLabel != null) _ModelMetaChip(label: contextLabel),
  ];
  if (chips.isEmpty) {
    return chips;
  }
  return chips
      .expand((chip) => <Widget>[const SizedBox(width: 4), chip])
      .toList();
}

/// Formats a context length token count as a compact chip label.
String? _formatContextChip(double? length) {
  if (length == null || length <= 0) {
    return null;
  }
  final k = length >= 10000 ? length / 1000.0 : length;
  if (k >= 950) {
    final millions = k / 1000.0;
    final millionsRounded = millions.round();
    if ((millions - millionsRounded).abs() < 0.08) {
      return '${millionsRounded}M';
    }
    return '${millions.toStringAsFixed(1)}M';
  }
  return '${k.round()}K';
}

class _SettingsHeaderRow extends StatelessWidget {
  const _SettingsHeaderRow({
    required this.icon,
    required this.title,
    required this.value,
    required this.expanded,
    required this.onTap,
    required this.onInfoClick,
    this.showChevron = true,
    this.showInfoButton = true,
    this.trailing,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final VoidCallback onInfoClick;
  final bool showChevron;
  final bool showInfoButton;
  final Widget? trailing;

  /// Builds a settings header row with a value and optional chevron.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return InkWell(
      onTap: onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(
                icon,
                size: 16,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
              if (showInfoButton) _InfoIconButton(onPressed: onInfoClick),
              const SizedBox(width: 12),
              Text(title, style: textTheme.bodySmall),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  value,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  textAlign: TextAlign.end,
                  style: textTheme.bodySmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              if (showChevron)
                Icon(
                  expanded
                      ? Icons.keyboard_arrow_up
                      : Icons.keyboard_arrow_down,
                  size: 20,
                  color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                ),
              ?trailing,
            ],
          ),
        ),
      ),
    );
  }
}

class _ManageIconButton extends StatelessWidget {
  const _ManageIconButton({required this.onPressed});

  final VoidCallback onPressed;

  /// Builds the compact manage-configurations header icon button.
  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 22,
      height: 22,
      child: IconButton(
        onPressed: onPressed,
        padding: EdgeInsets.zero,
        iconSize: 14,
        tooltip: '管理配置',
        visualDensity: VisualDensity.compact,
        icon: Icon(
          Icons.settings_outlined,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _InfoIconButton extends StatelessWidget {
  const _InfoIconButton({required this.onPressed});

  final VoidCallback onPressed;

  /// Builds the compact information icon button.
  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 24,
      height: 24,
      child: IconButton(
        onPressed: onPressed,
        padding: EdgeInsets.zero,
        iconSize: 16,
        icon: Icon(
          Icons.info_outline,
          color: Theme.of(
            context,
          ).colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
        ),
      ),
    );
  }
}

Future<_AgentModelSelectorData> _loadAgentModelSelectorData(
  GeneratedCoreProxyClients clients,
) async {
  final binding = await clients.preferencesFunctionalConfigManager
      .getModelBindingForFunction(functionType: core_proxy.FunctionType.chat);
  final thinkingSettings = await clients.preferencesModelConfigManager
      .getThinkingSettingsForModel(
        providerId: binding.providerId,
        modelId: binding.modelId,
      );
  return _AgentModelSelectorData(
    providers: await clients.preferencesModelConfigManager
        .getProviderProfiles(),
    summaries: await clients.preferencesModelConfigManager
        .getAllModelSummaries(),
    currentBinding: binding,
    thinkingSettings: thinkingSettings,
  );
}

/// Returns whether the model is reserved for UI control instead of chat.
bool _isDisallowedChatModel(String modelId) {
  return modelId.toLowerCase().contains('autoglm');
}
