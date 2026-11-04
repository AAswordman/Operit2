// ignore_for_file: file_names

import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../l10n/generated/app_localizations.dart';

/// Returns the name a thinking level is shown with: its API spelling, such as `low`
/// or `xhigh`, the same in every language.
String thinkingLevelName(core_proxy.ThinkingLevel level) => level.value;

/// Formats a reasoning-token budget compactly, in units of 1024 tokens.
String thinkingBudgetText(int tokens) {
  if (tokens < 1024) {
    return '$tokens';
  }
  final kilo = tokens / 1024;
  return kilo == kilo.roundToDouble()
      ? '${kilo.round()}k'
      : '${kilo.toStringAsFixed(1)}k';
}

/// Returns the note shown on the composer's model for its thinking, `null` when
/// it shows none: a model that is not thinking, or one that cannot.
String? thinkingBadgeText(
  AppLocalizations l10n,
  core_proxy.ModelThinkingSettings settings,
) {
  final current = settings.current;
  if (current == null) {
    return thinkingAlwaysOn(settings.spec) ? l10n.thinkingChip : null;
  }
  if (current == core_proxy.ThinkingLevel.off) {
    return null;
  }
  return current == core_proxy.ThinkingLevel.enabled
      ? l10n.thinkingChip
      : thinkingLevelName(current);
}

/// Returns whether a model reasons on every request and takes no control.
bool thinkingAlwaysOn(core_proxy.ModelThinkingSpec spec) =>
    spec.supported &&
    !spec.toggle &&
    spec.efforts.isEmpty &&
    spec.budget == null;
