// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class GlobalBehaviorSettingsPanel extends StatelessWidget {
  const GlobalBehaviorSettingsPanel({super.key});

  /// Builds global chat input behavior settings.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: <Widget>[
        _GlobalBehaviorSectionCard(
          title: l10n.settingsGlobalBehaviorChatInputSection,
          children: const <Widget>[_LongPastedTextBehaviorControl()],
        ),
      ],
    );
  }
}

class _GlobalBehaviorSectionCard extends StatelessWidget {
  const _GlobalBehaviorSectionCard({
    required this.title,
    required this.children,
  });

  final String title;
  final List<Widget> children;

  /// Builds a grouped settings card for this page.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 6),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _GlobalBehaviorSettingSwitch extends StatelessWidget {
  const _GlobalBehaviorSettingSwitch({
    required this.title,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final bool value;
  final ValueChanged<bool> onChanged;

  /// Builds a compact switch row for one global behavior preference.
  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      dense: true,
      visualDensity: VisualDensity.compact,
      title: Text(title),
      value: value,
      onChanged: onChanged,
    );
  }
}

class _LongPastedTextBehaviorControl extends StatefulWidget {
  const _LongPastedTextBehaviorControl();

  /// Creates the mutable state for the long-paste preference controls.
  @override
  State<_LongPastedTextBehaviorControl> createState() =>
      _LongPastedTextBehaviorControlState();
}

class _LongPastedTextBehaviorControlState
    extends State<_LongPastedTextBehaviorControl> {
  final UserPreferencesManager _preferences = const UserPreferencesManager();

  /// Starts loading persisted long-paste preferences for the settings controls.
  @override
  void initState() {
    super.initState();
    unawaited(_loadSettings());
  }

  /// Loads the persisted long-paste preferences into the shared settings value.
  Future<void> _loadSettings() async {
    try {
      await _preferences.loadLongPastedTextInputSettings();
    } catch (error, stackTrace) {
      debugPrint(
        'Unable to load long pasted text preferences: $error\n$stackTrace',
      );
    }
  }

  /// Persists one complete long-paste preference value.
  Future<void> _saveSettings({
    required bool enabled,
    required int threshold,
  }) async {
    try {
      await _preferences.saveLongPastedTextInputSettings(
        enabled: enabled,
        threshold: threshold,
      );
    } catch (error, stackTrace) {
      debugPrint(
        'Unable to save long pasted text preferences: $error\n$stackTrace',
      );
    }
  }

  /// Builds the switch and threshold slider for long pasted text conversion.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ValueListenableBuilder<LongPastedTextInputSettings>(
      valueListenable: UserPreferencesManager.longPastedTextInputSettings,
      builder: (context, settings, _) {
        return Column(
          children: <Widget>[
            _GlobalBehaviorSettingSwitch(
              title: l10n.settingsGlobalBehaviorLongPastedTextAsAttachment,
              value: settings.enabled,
              onChanged: (enabled) {
                UserPreferencesManager.longPastedTextInputSettings.value =
                    LongPastedTextInputSettings(
                      enabled: enabled,
                      threshold: settings.threshold,
                    );
                unawaited(
                  _saveSettings(
                    enabled: enabled,
                    threshold: settings.threshold,
                  ),
                );
              },
            ),
            _GlobalBehaviorValueSlider(
              label: l10n.settingsGlobalBehaviorLongPastedTextThreshold,
              value: settings.threshold.toDouble(),
              min: UserPreferencesManager.longPastedTextInputMinimumThreshold
                  .toDouble(),
              max: UserPreferencesManager.longPastedTextInputMaximumThreshold
                  .toDouble(),
              divisions:
                  (UserPreferencesManager.longPastedTextInputMaximumThreshold -
                      UserPreferencesManager
                          .longPastedTextInputMinimumThreshold) ~/
                  UserPreferencesManager.longPastedTextInputThresholdStep,
              valueText: l10n
                  .settingsGlobalBehaviorLongPastedTextThresholdValue(
                    settings.threshold,
                  ),
              onChanged: (value) {
                final threshold = value.round();
                UserPreferencesManager.longPastedTextInputSettings.value =
                    LongPastedTextInputSettings(
                      enabled: settings.enabled,
                      threshold: threshold,
                    );
              },
              onChangeEnd: (value) {
                final threshold = value.round();
                unawaited(
                  _saveSettings(
                    enabled: settings.enabled,
                    threshold: threshold,
                  ),
                );
              },
            ),
          ],
        );
      },
    );
  }
}

class _GlobalBehaviorValueSlider extends StatelessWidget {
  const _GlobalBehaviorValueSlider({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.divisions,
    required this.valueText,
    required this.onChanged,
    required this.onChangeEnd,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final int divisions;
  final String valueText;
  final ValueChanged<double> onChanged;
  final ValueChanged<double> onChangeEnd;

  /// Builds a labeled slider with its selected value.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(child: Text(label)),
              Text(
                valueText,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
          Slider(
            value: value.clamp(min, max).toDouble(),
            min: min,
            max: max,
            divisions: divisions,
            label: valueText,
            onChanged: onChanged,
            onChangeEnd: onChangeEnd,
          ),
        ],
      ),
    );
  }
}
