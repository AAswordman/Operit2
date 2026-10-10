// ignore_for_file: file_names

import '../../../../l10n/generated/app_localizations.dart';

/// Converts built-in identity names into localized labels.
String spaceIdentityDisplayName(String name, AppLocalizations l10n) {
  return switch (name) {
    'Administrator' => l10n.settingsRuntimeControlRoleAdministrator,
    'User' => l10n.settingsRuntimeControlRoleUser,
    'Relay' => l10n.settingsRuntimeControlRoleRelay,
    'Storage' => l10n.settingsRuntimeControlRoleStorage,
    'Runner' => l10n.settingsRuntimeControlRoleRunner,
    'Auditor' => l10n.settingsRuntimeControlRoleAuditor,
    _ => name,
  };
}

/// Converts one protocol capability into a readable label.
String spaceCapabilityLabel(String capability, AppLocalizations l10n) {
  return switch (capability) {
    '*' => l10n.settingsRuntimeControlCapabilityAll,
    'network.devices.view' => l10n.settingsRuntimeControlCapabilityViewDevices,
    'network.audit.read' => l10n.settingsRuntimeControlCapabilityAuditRead,
    'network.relay' => l10n.settingsRuntimeControlCapabilityNetworkRelay,
    'storage.provide' => l10n.settingsRuntimeControlCapabilityStorageProvide,
    'runtime.execute' => l10n.settingsRuntimeControlCapabilityRuntimeExecute,
    'network.user' => l10n.settingsRuntimeControlCapabilityNetworkUser,
    'chat.read' => l10n.settingsRuntimeControlCapabilityChatRead,
    'network.identity.manage' => l10n.settingsRuntimeControlManageIdentities,
    'network.identity.assign' => l10n.settingsRuntimeControlAssignIdentity,
    'network.approval' => l10n.settingsRuntimeControlCapabilityApproval,
    _ => capability.replaceAll('.', ' · '),
  };
}

/// Converts a generated capability collection into stable string values.
List<String> spaceCapabilityValues(Object? values) {
  if (values is Iterable<Object?>) {
    return values.map((value) => value.toString()).toList(growable: false);
  }
  throw StateError('network identity capabilities must be iterable');
}

/// One selectable identity capability and its readable label.
class SpaceCapabilityChoice {
  /// Creates one choice.
  const SpaceCapabilityChoice({required this.label, required this.value});

  final String label;
  final String value;
}

/// Returns the selectable identity capabilities exposed to administrators.
List<SpaceCapabilityChoice> spaceCapabilityChoices(AppLocalizations l10n) {
  return <SpaceCapabilityChoice>[
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityAll,
      value: '*',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityViewDevices,
      value: 'network.devices.view',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityAuditRead,
      value: 'network.audit.read',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityNetworkRelay,
      value: 'network.relay',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityStorageProvide,
      value: 'storage.provide',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityRuntimeExecute,
      value: 'runtime.execute',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityChatRead,
      value: 'chat.read',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlManageIdentities,
      value: 'network.identity.manage',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlAssignIdentity,
      value: 'network.identity.assign',
    ),
    SpaceCapabilityChoice(
      label: l10n.settingsRuntimeControlCapabilityApproval,
      value: 'network.approval',
    ),
  ];
}
