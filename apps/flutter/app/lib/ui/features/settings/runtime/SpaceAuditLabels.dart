// ignore_for_file: file_names

import '../../../../l10n/generated/app_localizations.dart';
import 'SpaceIdentityLabels.dart';

/// Separates a protocol audit summary into a readable action and its subject.
///
/// These patterns mirror NetworkControlStore::controlAuditSummary. Unknown
/// summaries are kept intact, so newer Core commands never silently disappear.
({String action, String target}) spaceAuditSummary(
  String summary,
  AppLocalizations l10n, {
  Iterable<String> identityNames = const <String>[],
}) {
  const bootstrap = 'Bootstrap control; administrator ';
  const define = 'Define role ';
  const assign = 'Set identity ';
  const reset = 'Reset identity on ';
  if (summary.startsWith(bootstrap)) {
    return (
      action: l10n.settingsRuntimeControlAuditBootstrap,
      target: summary.substring(bootstrap.length),
    );
  }
  if (summary.startsWith(define)) {
    return (
      action: l10n.settingsRuntimeControlAddRole,
      target: spaceIdentityDisplayName(summary.substring(define.length), l10n),
    );
  }
  if (summary.startsWith(assign)) {
    // Role and device names can contain " on "; prefer the known definition
    // over guessing where the backend's human-readable sentence splits.
    final names = identityNames.toList()
      ..sort((a, b) => b.length.compareTo(a.length));
    var separator = summary.indexOf(' on ', assign.length);
    for (final name in names) {
      if (summary.startsWith('$assign$name on ')) {
        separator = assign.length + name.length;
        break;
      }
    }
    if (separator >= 0) {
      return (
        action: l10n.settingsRuntimeControlAssignIdentity,
        target:
            '${summary.substring(separator + 4)} · ${spaceIdentityDisplayName(summary.substring(assign.length, separator), l10n)}',
      );
    }
  }
  const resetSuffix = ' to the default user';
  if (summary.startsWith(reset) && summary.endsWith(resetSuffix)) {
    return (
      action: l10n.settingsRuntimeControlClearIdentity,
      target: summary.substring(
        reset.length,
        summary.length - resetSuffix.length,
      ),
    );
  }
  for (final entry in <String, String>{
    'Admit device ': l10n.settingsRuntimeControlAuditAdmit,
    'Remove device ': l10n.settingsRuntimeControlRemoveDevice,
    'Disconnect device ': l10n.settingsRuntimeControlDisconnectDevice,
    'Update policy ': l10n.settingsRuntimeControlAuditPolicy,
  }.entries) {
    if (summary.startsWith(entry.key)) {
      return (action: entry.value, target: summary.substring(entry.key.length));
    }
  }
  if (summary.startsWith('Merge Space ')) {
    final separator = summary.indexOf('; admit ');
    if (separator >= 0) {
      return (
        action: l10n.settingsRuntimeControlAuditMerge,
        target: summary.substring(separator + '; admit '.length),
      );
    }
  }
  return (action: summary, target: '');
}
